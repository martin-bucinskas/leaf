use std::collections::HashMap;
use log::info;
use leaf_common::leaf_ast::{Arg, Line, OpCode};
use leaf_common::leaf_file::{LeafAsmObject, RelocationEntry, RelocationType, SymbolEntry};

#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Assembler {
  symbol_table: Vec<SymbolEntry>,
  labels: HashMap<String, (u8, u32)>, // name -> (section, offset)
  code: Vec<u8>,
  data: Vec<u8>,
  rodata: Vec<u8>,
  relocations: Vec<RelocationEntry>,
}

impl Assembler {
  pub fn new() -> Self {
    Self {
      symbol_table: Vec::new(),
      labels: HashMap::new(),
      code: Vec::new(),
      data: Vec::new(),
      rodata: Vec::new(),
      relocations: Vec::new(),
    }
  }

  pub fn assemble(program: &[Line], entry_point: Option<String>) -> LeafAsmObject {
    let mut assembler = Assembler::new();
    assembler.first_pass(program);
    assembler.second_pass(program);
    LeafAsmObject {
      bytecode: assembler.code,
      data: assembler.data,
      rodata: assembler.rodata,
      symbols: assembler.symbol_table,
      entry_point,
      relocations: assembler.relocations,
      debug_info: None,
    }
  }

  /// First pass: Collect all label definitions and externals
  pub fn first_pass(&mut self, program: &[Line]) {
    let mut pos = [0u32; 3]; // code, data, rodata
    let mut section = 0u8; // 0 = .text, 1 = .data, 2 = .rodata

    for line in program {
      info!("ℹ️ Handling line: {:?}", line);
      match line {
        Line::Section(s) => {
          section = match s.as_str() {
            ".text" => 0,
            ".data" => 1,
            ".rodata" => 2,
            _ => section,
          };
        }
        Line::LabelOnly(label) => {
          self.labels.insert(label.clone(), (section, pos[section as usize]));
          self.symbol_table.push(SymbolEntry {
            name: label.clone(),
            offset: pos[section as usize],
            section,
            kind: section, // kind: 0 = code label, 1 = data, 2 = rodata
            external: false,
          });
        }
        Line::Instruction(instr) => {
          if let Some(label) = &instr.label {
            self.labels.insert(label.clone(), (section, pos[section as usize]));
            self.symbol_table.push(SymbolEntry {
              name: label.clone(),
              offset: pos[section as usize],
              section,
              kind: section,
              external: false,
            });
          }
          if section == 0 {
            // .text: opcode + 4 bytes per arg
            pos[0] += 1 + 4 * instr.args.len() as u32;
          }
          // You could support data/rodata instructions if your ISA requires
        }
        Line::Extern(label) => {
          self.symbol_table.push(SymbolEntry {
            name: label.clone(),
            offset: 0,
            section: 0,
            kind: 0,
            external: true,
          });
        }
        Line::Directive(d) => {
          // .word and .ascii directives may exist in data or rodata sections
          match d.name.as_str() {
            "word" => {
              if let Some(args) = &d.args {
                let before_comment = args.split(';').next().unwrap_or("").trim();
                let word_count = before_comment.split_whitespace().count();
                pos[section as usize] += (word_count as u32) * 8;
              }
            }
            "string" => {
              if let Some(args) = &d.args {
                let s = args.split(';').next().unwrap_or("").trim().trim_matches('"');
                let parsed_bytes = parse_escaped_string(s);
                pos[section as usize] += (parsed_bytes.len() as u32) + 1;
              }
            }
            "ascii" => {
              if let Some(args) = &d.args {
                let s = args.trim().trim_matches('"');
                // ONLY increment pos for length, don't push data here!
                // Should use the escaped length!
                let parsed_bytes = parse_escaped_string(s);
                pos[section as usize] += parsed_bytes.len() as u32;
              }
            }
            "extern" => {
              info!("ℹ️ Found extern directive for: {}", d.args.as_ref().unwrap_or(&"".to_string()));
              if let Some(args) = &d.args {
                for label in args.split_whitespace() {
                  self.symbol_table.push(SymbolEntry {
                    name: label.to_string(),
                    offset: 0,
                    section: 0,
                    kind: 0, // Extern symbols are not section-specific
                    external: true,
                  });
                }
              }
            }
            _ => {}
          }
        }
        Line::Global(_) => {} // Could be used for exporting symbols (not needed for basic linking)
      }
    }
  }

  /// Second pass: Emit bytes and generate relocations
  pub fn second_pass(&mut self, program: &[Line]) {
    let mut section = 0u8; // 0=text, 1=data, 2=rodata
    let mut pos = [0u32; 3];

    for line in program {
      match line {
        Line::Section(s) => {
          section = match s.as_str() {
            ".text" => 0,
            ".data" => 1,
            ".rodata" => 2,
            _ => section,
          };
        }
        Line::LabelOnly(_) | Line::Extern(_) | Line::Global(_) => {}
        Line::Directive(d) => {
          match d.name.as_str() {
            "word" => {
              if let Some(args) = &d.args {
                let before_comment = args.split(';').next().unwrap_or("").trim();
                for num in before_comment.split_whitespace() {
                  let val: i64 = num.parse().unwrap();
                  let bytes = val.to_le_bytes();
                  self.append_to_section(section, &bytes);
                  pos[section as usize] += 8;
                }
              }
            }
            "string" => {
              if let Some(args) = &d.args {
                let s = args.split(';').next().unwrap_or("").trim().trim_matches('"');
                let mut parsed_bytes = parse_escaped_string(s);
                parsed_bytes.push(0); // Null terminator
                self.append_to_section(section, &parsed_bytes);
                pos[section as usize] += parsed_bytes.len() as u32;
              }
            }
            "ascii" => {
              if let Some(args) = &d.args {
                let s = args.trim().trim_matches('"');
                let parsed_bytes = parse_escaped_string(s);
                self.append_to_section(section, &parsed_bytes);
                pos[section as usize] += parsed_bytes.len() as u32;
              }
            }
            _ => {}
          }
        }
        Line::Instruction(instr) => {
          let mut instr_bytes = Vec::new();
          let opcode = &instr.opcode;
          let args = &instr.args;

          // Determine the actual opcode to emit (e.g. LOAD -> LOADI if using label/imm)
          let target_opcode = if args.len() >= 2 {
            match (opcode, &args[1]) {
              (OpCode::Load, Arg::Mem(inner)) => match &**inner { Arg::Register(_) => OpCode::Load, _ => OpCode::Loadi },
              (OpCode::Store, Arg::Mem(inner)) => match &**inner { Arg::Register(_) => OpCode::Store, _ => OpCode::Storei },
              _ => opcode.clone(),
            }
          } else {
            opcode.clone()
          };

          instr_bytes.push(OpCode::opcode_to_byte(&target_opcode));
          let mut current_instr_pos = pos[section as usize] + 1;

          match target_opcode {
            // Three register args: OP r1, r2, r3
            OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
            OpCode::And | OpCode::Or | OpCode::Xor |
            OpCode::Lt | OpCode::Gt | OpCode::Eq => {
              for i in 0..3 {
                Self::append_arg(&mut self.relocations, &mut self.symbol_table, &mut instr_bytes, &args[i], section, &mut current_instr_pos);
              }
            }
            // Two register args: OP r1, r2
            OpCode::Mov | OpCode::Load | OpCode::Store | OpCode::Not | OpCode::Jz | OpCode::Jnz | OpCode::Movi | OpCode::Loadi | OpCode::Storei => {
              for i in 0..2 {
                Self::append_arg(&mut self.relocations, &mut self.symbol_table, &mut instr_bytes, &args[i], section, &mut current_instr_pos);
              }
            }
            // One immediate/label: OP imm/label
            OpCode::Jmp | OpCode::Call | OpCode::Push | OpCode::Pop => {
              Self::append_arg(&mut self.relocations, &mut self.symbol_table, &mut instr_bytes, &args[0], section, &mut current_instr_pos);
            }
            // No args: OP
            OpCode::Ret | OpCode::Syscall | OpCode::Halt | OpCode::Nop | OpCode::Break => {
              // No arguments to emit
            }
            OpCode::Invalid => {}
          }

          self.append_to_section(section, &instr_bytes);
          pos[section as usize] = current_instr_pos;
        }
      }
    }
  }

  fn append_to_section(&mut self, section: u8, bytes: &[u8]) {
    match section {
      0 => self.code.extend_from_slice(bytes),
      1 => self.data.extend_from_slice(bytes),
      2 => self.rodata.extend_from_slice(bytes),
      _ => unreachable!(),
    }
  }

  fn append_arg(relocations: &mut Vec<RelocationEntry>, symbol_table: &mut Vec<SymbolEntry>, buffer: &mut Vec<u8>, arg: &Arg, section: u8, pos: &mut u32) {
    match arg {
      Arg::Register(name) => {
        let reg = Self::reg_number(name);
        buffer.extend_from_slice(&[reg, 0, 0, 0]);
        *pos += 4;
      }
      Arg::Immediate(val) => {
        buffer.extend_from_slice(&(*val as u32).to_le_bytes());
        *pos += 4;
      }
      Arg::Label(label) => {
        let symbol_idx = symbol_table.iter()
          .position(|s| s.name == *label)
          .expect(&format!("Reloc symbol '{}' must be in symbol table", label));
        let patch_offset = *pos;
        relocations.push(RelocationEntry {
          offset: patch_offset,
          symbol_index: symbol_idx as u32,
          reloc_type: RelocationType::Absolute,
          target_section: section,
        });
        buffer.extend_from_slice(&0u32.to_le_bytes());
        *pos += 4;
      }
      Arg::Mem(inner) => {
        Self::append_arg(relocations, symbol_table, buffer, inner, section, pos);
      }
    }
  }

  fn reg_number(name: &str) -> u8 {
    if let Some(n) = name.strip_prefix("r") {
      n.parse().unwrap_or(0xFF)
    } else {
      0xFF
    }
  }
}

fn parse_escaped_string(s: &str) -> Vec<u8> {
  let mut out = Vec::new();
  let mut chars = s.chars().peekable();
  while let Some(c) = chars.next() {
    if c == '\\' {
      match chars.next() {
        Some('0') => out.push(0),
        Some('n') => out.push(b'\n'),
        Some('t') => out.push(b'\t'),
        Some('r') => out.push(b'\r'),
        Some('\\') => out.push(b'\\'),
        Some('\'') => out.push(b'\''),
        Some('\"') => out.push(b'"'),
        Some(other) => {
          // Unknown escape, just push as char
          out.push(other as u8);
        }
        None => break,
      }
    } else {
      out.push(c as u8);
    }
  }
  out
}

#[cfg(test)]
mod tests {
  use leaf_common::leaf_ast::{Directive, Instruction};
  use super::*;

  fn line_instr(op: OpCode, args: Vec<Arg>, label: Option<&str>) -> Line {
    Line::Instruction(Instruction {
      label: label.map(|s| s.to_string()),
      opcode: op,
      args,
    })
  }

  #[test]
  fn assembles_simple_add_instruction() {
    // ADD r1, r2, r3
    let program = vec![
      Line::Section(".text".to_string()),
      line_instr(OpCode::Add,
                 vec![
                   Arg::Register("r1".to_string()),
                   Arg::Register("r2".to_string()),
                   Arg::Register("r3".to_string()),
                 ],
                 None),
    ];

    let obj = Assembler::assemble(&program, Some("main".to_string()));
    // Should encode as: opcode(1) + 3 * reg(4)
    // e.g., [0x01, r1, 0, 0, 0, r2, 0, 0, 0, r3, 0, 0, 0]
    assert_eq!(obj.bytecode[0], 0x01); // ADD opcode
    assert_eq!(obj.bytecode[1], 1); // r1
    assert_eq!(obj.bytecode[5], 2); // r2
    assert_eq!(obj.bytecode[9], 3); // r3
    assert!(obj.data.is_empty());
    assert!(obj.rodata.is_empty());
    assert!(obj.relocations.is_empty());
  }

  #[test]
  fn assembles_with_label_and_jmp() {
    // main: NOP, JMP to main (should resolve directly)
    let program = vec![
      Line::Section(".text".to_string()),
      Line::LabelOnly("main".to_string()),
      line_instr(OpCode::Nop, vec![], None),
      line_instr(OpCode::Jmp, vec![Arg::Label("main".to_string())], None),
    ];
    let obj = Assembler::assemble(&program, Some("main".to_string()));
    // Expect JMP opcode (0x09) and address 0 (main)
    assert_eq!(obj.bytecode[0], 0x00); // NOP
    assert_eq!(obj.bytecode[1], 0x09); // JMP
    // The address after JMP should be offset 0 (main label)
    let addr = u32::from_le_bytes([obj.bytecode[2], obj.bytecode[3], obj.bytecode[4], obj.bytecode[5]]);
    assert_eq!(addr, 0);
    // Symbol table includes main
    assert!(obj.symbols.iter().any(|s| s.name == "main" && s.offset == 0));
  }

  #[test]
  fn assembles_data_and_rodata_sections() {
    let program = vec![
      Line::Section(".data".to_string()),
      Line::Directive(Directive { name: "word".to_string(), args: Some("42 1337".to_string()) }),
      Line::Section(".rodata".to_string()),
      Line::Directive(Directive { name: "ascii".to_string(), args: Some("\"hello\"".to_string()) }),
    ];
    let obj = Assembler::assemble(&program, None);
    // .data = [42, 1337] as i32 LE
    assert_eq!(obj.data.len(), 8);
    assert_eq!(i32::from_le_bytes(obj.data[0..4].try_into().unwrap()), 42);
    assert_eq!(i32::from_le_bytes(obj.data[4..8].try_into().unwrap()), 1337);
    // .rodata = b"hello"
    assert_eq!(&obj.rodata, b"hello");
  }

  #[test]
  fn assembles_extern_symbol_and_relocation() {
    let program = vec![
      Line::Section(".text".to_string()),
      Line::Extern("external_func".to_string()),
      line_instr(OpCode::Call, vec![Arg::Label("external_func".to_string())], None),
    ];
    let obj = Assembler::assemble(&program, None);
    // Should create a relocation for external_func
    assert_eq!(obj.relocations.len(), 1);
    let reloc = &obj.relocations[0];
    // Should patch at offset 1 (after opcode)
    assert_eq!(reloc.offset, 1);
    assert_eq!(reloc.reloc_type, RelocationType::Absolute);
    // Symbol table should include the extern symbol
    assert!(obj.symbols.iter().any(|s| s.name == "external_func" && s.external));
  }

  #[test]
  fn assembles_label_prefixed_instruction() {
    // label: MOV r1, 123
    let program = vec![
      Line::Section(".text".to_string()),
      line_instr(OpCode::Mov,
                 vec![Arg::Register("r1".to_string()), Arg::Immediate(123)],
                 Some("start")),
    ];
    let obj = Assembler::assemble(&program, Some("start".to_string()));
    // Symbol table includes start at offset 0
    assert!(obj.symbols.iter().any(|s| s.name == "start" && s.offset == 0));
    // MOV r1, 123: opcode, r1, 123
    assert_eq!(obj.bytecode[0], 0x0C); // MOV
    assert_eq!(obj.bytecode[1], 1);    // r1
    let imm = u32::from_le_bytes([obj.bytecode[5], obj.bytecode[6], obj.bytecode[7], obj.bytecode[8]]);
    assert_eq!(imm, 123);
  }

  #[test]
  fn handles_unresolved_label_as_external_relocation() {
    // Will only work if the symbol is listed in the symbol_table as external
    let program = vec![
      Line::Section(".text".to_string()),
      Line::Extern("missing".to_string()),
      line_instr(OpCode::Jmp, vec![Arg::Label("missing".to_string())], None),
    ];
    let obj = Assembler::assemble(&program, None);
    // Should create a relocation for missing
    assert_eq!(obj.relocations.len(), 1);
    let reloc = &obj.relocations[0];
    assert_eq!(reloc.symbol_index as usize, 0); // Only symbol in table is missing
    assert_eq!(reloc.offset, 1);
  }
}

