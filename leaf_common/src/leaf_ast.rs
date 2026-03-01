#[derive(Debug, Eq, PartialEq, Clone)]
pub enum OpCode {
  Add, Mul, Sub, Div,
  And, Or, Xor, Not,
  Lt, Gt, Eq,
  Jmp, Jz, Jnz,
  Mov, Load, Store,
  Movi, Loadi, Storei,
  Call, Ret,
  Push, Pop,
  Halt, Break,
  Syscall, Nop,
  Invalid,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Arg {
  Immediate(i32),
  Register(String),
  Label(String),
  Mem(Box<Arg>),
}

#[derive(Debug, Eq, PartialEq)]
pub struct Instruction {
  pub label: Option<String>,
  pub opcode: OpCode,
  pub args: Vec<Arg>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Directive {
  pub name: String,
  pub args: Option<String>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Line {
  Instruction(Instruction),
  LabelOnly(String),
  Directive(Directive),
  Section(String),
  Global(String),
  Extern(String),
}

impl OpCode {
  pub fn opcode_to_byte(opcode: &OpCode) -> u8 {
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
      OpCode::Lt => 0x19,
      OpCode::Gt => 0x1A,
      OpCode::Eq => 0x1B,
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
      OpCode::Movi => 0x16,
      OpCode::Loadi => 0x17,
      OpCode::Storei => 0x18,
      OpCode::Invalid => 0xFF,
    }
  }

  pub fn byte_to_opcode(byte: u8) -> Option<OpCode> {
    match byte {
      0x00 => Some(OpCode::Nop),
      0x01 => Some(OpCode::Add),
      0x02 => Some(OpCode::Sub),
      0x03 => Some(OpCode::Mul),
      0x04 => Some(OpCode::Div),
      0x05 => Some(OpCode::And),
      0x06 => Some(OpCode::Or),
      0x07 => Some(OpCode::Xor),
      0x08 => Some(OpCode::Not),
      0x09 => Some(OpCode::Jmp),
      0x0A => Some(OpCode::Jz),
      0x0B => Some(OpCode::Jnz),
      0x0C => Some(OpCode::Mov),
      0x0D => Some(OpCode::Load),
      0x0E => Some(OpCode::Store),
      0x0F => Some(OpCode::Call),
      0x10 => Some(OpCode::Ret),
      0x11 => Some(OpCode::Push),
      0x12 => Some(OpCode::Pop),
      0x13 => Some(OpCode::Halt),
      0x14 => Some(OpCode::Break),
      0x15 => Some(OpCode::Syscall),
      0x16 => Some(OpCode::Movi),
      0x17 => Some(OpCode::Loadi),
      0x18 => Some(OpCode::Storei),
      0x19 => Some(OpCode::Lt),
      0x1A => Some(OpCode::Gt),
      0x1B => Some(OpCode::Eq),
      _ => None,
    }
  }
}
