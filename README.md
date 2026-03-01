# Leaf Virtual Machine & Assembler

A custom 64-bit Virtual Machine (VM) and a corresponding Assembler/Linker toolchain for a low-level programming language called Leaf.

## Project Structure

- `leaf_common`: Shared data structures, file format definitions (`.leafobj`, `.leafexe`), and the AST.
- `leaf_compiler`: The high-level language compiler that produces Leaf Assembly from `.leafc` files.
- `leaf_asm`: The toolchain for assembling source files into objects and linking them into executable binaries.
- `leaf_vm`: The execution engine that loads and runs `.leafexe` binaries.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable version recommended)

## Quick Start: Fibonacci Example

The project includes a Fibonacci example in `leaf_asm/fixtures/fibonacci.leaf`.

### 1. Assemble the source
Convert the `.leaf` assembly source into a relocatable `.leafobj` object file.

```powershell
cargo run -p leaf_asm -- assemble --inputs leaf_asm\fixtures\fibonacci.leaf -o fibonacci.leafobj
```

### 2. Link the object
Link the `.leafobj` file into a standalone `.leafexe` binary. You must specify the entry point label (usually `main`).

```powershell
cargo run -p leaf_asm -- link fibonacci.leafobj -o fibonacci.leafexe --entry main
```

### 3. Run the VM
Execute the binary using the Leaf VM.

```powershell
# Note: Ensure the path in leaf_vm/src/main.rs points to your generated .leafexe
cargo run -p leaf_vm
```

## High-Level Language: LeafC

The `leaf_compiler` allows you to write programs in a C/Python hybrid syntax and compile them to Leaf Assembly.

### 1. Create a `.leafc` source file
```leaf
fn main() -> int {
    int x = 123;
    int y = 456;
    print(x + y);
    return 0;
}
```

### 2. Compile to Assembly
```powershell
cargo run -p leaf_compiler -- --input my_program.leafc --output my_program.leaf
```

### 3. Build and Run
Follow the Assembly and Linking steps above to generate a `.leafexe` and run it in the VM.

## Instruction Set Overview

Leaf is a register-based VM with 32 general-purpose 64-bit registers (`r0` to `r31`).

- **Arithmetic:** `ADD`, `SUB`, `MUL`, `DIV`
- **Logic:** `AND`, `OR`, `XOR`, `NOT`
- **Control Flow:** `JMP`, `JZ`, `JNZ`, `CALL`, `RET`
- **Memory:** `LOAD`, `STORE` (register indirect), `LOADI`, `STOREI` (absolute address)
- **Immediate:** `MOVI rX, IMM`
- **Stack:** `PUSH`, `POP` (uses `r15` as Stack Pointer)
- **System:** `SYSCALL`, `BREAK`, `HALT`, `NOP`

### System Calls (`SYSCALL`)

The `SYSCALL` instruction uses `r0` for the syscall number and `r1` through `r4` for arguments. The return value is stored in `r0`.

- `r0 = 1`: `PRINT_STR` - Prints null-terminated string at address `r1`.
- `r0 = 2`: `PRINT_INT` - Prints integer value in `r1`.
- `r0 = 3`: `EXIT` - Exit with code in `r1`.
- `r0 = 4`: `READ` - Read up to `r3` bytes from fd `r1` into address `r2`.
- `r0 = 5`: `WRITE` - Write `r3` bytes to fd `r1` from address `r2`.
- `r0 = 6`: `OPEN` - Open file `r1` with flags `r2`.
- `r0 = 7`: `CLOSE` - Close fd `r1`.
- `r0 = 8`: `ALLOC` - Allocate `r1` bytes of memory.
- `r0 = 10`: `TIME` - Get current Unix timestamp.

## Leaf Decision Records (LDR)

Detailed design decisions and architecture specifications are documented in the `adr/` directory:

- [LDR-002: Assembly File Format Layout (With Symbol Table)](adr/ldr-002-assembly-file-format-layout-with-symbols.md)
- [LDR-003: Instruction Set Architecture (ISA) and Operand Encoding](adr/ldr-003-isa-and-operand-encoding.md)
- [LDR-004: Memory Management, Stack, and Word Size](adr/ldr-004-memory-and-stack.md)
- [LDR-005: Register File and System State](adr/ldr-005-register-file-and-syscalls.md)
- [LDR-006: Expanded Syscall Interface](adr/ldr-006-expanded-syscall-interface.md)
- [LDR-007: Leaf High-Level Language Specification](adr/ldr-007-leaf-high-level-language-specification.md)

## Standard Library

Leaf includes a basic standard library.
- `@std/math`: Basic mathematical functions (`abs`, `min`, `max`, `pow`).

Example usage:
```leaf
include @std/math;

fn main() -> int {
    print(pow(2, 10));
    return 0;
}
```

## Binary Format

Leaf uses a custom binary format (`LAF\0` magic) that supports multiple sections:
- `.text`: Executable code.
- `.data`: Mutable global data.
- `.rodata`: Read-only constants.

The format includes a symbol table and relocation entries to allow for static linking and address patching.
