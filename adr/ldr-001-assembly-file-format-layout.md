# LDR-001: Assembly File Format Layout

**Status:** Implemented
**Date:** 2025-07-07
**Context:**
This Leaf Decision Record (LDR) defines the binary format for files produced by the assembler,
including section layout and metadata, to be used by both the loader and VM runtime.

---

## 1. Decision
We will define a fixed, extensible binary file format for all machine programs produced by the Leaf assembler.
This format will support multiple sections (code, data, read-only data), allow for validation, and provide necessary
metadata for correct loading.

### File Structure
All files follow this structure:

| Offset | Field         | Size (bytes) | Description                                              |
| ------ | ------------- | ------------ | -------------------------------------------------------- |
| 0      | Magic         | 4            | Magic string: `"LAF\0"` (Lars Assembly File)             |
| 4      | Version       | 2            | Little-endian, e.g., `0x0001`                            |
| 6      | Reserved      | 2            | Reserved for future use (set to 0)                       |
| 8      | Checksum      | 4            | CRC32 checksum of the entire file with this field zeroed |
| 12     | Text offset   | 4            | Offset (from file start) to `.text` section              |
| 16     | Text size     | 4            | Size of `.text` section in bytes                         |
| 20     | Data offset   | 4            | Offset to `.data` section                                |
| 24     | Data size     | 4            | Size of `.data` section                                  |
| 28     | Rodata offset | 4            | Offset to `.rodata` section                              |
| 32     | Rodata size   | 4            | Size of `.rodata` section                                |
| 36     | ...           | ...          | Section contents                                         |


**Notes:**
- The header is 36 bytes long.
- All integers are **little-endian**.
- Checksum is computed using CRC32 over the entire file, but with the checksum field itself set to zero during computation.

### Section Content
- **.text:** Machine instructions for execution.
- **.data:** Mutable data (e.g., variables, buffers).
- **.rodata:** Read-only data (e.g., constants, string literals).

Sections appear **in this order**: text, data, rodata.

### Example

A file might look like this in binary (hex representation):

| Offset | Bytes       | Meaning                   |
| ------ | ----------- | ------------------------- |
| 0      | 4C 41 46 00 | "LAF\0" magic             |
| 4      | 01 00       | Version 1                 |
| 6      | 00 00       | Reserved                  |
| 8      | ab cd ef 01 | CRC32 checksum            |
| 12     | 24 00 00 00 | .text offset (36 bytes)   |
| 16     | 0A 00 00 00 | .text size (10 bytes)     |
| 20     | 2E 00 00 00 | .data offset (46 bytes)   |
| 24     | 08 00 00 00 | .data size (8 bytes)      |
| 28     | 36 00 00 00 | .rodata offset (54 bytes) |
| 32     | 02 00 00 00 | .rodata size (2 bytes)    |
| 36     | ...         | section contents          |

## 2. Consequences
- Loaders and virtual machines can robustly parse, validate, and load programs.
- Adding new sections in the future is possible by extending the header (using reserved space).
- Corruption or errors can be detected early using the checksum.
- Tools and programs can quickly seek to any section using offsets and sizes.

## 3. Rejected Alternatives
- **No Header:** Would not support validation or random access to sections.
- **Text-based format:** Not suitable for direct execution or efficient loading.

## 4. Open Questions
- Will we ever need more metadata, such as entry point, symbol table, or dynamic section? (This can be added later.)

## 5. References
- [ELF File Format](https://en.wikipedia.org/wiki/Executable_and_Linkable_Format) (for conceptual inspiration)
- [CRC32](https://en.wikipedia.org/wiki/Cyclic_redundancy_check) checksum algorithm
- [assembler.rs](../leaf-asm/src/assembler.rs) (Leaf assembler source code)

---
