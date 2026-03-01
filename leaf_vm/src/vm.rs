use log::{debug, error, info};
use leaf_common::leaf_ast::OpCode;
use leaf_common::leaf_file::LeafAsmFile;

pub struct VM {
  pub registers: [u64; 32],
  pub pc: usize,
  pub heap: Vec<u8>,
  pub halted: bool,
  pub code_len: usize,
  pub data_len: usize,
  pub rodata_len: usize,
  pub debug: bool,
  pub file_descriptors: std::collections::HashMap<u64, std::fs::File>,
  pub next_fd: u64,
}

impl VM {
  pub fn new(memory_size: usize) -> Self {
    VM {
      registers: [0; 32],
      pc: 0,
      heap: vec![0; memory_size],
      halted: false,
      code_len: 0,
      data_len: 0,
      rodata_len: 0,
      debug: true,
      file_descriptors: std::collections::HashMap::new(),
      next_fd: 3,
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
    
    // Ensure heap is large enough
    let total_required = code_len + data_len + rodata_len;
    if total_required > self.heap.len() {
        self.heap.resize(total_required + 0x1000, 0); // Add some padding for stack if needed
    } else {
        // Zero out the portion of the heap we will use
        for i in 0..total_required {
            self.heap[i] = 0;
        }
    }

    self.heap[..code_len].copy_from_slice(object.object.bytecode.as_slice());
    self.heap[code_len..code_len + data_len].copy_from_slice(object.object.data.as_slice());
    self.heap[code_len + data_len..code_len + data_len + rodata_len].copy_from_slice(object.object.rodata.as_slice());

    // Apply relocations
    for reloc in &object.object.relocations {
      let symbol = &object.object.symbols[reloc.symbol_index as usize];
      let section_offset = match symbol.section {
        0 => 0,
        1 => code_len,
        2 => code_len + data_len,
        _ => panic!("Invalid symbol section: {}", symbol.section),
      };
      let target_addr = (section_offset + symbol.offset as usize) as u32;

      let patch_section_offset = match reloc.target_section {
        0 => 0,
        1 => code_len,
        2 => code_len + data_len,
        _ => panic!("Invalid relocation target section: {}", reloc.target_section),
      };
      let patch_addr = patch_section_offset + reloc.offset as usize;

      if patch_addr + 4 > self.heap.len() {
        error!("Relocation out of bounds: patch_addr={}", patch_addr);
        panic!("Relocation out of bounds: patch_addr={}", patch_addr);
      }

      info!("Applying relocation at {:04X}: symbol '{}' at section {} offset {} (target_addr={:04X})",
        patch_addr, symbol.name, symbol.section, symbol.offset, target_addr);

      let bytes = target_addr.to_le_bytes();
      self.heap[patch_addr..patch_addr + 4].copy_from_slice(&bytes);
    }

    self.pc = 0;

    if let Some(entry) = &object.object.entry_point {
      if let Some(symbol) = object.object.symbols.iter().find(|s| s.name == *entry) {
        let section_offset = match symbol.section {
          0 => 0,
          1 => code_len,
          2 => code_len + data_len,
          _ => 0,
        };
        self.pc = section_offset + symbol.offset as usize;
      } else {
        error!("Entry point '{}' not found in symbols", entry);
        panic!("Entry point '{}' not found in symbols", entry);
      }
    }
  }

  pub fn run(&mut self) {
    info!("Heap initialized, size={}", self.heap.len());
    self.registers[15] = self.heap.len() as u64;
    while !self.halted {
      self.step();
    }
  }

  pub fn step(&mut self) {

    if self.pc >= self.code_len {
      info!("Reached end of code section at PC={:04X}. Halting.", self.pc);
      self.halted = true;
      return;
    }

    let opcode_byte = self.heap[self.pc];
    let opcode = match OpCode::byte_to_opcode(opcode_byte) {
      Some(op) => op,
      None => {
        error!("Invalid opcode: {:02X} at pc={:04X} -- halting", opcode_byte, self.pc);
        self.halted = true;
        return;
      }
    };

    if self.debug {
      // Debug dump
      info!("PC={:04X}: byte={:02X} op={:?} disasm={}", self.pc, opcode_byte, opcode, self.disassemble());
    }
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
        self.pc += 13;
      }
      OpCode::Mul => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2.wrapping_mul(v3));
        self.pc += 13;
      }
      OpCode::Sub => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2.wrapping_sub(v3));
        self.pc += 13;
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
        self.pc += 13;
      }
      OpCode::Lt => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2] as i64;
        let v3 = self.registers[r3] as i64;
        self.set_reg(r1, if v2 < v3 { 1 } else { 0 });
        self.pc += 13;
      }
      OpCode::Gt => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2] as i64;
        let v3 = self.registers[r3] as i64;
        self.set_reg(r1, if v2 > v3 { 1 } else { 0 });
        self.pc += 13;
      }
      OpCode::Eq => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, if v2 == v3 { 1 } else { 0 });
        self.pc += 13;
      }
      OpCode::And => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2 & v3);
        self.pc += 13;
      }
      OpCode::Or => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2 | v3);
        self.pc += 13;
      }
      OpCode::Xor => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let r3 = self.fetch_reg(self.pc + 9);
        let v2 = self.registers[r2];
        let v3 = self.registers[r3];
        self.set_reg(r1, v2 ^ v3);
        self.pc += 13;
      }
      OpCode::Not => {
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let v2 = self.registers[r2];
        self.set_reg(r1, !v2);
        self.pc += 9;
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
          self.pc += 9;
        }
      }
      OpCode::Jnz => {
        // JNZ r1, addr  -- jump if r1 != 0
        let r1 = self.fetch_reg(self.pc + 1);
        let target = self.fetch_u32(self.pc + 5) as usize;
        if self.registers[r1] != 0 {
          self.pc = target;
        } else {
          self.pc += 9;
        }
      }
      OpCode::Mov => {
        // MOV r1, r2  --> r1 = r2
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let v2 = self.registers[r2];
        self.set_reg(r1, v2);
        self.pc += 9;
      }
      OpCode::Load => {
        // LOAD r1, [r2]
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let addr = self.registers[r2] as usize;
        if addr + 8 > self.heap.len() {
          error!("LOAD out of bounds: addr={} (heap len={})", addr, self.heap.len());
          self.halted = true;
          return;
        }
        let value = u64::from_le_bytes([
          self.heap[addr], self.heap[addr + 1], self.heap[addr + 2], self.heap[addr + 3],
          self.heap[addr + 4], self.heap[addr + 5], self.heap[addr + 6], self.heap[addr + 7],
        ]);
        self.set_reg(r1, value);
        self.pc += 9;
      }
      OpCode::Store => {
        // STORE r1, [r2]
        let r1 = self.fetch_reg(self.pc + 1);
        let r2 = self.fetch_reg(self.pc + 5);
        let addr = self.registers[r2] as usize;
        if addr + 8 > self.heap.len() {
          error!("STORE out of bounds: addr={} (heap len={})", addr, self.heap.len());
          self.halted = true;
          return;
        }
        let value = self.registers[r1].to_le_bytes();
        self.heap[addr..addr + 8].copy_from_slice(&value);
        self.pc += 9;
      }
      OpCode::Movi => {
        // MOVI r1, imm  --> r1 = imm
        let r1 = self.fetch_reg(self.pc + 1);
        let imm = self.fetch_u32(self.pc + 5) as u64;
        self.set_reg(r1, imm);
        self.pc += 9;
      }
      OpCode::Loadi => {
        // LOADI r1, addr  --> r1 = [addr]
        let r1 = self.fetch_reg(self.pc + 1);
        let addr = self.fetch_u32(self.pc + 5) as usize;
        if addr + 8 > self.heap.len() {
          error!("LOADI out of bounds: addr={} (heap len={})", addr, self.heap.len());
          self.halted = true;
          return;
        }
        let value = u64::from_le_bytes([
          self.heap[addr], self.heap[addr + 1], self.heap[addr + 2], self.heap[addr + 3],
          self.heap[addr + 4], self.heap[addr + 5], self.heap[addr + 6], self.heap[addr + 7],
        ]);
        self.set_reg(r1, value);
        self.pc += 9;
      }
      OpCode::Storei => {
        // STOREI r1, addr  --> [addr] = r1
        let r1 = self.fetch_reg(self.pc + 1);
        let addr = self.fetch_u32(self.pc + 5) as usize;
        if addr + 8 > self.heap.len() {
          error!("STOREI out of bounds: addr={} (heap len={})", addr, self.heap.len());
          self.halted = true;
          return;
        }
        let value = self.registers[r1].to_le_bytes();
        self.heap[addr..addr + 8].copy_from_slice(&value);
        self.pc += 9;
      }
      OpCode::Call => {
        // CALL addr: push next_pc, then jump
        let addr = self.fetch_u32(self.pc + 1) as usize;
        let sp = self.registers[15] as usize;
        if sp < 8 {
          error!("Stack overflow in CALL!");
          self.halted = true;
          return;
        }
        let return_addr = (self.pc + 5) as u64;
        info!("CALL at PC={:04X}: target={:04X}, pushing return_addr={:04X}, sp={:04X}", self.pc, addr, return_addr, sp);
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
        info!("RET at PC={:04X}: popping return_addr={:04X}, sp={:04X}", self.pc, return_addr, sp);
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
        self.pc += 5;
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
        self.pc += 5;
      }
      OpCode::Halt => {
        self.halted = true;
        self.pc += 1;
        info!("Halting execution");
      }
      OpCode::Break => {
        info!("Breakpoint reached at PC={}", self.pc);
        self.pc += 1;
        self.halted = true; // or pause depending on the design
      }
      OpCode::Syscall => {
        debug!("SYSCALL called at PC={}", self.pc);
        debug!("Registers: {:?}", self.registers);
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
            print!("{}", s); // Use print! instead of println! to respect \n in string
          }
          2 => {
            println!("{}", self.registers[1]);
          }
          3 => {
            let code = self.registers[1];
            info!("Exiting with code {}", code);
            self.halted = true;
          }
          4 => {
            // READ fd, buf_ptr, count
            let fd = self.registers[1];
            let buf_ptr = self.registers[2] as usize;
            let count = self.registers[3] as usize;
            
            if buf_ptr.checked_add(count).map_or(true, |end| end > self.heap.len()) {
              error!("READ out of bounds or overflow: buf_ptr={}, count={}, heap_len={}", buf_ptr, count, self.heap.len());
              self.registers[0] = (-1i64) as u64; // Return -1 on error
            } else {
              match fd {
                0 => {
                  use std::io::Read;
                  let mut buf = vec![0u8; count];
                  match std::io::stdin().read(&mut buf) {
                    Ok(n) => {
                      self.heap[buf_ptr..buf_ptr + n].copy_from_slice(&buf[..n]);
                      self.registers[0] = n as u64;
                    }
                    Err(e) => {
                      error!("Error reading from stdin: {}", e);
                      self.registers[0] = (-1i64) as u64;
                    }
                  }
                }
                _ => {
                  if let Some(file) = self.file_descriptors.get_mut(&fd) {
                    use std::io::Read;
                    let mut buf = vec![0u8; count];
                    match file.read(&mut buf) {
                      Ok(n) => {
                        self.heap[buf_ptr..buf_ptr + n].copy_from_slice(&buf[..n]);
                        self.registers[0] = n as u64;
                      }
                      Err(e) => {
                        error!("Error reading from fd {}: {}", fd, e);
                        self.registers[0] = (-1i64) as u64;
                      }
                    }
                  } else {
                    error!("Invalid file descriptor for READ: {}", fd);
                    self.registers[0] = (-1i64) as u64;
                  }
                }
              }
            }
          }
          5 => {
            // WRITE fd, buf_ptr, count
            let fd = self.registers[1];
            let buf_ptr = self.registers[2] as usize;
            let count = self.registers[3] as usize;

            if buf_ptr.checked_add(count).map_or(true, |end| end > self.heap.len()) {
              error!("WRITE out of bounds or overflow: buf_ptr={}, count={}, heap_len={}", buf_ptr, count, self.heap.len());
              self.registers[0] = (-1i64) as u64;
            } else {
              match fd {
                1 | 2 => {
                  use std::io::Write;
                  let buf = &self.heap[buf_ptr..buf_ptr + count];
                  let result = if fd == 1 {
                    std::io::stdout().write(buf)
                  } else {
                    std::io::stderr().write(buf)
                  };
                  match result {
                    Ok(n) => self.registers[0] = n as u64,
                    Err(e) => {
                      error!("Error writing to fd {}: {}", fd, e);
                      self.registers[0] = (-1i64) as u64;
                    }
                  }
                }
                _ => {
                  if let Some(file) = self.file_descriptors.get_mut(&fd) {
                    use std::io::Write;
                    let buf = &self.heap[buf_ptr..buf_ptr + count];
                    match file.write(buf) {
                      Ok(n) => self.registers[0] = n as u64,
                      Err(e) => {
                        error!("Error writing to fd {}: {}", fd, e);
                        self.registers[0] = (-1i64) as u64;
                      }
                    }
                  } else {
                    error!("Invalid file descriptor for WRITE: {}", fd);
                    self.registers[0] = (-1i64) as u64;
                  }
                }
              }
            }
          }
          6 => {
            // OPEN name_ptr, flags, mode
            let name_ptr = self.registers[1] as usize;
            // Read null-terminated name
            let mut name_bytes = Vec::new();
            let mut i = name_ptr;
            while i < self.heap.len() && self.heap[i] != 0 {
              name_bytes.push(self.heap[i]);
              i += 1;
            }
            let name = String::from_utf8_lossy(&name_bytes).to_string();
            
            // For now, let's just support simple read/write flags
            // flags: 0 = read, 1 = write, 2 = read/write
            let flags = self.registers[2];
            let mut options = std::fs::OpenOptions::new();
            match flags {
              0 => options.read(true),
              1 => options.write(true).create(true).truncate(true),
              2 => options.read(true).write(true).create(true),
              _ => options.read(true),
            };

            match options.open(&name) {
              Ok(file) => {
                let fd = self.next_fd;
                self.file_descriptors.insert(fd, file);
                self.next_fd += 1;
                self.registers[0] = fd;
              }
              Err(e) => {
                error!("Error opening file '{}': {}", name, e);
                self.registers[0] = (-1i64) as u64;
              }
            }
          }
          7 => {
            // CLOSE fd
            let fd = self.registers[1];
            if self.file_descriptors.remove(&fd).is_some() {
              self.registers[0] = 0;
            } else {
              error!("Invalid file descriptor for CLOSE: {}", fd);
              self.registers[0] = (-1i64) as u64;
            }
          }
          8 => {
            // ALLOC size
            let size = self.registers[1] as usize;
            let current_len = self.heap.len();
            // Simple bump allocation at the end of the heap
            self.heap.resize(current_len + size, 0);
            self.registers[0] = current_len as u64;
            info!("ALLOCated {} bytes at {:04X}, new heap size={}", size, current_len, self.heap.len());
          }
          10 => {
            // TIME
            use std::time::{SystemTime, UNIX_EPOCH};
            let start = SystemTime::now();
            let since_the_epoch = start.duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            self.registers[0] = since_the_epoch.as_secs();
          }
          _ => {
            error!("Unknown syscall number: {}", syscall_num);
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
    let reg = self.heap[offset] as usize;
    if reg >= 32 {
        error!("Invalid register index: {} at pc={}", reg, self.pc);
    }
    reg
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
    if pc >= self.heap.len() { return "<invalid pc>".to_string(); }
    let op_byte = self.heap[pc];
    let op = OpCode::byte_to_opcode(op_byte).unwrap_or(OpCode::Nop);
    match op {
      OpCode::Add | OpCode::Sub | OpCode::Mul | OpCode::Div |
      OpCode::And | OpCode::Or | OpCode::Xor => {
        if pc + 13 > self.heap.len() { return format!("{:?} <truncated>", op); }
        let r1 = self.fetch_reg(pc + 1);
        let r2 = self.fetch_reg(pc + 5);
        let r3 = self.fetch_reg(pc + 9);
        format!("{:?} r{}, r{}, r{}", op, r1, r2, r3)
      }
      OpCode::Mov | OpCode::Load | OpCode::Store | OpCode::Not | OpCode::Jz | OpCode::Jnz | OpCode::Movi | OpCode::Loadi | OpCode::Storei => {
        if pc + 9 > self.heap.len() { return format!("{:?} <truncated>", op); }
        let r1 = self.fetch_reg(pc + 1);
        let arg2 = self.fetch_u32(pc + 5);
        match op {
            OpCode::Mov | OpCode::Load | OpCode::Store | OpCode::Not => {
                format!("{:?} r{}, r{}", op, r1, arg2)
            }
            OpCode::Jz | OpCode::Jnz => {
                let what = self.describe_addr(arg2 as usize);
                format!("{:?} r{}, {} ({})", op, r1, arg2, what)
            }
            OpCode::Movi => {
                format!("MOVI r{}, {}", r1, arg2)
            }
            OpCode::Loadi | OpCode::Storei => {
                let what = self.describe_addr(arg2 as usize);
                format!("{:?} r{}, [{}] ({})", op, r1, arg2, what)
            }
            _ => format!("{:?} r{}, {}", op, r1, arg2),
        }
      }
      OpCode::Jmp | OpCode::Call => {
        if pc + 5 > self.heap.len() { return format!("{:?} <truncated>", op); }
        let addr = self.fetch_u32(pc + 1);
        let what = self.describe_addr(addr as usize);
        format!("{:?} {} ({})", op, addr, what)
      }
      OpCode::Push | OpCode::Pop => {
        if pc + 5 > self.heap.len() { return format!("{:?} <truncated>", op); }
        let reg = self.fetch_reg(pc + 1);
        format!("{:?} r{}", op, reg)
      }
      OpCode::Ret => "RET".to_string(),
      OpCode::Syscall => "SYSCALL".to_string(),
      OpCode::Halt => "HALT".to_string(),
      OpCode::Break => "BREAK".to_string(),
      OpCode::Nop => "NOP".to_string(),
      _ => format!("{:?} ({:02X})", op, op_byte),
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

fn disassemble_at(_vm: &VM, code: &[u8], pc: usize) -> (String, usize) {
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
