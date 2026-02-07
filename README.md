# nand2tetris: From NAND Gates to Operating System

[![CI](https://github.com/zarko-visaurum/nand2tetris-i-ii/actions/workflows/ci.yml/badge.svg)](https://github.com/zarko-visaurum/nand2tetris-i-ii/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

A complete implementation of the [nand2tetris](https://www.nand2tetris.org/) course (Parts I & II) — building a general-purpose computer system from first principles: starting with a single NAND gate and ending with a functioning operating system, compiler, and application software.

```
                          nand2tetris Stack
   ┌─────────────────────────────────────────────────────────┐
   │  Project 09   HackTrader (Jack)                         │  Application
   │               Black-Scholes market making simulator     │
   ├─────────────────────────────────────────────────────────┤
   │  Project 12   Jack OS (Jack)                            │  Operating System
   │               Math, Memory, Screen, String, I/O         │
   ├─────────────────────────────────────────────────────────┤
   │  Project 11   Jack Compiler (Rust)                      │  Compiler Backend
   │               Codegen, optimizer, symbol tables         │
   ├─────────────────────────────────────────────────────────┤
   │  Project 10   Jack Analyzer (Rust)                      │  Compiler Frontend
   │               Tokenizer, recursive-descent parser, AST  │
   ├─────────────────────────────────────────────────────────┤
   │  Project 08   VM Translator II (Rust)                   │  Full VM Translation
   │               Functions, bootstrap, call frames         │
   ├─────────────────────────────────────────────────────────┤
   │  Project 07   VM Translator I (Rust)                    │  Stack VM Translation
   │               Arithmetic, memory segments               │
   ├─────────────────────────────────────────────────────────┤
   │  Project 06   Hack Assembler (Rust)                     │  Assembler
   │               Two-pass, symbol resolution               │
   ├─────────────────────────────────────────────────────────┤
   │  Projects     Assembly Programs (Hack ASM)              │  Software
   │  04-05        Multiplication, I/O, CPU, Computer        │
   ├─────────────────────────────────────────────────────────┤
   │  Projects     Digital Logic (HDL)                       │  Hardware
   │  01-03        Gates, ALU, RAM, PC                       │
   └─────────────────────────────────────────────────────────┘
                         NAND gate (axiom)
```

## Tech Stack

| Layer | Language | Key Techniques |
|-------|----------|----------------|
| **Hardware** (P01-03, P05) | HDL | Balanced-tree gate design, multi-output fan-out, CLA-aware adder chain |
| **Assembly** (P04) | Hack ASM | O(16) shift-and-add multiplication, memory-mapped I/O |
| **Assembler** (P06) | Rust 2024 | Two-pass assembly, typestate pattern (`Instruction` -> `ResolvedInstruction`), PHF symbol tables |
| **VM Translator** (P07-08) | Rust 2024 | Zero-allocation hot paths, `AM=M-1` instruction fusion, bootstrap code generation |
| **Compiler** (P10-11) | Rust 2024 | Recursive-descent parser, constant folding, peephole optimization, strength reduction |
| **OS** (P12) | Jack | Bidirectional-coalescing heap allocator, midpoint circle algorithm, binary long division with 2qy optimization |
| **Application** (P09) | Jack | Black-Scholes options pricing on 16-bit hardware, fixed-point arithmetic, pixel-art UI with dithered gradients |

## Why These Choices

Design decisions that shaped this implementation:

| Decision | Reasoning |
|----------|-----------|
| **Typestate pattern** for assembler | `Instruction` → `ResolvedInstruction` makes unresolved symbol references *unrepresentable* after pass 1. The compiler enforces correctness—no runtime checks needed. |
| **Zero-allocation hot paths** | Compilers are throughput-bound. `format!()` creates heap allocations; manual `push_str()` with pre-sized buffers does not. This pattern carries from P06 through P12. |
| **`AM=M-1` instruction fusion** | A single Hack instruction decrements SP and loads the address. Naive code uses 3 instructions. 40% code size reduction in VM arithmetic. |
| **Divide-first arithmetic** on 16-bit | `(a * b) / c` overflows; `(a / c) * b` often doesn't. VWAP, theta, and P&L calculations restructure operations to stay within ±32767. |
| **Bidirectional heap coalescing** | Standard allocators merge freed blocks with their successor. Merging with *both* neighbors (Doug Lea-style) reduces fragmentation under churn—critical for a 32KB heap. |
| **Lookup tables over Taylor series** | `exp()`, `ln()`, `Φ(x)` via Taylor expansion overflow 16-bit intermediates. Pre-computed tables with linear interpolation give O(1) evaluation and bounded error. |
| **Property-based fuzz testing** | Unit tests check *known* inputs; proptest checks *invariants* ("parser never panics," "optimizer reaches fixed point"). This catches edge cases humans don't anticipate. |
| **Visitor-ready AST with Span metadata** | Project 10's AST carries source location through the entire pipeline. Project 11 reuses it via visitor pattern—no parser rewrite needed. Design for extension, not modification. |
| **Symbol table mirrors VM architecture** | Two-level scoping (class/subroutine) with per-kind counters maps directly to Hack VM segments. The data structure *is* the domain model—invalid states are unrepresentable. |
| **Bounded error accumulation** | Collect up to 20 errors before stopping. One syntax error can cascade into hundreds; unbounded collection produces noise. The limit is a UX tradeoff, not a technical limitation. |
| **Strength reduction** for multiply | `x * 4` compiles to `x + x; result + result` instead of `call Math.multiply`. No multiply instruction exists—so eliminate the function call entirely when possible. |

## Highlights

### Hardware (Projects 01-05)

- **28 HDL chips** from NAND to a complete 16-bit computer
- **ALU**: 6-function unit with zero/negation status flags and multi-output fan-out
- **CPU**: Zero-redundant-gate instruction decoder with reference-quality documentation
- **RAM hierarchy**: Consistent 8-way / 64-way / 512-way / 4K-way / 4-way decomposition

### Software Tools (Projects 06-08, 10-11) — Rust 2024 Edition

Five production-grade tools sharing common engineering patterns:

- **Zero-allocation hot paths** — manual digit writing, `push_str` batching, pre-allocated buffers
- **Zero panic points** — no `.expect()`, no `unreachable!()`, compiler-verified exhaustive matches
- **Property-based fuzz testing** with `proptest` across all projects
- **Multi-stage Alpine Containerfiles** for portable deployment (~12 MB images)
- **Comprehensive test suites** — 400+ tests across all Rust projects

### HackTrader (Project 09) — Market Making on 16-bit Hardware

A Black-Scholes options pricing and market making simulator running entirely on the Hack platform:

- **Options Greeks** (Delta, Gamma, Theta, Vega) via lookup tables with linear interpolation
- **Order book engine** with limit orders, market orders, and liquidity replenishment
- **Overflow-safe arithmetic** — divide-first VWAP, safe weighted averaging
- **Pixel-art splash screen** with block-letter rendering and dithered candlestick charts

### Jack OS (Project 12)

- **Memory allocator**: First-fit free list with bidirectional coalescing (Doug Lea-style)
- **Screen rendering**: Midpoint circle algorithm (Bresenham-style), word-aligned fast fills
- **Math library**: O(n) multiply via shift-and-add with `powersOfTwo` bit-testing, binary long division with 2qy tracking

## Building & Testing

Each Rust project (P06, P07, P08, P10, P11) builds independently:

```bash
cd projects/06/hack-assembler && cargo build --release && cargo test
cd projects/07/vm-translator  && cargo build --release && cargo test
cd projects/08/vm-translator  && cargo build --release && cargo test
cd projects/10/jack-analyzer  && cargo build --release && cargo test
cd projects/11/jack-compiler  && cargo build --release && cargo test
```

HDL projects (P01-03, P05) are tested with the nand2tetris Hardware Simulator.
Assembly programs (P04) are tested with the CPU Emulator.
Jack programs (P09, P12) are tested with the VM Emulator.

## Repository Structure

```
nand2tetris-i-ii/
├── projects/
│   ├── 01/          15 HDL chips: Not, And, Or, Mux, DMux, *16, *Way
│   ├── 02/          5 HDL chips: HalfAdder, FullAdder, Add16, Inc16, ALU
│   ├── 03/          8 HDL chips: Bit, Register, RAM8..RAM16K, PC
│   ├── 04/          Mult.asm (shift-and-add), Fill.asm (screen I/O)
│   ├── 05/          CPU.hdl, Memory.hdl, Computer.hdl
│   ├── 06/          hack-assembler/  (Rust crate)
│   ├── 07/          vm-translator/   (Rust crate, single-file VM)
│   ├── 08/          vm-translator/   (Rust crate, full VM with functions)
│   ├── 09/          HackTrader/      (9 Jack source files)
│   ├── 10/          jack-analyzer/   (Rust crate)
│   ├── 11/          jack-compiler/   (Rust crate)
│   ├── 12/          8 Jack OS modules: Math, Memory, Screen, Output, ...
│   └── 13/          Next steps: high-performance Hack emulator (500 MHz - 1 GHz)
├── LICENSE          MIT
└── README.md        This file
```

## Project Documentation

Detailed documentation for each major component:

| Project | Documentation | Description |
|---------|---------------|-------------|
| **P06** | [Hack Assembler](projects/06/hack-assembler/README.md) | Two-pass assembler with PHF symbol tables, typestate pattern |
| **P07** | [VM Translator I](projects/07/vm-translator/README.md) | Stack arithmetic and memory segment translation |
| **P08** | [VM Translator II](projects/08/vm-translator/README.md) | Function calls, bootstrap, frame management |
| **P09** | [HackTrader](projects/09/HackTrader/README.md) | Black-Scholes options pricing on 16-bit hardware |
| **P10** | [Jack Analyzer](projects/10/jack-analyzer/README.md) | Tokenizer, recursive-descent parser, AST with Span |
| **P11** | [Jack Compiler](projects/11/jack-compiler/README.md) | Code generation, constant folding, peephole optimization |
| **P12** | [Jack OS](projects/12/README.md) | Memory allocator, screen rendering, math library |

## Requirements

- **Rust 1.92+** (edition 2024) for software tools
- **Python 3.10+** for Coursera submission scripts (standard library only—no external packages)
- **nand2tetris Software Suite** for hardware simulation and VM emulation

## Author

**Žarko Gvozdenović** (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)

## License

[MIT](LICENSE)
