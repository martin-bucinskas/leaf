# LDR-006: Expanded Syscall Interface for General-Purpose Programming

**Status:** Implemented
**Date:** 2026-03-01
**Context:**
The initial Leaf VM syscall set (EXIT, PRINT_STR, PRINT_INT) is insufficient for building a general-purpose programming environment. To support more complex applications, the syscall interface must be expanded to include file I/O, dynamic memory management, and system information.

---

## 1. Decision

The Leaf VM SYSCALL interface will use `r0` for the syscall number and `r1` through `r6` for arguments. The return value will be stored in `r0`. A negative return value indicates an error (standardized to positive error codes, returned as -ERR).

### Updated Syscall Table:

| r0 (ID) | Name        | r1          | r2          | r3          | r4          | Description |
| :---    | :---        | :---        | :---        | :---        | :---        | :--- |
| 1       | `PRINT_STR` | ptr         | -           | -           | -           | Prints a null-terminated string to stdout. |
| 2       | `PRINT_INT` | value       | -           | -           | -           | Prints a 64-bit integer as decimal to stdout. |
| 3       | `EXIT`      | status      | -           | -           | -           | Terminates the VM with the given status code. |
| 4       | `READ`      | fd          | buf_ptr     | count       | -           | Reads up to `count` bytes from `fd` into `buf_ptr`. Returns bytes read. |
| 5       | `WRITE`     | fd          | buf_ptr     | count       | -           | Writes `count` bytes from `buf_ptr` to `fd`. Returns bytes written. |
| 6       | `OPEN`      | name_ptr    | flags       | mode        | -           | Opens the file `name_ptr`. Flags and mode follow POSIX-like conventions. |
| 7       | `CLOSE`     | fd          | -           | -           | -           | Closes the file descriptor `fd`. |
| 8       | `ALLOC`     | size        | -           | -           | -           | Allocates `size` bytes of memory. Returns pointer to the start of memory. |
| 9       | `FREE`      | ptr         | -           | -           | -           | Frees previously allocated memory at `ptr`. (Placeholder: simple bump allocator for now). |
| 10      | `TIME`      | -           | -           | -           | -           | Returns the current Unix timestamp in seconds. |

### File Descriptors:
- 0: `stdin`
- 1: `stdout`
- 2: `stderr`

---

## 2. Consequences

- **Enhanced Capabilities:** Programs can now interact with the file system and manage memory more dynamically.
- **Complexity in VM:** The VM must now manage file descriptors and a more sophisticated heap/memory model.
- **Future Growth:** This set of syscalls provides a solid foundation for building a standard library for the Leaf programming language.

---

## 3. References

- [vm.rs](../leaf_vm/src/vm.rs)
- [LDR-005: Register File and System State](ldr-005-register-file-and-syscalls.md)
