# LDR-004: Memory Management, Stack, and Word Size

**Status:** Implemented
**Date:** 2026-03-01
**Context:**
The Leaf VM needed a consistent model for memory addressing, data sizes, and stack management. Early versions had issues with stack-code collisions and 32-bit vs 64-bit confusion.

---

## 1. Decision

### Unified Heap Memory
The VM uses a single, unified byte-array as its address space (heap). This space is logically partitioned during program loading:

1. **.text Section:** Loaded at address 0. Contains executable instructions.
2. **.data Section:** Loaded immediately after `.text`. Contains initialized mutable data.
3. **.rodata Section:** Loaded after `.data`. Contains read-only data.
4. **Stack Space:** The remainder of the memory after the program sections.

### 64-bit Word Size
All data and address calculations in the VM are based on a **64-bit word size**.
- Registers are 64 bits wide.
- Memory `LOAD/STORE` operations (including `LOADI/STOREI`) always move 8 bytes.
- Pointers and offsets in memory are treated as 64-bit values (though instructions currently use 32-bit immediates for addresses to save space).

### Stack Management
- **Stack Pointer (SP):** Register `r15` is dedicated as the stack pointer.
- **Initialization:** At VM startup, `r15` is initialized to the total memory size (the end of the heap).
- **Growth:** The stack grows **downwards** (towards lower addresses).
- **Instruction Support:** `PUSH` decrements SP by 8 bytes and stores a word. `POP` loads a word and increments SP by 8 bytes. `CALL` pushes the return address (8 bytes).

### Memory Safety
- The VM checks bounds for all memory operations.
- `LOAD/STORE` operations must not exceed the current heap length.
- The VM loader ensures the heap size is large enough to contain the program plus a minimum stack buffer.

---

## 2. Consequences

- **Consistent Data Access:** Using 8-byte words for all memory operations simplifies the ISA and prevents alignment-related bugs.
- **Stack Safety:** Initializing SP to the end of memory and growing downwards maximizes available space for the stack, provided the program doesn't overrun its sections.
- **Relocation Alignment:** Data labels in `.data` or `.rodata` are 8-byte aligned to ensure efficient word-sized access.

---

## 3. References

- [vm.rs](../leaf_vm/src/vm.rs)
- [LDR-003: ISA and Operand Encoding](ldr-003-isa-and-operand-encoding.md)
