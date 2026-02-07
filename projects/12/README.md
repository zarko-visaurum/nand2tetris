# Jack OS - Operating System for the Hack Computer

A Jack OS implementation for the nand2tetris course (Project 12).

## Features

### Memory Management (Memory.jack, Array.jack)
- **First-fit heap allocation** with block coalescing
- Address-sorted free list for O(1) adjacency detection
- Block format: `[size][next_ptr | user_data...]`
- Heap range: 2048-16383 (14,336 words)
- Uses the "array trick": `memory = 0` allows `memory[addr]` to access `RAM[addr]`

### Integer Arithmetic (Math.jack)
- **Shift-and-add multiplication**: O(log n) iterations, iterates over smaller operand
- **Binary long division** with `twoQY` optimization avoiding redundant multiplication
- **Binary search square root**: exactly 8 iterations, overflow-safe via sign check
- Pre-computed powers of two for O(1) bit tests
- Handles all edge cases: zero, negatives, -32768

### Graphics (Screen.jack)
- **Bresenham's line algorithm** for diagonal lines (integer arithmetic only)
- **Word-aligned horizontal fills**: 16x speedup for rectangles
- **Midpoint circle algorithm** with symmetric horizontal fill
- Pre-computed bit masks for O(1) pixel manipulation
- Radius constraint: r ≤ 181 (since 182² overflows 15 bits)

### Text Output (Output.jack)
- 23×64 character grid (8×11 pixels per character)
- Character bitmaps stored as 11-row arrays
- **Zero-allocation integer printing** via recursion
- Cursor management with automatic line wrapping

### Keyboard Input (Keyboard.jack)
- Memory-mapped I/O at address 24576
- Busy-wait key press/release detection
- Backspace handling in `readLine()`
- Automatic character echo to screen

### System Control (Sys.jack)
- Critical initialization order: Memory → Math → Screen → Output → Keyboard
- Calibrated busy-loop `wait()`
- Zero-allocation `error()` display

## Build & Test

### Prerequisites
- Python 3 (for JackCompiler.py from Project 11)
- nand2tetris tools (VM Emulator)

### Compilation

Compile all OS classes:

```bash
cd projects/12
for f in Array Math Memory String Screen Output Keyboard Sys; do
    python3 ../11/JackCompiler.py ${f}.jack
done
mv *.vm compiled_os_vm/
```

### Automated VM Comparison Test

Compare compiled output against the reference implementation:

```bash
python3 compare_vm.py
```

Expected output:
```
======================================================================
Jack OS VM Code Comparison: Implementation vs Reference
======================================================================

[PASS] Array.vm       Size: ref=359, impl=361 bytes (ratio=1.01)
[PASS] Math.vm        Size: ref=5,372, impl=5,533 bytes (ratio=1.03)
[PASS] Memory.vm      Size: ref=4,920, impl=5,077 bytes (ratio=1.03)
[PASS] String.vm      Size: ref=5,531, impl=5,517 bytes (ratio=1.00)
[PASS] Screen.vm      Size: ref=11,210, impl=11,577 bytes (ratio=1.03)
[PASS] Output.vm      Size: ref=30,975, impl=28,827 bytes (ratio=0.93)
[PASS] Keyboard.vm    Size: ref=1,629, impl=1,644 bytes (ratio=1.01)
[PASS] Sys.vm         Size: ref=1,267, impl=1,245 bytes (ratio=0.98)

Modules: 8/8 passed
Total size: ref=61,263 bytes, impl=59,781 bytes (ratio=0.98x)
[SUCCESS] All modules have matching function signatures!
```

### VM Emulator Unit Tests

1. **MathTest**: Tests multiply, divide, sqrt, min, max, abs
   - Load `MathTest/` in VM Emulator with our compiled `Math.vm`
   - Run `MathTest.tst`
   - Expected: RAM[8000-8013] = `6, -180, -18000, -18000, 0, 3, -3000, 0, 3, 181, 123, 123, 27, 32767`

2. **MemoryTest**: Tests alloc, deAlloc, coalescing
   - Load `MemoryTest/` with our `Memory.vm` and `Array.vm`
   - Run `MemoryTest.tst`
   - Expected: RAM[8000-8005] = `333, 334, 222, 122, 100, 10`

3. **ArrayTest**: Tests Array.new, dispose
   - Load `ArrayTest/` with our `Array.vm` and `Memory.vm`
   - Run `ArrayTest.tst`

### Visual Tests

These require manual verification in VM Emulator:

4. **StringTest**: String operations display correctly
5. **OutputTest**: Text positioning and character rendering
6. **ScreenTest**: House + sun picture renders correctly
7. **KeyboardTest**: Key press/release, readLine, readInt
8. **SysTest**: 2-second delay via Sys.wait

### Integration Test (Pong)

```bash
# Copy all OS classes to Pong game
cp *.jack ../11/Pong/
cd ../11/Pong
python3 ../JackCompiler.py .
# Run in VM Emulator
```

## Architecture

| Class | Lines | Purpose | Key Algorithm |
|-------|-------|---------|---------------|
| Memory.jack | 243 | Heap management | First-fit + coalescing |
| Array.jack | 51 | Dynamic arrays | Thin Memory wrapper |
| Math.jack | 332 | Integer arithmetic | Shift-add, binary div/sqrt |
| String.jack | 318 | Character strings | Recursive setInt |
| Screen.jack | 489 | Graphics primitives | Bresenham, midpoint circle |
| Output.jack | 595 | Text output | Character bitmaps |
| Keyboard.jack | 191 | Input handling | Busy-wait I/O |
| Sys.jack | 150 | System control | Init ordering |

**Total**: ~2,201 lines of Jack code

## Error Codes

| Code | Class | Condition |
|------|-------|-----------|
| 1 | Sys.wait | duration ≤ 0 |
| 2 | Array.new | size ≤ 0 |
| 3 | Math.divide | y = 0 |
| 4 | Math.sqrt | x < 0 |
| 5 | Memory.alloc | size ≤ 0 |
| 6 | Memory.alloc | heap exhausted |
| 7-9 | Screen | coordinates out of bounds |
| 12-13 | Screen.drawCircle | invalid radius |
| 14-19 | String | various string errors |
| 20 | Output.moveCursor | position out of bounds |

## Technical Notes

### Overflow Handling
- **sqrt**: Detects overflow via sign check (negative result means overflow)
- **divide**: Checks `y > 16383` before doubling to prevent overflow
- **multiply**: Works with absolute values, applies sign at end

### Performance Optimizations
- Pre-computed bit masks avoid per-pixel multiplication
- Horizontal lines fill entire 16-bit words where possible
- Circle algorithm computes one octant, reflects to all 8
- String.setInt uses recursion to avoid digit array allocation

### Memory Layout
```
0-15:       Virtual registers (SP, LCL, ARG, THIS, THAT, R5-R15)
16-255:     Static variables
256-2047:   Stack
2048-16383: Heap (managed by Memory.jack)
16384-24575: Screen (8K words = 256 rows × 32 words)
24576:      Keyboard
```

## Files

```
projects/12/
├── Array.jack          # Dynamic array allocation
├── Keyboard.jack       # Keyboard input
├── Math.jack           # Integer arithmetic
├── Memory.jack         # Heap management
├── Output.jack         # Text output
├── Screen.jack         # Graphics primitives
├── String.jack         # String operations
├── Sys.jack            # System control
├── LICENSE             # MIT License
├── README.md           # This file
├── compare_vm.py       # VM comparison tool
├── ref_os_vm/          # Reference VM implementation
├── compiled_os_vm/     # Our compiled VM output
└── *Test/              # Unit test directories
```

## Author

**Žarko Gvozdenović** (zarko@visaurum.nl) — [Visaurum](https://www.linkedin.com/company/visaurum-b-v/)

## License

MIT License. See [LICENSE](LICENSE) for details. Part of nand2tetris course materials.


