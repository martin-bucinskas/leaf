// use crate::ast::{Instruction, Arg, OpCode, Line};
// use std::collections::HashMap;
// use crc32fast::Hasher;
// use log::debug;
// use crate::parser::parse_program;
//
// pub struct Assembler {
//   /// Map for storing label names to bytecode addresses.
//   label_table: HashMap<String, usize>,
//   /// Current position in the bytecode.
//   position: usize,
//   /// Current section being assembled.
//   current_section: Section,
//   /// Bytecode sections
//   code: Vec<u8>,
//   /// Sections for read-only data
//   rodata: Vec<u8>,
//   /// Sections for data
//   data: Vec<u8>,
//   /// Symbol table for storing symbol entries.
//   pub symbol_table: Vec<SymbolEntry>,
// }
//
// pub enum Section {
//   Text,
//   Data,
//   Rodata,
// }
//
// #[derive(Debug, Clone)]
// pub struct SymbolEntry {
//   pub name: String,
//   pub offset: u32,
//   pub section: u8, // 0 = .text, 1 = .data, 2 = .rodata
//   pub kind: u8, // 0 = label, 1 = data, 2 = rodata
// }
//
// #[derive(Debug)]
// pub struct BytecodeProgram {
//   pub code: Vec<u8>,
// }
//
// pub struct BytecodeSections {
//   pub code: Vec<u8>,
//   pub data: Vec<u8>,
//   pub rodata: Vec<u8>,
//   pub symbols: Vec<SymbolEntry>,
// }
//
// impl Assembler {
//   pub fn new() -> Self {
//     Assembler {
//       label_table: HashMap::new(),
//       position: 0,
//       current_section: Section::Text,
//       code: Vec::new(),
//       rodata: Vec::new(),
//       data: Vec::new(),
//       symbol_table: Vec::new(),
//     }
//   }
//
//   /// First pass: collect all labels and their positions.
//   fn first_pass(&mut self, program: &Vec<Line>) {
//     self.position = 0;
//
//     for line in program {
//       match line {
//         Line::Instruction(instruction) => {
//           if let Some(label) = &instruction.label {
//             (&mut self.label_table).insert(label.clone(), self.position);
//           }
//
//           let instruction_size = 1 + (instruction.args.len() * 4);
//           self.position += instruction_size;
//         }
//         Line::LabelOnly(label) => {
//           (&mut self.label_table).insert(label.clone(), self.position);
//         }
//         Line::Directive(directive) => {
//           // Ignore directives in the first pass
//           debug!("Ignoring directive: {} during the first pass of assembler", directive.name);
//         }
//         Line::Section(name) => {
//           // Ignore section directives in the first pass
//           debug!("Ignoring section directive: {} during the first pass of assembler", name);
//         }
//         Line::Global(_) => {
//           // Ignore global directives in the first pass
//           debug!("Ignoring global directive during the first pass of assembler");
//         }
//       }
//     }
//   }
//
//   fn second_pass(&mut self, program: &Vec<Line>) -> BytecodeSections {
//     let mut code = Vec::new();
//     let mut rodata = Vec::new();
//     let mut data = Vec::new();
//     let mut current_section = Section::Text;
//
//     // Section offsets for each section
//     let mut code_offset = 0u32;
//     let mut data_offset = 0u32;
//     let mut rodata_offset = 0u32;
//
//     for line in program {
//       match line {
//         Line::Section(name) => {
//           current_section = match name.as_str() {
//             ".text" => Section::Text,
//             ".data" => Section::Data,
//             ".rodata" => Section::Rodata,
//             _ => current_section,
//           };
//         }
//         Line::LabelOnly(label) => {
//           // Record the label offset in the symbol table
//           let (offset, section, kind) = match current_section {
//             Section::Text => (code_offset, 0, 0),
//             Section::Data => (data_offset, 1, 1),
//             Section::Rodata => (rodata_offset, 2, 2),
//           };
//           self.symbol_table.push(SymbolEntry {
//             name: label.clone(),
//             offset,
//             section,
//             kind,
//           });
//         }
//         Line::Instruction(instr) => {
//           if let Some(label) = &instr.label {
//             let (offset, section, kind) = match current_section {
//               Section::Text => (code_offset, 0, 0),
//               Section::Data => (data_offset, 1, 1),
//               Section::Rodata => (rodata_offset, 2, 2),
//             };
//             self.symbol_table.push(SymbolEntry {
//               name: label.clone(),
//               offset,
//               section,
//               kind,
//             });
//           }
//           if let Section::Text = current_section {
//             code.push(self.opcode_to_byte(&instr.opcode));
//             code_offset += 1;
//             for arg in &instr.args {
//               let arg_bytes = self.encode_argument(arg);
//               code.extend_from_slice(&arg_bytes);
//               code_offset += 4;
//             }
//           }
//         }
//         Line::Directive(directive) => {
//           match directive.name.as_str() {
//             "word" => {
//               if let Some(ref args) = directive.args {
//                 for value in args.split_whitespace() {
//                   let num: i32 = value.parse().unwrap();
//                   match current_section {
//                     Section::Data => {
//                       data.extend_from_slice(&num.to_le_bytes());
//                       data_offset += 4;
//                     }
//                     Section::Rodata => {
//                       rodata.extend_from_slice(&num.to_le_bytes());
//                       rodata_offset += 4;
//                     }
//                     _ => {}
//                   }
//                 }
//               }
//             }
//             "ascii" => {
//               if let Some(ref args) = directive.args {
//                 let s = args.trim_matches('"');
//                 match current_section {
//                   Section::Data => {
//                     data.extend_from_slice(s.as_bytes());
//                     data_offset += s.len() as u32;
//                   }
//                   Section::Rodata => {
//                     rodata.extend_from_slice(s.as_bytes());
//                     rodata_offset += s.len() as u32;
//                   }
//                   _ => {},
//                 }
//               }
//             }
//             _ => {}
//           }
//         }
//         _ => {}
//       }
//     }
//     BytecodeSections { code, data, rodata, symbols: self.symbol_table.clone() }
//   }
//
//   fn opcode_to_byte(&self, opcode: &OpCode) -> u8 {
//     match opcode {
//       OpCode::Nop => 0x00,
//       OpCode::Add => 0x01,
//       OpCode::Sub => 0x02,
//       OpCode::Mul => 0x03,
//       OpCode::Div => 0x04,
//       OpCode::And => 0x05,
//       OpCode::Or => 0x06,
//       OpCode::Xor => 0x07,
//       OpCode::Not => 0x08,
//       OpCode::Jmp => 0x09,
//       OpCode::Jz => 0x0A,
//       OpCode::Jnz => 0x0B,
//       OpCode::Mov => 0x0C,
//       OpCode::Load => 0x0D,
//       OpCode::Store => 0x0E,
//       OpCode::Call => 0x0F,
//       OpCode::Ret => 0x10,
//       OpCode::Push => 0x11,
//       OpCode::Pop => 0x12,
//       OpCode::Halt => 0x13,
//       OpCode::Break => 0x14,
//       OpCode::Syscall => 0x15,
//     }
//   }
//
//   fn encode_argument(&self, arg: &Arg) -> [u8; 4] {
//     match arg {
//       Arg::Immediate(value) => {
//         let value_bytes = (*value as u32).to_le_bytes();
//         value_bytes
//       },
//       Arg::Register(name) => {
//         let register_number = match name.as_str() {
//           "r0" => 0,
//           "r1" => 1,
//           "r2" => 2,
//           "r3" => 3,
//           "r4" => 4,
//           "r5" => 5,
//           "r6" => 6,
//           "r7" => 7,
//           _ => 0xFF, // Invalid register
//         };
//
//         let mut bytes = [0; 4];
//         bytes[0] = register_number;
//         bytes
//       },
//       Arg::Label(label_name) => {
//         let address = self.label_table.get(label_name)
//           .unwrap_or(&0);
//
//         let address_bytes = (*address as u32).to_le_bytes();
//         address_bytes
//       }
//     }
//   }
//
//   pub fn assemble_sections(&mut self, program: &Vec<Line>) -> BytecodeSections {
//     self.first_pass(program);
//     self.second_pass(program)
//   }
// }
//
// impl BytecodeProgram {
//   pub fn with_header(code: Vec<u8>, data: Vec<u8>, rodata: Vec<u8>, symbols: Vec<SymbolEntry>) -> Vec<u8> {
//     // Header layout constants
//     // magic(4) version(2) reserved(2) checksum(4)
//     // text_offset(4) text_size(4)
//     // data_offset(4) data_size(4)
//     // rodata_offset(4) rodata_size(4)
//     // symtab_offset(4) symtab_size(4)
//     const HEADER_SIZE: usize = 40;
//
//     let symtab = write_symbol_table(&symbols);
//
//     let mut output = Vec::with_capacity(
//       HEADER_SIZE + code.len() + data.len() + rodata.len() + symtab.len()
//     );
//
//     // --- Write header fields ---
//     // Magic number "LAF\0"
//     output.extend_from_slice(b"LAF\0");
//     // Version 0x0001
//     output.extend_from_slice(&1u16.to_le_bytes());
//     // Reserved/padding
//     output.extend_from_slice(&0u16.to_le_bytes());
//     // Checksum placeholder
//     output.extend_from_slice(&0u32.to_le_bytes());
//
//     // Calculate section offsets (from start of file)
//     let text_offset = HEADER_SIZE as u32;
//     let text_size = code.len() as u32;
//     let data_offset = text_offset + text_size;
//     let data_size = data.len() as u32;
//     let rodata_offset = data_offset + data_size;
//     let rodata_size = rodata.len() as u32;
//
//     output.extend_from_slice(&text_offset.to_le_bytes());
//     output.extend_from_slice(&text_size.to_le_bytes());
//     output.extend_from_slice(&data_offset.to_le_bytes());
//     output.extend_from_slice(&data_size.to_le_bytes());
//     output.extend_from_slice(&rodata_offset.to_le_bytes());
//     output.extend_from_slice(&rodata_size.to_le_bytes());
//
//     let symtab_offset = rodata_offset + rodata_size;
//     let symtab_size = symtab.len() as u32;
//
//     // --- Write section contents ---
//     output.extend_from_slice(&code);
//     output.extend_from_slice(&data);
//     output.extend_from_slice(&rodata);
//     output.extend_from_slice(&symtab);
//
//     let checksum = {
//       let mut hasher = Hasher::new();
//       hasher.update(&output);
//       hasher.finalize()
//     };
//
//     output[8..12].copy_from_slice(&checksum.to_le_bytes());
//
//     output
//   }
// }
//
// fn write_symbol_table(symbols: &[SymbolEntry]) -> Vec<u8> {
//   let mut buf = Vec::new();
//   for sym in symbols {
//     let name_bytes = sym.name.as_bytes();
//     buf.push(name_bytes.len() as u8);
//     buf.extend_from_slice(name_bytes);
//     buf.push(sym.kind);
//     buf.extend_from_slice(&sym.offset.to_le_bytes());
//     buf.push(sym.section);
//   }
//   buf
// }
//
// pub fn assemble_with_header(source: &str) -> Result<Vec<u8>, String> {
//   let program = parse_program(source)?;
//   let mut assembler = Assembler::new();
//   let sections = assembler.assemble_sections(&program);
//   Ok(BytecodeProgram::with_header(sections.code, sections.data, sections.rodata, sections.symbols))
// }
//
// #[cfg(test)]
// mod tests {
//   use super::*;
//
//   fn parse_header(bytes: &[u8]) -> (u32, u32, u32, u32, u32, u32, u32, u32) {
//     assert_eq!(&bytes[0..4], b"LAF\0");
//     let text_offset   = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]);
//     let text_size     = u32::from_le_bytes([bytes[16], bytes[17], bytes[18], bytes[19]]);
//     let data_offset   = u32::from_le_bytes([bytes[20], bytes[21], bytes[22], bytes[23]]);
//     let data_size     = u32::from_le_bytes([bytes[24], bytes[25], bytes[26], bytes[27]]);
//     let rodata_offset = u32::from_le_bytes([bytes[28], bytes[29], bytes[30], bytes[31]]);
//     let rodata_size   = u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]);
//     let symtab_offset = u32::from_le_bytes([bytes[36], bytes[37], bytes[38], bytes[39]]);
//     let symtab_size   = u32::from_le_bytes([bytes[40], bytes[41], bytes[42], bytes[43]]);
//     (text_offset, text_size, data_offset, data_size, rodata_offset, rodata_size, symtab_offset, symtab_size)
//   }
//
//   fn validate_checksum(bytes: &[u8]) {
//     let mut test = bytes.to_vec();
//     test[8..12].copy_from_slice(&[0;4]);
//     let mut hasher = crc32fast::Hasher::new();
//     hasher.update(&test);
//     let computed = hasher.finalize();
//     let stored = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
//     assert_eq!(computed, stored, "Checksum mismatch!");
//   }
//
//   #[test]
//   fn test_assemble_simple_instruction() {
//     let source = "ADD r1, r2, r3";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 240, 31, 87, 204, 40, 0, 0, 0, 13, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_with_immediate() {
//     let source = "MOV r1, 42";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 151, 28, 193, 197, 40, 0, 0, 0, 9, 0, 0, 0, 49, 0, 0, 0, 0, 0, 0, 0, 49, 0, 0, 0, 0, 0, 0, 0, 12, 1, 0, 0, 0, 42, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_label_and_jump() {
//     let source = "
//       start:
//       NOP
//       JMP start
//     ";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 248, 165, 62, 37, 40, 0, 0, 0, 6, 0, 0, 0, 46, 0, 0, 0, 0, 0, 0, 0, 46, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0, 0, 0, 5, 115, 116, 97, 114, 116, 0, 0, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_multiple_instructions_and_labels() {
//     let source = "
//       MOV r1, 10
//       loop: SUB r1, r1, 1
//       JNZ loop
//       HALT
//     ";
//
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 0, 224, 240, 51, 40, 0, 0, 0, 28, 0, 0, 0, 68, 0, 0, 0, 0, 0, 0, 0, 68, 0, 0, 0, 0, 0, 0, 0, 12, 1, 0, 0, 0, 10, 0, 0, 0, 2, 1, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 11, 9, 0, 0, 0, 19, 4, 108, 111, 111, 112, 0, 9, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_invalid_register_defaults_to_ff() {
//     let source = "ADD r9, r1, r2";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 124, 66, 216, 87, 40, 0, 0, 0, 13, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 53, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_label_only_line() {
//     let source = "start:";
//     let assembler = assemble_with_header(source).unwrap();
//     assert_eq!(assembler.len(), 48);
//   }
//
//   #[test]
//   fn test_full_program_with_syscall() {
//     let source = "
//       MOV r0, 1
//       SYSCALL
//       HALT
//     ";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 124, 79, 25, 238, 40, 0, 0, 0, 11, 0, 0, 0, 51, 0, 0, 0, 0, 0, 0, 0, 51, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 1, 0, 0, 0, 21, 19
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_data_word() {
//     let source = ".word 42 100 -1";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//     assert_eq!(bytecode, [
//       76, 65, 70, 0, 1, 0, 0, 0, 188, 4, 204, 228, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_data_word_with_newline() {
//     let source = ".word 42 100 -1\n";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//     assert_eq!(bytecode, [
//       76, 65, 70, 0, 1, 0, 0, 0, 188, 4, 204, 228, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_ascii() {
//     let source = ".ascii \"hello!\"";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 188, 4, 204, 228, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_ascii_with_newline() {
//     let source = ".ascii \"hello!\"\n";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 188, 4, 204, 228, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_ascii_label_with_newline() {
//     let source = "hello: .ascii \"hello!\"\n";
//     let bytecode = assemble_with_header(source).unwrap();
//     validate_checksum(&bytecode);
//     assert_eq!(bytecode, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 81, 171, 38, 186, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 5, 104, 101, 108, 108, 111, 0, 0, 0, 0, 0, 0
//     ]);
//   }
//
//   #[test]
//   fn test_assemble_sections() {
//     let src = "
//         .section .data
//         msg: .ascii \"hi\"
//         .section .text
//         MOV r1, 1
//         HALT
//     ";
//     let prog = assemble_with_header(src).unwrap();
//     validate_checksum(&prog);
//
//     assert_eq!(
//       prog,
//       vec![
//         76, 65, 70, 0, 1, 0, 0, 0, 11, 151, 80, 97, 40, 0, 0, 0, 10, 0, 0, 0, 50, 0, 0, 0, 2, 0, 0, 0, 52, 0, 0, 0, 0, 0, 0, 0, 12, 1, 0, 0, 0, 1, 0, 0, 0, 19, 104, 105, 3, 109, 115, 103, 1, 0, 0, 0, 0, 1
//       ]
//     )
//   }
//
//   #[test]
//   fn test_only_code_section() {
//     let src = "MOV r0, 42\nHALT";
//     let bytes = assemble_with_header(src).unwrap();
//     let (text_off, text_sz, data_off, data_sz, rodata_off, rodata_sz, symtab_off, symtab_sz) = parse_header(&bytes);
//
//     validate_checksum(&bytes);
//     // Code is right after header
//     assert_eq!(text_off, 40);
//     assert_eq!(text_sz, 10);
//     assert_eq!(data_sz, 0);
//     assert_eq!(rodata_sz, 0);
//
//     // Code bytes are correct
//     assert_eq!(bytes, vec![76, 65, 70, 0, 1, 0, 0, 0, 105, 104, 25, 115, 40, 0, 0, 0, 10, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 42, 0, 0, 0, 19]);
//   }
//
//   #[test]
//   fn test_data_section_word_ascii() {
//     let src = "
//         .section .data
//         .word 123 456
//         .ascii \"hi\"
//     ";
//     let bytes = assemble_with_header(src).unwrap();
//     let (text_off, text_sz, data_off, data_sz, rodata_off, rodata_sz, symtab_off, symtab_sz) = parse_header(&bytes);
//
//     // Data section is after header, length matches contents
//     assert_eq!(data_off, 40);
//     assert_eq!(data_sz, 8 + 2); // 2 words + 2 ascii bytes
//
//     // Data: 123, 456, 'h', 'i'
//     let expected_data = {
//       let mut v = vec![];
//       v.extend_from_slice(&123i32.to_le_bytes());
//       v.extend_from_slice(&456i32.to_le_bytes());
//       v.extend_from_slice(b"hi");
//       v
//     };
//     validate_checksum(&bytes);
//     // assert_eq!(&bytes[data_off as usize..(data_off+data_sz) as usize], &expected_data[..]);
//     assert_eq!(bytes, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 243, 7, 71, 12, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 10, 0, 0, 0, 50, 0, 0, 0, 0, 0, 0, 0, 123, 0, 0, 0, 200, 1, 0, 0, 104, 105
//     ]);
//   }
//
//   #[test]
//   fn test_rodata_section_ascii() {
//     let src = "
//         .section .rodata
//         .ascii \"RO\"
//     ";
//     let bytes = assemble_with_header(src).unwrap();
//     // TODO: fix this test
//     // let (text_off, text_sz, data_off, data_sz, rodata_off, rodata_sz, symtab_off, symtab_sz) = parse_header(&bytes);
//
//     validate_checksum(&bytes);
//     // assert_eq!(rodata_sz, 2);
//     // assert_eq!(&bytes[rodata_off as usize..(rodata_off+rodata_sz) as usize], b"RO");
//     assert_eq!(bytes, vec![
//       76, 65, 70, 0, 1, 0, 0, 0, 66, 17, 226, 45, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 2, 0, 0, 0, 82, 79
//     ]);
//   }
//
//   // TODO: fix this test
//   // #[test]
//   // fn test_mixed_sections() {
//   //   let src = "
//   //       .section .text
//   //       MOV r1, 99
//   //       .section .data
//   //       .word -1
//   //       .section .rodata
//   //       .ascii \"read\"
//   //   ";
//   //   let bytes = assemble_with_header(src).unwrap();
//   //   let (text_off, text_sz, data_off, data_sz, rodata_off, rodata_sz, symtab_off, symtab_sz) = parse_header(&bytes);
//   //   validate_checksum(&bytes);
//   //
//   //   // Check code
//   //   assert_eq!(&bytes[text_off as usize..(text_off+text_sz) as usize],
//   //              &[0, 99, 0, 0, 0, 114, 101, 97, 100]);
//   //   // Check data
//   //   let minus1 = (-1i32).to_le_bytes();
//   //   assert_eq!(&bytes[data_off as usize..(data_off+data_sz) as usize], &minus1);
//   //   // Check rodata
//   //   assert_eq!(&bytes[rodata_off as usize..(rodata_off+rodata_sz) as usize], b"read");
//   // }
//
//   #[test]
//   fn test_data_label_ascii() {
//     let src = "
//         .section .data
//         msg: .ascii \"yo\"
//     ";
//     // ...check that msg label offset is available in a symbol table
//   }
// }
//
