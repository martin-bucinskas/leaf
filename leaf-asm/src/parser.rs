use pest::Parser;
use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;
use crate::ast::{Line, Instruction, OpCode, Arg, Directive};

#[derive(Parser)]
#[grammar = "grammar/leaf_asm.pest"]
pub struct LeafAsmParser;

pub fn parse_program(source: &str) -> Result<Vec<Line>, String> {
  let pairs = LeafAsmParser::parse(Rule::program, source)
    .map_err(|e| format!("Parse error: {}", e))?;
  let mut lines = Vec::new();

  for pair in pairs {
    match pair.as_rule() {
      Rule::program => {
        for item in pair.into_inner() {
          match item.as_rule() {
            Rule::line | Rule::last_line => {
              if let Some(line) = parse_line(item) {
                lines.push(line);
              }
            }
            _ => {}
          }
        }
      }
      _ => {}
    }
  }

  Ok(lines)
}

fn parse_line(pair: Pair<Rule>) -> Option<Line> {
  match pair.as_rule() {
    Rule::line | Rule::last_line => {
      let mut inner = pair.into_inner();
      match inner.next() {
        Some(l) => match l.as_rule() {
          Rule::label_only => {
            let ident = l.into_inner().next().unwrap().as_str();
            Some(Line::LabelOnly(ident.to_string()))
          }
          Rule::instruction_decl => Some(parse_instruction_decl(l)),
          Rule::directive => Some(parse_directive(l)),
          _ => None,
        },
        None => None,
      }
    }
    _ => None,
  }
}

fn parse_directive(pair: Pair<Rule>) -> Line {
  let mut inner = pair.into_inner();
  let name = inner.next().unwrap().as_str().to_string();
  let args = inner.next().map(|p| p.as_str().trim().to_string());

  match name.as_str() {
    "section" => Line::Section(args.unwrap_or_default()),
    "global"  => Line::Global(args.unwrap_or_default()),
    _         => Line::Directive(Directive { name, args }),
  }
}

fn parse_instruction_decl(pair: Pair<Rule>) -> Line {
  let mut inner = pair.clone().into_inner().peekable();
  let mut label = None;
  let mut opcode_str = None;
  let mut args = Vec::new();

  // If label_prefix exists, it's first
  if let Some(peek) = inner.peek() {
    if peek.as_rule() == Rule::label_prefix {
      let prefix = inner.next().unwrap();
      label = Some(prefix.into_inner().next().unwrap().as_str().to_string());
    }
  }

  // At this point, the next part of the string is the opcode (as a slice of the parent)
  // Get the original str slice, subtract label if present, and trim
  let full_str = pair.as_str();
  let mut rest = full_str;

  if let Some(ref l) = label {
    // Find and skip label prefix in string
    let label_part = format!("{}:", l);
    if rest.starts_with(&label_part) {
      rest = &rest[label_part.len()..];
    }
  }
  // Remove leading whitespace
  rest = rest.trim_start();

  // Now the opcode is at the start; let's find the first space or comma or EOL
  let mut opcode_end = 0;
  for (i, c) in rest.char_indices() {
    if c.is_whitespace() || c == ',' {
      opcode_end = i;
      break;
    }
  }
  if opcode_end == 0 {
    // opcode is up to end
    opcode_end = rest.len();
  }
  let opcode = &rest[..opcode_end].trim();
  opcode_str = Some(opcode.to_string());

  // The remaining pairs (if any) are arg_list
  while let Some(pair) = inner.next() {
    match pair.as_rule() {
      Rule::arg_list => {
        args = pair.into_inner().map(parse_arg).collect();
      }
      _ => {
        // Comments or similar, skip
      }
    }
  }

  Line::Instruction(Instruction {
    label,
    opcode: parse_opcode(&opcode_str.expect("opcode required")),
    args,
  })
}



fn parse_opcode(s: &str) -> OpCode {
  match s {
    "ADD" => OpCode::Add,
    "SUB" => OpCode::Sub,
    "MUL" => OpCode::Mul,
    "DIV" => OpCode::Div,
    "AND" => OpCode::And,
    "OR" => OpCode::Or,
    "XOR" => OpCode::Xor,
    "NOT" => OpCode::Not,
    "JMP" => OpCode::Jmp,
    "JZ" => OpCode::Jz,
    "JNZ" => OpCode::Jnz,
    "MOV" => OpCode::Mov,
    "LOAD" => OpCode::Load,
    "STORE" => OpCode::Store,
    "CALL" => OpCode::Call,
    "RET" => OpCode::Ret,
    "PUSH" => OpCode::Push,
    "POP" => OpCode::Pop,
    "HALT" => OpCode::Halt,
    "BREAK" => OpCode::Break,
    "SYSCALL" => OpCode::Syscall,
    "NOP" => OpCode::Nop,
    _ => panic!("Unknown opcode: {s}"),
  }
}

fn parse_arg(pair: Pair<Rule>) -> Arg {
  match pair.as_rule() {
    Rule::num => {
      let n: i32 = pair.as_str().parse().unwrap();
      Arg::Immediate(n)
    }
    Rule::register => Arg::Register(pair.as_str().to_string()),
    Rule::ident => Arg::Label(pair.as_str().to_string()),
    Rule::mem => {
      let inner = pair.into_inner().next().unwrap();
      match inner.as_rule() {
        Rule::register => Arg::Mem(Box::new(Arg::Register(inner.as_str().to_string()))),
        Rule::num => {
          let n: i32 = inner.as_str().parse().unwrap();
          Arg::Mem(Box::new(Arg::Immediate(n)))
        }
        _ => panic!("Unexpected memory argument: {:?}", inner.as_rule()),
      }
    }
    _ => panic!("Unexpected rule in argument: {:?}", pair.as_rule()),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::{Line, Instruction, OpCode, Arg};

  #[test]
  fn parse_simple_add() {
    let asm = "ADD r1, r2, r3";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Add);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Register("r2".to_string()),
          Arg::Register("r3".to_string()),
        ]);
        assert_eq!(instr.label, None);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_label_only() {
    let asm = "start:";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines, vec![
      Line::LabelOnly("start".to_string())
    ]);
  }

  #[test]
  fn parse_label_prefixed_instruction() {
    let asm = "start: MOV r1, r2";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.label, Some("start".to_string()));
        assert_eq!(instr.opcode, OpCode::Mov);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Register("r2".to_string())
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_label_prefixed_instruction_with_newline() {
    let asm = "start: MOV r1, r2\n";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.label, Some("start".to_string()));
        assert_eq!(instr.opcode, OpCode::Mov);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Register("r2".to_string())
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_immediate_args() {
    let asm = "ADD r1, -42, 99";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Add);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Immediate(-42),
          Arg::Immediate(99),
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_label_arg() {
    let asm = "JMP start";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Jmp);
        assert_eq!(instr.args, vec![
          Arg::Label("start".to_string()),
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_instruction_with_comment() {
    let asm = "ADD r1, r2 ; this is a comment";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Add);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Register("r2".to_string()),
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_whitespace_and_empty_lines() {
    let asm = "\n  \nADD r1, r2\n\n  SUB r3, 1  \n\n";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 2);
    match &lines[0] {
      Line::Instruction(instr) => assert_eq!(instr.opcode, OpCode::Add),
      _ => panic!("Expected instruction"),
    }
    match &lines[1] {
      Line::Instruction(instr) => assert_eq!(instr.opcode, OpCode::Sub),
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_zero_arg_instruction() {
    let asm = "HALT";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Halt);
        assert_eq!(instr.args.len(), 0);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_single_arg_instruction() {
    let asm = "PUSH r5";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 1);
    match &lines[0] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Push);
        assert_eq!(instr.args, vec![Arg::Register("r5".to_string())]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_mixed_labels_and_instructions() {
    let asm = "
        start:
        MOV r1, 5
        ADD r2, r1, r1
        JMP start
        ";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], Line::LabelOnly("start".to_string()));
    match &lines[1] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Mov);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Immediate(5)
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }

  #[test]
  fn parse_mixed_labels_and_instructions_complex() {
    let asm = "
        start:
        MOV r1, 5
        add: ADD r2, r1, r1
        JMP start
        ";
    let lines = parse_program(asm).unwrap();
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[0], Line::LabelOnly("start".to_string()));
    match &lines[1] {
      Line::Instruction(instr) => {
        assert_eq!(instr.opcode, OpCode::Mov);
        assert_eq!(instr.args, vec![
          Arg::Register("r1".to_string()),
          Arg::Immediate(5)
        ]);
      }
      _ => panic!("Expected instruction"),
    }
  }
}