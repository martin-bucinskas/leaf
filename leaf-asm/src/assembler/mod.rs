use std::io::{Read, Write};
use bincode::{Decode, Encode};
use log::info;
use crate::common::{ReadableResource, WriteableResource};

pub mod asm;
pub mod assemble;

#[derive(Debug, Eq, PartialEq, Clone, Encode, Decode)]
pub struct SymbolEntry {
  /// The name of the symbol, e.g. "main", "data_buffer", etc.
  pub name: String,
  /// The offset of the symbol in the section it belongs to.
  pub offset: u32,
  /// The size of the symbol in bytes: 0 = .text, 1 = .data, 2 = .rodata
  pub section: u8,
  /// The kind of symbol: 0 = label, 1 = data, 2 = rodata
  pub kind: u8,
  /// Indicates whether the symbol is extern or not.
  pub external: bool,
}

#[derive(Debug, Eq, PartialEq, Clone, Encode, Decode)]
pub enum RelocationType {
  Absolute,
  Relative
}

#[derive(Debug, Eq, PartialEq, Clone, Encode, Decode)]
pub struct RelocationEntry {
  pub offset: u32,
  pub symbol_index: u32,
  pub reloc_type: RelocationType,
}

#[derive(Debug, Eq, PartialEq, Clone, Encode, Decode)]
pub struct LeafAsmObjectHeader {
  pub magic: [u8; 4],
  pub version: u16,
  pub reserved: u16,
  pub checksum: u32,
}

#[derive(Debug, Eq, PartialEq, Clone, Encode, Decode)]
pub struct LeafAsmObject {
  pub bytecode: Vec<u8>,
  pub data: Vec<u8>,
  pub rodata: Vec<u8>,
  pub symbols: Vec<SymbolEntry>,
  pub entry_point: Option<String>,
  pub relocations: Vec<RelocationEntry>,
  pub debug_info: Option<String>,
}

#[derive(Debug, Eq, PartialEq, Clone, Encode, Decode)]
pub struct LeafAsmFile {
  pub header: LeafAsmObjectHeader,
  pub object: LeafAsmObject,
}

impl WriteableResource for LeafAsmFile {
  fn write_to(&self, writer: &mut dyn Write) -> std::io::Result<()> {
    let config = bincode::config::standard();

    info!("Generating checksum...");
    let mut file_with_zero_checksum = self.clone();
    file_with_zero_checksum.header.checksum = 0;

    let encoded_without_checksum = bincode::encode_to_vec(&file_with_zero_checksum, config)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let checksum = crc32fast::hash(&encoded_without_checksum);

    info!("Checksum generated: {}, writing to writer...", checksum);
    let mut final_file = self.clone();
    final_file.header.checksum = checksum;

    let final_encoded = bincode::encode_to_vec(&final_file, config)
      .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    writer.write_all(&final_encoded)?;
    Ok(())
  }
}

impl ReadableResource for LeafAsmFile {
  fn read_from(reader: &mut dyn Read) -> std::io::Result<Self>
  where
    Self: Sized
  {
    let mut buffer = Vec::new();
    reader.read_to_end(&mut buffer)?;

    let config = bincode::config::standard();
    match bincode::decode_from_slice(&buffer, config) {
      Ok((obj, _)) => Ok(obj),
      Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialization() {
    let symbol = SymbolEntry {
      name: "main".to_string(),
      offset: 0x1000,
      section: 0, // .text
      kind: 0, // label
      external: false,
    };

    let reloc = RelocationEntry {
      offset: 0x1004,
      symbol_index: 0,
      reloc_type: RelocationType::Absolute,
    };

    let object = LeafAsmObject {
      bytecode: vec![0x90, 0x90, 0x90], // NOP instructions
      symbols: vec![symbol],
      data: vec![],
      rodata: vec![],
      entry_point: Some("main".to_string()),
      relocations: vec![reloc],
      debug_info: Some("Debug info".to_string()),
    };

    let header = LeafAsmObjectHeader {
      magic: *b"LAF\0",
      version: 1,
      reserved: 0,
      checksum: 12345678,
    };

    let header_clone = header.clone();
    let object_clone = object.clone();

    let leaf_asm_file = LeafAsmFile {
      header,
      object,
    };

    let mut buffer = Vec::new();
    leaf_asm_file.write_to(&mut buffer).unwrap();

    let decoded = LeafAsmFile::read_from(&mut buffer.as_slice()).unwrap();

    assert_eq!(decoded.object, object_clone);
    assert_eq!(decoded.header.magic, header_clone.magic);
    assert_eq!(decoded.header.version, header_clone.version);
    assert_eq!(decoded.header.reserved, header_clone.reserved);
    assert_eq!(decoded.header.checksum, 310412118);
  }
}
