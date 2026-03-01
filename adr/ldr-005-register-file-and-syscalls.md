# LDR-005: Register File and System State

**Status:** Implemented
**Date:** 2026-03-01
**Context:**
The Leaf VM initially had only 16 registers, which proved insufficient for complex programs like Fibonacci that require many temporary variables.

---

## 1. Decision

### Expanded Register Set
The Leaf VM features **32 general-purpose registers** (`r0` to `r31`).
- All registers are 64 bits wide.
- Register `r15` is conventionally used as the **Stack Pointer (SP)**.
- Register `r0` and `r1` are conventionally used for **SYSCALL arguments and return values**.

### System State and Control Flow
- **Program Counter (PC):** An internal register (not directly addressable as `rn`) that holds the memory offset of the current instruction.
- **Halted Flag:** A boolean flag that, when true, stops the VM from fetching or executing further instructions.
- **Zero/Negative Flags:** Currently, the VM does not use a status register for conditional execution. Instead, instructions like `JZ` and `JNZ` check the value of a source register directly.

### Syscall Interface
The `SYSCALL` instruction transfers control to the VM's built-in system call handler. The interface is defined by:
- **r0:** The system call number.
- **r1:** The first argument (e.g., value to print).
- **Return Value:** Any return value from the syscall is stored back into `r0`.

#### Supported Syscalls:
| r0 | Name | Argument (r1) | Description |
| :--- | :--- | :--- | :--- |
| 1 | `EXIT` | Exit code | Terminates the VM with the specified code. |
| 2 | `PRINT` | Value | Prints the 64-bit word value as a decimal to stdout. |

---

## 2. Consequences

- **Greater Flexibility:** 32 registers allow for more complex mathematical computations and reduce the need for frequent memory spills/fills.
- **Standardized ABI:** Using `r0` and `r1` for syscalls establishes a simple calling convention for interacting with the VM environment.
- **Simple Logic:** Direct register checks in jump instructions (`JZ/JNZ`) simplify the VM's internal pipeline by eliminating the need for complex condition flag management.

---

## 3. References

- [vm.rs](../leaf_vm/src/vm.rs)
- [LDR-004: Memory Management, Stack, and Word Size](ldr-004-memory-and-stack.md)
