use log::{debug, info};
use crate::assembler::{LeafAsmObject, RelocationType, SymbolEntry};

pub fn link(objects: &[LeafAsmObject], entry_point: &str) -> Result<LeafAsmObject, String> {
  let mut final_bytecode = vec![];
  let mut final_data = vec![];
  let mut final_rodata = vec![];
  let mut symbol_table = vec![];

  let mut text_bases = Vec::new();
  let mut data_bases = Vec::new();
  let mut rodata_bases = Vec::new();

  let mut text_offset = 0u32;
  let mut data_offset = 0u32;
  let mut rodata_offset = 0u32;

  for object in objects {
    text_bases.push(text_offset);
    data_bases.push(data_offset);
    rodata_bases.push(rodata_offset);

    text_offset += object.bytecode.len() as u32;
    data_offset += object.data.len() as u32;
    rodata_offset += object.rodata.len() as u32;
  }

  for object in objects {
    final_bytecode.extend(&object.bytecode);
    final_data.extend(&object.data);
    final_rodata.extend(&object.rodata);
  }

  for (index, object) in objects.iter().enumerate() {
    let text_base = text_bases[index];
    let data_base = data_bases[index];
    let rodata_base = rodata_bases[index];

    for symbol in &object.symbols {
      let adjusted_offset = match symbol.section {
        0 => symbol.offset + text_base,
        1 => symbol.offset + data_base,
        2 => symbol.offset + rodata_base,
        _ => symbol.offset,
      };
      symbol_table.push(SymbolEntry {
        name: symbol.name.clone(),
        offset: adjusted_offset,
        section: symbol.section,
        kind: symbol.kind,
        external: symbol.external,
      });
    }
  }

  // apply relocations
  for (index, object) in objects.iter().enumerate() {
    let text_base = text_bases[index];

    for reloc in &object.relocations {
      let symbol = &object.symbols[reloc.symbol_index as usize];
      // find symbol in the global symbol table
      let resolved = symbol_table.iter().find(|s| s.name == symbol.name && !s.external);
      let resolved_offset = match resolved {
        Some(s) => s.offset,
        None => return Err(format!("Unresolved symbol: {}", symbol.name))
      };

      let patch_offset = (text_base + reloc.offset) as usize;

      if patch_offset + 4 > final_bytecode.len() {
        return Err(format!(
          "Relocation offset {} out of bounds (bytecode size: {})",
          patch_offset,
          final_bytecode.len()
        ));
      }

      match reloc.reloc_type {
        RelocationType::Absolute => {
          info!("Patching absolute relocation at offset {} for symbol {} with resolved offset {}",
                patch_offset, symbol.name, resolved_offset);
          final_bytecode[patch_offset..patch_offset + 4]
            .copy_from_slice(&resolved_offset.to_le_bytes());
        }
        RelocationType::Relative => {
          let rel = (resolved_offset as i32) - (patch_offset as i32 + 4);
          info!("Patching relative relocation at offset {} for symbol {} with relative value {}",
                patch_offset, symbol.name, rel);
          final_bytecode[patch_offset..patch_offset + 4]
            .copy_from_slice(&(rel as u32).to_le_bytes());
        }
      }
    }
  }

  let entry_offset = symbol_table.iter()
    .find(|s| s.name == entry_point && !s.external)
    .map(|s| s.offset);

  info!("Entry point: {} with offset: {}", entry_point, entry_offset.unwrap_or(0));

  Ok(LeafAsmObject {
    bytecode: final_bytecode,
    data: final_data,
    rodata: final_rodata,
    symbols: symbol_table,
    entry_point: Some(entry_point.to_string()),
    relocations: vec![], // No relocations in the final object
    debug_info: None, // No debug info in the final object
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::assembler::{RelocationType, RelocationEntry, SymbolEntry, LeafAsmObject};

  fn mock_obj(
    bytecode: Vec<u8>,
    data: Vec<u8>,
    rodata: Vec<u8>,
    symbols: Vec<SymbolEntry>,
    relocations: Vec<RelocationEntry>,
  ) -> LeafAsmObject {
    LeafAsmObject {
      bytecode,
      data,
      rodata,
      symbols,
      entry_point: None,
      relocations,
      debug_info: None,
    }
  }

  #[test]
  fn test_link_single_object_no_relocations() {
    // .text = [NOP, NOP]
    let symbols = vec![SymbolEntry {
      name: "main".to_string(),
      offset: 0,
      section: 0,
      kind: 0,
      external: false,
    }];
    let obj = mock_obj(vec![0x90, 0x90], vec![], vec![], symbols.clone(), vec![]);

    let linked = link(&[obj], "main").expect("Should link");
    assert_eq!(linked.bytecode, vec![0x90, 0x90]);
    assert!(linked.data.is_empty());
    assert!(linked.rodata.is_empty());
    assert_eq!(linked.symbols, symbols);
    assert_eq!(linked.entry_point, Some("main".to_string()));
  }

  #[test]
  fn test_link_two_objects_merge_text() {
    let symbols1 = vec![SymbolEntry {
      name: "main".to_string(),
      offset: 0,
      section: 0,
      kind: 0,
      external: false,
    }];
    let symbols2 = vec![SymbolEntry {
      name: "func".to_string(),
      offset: 0,
      section: 0,
      kind: 0,
      external: false,
    }];
    let obj1 = mock_obj(vec![0xAA, 0xBB], vec![], vec![], symbols1, vec![]);
    let obj2 = mock_obj(vec![0xCC], vec![], vec![], symbols2, vec![]);

    let linked = link(&[obj1, obj2], "main").expect("Should link");
    assert_eq!(linked.bytecode, vec![0xAA, 0xBB, 0xCC]);
    // main at 0, func at 2
    assert!(linked.symbols.iter().any(|s| s.name == "main" && s.offset == 0));
    assert!(linked.symbols.iter().any(|s| s.name == "func" && s.offset == 2));
  }

  #[test]
  fn test_link_absolute_relocation() {
    // obj1: references 'func' (external, in obj2)
    // At offset 1 in obj1, needs patching to func's address in final image
    let mut symbols1 = vec![
      SymbolEntry { name: "main".to_string(), offset: 0, section: 0, kind: 0, external: false },
      SymbolEntry { name: "func".to_string(), offset: 0, section: 0, kind: 0, external: true }
    ];
    let mut reloc1 = vec![
      RelocationEntry { offset: 1, symbol_index: 1, reloc_type: RelocationType::Absolute }
    ];
    // .text = [CALL, 0, 0, 0, 0] (CALL opcode, then placeholder for address)
    let obj1 = mock_obj(vec![0x01, 0x00, 0x00, 0x00, 0x00], vec![], vec![], symbols1, reloc1);

    // obj2: defines 'func'
    let symbols2 = vec![
      SymbolEntry { name: "func".to_string(), offset: 0, section: 0, kind: 0, external: false }
    ];
    let obj2 = mock_obj(vec![0xFE, 0xED], vec![], vec![], symbols2, vec![]);

    let linked = link(&[obj1, obj2], "main").expect("Should link");
    // func is at offset 5 in final code ([0x01, address(4B), 0xFE, 0xED])
    let func_offset = 5u32;
    let patched = &linked.bytecode[1..5];
    assert_eq!(patched, &func_offset.to_le_bytes());
  }

  #[test]
  fn test_link_relative_relocation() {
    // Similar to above, but with relative addressing
    let mut symbols1 = vec![
      SymbolEntry { name: "main".to_string(), offset: 0, section: 0, kind: 0, external: false },
      SymbolEntry { name: "func".to_string(), offset: 0, section: 0, kind: 0, external: true }
    ];
    let mut reloc1 = vec![
      RelocationEntry { offset: 1, symbol_index: 1, reloc_type: RelocationType::Relative }
    ];
    // .text = [JMP, 0, 0, 0, 0] (JMP opcode, then placeholder for relative addr)
    let obj1 = mock_obj(vec![0x02, 0x00, 0x00, 0x00, 0x00], vec![], vec![], symbols1, reloc1);

    let symbols2 = vec![
      SymbolEntry { name: "func".to_string(), offset: 0, section: 0, kind: 0, external: false }
    ];
    let obj2 = mock_obj(vec![0xCC], vec![], vec![], symbols2, vec![]);

    let linked = link(&[obj1, obj2], "main").expect("Should link");
    // relative = func_offset - (patch_offset + 4)
    let func_offset = 5i32;
    let patch_offset = 1i32;
    let rel = func_offset - (patch_offset + 4); // = 0

    let patched = i32::from_le_bytes([
      linked.bytecode[1], linked.bytecode[2],
      linked.bytecode[3], linked.bytecode[4],
    ]);
    assert_eq!(patched, rel);
  }

  #[test]
  fn test_link_unresolved_symbol_error() {
    // Reference to symbol not defined in any object
    let symbols = vec![
      SymbolEntry { name: "main".to_string(), offset: 0, section: 0, kind: 0, external: false },
      SymbolEntry { name: "missing".to_string(), offset: 0, section: 0, kind: 0, external: true }
    ];
    let reloc = vec![
      RelocationEntry { offset: 1, symbol_index: 1, reloc_type: RelocationType::Absolute }
    ];
    let obj = mock_obj(vec![0xDE, 0, 0, 0, 0], vec![], vec![], symbols, reloc);

    let result = link(&[obj], "main");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unresolved symbol"));
  }

  #[test]
  fn test_link_entry_point_missing() {
    let symbols = vec![
      SymbolEntry { name: "foo".to_string(), offset: 0, section: 0, kind: 0, external: false },
    ];
    let obj = mock_obj(vec![0x90], vec![], vec![], symbols, vec![]);
    // This should not fail, but entry_offset is None
    let linked = link(&[obj], "main").expect("Should link");
    assert_eq!(linked.entry_point, Some("main".to_string()));
    // But the symbol does not exist
    assert!(!linked.symbols.iter().any(|s| s.name == "main"));
  }
}

