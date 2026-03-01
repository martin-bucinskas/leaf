# LDR-003: Instruction Set Architecture (ISA) and Operand Encoding

**Status:** Implemented
**Date:** 2026-03-01
**Context:**
The Leaf VM initially had inconsistent instruction lengths and operand sizes. This led to memory corruption and incorrect relocation patching. To ensure stability and correctness, we needed a unified encoding strategy.

---

## 1. Decision

We define a fixed-operand encoding for the Leaf VM. Each instruction consists of a 1-byte OpCode followed by zero or more 4-byte operands.

### Operand Encoding
- **OpCode:** 1 byte (0x00 - 0xFF).
- **Operands:** Each operand is encoded as a 4-byte little-endian integer.
- **Register Indices:** Encoded as a 4-byte immediate (0 - 31).
- **Immediates/Addresses:** Encoded as a 4-byte immediate.

### Instruction Lengths
| Instruction Type | Length (bytes) | Encoding |
| :--- | :--- | :--- |
| **0 Operands** | 1 | `[OP]` |
| **1 Operand** | 5 | `[OP] [ARG1 (4B)]` |
| **2 Operands** | 9 | `[OP] [ARG1 (4B)] [ARG2 (4B)]` |
| **3 Operands** | 13 | `[OP] [ARG1 (4B)] [ARG2 (4B)] [ARG3 (4B)]` |

### Supported Instructions
| OpCode | Instruction | Operands | Description |
| :--- | :--- | :--- | :--- |
| 0x00 | `NOP` | 0 | No operation |
| 0x01 | `ADD` | 3 | `rd, rs1, rs2` -> `rd = rs1 + rs2` |
| 0x02 | `SUB` | 3 | `rd, rs1, rs2` -> `rd = rs1 - rs2` |
| 0x03 | `MUL` | 3 | `rd, rs1, rs2` -> `rd = rs1 * rs2` |
| 0x04 | `DIV` | 3 | `rd, rs1, rs2` -> `rd = rs1 / rs2` |
| 0x05 | `AND` | 3 | `rd, rs1, rs2` -> `rd = rs1 & rs2` |
| 0x06 | `OR` | 3 | `rd, rs1, rs2` -> `rd = rs1 \| rs2` |
| 0x07 | `XOR` | 3 | `rd, rs1, rs2` -> `rd = rs1 ^ rs2` |
| 0x08 | `NOT` | 2 | `rd, rs1` -> `rd = ~rs1` |
| 0x19 | `LT` | 3 | `rd, rs1, rs2` -> `rd = rs1 < rs2 ? 1 : 0` |
| 0x1A | `GT` | 3 | `rd, rs1, rs2` -> `rd = rs1 > rs2 ? 1 : 0` |
| 0x1B | `EQ` | 3 | `rd, rs1, rs2` -> `rd = rs1 == rs2 ? 1 : 0` |
| 0x09 | `JMP` | 1 | `addr` -> `pc = addr` |
| 0x0A | `JZ` | 2 | `rs1, addr` -> `if rs1 == 0, pc = addr` |
| 0x0B | `JNZ` | 2 | `rs1, addr` -> `if rs1 != 0, pc = addr` |
| 0x0C | `MOV` | 2 | `rd, rs1` -> `rd = rs1` |
| 0x0D | `LOAD` | 2 | `rd, [rs1]` -> `rd = [rs1]` (64-bit load) |
| 0x0E | `STORE` | 2 | `rs1, [rd]` -> `[rd] = rs1` (64-bit store) |
| 0x0F | `CALL` | 1 | `addr` -> `push pc+5, pc = addr` |
| 0x10 | `RET` | 0 | `pop pc` |
| 0x11 | `PUSH` | 1 | `rs1` -> `sp -= 8, [sp] = rs1` |
| 0x12 | `POP` | 1 | `rd` -> `rd = [sp], sp += 8` |
| 0x13 | `HALT` | 0 | Stop VM execution |
| 0x14 | `BREAK` | 0 | Debugger break (optional) |
| 0x15 | `SYSCALL`| 0 | Execute system call defined by `r0`, `r1` |
| 0x16 | `MOVI` | 2 | `rd, imm` -> `rd = imm` (relocatable) |
| 0x17 | `LOADI` | 2 | `rd, [imm]` -> `rd = [imm]` (relocatable) |
| 0x18 | `STOREI`| 2 | `rs1, [imm]` -> `[imm] = rs1` (relocatable) |

---

## 2. Consequences

- **Simplified Relocation:** Since all labels/addresses are encoded as 4-byte immediates at fixed offsets within the instruction, the linker can easily patch them by simply overwriting the 4 bytes.
- **Deterministic Fetch:** The VM can fetch operands using a simple `u32` read from memory.
- **Memory Alignment:** Instruction boundaries are not strictly 4-byte or 8-byte aligned (opcodes are 1 byte), but operands themselves are 4-byte values.

---

## 3. References

- [LDR-002: Assembly File Format Layout (With Symbol Table)](ldr-002-assembly-file-format-layout-with-symbols.md)
- [leaf_ast.rs](../leaf_common/src/leaf_ast.rs)
- [vm.rs](../leaf_vm/src/vm.rs)
