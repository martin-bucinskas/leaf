# LDR-007: Leaf High-Level Language Specification

This document defines the architecture and syntax for the high-level programming language `Leaf`, which compiles down to `Leaf Assembly` and runs on the `Leaf VM`.

## Design Philosophy

The language is a blend of:
- **C**: Manual memory control (pointers), simplicity, performance.
- **Java**: Strong typing, syntax structure (classes/methods), garbage collection (future consideration).
- **Python**: Readable syntax (indented code blocks or simplified control flow), expressive high-level constructs.

## 1. Type System

Leaf is statically typed. Initial types include:
- `int`: 64-bit signed integer (VM word size).
- `str`: Pointer to a null-terminated character sequence.
- `bool`: Boolean value (`true`, `false`).
- `void`: Used for functions that return no value.

## 2. Syntax Overview

### Comments
- Single-line: `// comment`
- Multi-line: `/* comment */`

### Variables
Variable declaration follows C/Java style:
```leaf
int x = 10;
str s = "Hello, Leaf!";
bool isActive = true;
```

### Functions
Functions are declared using the `fn` keyword:
```leaf
fn add(int a, int b) -> int {
    return a + b;
}

fn main() -> int {
    int result = add(5, 10);
    print(result);
    return 0;
}
```

### Control Flow
Classic control flow with blocks:
```leaf
if (x > 10) {
    print("Greater");
} else {
    print("Smaller");
}

while (x > 0) {
    x = x - 1;
}
```

### Standard Library (Includes)
Leaf supports including files from the standard library or local directories:
```leaf
include @std/math; // Included from standard library
include "my_lib.leafc"; // Included from local directory
```

Standard library modules are located in the `leaf_compiler/resources/std` directory.

## 3. Standard Library (Built-ins)

The compiler will map these to specific VM syscalls:
- `print(int)`: Prints an integer.
- `print(str)`: Prints a string.
- `exit(int)`: Terminates the process.

## 4. Compilation Pipeline

1. **Lexing**: Tokenize source code into symbols (keywords, identifiers, literals, operators).
2. **Parsing**: Generate an Abstract Syntax Tree (AST).
3. **Semantic Analysis**: Check for type consistency and scoping.
4. **Code Generation**: Transform AST into `.leaf` assembly source code.
5. **Assembly & Linking**: Use existing `leaf_asm` toolchain to produce `.leafexe`.

## 5. Memory Management

- Global variables are stored in the `.data` or `.rodata` sections.
- Local variables are stored on the stack (`r15` / SP).
- Dynamic memory (future) will use the `ALLOC` syscall and a heap manager.
