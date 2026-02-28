use log::{debug, error, info};
use leaf_common::leaf_ast::OpCode;
use leaf_common::leaf_file::LeafAsmFile;

pub struct VM {
  pub registers: [u64; 16],
  pub pc: usize,
  pub heap: Vec<u8>,
  pub halted: bool,
  pub code_len: usize,
  pub data_len: usize,
  pub rodata_len: usize,
  pub debug: bool,
}

impl VM {
  pub fn new(memory_size: usize) -> Self {
    VM {
      registers: [0; 16],
      pc: 0,
      heap: vec![0; memory_size],
      halted: false,
      code_len: 0,
      data_len: 0,
      rodata_len: 0,
      debug: true,
    }
  }

  pub fn load_program(&mut self, object: &LeafAsmFile) {

    disassembly_dump(&object, &self);

    // TODO: assert the CRC32 checksum
    if object.header.magic != *b"LAF\0" {
      error!("Magic flag does not match");
      panic!("Invalid magic number in object file");
    }
    if object.header.version != 1 {
      error!("Unsupported object file version: {}", object.header.version);
      panic!("Unsupported object file version: {}", object.header.version);
    }

    let code_len = object.object.bytecode.len();
    let data_len = object.object.data.len();
    let rodata_len = object.object.rodata.len();
    self.code_len = code_len;
    self.data_len = data_len;
    self.rodata_len = rodata_len;

    info!("Loading program with code length: {}, data length: {}, rodata length: {}", code_len, data_len, rodata_len);
    info!("Code: {:?}", object.object.bytecode);

    info!("Data: {:?}", object.object.data);
    info!("ROData: {:?}", object.object.rodata);

    self.heap = vec![0; code_len + data_len + rodata_len];
    self.heap[..code_len].copy_from_slice(object.object.bytecode.as_slice());
    self.heap[code_len..code_len + data_len].copy_from_slice(object.object.data.as_slice());
    self.heap[code_len + data_len..code_len + data_len + rodata_len].copy_from_slice(object.object.rodata.as_slice());
    self.pc = 0;

    if let Some(entry) = &object.object.entry_point {
      if let Some(symbol) = object.object.symbols.iter().find(|s| s.name == *entry) {
        self.pc = symbol.offset as usize;
      } else {
        error!("Entry point '{}' not found in symbols", entry);
        panic!("Entry point '{}' not found in symbols", entry);
      }
    }
  }

  pub fn run(&mut self) {
    info!("Code: {:?}", self.heap);
    self.registers[15] = self.heap.len() as u64;
    while !self.halted {
      self.step();
    }
  }

  pub fn step(&mut self) {

    if self.debug {
      // Debug dump
      info!("PC={:04X}: {}", self.pc, self.disassemble());
      info!("  Registers: {:?}", self.registers);
      info!("  Stack pointer: r15={}", self.registers[15]);
      info!("  Heap (code)   [{}..{}]: {:?}", 0, self.code_len, &self.heap[0..self.code_len.min(self.heap.len())]);
      info!("  Heap (data)   [{}..{}]: {:?}", self.code_len, self.code_len+self.data_len, &self.heap[self.code_len..(self.code_len+self.data_len).min(self.heap.len())]);
      info!("  Heap (rodata) [{}..{}]: {:?}", self.code_len+self.data_len, self.code_len+self.data_len+self.rodata_len, &self.heap[(self.code_len+self.data_len)..(self.code_len+self.data_len+self.rodata_len).min(self.heap.len())]);

      // Optionally, for a more compact/less verbose print:
      info!("  Heap (data, as string): {}", String::from_utf8_lossy(&self.heap[self.code_len..self.code_len+self.data_len]));
      info!("  Heap (rodata, as string): {}", String::from_utf8_lossy(&self.heap[self.code_len+self.data_len..self.code_len+self.data_len+self.rodata_len]));
    }

    if self.pc >= self.code_len {
      error!("PC {} out of code section! (code_len={})", self.pc, self.code_len);
      panic!("PC out of bounds: {}", self.pc);
    }

    let opcode_byte = self.heap[self.pc];
    if let None = OpCode::byte_to_opcode(opcode_byte) {
      error!("Invalid opcode: {:02X} at pc={:04X}", opcode_byte, self.pc);
      let start = self.pc.saturating_sub(16);
      let end = (self.pc + 16).min(self.code_len);
      error!("Surrounding code bytes: {:?}", &self.heap[start..end]);
    }
    let opcode = match OpCode::byte_to_opcode(opcode_byte) {
      Some(op) => op,
      None => {
        error!("Invalid opcode: {:02X} at pc={:04X} -- halting", opcode_byte, self.pc);
        self.halted = true;
        return;
      }
    };
    // self.halted = true; if opcode error
    debug!("Executing opcode {:?} at pc={}", opcode, self.pc);
    match opcode {
      OpCode::Invalid => {
        self.pc += 1;
      }
      OpCode::Add => {
        // ADD r1, r2, r3  --> r1 = r2 + r3
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2.wrapping_add(v3));
        self.pc += 1 + 4 * 3;
      }
      OpCode::Mul => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2.wrapping_mul(v3));
        self.pc += 1 + 4 * 3;
      }
      OpCode::Sub => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2.wrapping_sub(v3));
        self.pc += 1 + 4 * 3;
      }
      OpCode::Div => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        if v3 == 0 {
          error!("Division by zero at pc={}", self.pc);
          self.halted = true;
          return;
        }
        self.set_reg(r1, v2 / v3);
        self.pc += 1 + 4 * 3;
      }
      OpCode::And => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2 & v3);
        self.pc += 1 + 4 * 3;
      }
      OpCode::Or => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2 | v3);
        self.pc += 1 + 4 * 3;
      }
      OpCode::Xor => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2 ^ v3);
        self.pc += 1 + 4 * 3;
      }
      OpCode::Not => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let v2 = self.registers[r2];
        self.set_reg(r1, !v2);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Jmp => {
        // JMP addr
        let target = self.fetch_u32(self.pc + 1) as usize;
        self.pc = target;
      }
      OpCode::Jz => {
        // JZ r1, addr  -- jump if r1 == 0
        let r1 = self.fetch_reg(self.pc + 1);
        let target = self.fetch_u32(self.pc + 5) as usize;
        if self.registers[r1] == 0 {
          self.pc = target;
        } else {
          self.pc += 1 + 4 * 2;
        }
      }
      OpCode::Jnz => {
        // JNZ r1, addr  -- jump if r1 != 0
        let r1 = self.fetch_reg(self.pc + 1);
        let target = self.fetch_u32(self.pc + 5) as usize;
        if self.registers[r1] != 0 {
          self.pc = target;
        } else {
          self.pc += 1 + 4 * 2;
        }
      }
      OpCode::Mov => {
        // MOV r1, r2  --> r1 = r2
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let v2 = self.registers[r2];
        self.set_reg(r1, v2);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Load => {
        // LOAD r1, [r2]
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let addr = self.registers[r2] as usize;
        if addr + 8 > self.heap.len() {
          error!("LOAD out of bounds: addr={}", addr);
          self.halted = true;
          return;
        }
        let value = u64::from_le_bytes([
          self.heap[addr], self.heap[addr + 1], self.heap[addr + 2], self.heap[addr + 3],
          self.heap[addr + 4], self.heap[addr + 5], self.heap[addr + 6], self.heap[addr + 7],
        ]);
        self.set_reg(r1, value);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Store => {
        // STORE r1, [r2]
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let addr = self.registers[r2] as usize;
        if addr + 8 > self.heap.len() {
          error!("STORE out of bounds: addr={}", addr);
          self.halted = true;
          return;
        }
        let value = self.registers[r1].to_le_bytes();
        self.heap[addr..addr + 8].copy_from_slice(&value);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Movi => {
        // MOVI r1, imm  --> r1 = imm
        let r1 = self.fetch_reg(self.pc + 1);
        let imm = self.fetch_u32(self.pc + 5) as u64;
        self.set_reg(r1, imm);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Loadi => {
        // LOADI r1, addr  --> r1 = [addr]
        let r1 = self.fetch_reg(self.pc + 1);
        let addr = self.fetch_u32(self.pc + 5) as usize;
        if addr + 8 > self.heap.len() {
          error!("LOADI out of bounds: addr={}", addr);
          self.halted = true;
          return;
        }
        let value = u64::from_le_bytes([
          self.heap[addr], self.heap[addr + 1], self.heap[addr + 2], self.heap[addr + 3],
          self.heap[addr + 4], self.heap[addr + 5], self.heap[addr + 6], self.heap[addr + 7],
        ]);
        self.set_reg(r1, value);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Storei => {
        // STOREI r1, addr  --> [addr] = r1
        let r1 = self.fetch_reg(self.pc + 1);
        let addr = self.fetch_u32(self.pc + 5) as usize;
        if addr + 8 > self.heap.len() {
          error!("STOREI out of bounds: addr={}", addr);
          self.halted = true;
          return;
        }
        let value = self.registers[r1].to_le_bytes();
        self.heap[addr..addr + 8].copy_from_slice(&value);
        self.pc += 1 + 4 * 2;
      }
      OpCode::Call => {
        // CALL addr: push next_pc, then jump
        debug!("Heap: {:?}", self.heap);
        debug!("Registers: {:?}", self.registers);
        debug!("CALL instruction at PC={}", self.pc);
        let addr = self.fetch_u32(self.pc + 1) as usize;
        debug!("Calling function at address {}", addr);
        let sp = self.registers[15] as usize;
        debug!("Calling arguments at address {}", sp);
        if sp < 8 {
          error!("Stack overflow in CALL!");
          self.halted = true;
          return;
        }
        let return_addr = (self.pc + 1 + 4) as u64;
        self.heap[sp - 8..sp].copy_from_slice(&return_addr.to_le_bytes());
        self.registers[15] = (sp - 8) as u64;
        self.pc = addr;
      }
      OpCode::Ret => {
        // RET: pop PC from stack
        let sp = self.registers[15] as usize;
        if sp + 8 > self.heap.len() {
          error!("Stack underflow in RET!");
          self.halted = true;
          return;
        }
        let return_addr = u64::from_le_bytes([
          self.heap[sp], self.heap[sp + 1], self.heap[sp + 2], self.heap[sp + 3],
          self.heap[sp + 4], self.heap[sp + 5], self.heap[sp + 6], self.heap[sp + 7],
        ]);
        self.registers[15] = (sp + 8) as u64;
        self.pc = return_addr as usize;
      }
      OpCode::Push => {
        // PUSH r1  --> [SP] = r1; SP -= 8
        let r1 = self.fetch_reg(self.pc + 1);
        let sp = self.registers[15] as usize;
        if sp < 8 {
          error!("Stack overflow!");
          self.halted = true;
          return;
        }
        let value = self.registers[r1].to_le_bytes();
        self.heap[sp - 8..sp].copy_from_slice(&value);
        self.registers[15] = (sp - 8) as u64;
        self.pc += 1 + 4;
      }
      OpCode::Pop => {
        // POP r1  --> r1 = [SP]; SP += 8
        let r1 = self.fetch_reg(self.pc + 1);
        let sp = self.registers[15] as usize;
        if sp + 8 > self.heap.len() {
          error!("Stack underflow!");
          self.halted = true;
          return;
        }
        let value = u64::from_le_bytes([
          self.heap[sp], self.heap[sp + 1], self.heap[sp + 2], self.heap[sp + 3],
          self.heap[sp + 4], self.heap[sp + 5], self.heap[sp + 6], self.heap[sp + 7],
        ]);
        self.set_reg(r1, value);
        self.registers[15] = (sp + 8) as u64;
        self.pc += 1 + 4;
      }
      OpCode::Halt => {
        self.halted = true;
        info!("Halting execution");
      }
      OpCode::Break => {
        info!("Breakpoint reached at PC={}", self.pc);
        self.halted = true; // or pause depending on the design
      }
      OpCode::Syscall => {
        debug!("SYSCALL called at PC={}", self.pc);
        debug!("Registers: {:?}", self.registers);
        debug!("Heap: {:?}", self.heap);
        let syscall_num = self.registers[0];
        match syscall_num {
          1 => {
            let ptr = self.registers[1] as usize;
            let mut s = Vec::new();
            let mut i = ptr;
            // Read bytes until null terminator or heap end
            while i < self.heap.len() && self.heap[i] != 0 {
              s.push(self.heap[i]);
              i += 1;
            }
            let s = String::from_utf8_lossy(&s);
            println!("{}", s);
          }
          2 => {
            println!("{}", self.registers[1]);
          }
          3 => {
            let code = self.registers[1];
            info!("Exiting with code {}", code);
            self.halted = true;
          }
          _ => {
            error!("Unknown syscall number: {}", syscall_num);
            // self.halted = true;
          }
        }
        self.pc += 1;
      }
      OpCode::Nop => {
        self.pc += 1;
      }
    }
  }

  fn fetch_u32(&self, offset: usize) -> u32 {
    u32::from_le_bytes([
      self.heap[offset],
      self.heap[offset + 1],
      self.heap[offset + 2],
      self.heap[offset + 3],
    ])
  }

  // Helper: Fetch a register index (from the first byte of a 4-byte arg)
  fn fetch_reg(&self, offset: usize) -> usize {
    self.heap[offset] as usize
  }
  // Helper: Write to a register
  fn set_reg(&mut self, reg: usize, value: u64) {
    if reg < self.registers.len() {
      self.registers[reg] = value;
    } else {
      error!("Invalid register: {}", reg);
      self.halted = true;
    }
  }

  fn disassemble(&self) -> String {
    let pc = self.pc;
    let op = OpCode::byte_to_opcode(self.heap[pc]).unwrap_or(OpCode::Nop);
    match op {
      OpCode::Movi => {
        let reg = self.fetch_reg(pc + 1);
        let imm = self.fetch_u32(pc + 5);
        format!("MOVI r{}, {}", reg, imm)
      }
      OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
      OpCode::And | OpCode::Or | OpCode::Xor => {
        let r1 = self.fetch_reg(pc + 1);
        let r2 = self.fetch_reg(pc + 5);
        let r3 = self.fetch_reg(pc + 9);
        format!("{:?} r{}, r{}, r{}", op, r1, r2, r3)
      }
      OpCode::Load | OpCode::Store => {
        let r1 = self.fetch_reg(pc + 1);
        let r2 = self.fetch_reg(pc + 5);
        format!("{:?} r{}, [r{}]", op, r1, r2)
      }
      OpCode::Loadi | OpCode::Storei => {
        let r1 = self.fetch_reg(pc + 1);
        let addr = self.fetch_u32(pc + 5);
        let what = self.describe_addr(addr as usize);
        format!("{:?} r{}, [{}] ({})", op, r1, addr, what)
      }
      OpCode::Syscall => format!("SYSCALL"),
      OpCode::Jmp => {
        let addr = self.fetch_u32(pc + 1);
        let what = self.describe_addr(addr as usize);
        format!("JMP {} ({})", addr, what)
      }
      OpCode::Halt => "HALT".to_string(),
      _ => format!("{:?}", op),
    }
  }

  fn describe_addr(&self, addr: usize) -> String {
    if addr < self.code_len {
      format!(".text+{}", addr)
    } else if addr < self.code_len + self.data_len {
      let off = addr - self.code_len;
      // Try to show data, as string if printable
      let mut end = off;
      while end < self.data_len && self.heap[self.code_len + end] != 0 {
        end += 1;
      }
      let data = &self.heap[addr..(addr + (end - off)).min(self.heap.len())];
      let text = String::from_utf8_lossy(data);
      format!(".data+{} ('{}')", off, text)
    } else if addr < self.code_len + self.data_len + self.rodata_len {
      let off = addr - self.code_len - self.data_len;
      let mut end = off;
      while end < self.rodata_len && self.heap[self.code_len + self.data_len + end] != 0 {
        end += 1;
      }
      let data = &self.heap[addr..(addr + (end - off)).min(self.heap.len())];
      let text = String::from_utf8_lossy(data);
      format!(".rodata+{} ('{}')", off, text)
    } else {
      format!("heap+{}", addr)
    }
  }
}

pub fn disassembly_dump(object: &LeafAsmFile, vm: &VM) {
  let code = &object.object.bytecode;
  let mut pc = 0usize;

  info!("offset | bytes                                    | expected");
  info!("-----------------------------------------------------------------------");

  while pc < code.len() {
    let offset = pc;
    let (disasm, instr_len) = disassemble_at(vm, code, pc);
    let instr_bytes: Vec<String> = code[pc..pc + instr_len.min(code.len() - pc)]
      .iter()
      .map(|b| format!("{:02X}", b))
      .collect();

    info!("0x{:04X} | {:<40} | {}", offset, instr_bytes.join(" "), disasm);

    pc += instr_len;
  }
}

fn disassemble_at(vm: &VM, code: &[u8], pc: usize) -> (String, usize) {
  if pc >= code.len() {
    return ("<invalid PC>".to_string(), 1);
  }

  let op = OpCode::byte_to_opcode(code[pc]).unwrap_or(OpCode::Invalid);

  match op {
    OpCode::Movi => {
      // MOVI reg, imm/label
      if pc + 9 <= code.len() {
        let reg = code[pc + 1];
        let imm = u32::from_le_bytes([code[pc + 5], code[pc + 6], code[pc + 7], code[pc + 8]]);
        (format!("MOVI r{}, {}", reg, imm), 9)
      } else {
        ("MOVI <truncated>".to_string(), code.len() - pc)
      }
    }
    OpCode::Mov => {
      if pc + 9 <= code.len() {
        let r1 = code[pc + 1];
        let r2 = code[pc + 5];
        (format!("MOV r{}, r{}", r1, r2), 9)
      } else {
        ("MOV <truncated>".to_string(), code.len() - pc)
      }
    }
    OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
    OpCode::And | OpCode::Or | OpCode::Xor => {
      if pc + 13 <= code.len() {
        let r1 = code[pc + 1];
        let r2 = code[pc + 5];
        let r3 = code[pc + 9];
        (format!("{:?} r{}, r{}, r{}", op, r1, r2, r3), 13)
      } else {
        (format!("{:?} <truncated>", op), code.len() - pc)
      }
    }
    OpCode::Not => {
      if pc + 9 <= code.len() {
        let r1 = code[pc + 1];
        let r2 = code[pc + 5];
        (format!("NOT r{}, r{}", r1, r2), 9)
      } else {
        ("NOT <truncated>".to_string(), code.len() - pc)
      }
    }
    OpCode::Jmp => {
      if pc + 5 <= code.len() {
        let addr = u32::from_le_bytes([code[pc + 1], code[pc + 2], code[pc + 3], code[pc + 4]]);
        (format!("JMP {}", addr), 5)
      } else {
        ("JMP <truncated>".to_string(), code.len() - pc)
      }
    }
    OpCode::Jz | OpCode::Jnz => {
      if pc + 9 <= code.len() {
        let r1 = code[pc + 1];
        let addr = u32::from_le_bytes([code[pc + 5], code[pc + 6], code[pc + 7], code[pc + 8]]);
        (format!("{:?} r{}, {}", op, r1, addr), 9)
      } else {
        (format!("{:?} <truncated>", op), code.len() - pc)
      }
    }
    OpCode::Load | OpCode::Store => {
      if pc + 9 <= code.len() {
        let r1 = code[pc + 1];
        let r2 = code[pc + 5];
        (format!("{:?} r{}, [r{}]", op, r1, r2), 9)
      } else {
        (format!("{:?} <truncated>", op), code.len() - pc)
      }
    }
    OpCode::Loadi | OpCode::Storei => {
      if pc + 9 <= code.len() {
        let r1 = code[pc + 1];
        let addr = u32::from_le_bytes([code[pc + 5], code[pc + 6], code[pc + 7], code[pc + 8]]);
        (format!("{:?} r{}, [{}]", op, r1, addr), 9)
      } else {
        (format!("{:?} <truncated>", op), code.len() - pc)
      }
    }
    OpCode::Call => {
      if pc + 5 <= code.len() {
        let addr = u32::from_le_bytes([code[pc + 1], code[pc + 2], code[pc + 3], code[pc + 4]]);
        (format!("CALL {}", addr), 5)
      } else {
        ("CALL <truncated>".to_string(), code.len() - pc)
      }
    }
    OpCode::Ret | OpCode::Break | OpCode::Halt | OpCode::Syscall | OpCode::Nop => {
      (format!("{:?}", op), 1)
    }
    OpCode::Push | OpCode::Pop => {
      if pc + 5 <= code.len() {
        let r1 = code[pc + 1];
        (format!("{:?} r{}", op, r1), 5)
      } else {
        (format!("{:?} <truncated>", op), code.len() - pc)
      }
    }
    _ => ("<invalid>".to_string(), 1)
  }
}
