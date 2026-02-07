# VM Translator (Full Stack)

**Stack VM to Hack Assembly translator for nand2tetris Project 08**

Version: 2.0.2
Rust Edition: 2024 (1.92+)
Python3 version: 3.10+

---

## Executive Summary

A high-performance, zero-allocation VM translator that converts Stack VM bytecode (.vm) into Hack assembly language (.asm). Implements the complete VM specification including all 20 VM commands: 9 arithmetic/logical, 8 memory access patterns, 3 branching commands, and 3 function commands with bootstrap code generation.

**Pipeline Overview:**

```
┌─────────────────────────────────────────────────────────────────┐
│                     VM Translation Pipeline                     │
└─────────────────────────────────────────────────────────────────┘

    VM Bytecode                Hack Assembly              Machine Code
    (.vm files)                (.asm files)               (.hack files)
         │                          │                          │
         │                          │                          │
    ┌────▼────┐              ┌──────▼──────┐           ┌───────▼───────┐
    │  push   │              │    @7       │           │ 0000000000111 │
    │constant │   Parser     │    D=A      │ Assembler │ 1110110000010 │
    │    7    │─────────────▶│    @SP      │──────────▶│ 0000000000000 │
    │         │              │    A=M      │  (Proj 6) │ 1111110000100 │
    │   call  │   CodeGen    │    M=D      │           │ 1110001100001 │
    │ Foo.bar │              │    @SP      │           │ 0000000000000 │
    └─────────┘              │    M=M+1    │           └───────────────┘
         │                   └─────────────┘                  │
         │                          │                         │
         └──────────────────────────┴─────────────────────────┘
                        This Project (vm-translator)
```

**Key Features:**

- **20 VM Commands**: Full VM specification (arithmetic, memory, branching, functions)
- **8 Memory Segments**: constant, local, argument, this, that, pointer, temp, static
- **Bootstrap Code**: Automatic SP initialization and Sys.init call for multi-file programs
- **Directory Mode**: Translate entire directories into single .asm files
- **Zero-Allocation Hot Paths**: Manual digit writing eliminates format!() allocations
- **Type-Driven Safety**: SegmentAccess enum makes impossible states unrepresentable
- **Zero Panic Points**: No .expect(), no unreachable!() in critical paths
- **Comprehensive Testing**: Unit + integration + property-based fuzzing tests
- **Production-Grade Error Handling**: Line-numbered errors with clear context
- **Coursera Compatible**: Includes Python 3 standalone for autograder submission

---

## Quick Start

### Build

```bash
cd projects/08/vm-translator
cargo build --release
```

The optimized binary will be at `target/release/vm-translator`.

### Run

**Single File (no bootstrap):**
```bash
./target/release/vm-translator SimpleAdd.vm
# Produces: SimpleAdd.asm
```

**Directory Mode (with bootstrap):**
```bash
./target/release/vm-translator FibonacciElement/
# Produces: FibonacciElement/FibonacciElement.asm
```

**Verbose Output:**
```bash
./target/release/vm-translator -v NestedCall/
# Shows detailed translation progress
```

### Test

```bash
# Run all tests (unit + integration + fuzzing)
cargo test

# Run only unit tests
cargo test --lib

# Run only integration tests
cargo test --test integration_test

# Run fuzzing tests (requires more time)
cargo test --test fuzz_test

# Extensive fuzzing (10000 cases per test)
PROPTEST_CASES=10000 cargo test --test fuzz_test
```

### Lint

```bash
# Check for warnings
cargo clippy

# Format code
cargo fmt
```

### Container (Podman/Docker)

```bash
# Build container image (~12MB Alpine-based)
./vm-translate.sh build

# Translate single file
./vm-translate.sh SimpleAdd.vm

# Translate directory (with bootstrap)
./vm-translate.sh FunctionCalls/FibonacciElement/

# Run tests in container
./vm-translate.sh test

# Interactive shell
./vm-translate.sh shell

# Use Docker instead of Podman
CONTAINER_ENGINE=docker ./vm-translate.sh build
```

**Direct Podman commands:**
```bash
# Build image
podman build -t vm-translator:2.0.2 -f Containerfile .

# Run translation (single file)
podman run --rm -v $(pwd):/workspace vm-translator:2.0.2 SimpleAdd.vm

# Run translation (directory mode with bootstrap)
podman run --rm -v $(pwd):/workspace vm-translator:2.0.2 /workspace/FunctionCalls/FibonacciElement/

# Interactive debugging
podman run --rm -it -v $(pwd):/workspace --entrypoint /bin/sh vm-translator:2.0.2
```

---

## Architecture Overview

### Module Structure

```
vm-translator/
├── src/
│   ├── main.rs       # CLI interface with directory mode
│   ├── lib.rs        # Translation orchestration
│   ├── parser.rs     # VM command lexing and parsing
│   ├── codegen.rs    # Hack assembly code generation
│   ├── memory.rs     # Memory segment address calculation
│   ├── bootstrap.rs  # VM initialization code
│   └── error.rs      # Comprehensive error types
└── tests/
    ├── integration_test.rs  # End-to-end validation
    └── fuzz_test.rs         # Property-based fuzzing
```

### Data Flow

```
Input: "push constant 7" or Directory with .vm files
         │
         ▼
┌────────────────────┐
│   parse_line()     │  Tokenize, validate, construct VMCommand
│   (parser.rs)      │
└────────┬───────────┘
         │ VMCommand::Push { segment: Constant, index: 7 }
         ▼
┌────────────────────┐
│  CodeGenerator     │  Generate assembly instructions
│  translate()       │
│  (codegen.rs)      │
└────────┬───────────┘
         │ Assembly code written to buffer
         ▼
Output:
@7
D=A
@SP
A=M
M=D
@SP
M=M+1
```

### Core Types

**VMCommand (parser.rs):**
```rust
#[derive(Debug, Clone, PartialEq)]
pub enum VMCommand {
    Arithmetic(ArithmeticOp),
    Push { segment: Segment, index: u16 },
    Pop { segment: Segment, index: u16 },
    Label { name: String },
    Goto { label: String },
    IfGoto { label: String },
    Function { name: String, num_locals: u16 },
    Call { name: String, num_args: u16 },
    Return,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArithmeticOp {
    Add, Sub, Neg,     // Arithmetic
    Eq, Lt, Gt,        // Comparisons
    And, Or, Not,      // Logic
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Constant,          // Push only, immediate values
    Local,             // Function local variables
    Argument,          // Function arguments
    This,              // Object fields (this object)
    That,              // Array elements (that array)
    Pointer,           // THIS/THAT pointer manipulation
    Temp,              // Temporary variables (RAM[5-12])
    Static,            // File-scope static variables
}
```

---

## VM Language Specification

### Part I: Arithmetic/Logical Commands (9 commands)

| Command | Stack Effect | Operation | Assembly Size |
|---------|-------------|-----------|---------------|
| `add` | `x, y -> x+y` | Binary addition | ~5 instructions |
| `sub` | `x, y -> x-y` | Binary subtraction | ~5 instructions |
| `neg` | `x -> -x` | Unary negation | ~3 instructions |
| `eq` | `x, y -> (x==y ? -1 : 0)` | Equality comparison | ~15 instructions |
| `lt` | `x, y -> (x<y ? -1 : 0)` | Less than | ~15 instructions |
| `gt` | `x, y -> (x>y ? -1 : 0)` | Greater than | ~15 instructions |
| `and` | `x, y -> x&y` | Bitwise AND | ~5 instructions |
| `or` | `x, y -> x\|y` | Bitwise OR | ~5 instructions |
| `not` | `x -> !x` | Bitwise NOT | ~3 instructions |

### Part II: Memory Access Commands (8 segments)

| Segment | Base Pointer | Valid Range | Access Type | Example |
|---------|--------------|-------------|-------------|---------|
| `constant` | N/A | 0-32767 | Push only | `push constant 42` |
| `local` | RAM[1] (LCL) | 0+ | Indirect | `push local 2` |
| `argument` | RAM[2] (ARG) | 0+ | Indirect | `pop argument 1` |
| `this` | RAM[3] (THIS) | 0+ | Indirect | `push this 5` |
| `that` | RAM[4] (THAT) | 0+ | Indirect | `pop that 8` |
| `pointer` | RAM[3-4] | 0-1 only | Direct | `pop pointer 0` |
| `temp` | RAM[5-12] | 0-7 only | Direct | `push temp 3` |
| `static` | RAM[16+] | 0-239 | Direct | `pop static 5` |

### Part III: Program Flow Commands (3 commands)

| Command | Format | Description |
|---------|--------|-------------|
| `label` | `label LOOP` | Declares branch target |
| `goto` | `goto LOOP` | Unconditional jump |
| `if-goto` | `if-goto LOOP` | Pop stack, jump if != 0 |

**Label Scoping:**
- Labels are scoped to the current function: `functionName$labelName`
- Example: `label LOOP` in `Foo.bar` becomes `(Foo.bar$LOOP)`

### Part IV: Function Commands (3 commands)

| Command | Format | Description |
|---------|--------|-------------|
| `function` | `function f n` | Declare function with n locals |
| `call` | `call f n` | Call function with n args on stack |
| `return` | `return` | Return from function |

**Call Frame Layout:**
```
         ┌───────────────┐
ARG ──▶  │ argument 0    │
         │ argument 1    │
         │ ...           │
         │ argument n-1  │
         ├───────────────┤
         │ return addr   │  ◀── saved by call
         │ saved LCL     │
         │ saved ARG     │
         │ saved THIS    │
         │ saved THAT    │
         ├───────────────┤
LCL ──▶  │ local 0       │  ◀── initialized to 0 by function
         │ local 1       │
         │ ...           │
SP  ──▶  │ (working)     │
         └───────────────┘
```

---

## Bootstrap Code

For multi-file programs containing Sys.vm, the translator generates bootstrap code:

```asm
// Bootstrap code
@256
D=A
@SP
M=D              // SP = 256
// call Sys.init 0
@Sys.init$ret.BOOTSTRAP
D=A
@SP
A=M
M=D
@SP
M=M+1            // push return address
@LCL
D=M
@SP
A=M
M=D
@SP
M=M+1            // push LCL
// ... push ARG, THIS, THAT
@SP
D=M
@5
D=D-A
@ARG
M=D              // ARG = SP - 5
@SP
D=M
@LCL
M=D              // LCL = SP
@Sys.init
0;JMP            // goto Sys.init
(Sys.init$ret.BOOTSTRAP)
(HALT)
@HALT
0;JMP            // halt sentinel
```

---

## Memory Layout

### Hack Computer Memory Map

```
┌────────────────────┬─────────────────────────────────────┐
│  Address Range     │  Usage                              │
├────────────────────┼─────────────────────────────────────┤
│  RAM[0]            │  SP (Stack Pointer)                 │
│  RAM[1]            │  LCL (Local base pointer)           │
│  RAM[2]            │  ARG (Argument base pointer)        │
│  RAM[3]            │  THIS (this pointer)                │
│  RAM[4]            │  THAT (that pointer)                │
│  RAM[5-12]         │  Temp segment (temp 0-7)            │
│  RAM[13-15]        │  General purpose (R13-R15)          │
│  RAM[16-255]       │  Static variables                   │
│  RAM[256-2047]     │  Stack                              │
│  RAM[2048-16383]   │  Heap (this/that segments)          │
│  RAM[16384-24575]  │  Memory-mapped I/O                  │
└────────────────────┴─────────────────────────────────────┘
```

---

## Error Handling

All errors include line numbers and contextual information:

```rust
pub enum VMError {
    InvalidCommand { line: usize, file: String, command: String },
    InvalidSegment { line: usize, file: String, segment: String },
    IndexOutOfRange { line: usize, file: String, index: u16, segment: String },
    PopToConstant { line: usize, file: String },
    InvalidPointerIndex { line: usize, file: String, index: u16 },
    InvalidTempIndex { line: usize, file: String, index: u16 },
    MissingArgument { line: usize, file: String, command: String },
    InvalidNumber { line: usize, file: String, value: String },
    InvalidLabelName { line: usize, file: String, name: String },
    InvalidFunctionName { line: usize, file: String, name: String },
    FileRead { path: String, source: std::io::Error },
    FileWrite { path: String, source: std::io::Error },
    NoVmFiles { path: String },
    InvalidPath { path: String },
}
```

---

## Testing

### Test Programs

The translator includes tests for all 11 nand2tetris test programs:

**Project 07 (5 tests):**
1. SimpleAdd - Basic arithmetic
2. StackTest - All 9 arithmetic/logical operations
3. BasicTest - All memory segments
4. PointerTest - Pointer manipulation
5. StaticTest - Static variables

**Project 08 (6 tests):**
1. BasicLoop - Branching with label/if-goto
2. FibonacciSeries - Branching with goto
3. SimpleFunction - Function declaration and return
4. NestedCall - Nested function calls with bootstrap
5. FibonacciElement - Recursion with bootstrap
6. StaticsTest - Multi-file statics with bootstrap

### Test Coverage

```
Unit tests:        43 tests (parser, codegen, memory, bootstrap)
Integration tests: 23 tests (in-memory + all 11 file-based test programs)
Fuzzing tests:     23 tests (property-based with proptest)
Total:             89 tests
```

---

## Dependencies

**Production:**
- `thiserror = "2.0"` - Zero-cost error types

**Development:**
- `proptest = "1.4"` - Property-based fuzzing

**Total:** 2 dependencies (1 prod, 1 dev)

---

---

## Coursera Submission

For Coursera autograder submission, use the Python standalone:

```bash
# Copy VMTranslator.py to the directory containing .vm files
cp VMTranslator.py /path/to/test/

# Single file translation
python3 VMTranslator.py SimpleAdd.vm

# Directory translation (with bootstrap)
python3 VMTranslator.py FibonacciElement/
```

The Python implementation is a complete, standalone translation of the Rust codebase.

---

## FAQ

**Q: When is bootstrap code generated?**
A: Bootstrap code is generated when translating a directory that contains Sys.vm.

**Q: How are labels scoped?**
A: Labels use the format `functionName$labelName`. For example, `label LOOP` inside `Foo.bar` becomes `(Foo.bar$LOOP)`.

**Q: How are return addresses generated?**
A: Return addresses use the format `functionName$ret.N` where N is a unique counter.

**Q: What if I get "no .vm files found"?**
A: Ensure the directory contains at least one .vm file.

**Q: Can I translate multiple directories?**
A: Run the translator separately for each directory.

---

## References

- [nand2tetris Project 08](https://www.nand2tetris.org/project08)
- [VM Language Specification](https://www.nand2tetris.org/_files/ugd/44046b_7f9c6c8344064e2696f29bc3f066dd17.pdf)
- [Hack Assembly Language](https://www.nand2tetris.org/_files/ugd/44046b_89a8e226476741a3b7c5204575b8a0b2.pdf)

---

## License

MIT License - Part of nand2tetris course materials

---

## Author

**Žarko Gvozdenović** (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)
