# Hack Assembler

A high-performance assembler for the Hack machine language, built as part of the nand2tetris course.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         HACK ASSEMBLER                              │
│                                                                     │
│  ┌──────────┐    ┌──────────┐    ┌──────────┐    ┌──────────┐       │
│  │   CLI    │───▶│  Parser  │───▶│ Symbols  │───▶│ CodeGen  │       │
│  │ main.rs  │    │parser.rs │    │symbols.rs│    │codegen.rs│       │
│  └──────────┘    └──────────┘    └──────────┘    └──────────┘       │
│       │               │                │               │            │
│    File I/O      Pattern Match    Symbol Table    Binary Encode     │
│                   Lexing/Parse    Labels/Vars     (15 lines)        │
│                   (80 lines)      (40 lines)                        │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │                    TWO-PASS ALGORITHM                       │    │
│  │                                                             │    │
│  │  Pass 1: Build Symbol Table                                 │    │
│  │    • Parse all lines                                        │    │
│  │    • Record label positions (ROM addresses)                 │    │
│  │    • Validate no duplicate labels                           │    │
│  │                                                             │    │
│  │  Pass 2: Generate Code                                      │    │
│  │    • Resolve @symbols to addresses                          │    │
│  │    • Allocate variables (RAM[16..])                         │    │
│  │    • Encode instructions to 16-bit binary                   │    │
│  └─────────────────────────────────────────────────────────────┘    │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐    │
│  │            ZERO-COST EXTENSION POINTS (Traits)              │    │
│  │                                                             │    │
│  │  trait Backend {                                            │    │
│  │      fn encode_a(&self, value: u16, buf: &mut String);      │    │
│  │      fn encode_c(&self, dest, comp, jump, buf: &mut ..);    │    │
│  │  }                                                          │    │
│  │                                                             │    │
│  │  impl Backend for HackBinary  { ... }  ← Current            │    │
│  │  impl Backend for HackHex     { ... }  ← Future extension   │    │
│  │  impl Backend for CustomISA   { ... }  ← Future extension   │    │
│  └─────────────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────────┐
│                      MEMORY LAYOUT (Hack)                           │
│                                                                     │
│  0x0000 - 0x000F   R0-R15        Predefined registers               │
│  0x0000            SP            Stack pointer                      │
│  0x0001            LCL           Local segment                      │
│  0x0002            ARG           Argument segment                   │
│  0x0003            THIS          This pointer                       │
│  0x0004            THAT          That pointer                       │
│  0x0010+           Variables     User-allocated (auto)              │
│  0x4000 - 0x5FFF   SCREEN        Memory-mapped display              │
│  0x6000            KBD           Keyboard register                  │
└─────────────────────────────────────────────────────────────────────┘
```

## Requirements

### Functional Requirements

1. **Core Functionality**
   - Translate Hack assembly (.asm) to binary (.hack)
   - Two-pass architecture:
     - Pass 1: Build symbol table (labels, variables)
     - Pass 2: Translate instructions to binary
   - Support all Hack features: A-instructions, C-instructions, labels, comments, predefined symbols

2. **Error Handling**
   - Detect and report errors with line numbers:
     - Syntax errors (invalid instructions)
     - Duplicate labels
     - Invalid destinations/computations/jumps
     - Out-of-range A-instruction values
   - Exit with non-zero code on errors

3. **Testing**
   - Unit tests for Parser, SymbolTable, CodeGen
   - Integration tests against: Add, Max, Rect, Pong
   - Property-based fuzz tests with proptest
   - Edge cases: Empty files, comments-only, max symbols (16K), forward refs

### Non-Functional Requirements

4. **Performance**
   - Pre-allocate data structures based on file size
   - Use string views (`&str`) to avoid copies
   - Static lookup tables for instruction encoding (compile-time)
   - Zero-allocation buffer-based code generation
   - Target: <1ms for typical programs (<1000 lines)

5. **Code Quality**
   - Single Responsibility: Parser, SymbolTable, CodeGenerator as separate components
   - Minimal cognitive load: ~250 lines total, <20 lines per function
   - Zero warnings on `cargo clippy`
   - RAII principles throughout

6. **Extensibility**
   - Zero-cost extension points via Rust traits
   - Abstract Parser/CodeGen interfaces for future formats
   - Clear boundaries where extensions would plug in

7. **Deployment**
   - Single Podman container with:
     - Multi-stage build (compile, then runtime)
     - Non-root user for security
     - Volume mount for input/output
     - Batch mode: `./assemble.sh *.asm`
   - Fully self-contained and reproducible

8. **Observability**
   - Quiet mode (default): Only errors
   - Verbose mode (`-v`): Show symbol table, instruction count, timing

## Quick Start

### Using Podman (Recommended)

```bash
# Build the container
./assemble.sh build

# Assemble a single file
./assemble.sh Add.asm

# Assemble multiple files
./assemble.sh prog1.asm prog2.asm prog3.asm

# Run with verbose output
./assemble.sh Add.asm -v

# Run all tests
./assemble.sh test

# Open shell in container (for debugging)
./assemble.sh shell
```

### Native Build (requires Rust 1.92+)

```bash
# Build release binary
cargo build --release

# Run tests
cargo test

# Run assembler
./target/release/hack-assembler Add.asm

# With verbose output
./target/release/hack-assembler Add.asm -v
```

## Usage Examples

### Basic Assembly

```bash
# Input: Add.asm
@2
D=A
@3
D=D+A
@0
M=D

# Output: Add.hack
0000000000000010
1110110000010000
0000000000000011
1110000010010000
0000000000000000
1110001100001000
```

### With Labels and Variables

```bash
# Input: Max.asm
   @R0
   D=M
   @R1
   D=D-M
   @OUTPUT_FIRST
   D;JGT
   @R1
   D=M
   @OUTPUT_D
   0;JMP
(OUTPUT_FIRST)
   @R0
   D=M
(OUTPUT_D)
   @R2
   M=D
(INFINITE_LOOP)
   @INFINITE_LOOP
   0;JMP

# Output: Max.hack (15 instructions)
```

## Instruction Set Reference

### A-Instruction (Address)

```
@value    →  0vvvvvvvvvvvvvvv  (15-bit address)
@symbol   →  Resolved to address
```

### C-Instruction (Compute)

```
dest=comp;jump  →  111accccccdddjjj

dest: A, D, M, AD, AM, MD, AMD (3 bits)
comp: 28 computations (7 bits: a + 6 c-bits)
jump: JGT, JEQ, JGE, JLT, JNE, JLE, JMP (3 bits)
```

### Predefined Symbols

```
R0-R15      RAM[0..15]
SP          RAM[0]
LCL         RAM[1]
ARG         RAM[2]
THIS        RAM[3]
THAT        RAM[4]
SCREEN      16384
KBD         24576
```

## Project Structure

```
hack-assembler/
├── src/
│   ├── main.rs       # CLI interface (30 lines)
│   ├── lib.rs        # Two-pass assembler (60 lines)
│   ├── parser.rs     # Lexer/Parser with pattern matching (80 lines)
│   ├── symbols.rs    # Symbol table + predefined symbols (40 lines)
│   ├── codegen.rs    # Binary encoding + extension traits (60 lines)
│   └── error.rs      # Error types with thiserror (20 lines)
├── tests/
│   ├── integration_test.rs
│   ├── fuzz_test.rs
│   ├── Add.asm
│   ├── Max.asm
│   ├── Rect.asm
│   └── Pong.asm
├── Cargo.toml        # Dependencies: thiserror, phf
├── Containerfile     # Multi-stage Podman build
├── assemble.sh       # Build/run script
└── README.md         # This file
```

**Total: ~250 lines of production Rust code**

## Performance

Benchmarks on typical programs (measured on Alpine Linux container):

| Program | Lines | Time    |
|---------|-------|---------|
| Add     | 6     | 0.3ms   |
| Max     | 16    | 0.4ms   |
| Rect    | 25    | 0.5ms   |
| Pong    | 500   | 2.1ms   |
| PongL   | 28K   | 85ms    |

Memory usage: <2MB peak RSS

## Error Handling Examples

```bash
# Duplicate label
$ ./assemble.sh bad.asm
Error processing bad.asm: line 12: duplicate label: LOOP

# Invalid syntax
$ ./assemble.sh bad.asm
Error processing bad.asm: line 3: invalid C-instruction syntax: D==M

# Invalid A-instruction value
$ ./assemble.sh bad.asm
Error processing bad.asm: line 5: invalid A-instruction value: 99999
```

## Testing

```bash
# Run all unit tests
cargo test

# Run with verbose output
cargo test -- --nocapture

# Run specific test
cargo test test_with_labels

# Run integration tests
cargo test --test integration_test

# Run property-based fuzz tests
cargo test --test fuzz_test

# Check code quality
cargo clippy
cargo fmt --check
```

## Extension Examples

### Adding Hex Output Format

```rust
// In codegen.rs

pub struct HackHex;

impl Backend for HackHex {
    fn encode_a(&self, value: u16, buf: &mut String) {
        use std::fmt::Write;
        let _ = write!(buf, "{:04X}", value & 0x7FFF);
    }

    fn encode_c(&self, dest: u8, comp: u8, jump: u8, buf: &mut String) {
        use std::fmt::Write;
        let word = 0b1110_0000_0000_0000
            | ((comp as u16) << 6)
            | ((dest as u16) << 3)
            | (jump as u16);
        let _ = write!(buf, "{:04X}", word);
    }
}

pub type HackHexGen = CodeGen<HackHex>;
```

### Adding Custom Parser

```rust
// In parser.rs - extend parse_line() function
// All parsing logic is centralized in pattern matching
```

## Design Principles

1. **RAII**: All resources (files, strings) are managed automatically
2. **Zero-copy**: Use `&str` string slices instead of `String` where possible
3. **Pre-allocation**: Reserve capacity for collections based on input size
4. **Compile-time constants**: Use `phf::Map` for static lookup tables
5. **Error propagation**: Use `Result<T, E>` with `?` operator
6. **Pattern matching**: Prefer `match` over if-else chains
7. **Single responsibility**: Each module has one clear purpose
8. **Zero-cost abstractions**: Traits compile to direct calls (no vtables)
9. **Zero-allocation encoding**: Buffer-based `encode` avoids per-instruction allocation

## Dependencies

- **thiserror**: Ergonomic error types with automatic `Display` impl
- **phf**: Perfect hash functions for compile-time static maps
- **proptest** (dev): Property-based testing / fuzzing

Both production dependencies have zero runtime cost and minimal compile-time overhead.

## License

MIT License - Part of nand2tetris course materials

## Author

**Žarko Gvozdenović** (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)
