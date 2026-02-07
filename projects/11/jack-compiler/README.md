# JackCompiler

Full Jack to VM code compiler with optimizations (nand2tetris Project 11).

## Overview

JackCompiler compiles Jack source files to VM code for the nand2tetris virtual machine. It extends the `jack-analyzer` (Project 10) with code generation and optimization passes.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/JackCompiler`.

## Usage

```bash
# Compile a single file
./JackCompiler Main.jack

# Compile a directory (parallel processing)
./JackCompiler Square/

# Disable peephole optimization
./JackCompiler --no-optimize Main.jack

# Specify output directory
./JackCompiler -o output/ Square/
```

### Output

For each input file `Foo.jack`, the compiler produces:
- `Foo.vm` - Generated VM code

## Python Version

A single-file Python implementation is also provided for Coursera submission:

```bash
python3 ../JackCompiler.py Main.jack
python3 ../JackCompiler.py Square/
```

## Architecture

```
src/
├── main.rs          # CLI entry point
├── lib.rs           # Public API, orchestration
├── symbol_table.rs  # Two-level symbol table (class/subroutine scope)
├── codegen.rs       # VM code generator (AST traversal)
├── vm_writer.rs     # VM command emitter
├── optimizer.rs     # Peephole & constant folding
└── error.rs         # Error types and diagnostics
```

## Features

- **Complete Jack Language**: All constructs including classes, constructors, methods, arrays, strings
- **Two-Level Symbol Table**: Class scope (static, field) + subroutine scope (argument, local) with proper shadowing
- **Constant Folding**: Compile-time evaluation of constant expressions (e.g., `1 + 2 + 3` becomes `push constant 6`)
- **Strength Reduction**: Power-of-2 multiplications replaced with shift sequences (e.g., `x * 4` uses `add` instead of `Math.multiply`)
- **Peephole Optimization**: Eliminates redundant patterns (double not/neg, push-pop same location, identity add)
- **Parallel Processing**: Directory mode uses Rayon for concurrent file compilation
- **Containerization**: Podman/Docker multi-stage build (~12MB image)

## Testing

```bash
# Unit tests
cargo test

# All tests (unit + integration + fuzz + optimizer)
cargo test --release

# Integration tests only
cargo test --test integration_test

# Property-based fuzz tests
cargo test --test fuzz_test

# Optimizer integration tests
cargo test --test optimizer_test

# Compile test programs
cargo run -- ../Seven/
cargo run -- ../ConvertToBin/
cargo run -- ../Square/
cargo run -- ../Average/
cargo run -- ../Pong/
cargo run -- ../ComplexArrays/
```

### Test Coverage

- **159 total tests** (97 unit + 21 fuzz + 18 integration + 22 optimizer + 1 doc)
- Property-based tests using proptest for invariant verification
- Optimizer-specific integration tests for constant folding and peephole optimization

### Verification in VM Emulator

1. Open `tools/VMEmulator.sh`
2. Load the compiled directory (e.g., `Square/`)
3. Run with "No animation" for speed
4. Verify expected program behavior

## Container Build (Podman/Docker)

```bash
# Build the container image
podman build -t jack-compiler:1.3.0 -f Containerfile .

# Run the compiler
podman run --rm -v $(pwd):/workspace jack-compiler:1.3.0 Main.jack

# Compile a directory
podman run --rm -v $(pwd):/workspace jack-compiler:1.3.0 /workspace/Square/

# Disable optimization
podman run --rm -v $(pwd):/workspace jack-compiler:1.3.0 --no-optimize Main.jack
```

## Compilation Rules

### Subroutines

| Kind | Preamble |
|------|----------|
| Constructor | `push constant nFields` + `call Memory.alloc 1` + `pop pointer 0` |
| Method | `push argument 0` + `pop pointer 0` |
| Function | (none) |

### Expressions

| Jack | VM Code |
|------|---------|
| `true` | `push constant 0` + `not` |
| `false`, `null` | `push constant 0` |
| `this` | `push pointer 0` |
| `x * y` | `call Math.multiply 2` (or shift sequence if y is power of 2) |
| `x / y` | `call Math.divide 2` |
| `-x` | `neg` |
| `~x` | `not` |

### Array Access

**Read `a[i]`:**
```
push [a]
[compile i]
add
pop pointer 1
push that 0
```

**Write `let a[i] = expr`:**
```
push [a]
[compile i]
add
[compile expr]
pop temp 0
pop pointer 1
push temp 0
pop that 0
```

## License

MIT License - Part of nand2tetris course materials

## Author

**Žarko Gvozdenović** (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)
