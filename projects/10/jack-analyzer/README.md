# JackAnalyzer

Syntax analyzer for the Jack programming language (nand2tetris Project 10).

## Overview

JackAnalyzer performs lexical analysis (tokenization) and syntactic analysis (parsing) on Jack source files, producing XML output that represents the program structure.

## Building

```bash
cargo build --release
```

The binary will be at `target/release/JackAnalyzer`.

## Usage

```bash
# Analyze a single file
./JackAnalyzer Main.jack

# Analyze a directory (parallel processing)
./JackAnalyzer Square/
```

### Output

For each input file `Foo.jack`, the analyzer produces:
- `FooT.xml` - Token stream (flat list of all tokens)
- `Foo.xml` - Parse tree (nested XML structure)

## Python Version

A single-file Python implementation is also provided for Coursera submission:

```bash
python3 ../JackAnalyzer.py Main.jack
python3 ../JackAnalyzer.py Square/
```

## Architecture

```
src/
├── main.rs      # CLI entry point
├── lib.rs       # Public API, orchestration
├── tokenizer.rs # Lexical analysis
├── token.rs     # Token types and spans
├── parser.rs    # Recursive descent parser
├── ast.rs       # AST node definitions
├── xml.rs       # XML output generation
└── error.rs     # Error types and diagnostics
```

## Features

- **Parallel Processing**: Directory mode uses Rayon for concurrent file analysis
- **Rich Diagnostics**: Source-context error messages with line numbers, caret pointers, and expected-token hints
- **Multi-Error Reporting**: Accumulates multiple errors instead of stopping at first
- **Error Recovery**: Synchronizes at statement/declaration boundaries
- **O(N) Tokenization**: Incremental byte-offset tracking avoids per-token rescanning
- **Project 11 Ready**: AST supports Visitor pattern for code generation
- **Property-Based Fuzzing**: 21 proptest tests for robustness
- **Containerization**: Podman/Docker multi-stage build (~12MB image)
- **Zero-Allocation XML**: Pre-sized buffers eliminate hot-path allocations

## Testing

```bash
# Unit tests
cargo test

# Fuzz tests (property-based)
cargo test --test fuzz_test

# All tests
cargo test --release

# Integration tests
cargo run -- ../Square/
cargo run -- ../ArrayTest/
cargo run -- ../ExpressionLessSquare/
```

## Container Build (Podman/Docker)

```bash
# Build the container image
podman build -t jack-analyzer:1.0.4 -f Containerfile .

# Run the analyzer
podman run --rm -v $(pwd):/workspace jack-analyzer:1.0.4 Main.jack

# Analyze a directory
podman run --rm -v $(pwd):/workspace jack-analyzer:1.0.4 /workspace/Square/
```

## License

MIT License - Part of nand2tetris course materials

## Author

**Žarko Gvozdenović** (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)
