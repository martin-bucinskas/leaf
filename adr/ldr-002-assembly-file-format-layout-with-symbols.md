# LDR-002: Assembly File Format Layout (With Symbol Table)

**Status:** Implemented
**Date:** 2025-07-07
**Supersedes:** [LDR-001](ldr-001-assembly-file-format-layout.md)
**Context:**
[LDR-001](ldr-001-assembly-file-format-layout.md) established a fixed binary file format for the Leaf assembler output.
This decision record updates the layout to introduce a symbol table section for supporting debugging, external linking, and introspection of binary programs.

---

## 1. Decision

We extend the binary file format for machine programs produced by the Leaf assembler to support an explicit symbol table section.
This will enable tools and loaders to easily resolve symbol locations and types, and allow for debugging and richer runtime features.

### File Structure

All files follow this updated structure:

| Offset | Field         | Size (bytes) | Description                                              |
| ------ | ------------- | ------------ | -------------------------------------------------------- |
| 0      | Magic         | 4            | Magic string: `"LAF\0"`                                  |
| 4      | Version       | 2            | Little-endian, e.g., `0x0001`                            |
| 6      | Reserved      | 2            | Reserved for future use (set to 0)                       |
| 8      | Checksum      | 4            | CRC32 checksum of the entire file with this field zeroed |
| 12     | Text offset   | 4            | Offset to `.text` section                                |
| 16     | Text size     | 4            | Size of `.text` section                                  |
| 20     | Data offset   | 4            | Offset to `.data` section                                |
| 24     | Data size     | 4            | Size of `.data` section                                  |
| 28     | Rodata offset | 4            | Offset to `.rodata` section                              |
| 32     | Rodata size   | 4            | Size of `.rodata` section                                |
| 36     | Symtab offset | 4            | Offset to symbol table section                           |
| 40     | Symtab size   | 4            | Size of symbol table section                             |
| 44     | ...           | ...          | Section contents (text, data, rodata, symtab)            |

**Notes:**
- The header is **44 bytes** long.
- All integers are **little-endian**.
- Checksum is computed using CRC32 over the entire file, but with the checksum field itself set to zero during computation.

### Section Content
- **.text:** Machine instructions for execution.
- **.data:** Mutable data (e.g., variables, buffers).
- **.rodata:** Read-only data (e.g., constants, string literals).
- **.symtab:** Symbol table, encoding all labels and symbols in the binary.

#### Symbol Table Format

The symbol table is a sequence of symbol entries. Each entry is encoded as:

| Field    | Size (bytes)        | Description                                   |
| -------- | ------------------- | --------------------------------------------- |
| Name len | 1                   | Length of the symbol name                     |
| Name     | variable (Name len) | UTF-8 encoded symbol name                     |
| Kind     | 1                   | Symbol kind: 0 = label, 1 = data, 2 = rodata  |
| Offset   | 4                   | Offset (relative to start of section)         |
| Section  | 1                   | Section ID: 0 = .text, 1 = .data, 2 = .rodata |

Entries are packed back-to-back with no padding.

### Example

A file might look like this in binary (hex representation):

| Offset | Bytes       | Meaning                                      |
| ------ | ----------- | -------------------------------------------- |
| 0      | 4C 41 46 00 | `"LAF\0"` magic                              |
| 4      | 01 00       | Version 1                                    |
| 6      | 00 00       | Reserved                                     |
| 8      | ab cd ef 01 | CRC32 checksum                               |
| 12     | 2C 00 00 00 | .text offset (44 bytes)                      |
| 16     | 0A 00 00 00 | .text size (10 bytes)                        |
| 20     | 36 00 00 00 | .data offset (54 bytes)                      |
| 24     | 08 00 00 00 | .data size (8 bytes)                         |
| 28     | 3E 00 00 00 | .rodata offset (62 bytes)                    |
| 32     | 02 00 00 00 | .rodata size (2 bytes)                       |
| 36     | 40 00 00 00 | .symtab offset (64 bytes)                    |
| 40     | 0C 00 00 00 | .symtab size (12 bytes)                      |
| 44     | ...         | section contents: text, data, rodata, symtab |

The symbol table at offset 64 contains one or more symbol entries as described above.

---

## 2. Consequences

- **Enables richer tooling:** Loaders, debuggers, and linkers can resolve symbol names and locations.
- **Future extensibility:** More sections or metadata can be added by further extending the header.
- **Validation and parsing:** Checksum and explicit section offsets/sizes ensure robust file validation and parsing.
- **Compatibility:** Old tools must be updated to handle the longer header and symtab section.

---

## 3. Rejected Alternatives

- **Leaving symbol table as a separate file:** Would complicate loading, linking and sharing binaries.
- **Text-based symbol maps:** Not efficient or suitable for direct binary consumption.

---

## 4. Open Questions

- Should symbol table support other symbol types (extern, weak, etc.)?
- Should we support relocations in the future?

---

## 5. References

- [LDR-001: Assembly File Format Layout](ldr-001-assembly-file-format-layout.md)
- [assembler.rs](../leaf-asm/src/assembler.rs)

---

**This LDR supersedes LDR-001 and formalizes the symbol table in the binary format.**
