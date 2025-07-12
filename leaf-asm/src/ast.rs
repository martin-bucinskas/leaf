#[derive(Debug, Eq, PartialEq)]
pub enum OpCode {
  Add, Mul, Sub, Div,
  And, Or, Xor, Not,
  Jmp, Jz, Jnz,
  Mov, Load, Store,
  Call, Ret,
  Push, Pop,
  Halt, Break,
  Syscall, Nop,
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
