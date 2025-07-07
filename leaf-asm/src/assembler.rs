use crate::ast::{Instruction, Arg, OpCode, Line};
use std::collections::HashMap;
use log::debug;
use crate::parser::parse_program;

pub struct Assembler {
  /// Map for storing label names to bytecode addresses.
  label_table: HashMap<String, usize>,
  /// Current position in the bytecode.
  position: usize,
  /// Current section being assembled.
  current_section: Section,
  /// Bytecode sections
  code: Vec<u8>,
  /// Sections for read-only data
  rodata: Vec<u8>,
  /// Sections for data
  data: Vec<u8>,
}

pub enum Section {
  Text,
  Data,
  Rodata,
}

#[derive(Debug)]
pub struct BytecodeProgram {
  pub code: Vec<u8>,
}

pub struct BytecodeSections {
  pub code: Vec<u8>,
  pub data: Vec<u8>,
  pub rodata: Vec<u8>,
}

impl Assembler {
  pub fn new() -> Self {
    Assembler {
      label_table: HashMap::new(),
      position: 0,
      current_section: Section::Text,
      code: Vec::new(),
      rodata: Vec::new(),
      data: Vec::new(),
    }
  }

  /// First pass: collect all labels and their positions.
  fn first_pass(&mut self, program: &Vec<Line>) {
    self.position = 0;

    for line in program {
      match line {
        Line::Instruction(instruction) => {
          if let Some(label) = &instruction.label {
            (&mut self.label_table).insert(label.clone(), self.position);
          }

          let instruction_size = 1 + (instruction.args.len() * 4);
          self.position += instruction_size;
        }
        Line::LabelOnly(label) => {
          (&mut self.label_table).insert(label.clone(), self.position);
        }
        Line::Directive(directive) => {
          // Ignore directives in the first pass
          debug!("Ignoring directive: {} during the first pass of assembler", directive.name);
        }
        Line::Section(name) => {
          // Ignore section directives in the first pass
          debug!("Ignoring section directive: {} during the first pass of assembler", name);
        }
        Line::Global(_) => {
          // Ignore global directives in the first pass
          debug!("Ignoring global directive during the first pass of assembler");
        }
      }
    }
  }

  fn second_pass(&mut self, program: &Vec<Line>) -> BytecodeSections {
    let mut code = Vec::new();
    let mut rodata = Vec::new();
    let mut data = Vec::new();
    let mut current_section = Section::Text; // default

    for line in program {
      match line {
        Line::Section(name) => {
          current_section = match name.as_str() {
            ".text"   => Section::Text,
            ".data"   => Section::Data,
            ".rodata" => Section::Rodata,
            _         => current_section, // ignore unknown
          };
        }
        Line::Instruction(instr) => {
          if let Section::Text = current_section {
            code.push(self.opcode_to_byte(&instr.opcode));
            for arg in &instr.args {
              let arg_bytes = self.encode_argument(arg);
              code.extend_from_slice(&arg_bytes);
            }
          }
          // Could error if instructions are in wrong section!
        }
        Line::Directive(directive) => {
          match directive.name.as_str() {
            "word" => {
              if let Some(ref args) = directive.args {
                for value in args.split_whitespace() {
                  let num: i32 = value.parse().unwrap();
                  match current_section {
                    Section::Data => data.extend_from_slice(&num.to_le_bytes()),
                    Section::Rodata => rodata.extend_from_slice(&num.to_le_bytes()),
                    _ => {}, // ignore for now
                  }
                }
              }
            }
            "ascii" => {
              if let Some(ref args) = directive.args {
                let s = args.trim_matches('"');
                match current_section {
                  Section::Data => data.extend_from_slice(s.as_bytes()),
                  Section::Rodata => rodata.extend_from_slice(s.as_bytes()),
                  _ => {},
                }
              }
            }
            _ => {}
          }
        }
        _ => {}
      }
    }

    // At the end, combine sections as needed (or export them separately)
    // For a simple VM, maybe concatenate: [code][data][rodata]
    BytecodeSections { code, data, rodata }
  }

  fn opcode_to_byte(&self, opcode: &OpCode) -> u8 {
    match opcode {
      OpCode::Nop => 0x00,
      OpCode::Add => 0x01,
      OpCode::Sub => 0x02,
      OpCode::Mul => 0x03,
      OpCode::Div => 0x04,
      OpCode::And => 0x05,
      OpCode::Or => 0x06,
      OpCode::Xor => 0x07,
      OpCode::Not => 0x08,
      OpCode::Jmp => 0x09,
      OpCode::Jz => 0x0A,
      OpCode::Jnz => 0x0B,
      OpCode::Mov => 0x0C,
      OpCode::Load => 0x0D,
      OpCode::Store => 0x0E,
      OpCode::Call => 0x0F,
      OpCode::Ret => 0x10,
      OpCode::Push => 0x11,
      OpCode::Pop => 0x12,
      OpCode::Halt => 0x13,
      OpCode::Break => 0x14,
      OpCode::Syscall => 0x15,
    }
  }

  fn encode_argument(&self, arg: &Arg) -> [u8; 4] {
    match arg {
      Arg::Immediate(value) => {
        let value_bytes = (*value as u32).to_le_bytes();
        value_bytes
      },
      Arg::Register(name) => {
        let register_number = match name.as_str() {
          "r0" => 0,
          "r1" => 1,
          "r2" => 2,
          "r3" => 3,
          "r4" => 4,
          "r5" => 5,
          "r6" => 6,
          "r7" => 7,
          _ => 0xFF, // Invalid register
        };

        let mut bytes = [0; 4];
        bytes[0] = register_number;
        bytes
      },
      Arg::Label(label_name) => {
        let address = self.label_table.get(label_name)
          .unwrap_or(&0);

        let address_bytes = (*address as u32).to_le_bytes();
        address_bytes
      }
    }
  }

  pub fn assemble_sections(&mut self, program: &Vec<Line>) -> BytecodeSections {
    self.first_pass(program);
    self.second_pass(program)
  }
}

impl BytecodeProgram {
  pub fn with_header(code: Vec<u8>, data: Vec<u8>, rodata: Vec<u8>) -> Vec<u8> {
    // Header layout constants
    const HEADER_SIZE: usize = 32;
    let mut output = Vec::with_capacity(
      HEADER_SIZE + code.len() + data.len() + rodata.len()
    );

    // --- Write header fields ---
    // Magic number "LAF\0"
    output.extend_from_slice(b"LAF\0");
    // Version 0x0001
    output.extend_from_slice(&1u16.to_le_bytes());
    // Reserved/padding
    output.extend_from_slice(&0u16.to_le_bytes());

    // Calculate section offsets (from start of file)
    let text_offset = HEADER_SIZE as u32;
    let text_size = code.len() as u32;
    let data_offset = text_offset + text_size;
    let data_size = data.len() as u32;
    let rodata_offset = data_offset + data_size;
    let rodata_size = rodata.len() as u32;

    output.extend_from_slice(&text_offset.to_le_bytes());
    output.extend_from_slice(&text_size.to_le_bytes());
    output.extend_from_slice(&data_offset.to_le_bytes());
    output.extend_from_slice(&data_size.to_le_bytes());
    output.extend_from_slice(&rodata_offset.to_le_bytes());
    output.extend_from_slice(&rodata_size.to_le_bytes());

    // --- Write section contents ---
    output.extend_from_slice(&code);
    output.extend_from_slice(&data);
    output.extend_from_slice(&rodata);

    output
  }
}

pub fn assemble_with_header(source: &str) -> Result<Vec<u8>, String> {
  let program = parse_program(source)?;
  let mut assembler = Assembler::new();
  let sections = assembler.assemble_sections(&program);
  Ok(BytecodeProgram::with_header(sections.code, sections.data, sections.rodata))
}

#[cfg(test)]
mod tests {
  use super::*;

  fn parse_header(bytes: &[u8]) -> (u32, u32, u32, u32, u32, u32) {
    assert_eq!(&bytes[0..4], b"LAF\0");
    let text_offset = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]);
    let text_size   = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
    let data_offset = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
    let data_size   = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
    let rodata_offset = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
    let rodata_size   = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
    (text_offset, text_size, data_offset, data_size, rodata_offset, rodata_size)
  }

  #[test]
  fn test_assemble_simple_instruction() {
    let source = "ADD r1, r2, r3";
    let bytecode = assemble_with_header(source).unwrap();

    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 13, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_with_immediate() {
    let source = "MOV r1, 42";
    let bytecode = assemble_with_header(source).unwrap();

    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 9, 0, 0, 0, 41, 0, 0, 0, 0, 0, 0, 0, 41, 0, 0, 0, 0, 0, 0, 0, 12, 1, 0, 0, 0, 42, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_label_and_jump() {
    let source = "
      start:
      NOP
      JMP start
    ";
    let bytecode = assemble_with_header(source).unwrap();

    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 6, 0, 0, 0, 38, 0, 0, 0, 0, 0, 0, 0, 38, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_multiple_instructions_and_labels() {
    let source = "
      MOV r1, 10
      loop: SUB r1, r1, 1
      JNZ loop
      HALT
    ";

    let bytecode = assemble_with_header(source).unwrap();

    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 28, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 60, 0, 0, 0, 0, 0, 0, 0, 12, 1, 0, 0, 0, 10, 0, 0, 0, 2, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 11, 9, 0, 0, 0, 19
    ]);
  }

  #[test]
  fn test_invalid_register_defaults_to_ff() {
    let source = "ADD r9, r1, r2";
    let bytecode = assemble_with_header(source).unwrap();

    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 13, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 45, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0
    ]);
  }

  #[test]
  fn test_label_only_line() {
    let source = "start:";
    let assembler = assemble_with_header(source).unwrap();
    assert_eq!(assembler.len(), 32); // Header size only
  }

  #[test]
  fn test_full_program_with_syscall() {
    let source = "
      MOV r0, 1
      SYSCALL
      HALT
    ";
    let bytecode = assemble_with_header(source).unwrap();

    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 11, 0, 0, 0, 43, 0, 0, 0, 0, 0, 0, 0, 43, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 1, 0, 0, 0, 21, 19
    ]);
  }

  #[test]
  fn test_assemble_data_word() {
    let source = ".word 42 100 -1";
    let bytecode = assemble_with_header(source).unwrap();
    // TODO: Check this as I think it might be incorrect.
    assert_eq!(bytecode, [
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_data_word_with_newline() {
    let source = ".word 42 100 -1\n";
    let bytecode = assemble_with_header(source).unwrap();
    assert_eq!(bytecode, [
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_ascii() {
    let source = ".ascii \"hello!\"";
    let bytecode = assemble_with_header(source).unwrap();
    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_ascii_with_newline() {
    let source = ".ascii \"hello!\"\n";
    let bytecode = assemble_with_header(source).unwrap();
    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_ascii_label_with_newline() {
    let source = "hello: .ascii \"hello!\"\n";
    let bytecode = assemble_with_header(source).unwrap();
    assert_eq!(bytecode, vec![
      76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0
    ]);
  }

  #[test]
  fn test_assemble_sections() {
    let src = "
        .section .data
        msg: .ascii \"hi\"
        .section .text
        MOV r1, 1
        HALT
    ";
    let prog = assemble_with_header(src).unwrap();
    // code = [MOV ... HALT ...], data = [b'h', b'i']
    // You can test that the bytecode layout is correct!

    assert_eq!(
      prog,
      vec![
        76, 65, 70, 0, 1, 0, 0, 0, 32, 0, 0, 0, 10, 0, 0, 0, 42, 0, 0, 0, 2, 0, 0, 0, 44, 0, 0, 0, 0, 0, 0, 0, 12, 1, 0, 0, 0, 1, 0, 0, 0, 19, 104, 105
      ]
    )
  }

  #[test]
  fn test_only_code_section() {
    let src = "MOV r0, 42\nHALT";
    let bytes = assemble_with_header(src).unwrap();
    let (text_off, text_sz, data_off, data_sz, rodata_off, rodata_sz) = parse_header(&bytes);

    // Code is right after header
    assert_eq!(text_off, 32);
    assert_eq!(text_sz, 10);
    assert_eq!(data_sz, 0);
    assert_eq!(rodata_sz, 0);

    // Code bytes are correct
    assert_eq!(&bytes[text_off as usize..(text_off+text_sz) as usize],
               &[0x0C, 0x00, 0x00, 0x00, 0x00, 0x2A, 0x00, 0x00, 0x00, 0x13]);
  }

  #[test]
  fn test_data_section_word_ascii() {
    let src = "
        .section .data
        .word 123 456
        .ascii \"hi\"
    ";
    let bytes = assemble_with_header(src).unwrap();
    let (_text_off, _text_sz, data_off, data_sz, _rodata_off, _rodata_sz) = parse_header(&bytes);

    // Data section is after header, length matches contents
    assert_eq!(data_off, 32);
    assert_eq!(data_sz, 8 + 2); // 2 words + 2 ascii bytes

    // Data: 123, 456, 'h', 'i'
    let expected_data = {
      let mut v = vec![];
      v.extend_from_slice(&123i32.to_le_bytes());
      v.extend_from_slice(&456i32.to_le_bytes());
      v.extend_from_slice(b"hi");
      v
    };
    assert_eq!(&bytes[data_off as usize..(data_off+data_sz) as usize], &expected_data[..]);
  }

  #[test]
  fn test_rodata_section_ascii() {
    let src = "
        .section .rodata
        .ascii \"RO\"
    ";
    let bytes = assemble_with_header(src).unwrap();
    let (_text_off, _text_sz, _data_off, _data_sz, rodata_off, rodata_sz) = parse_header(&bytes);

    assert_eq!(rodata_sz, 2);
    assert_eq!(&bytes[rodata_off as usize..(rodata_off+rodata_sz) as usize], b"RO");
  }

  #[test]
  fn test_mixed_sections() {
    let src = "
        .section .text
        MOV r1, 99
        .section .data
        .word -1
        .section .rodata
        .ascii \"read\"
    ";
    let bytes = assemble_with_header(src).unwrap();
    let (text_off, text_sz, data_off, data_sz, rodata_off, rodata_sz) = parse_header(&bytes);

    // Check code
    assert_eq!(&bytes[text_off as usize..(text_off+text_sz) as usize],
               &[0x0C, 0x01,0,0,0, 99,0,0,0]);
    // Check data
    let minus1 = (-1i32).to_le_bytes();
    assert_eq!(&bytes[data_off as usize..(data_off+data_sz) as usize], &minus1);
    // Check rodata
    assert_eq!(&bytes[rodata_off as usize..(rodata_off+rodata_sz) as usize], b"read");
  }

  #[test]
  fn test_data_label_ascii() {
    let src = "
        .section .data
        msg: .ascii \"yo\"
    ";
    // ...check that msg label offset is available in a symbol table
  }
}

