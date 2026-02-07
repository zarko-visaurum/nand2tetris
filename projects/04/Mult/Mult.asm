// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/4/Mult.asm

// Multiplies R0 and R1 and stores the result in R2.
// (R0, R1, R2 refer to RAM[0], RAM[1], and RAM[2], respectively.)
// The algorithm is based on shift-and-add (binary multiplication):
//
// ## Algorithm (O(16) — constant time)
//
// ```
// result = 0
// shifted = R0        // value to be shifted left
// mask = 1            // bit tester for R1
// for i = 0..15:
//     if (R1 & mask) != 0:
//         result += shifted
//     shifted += shifted  // left shift
//     mask += mask         // advance to next bit
// R2 = result
// ```

    @R2
    M=0             // R2 = 0 (initialize result)

    @R0
    D=M
    @shifted
    M=D             // shifted = R0

    @mask
    M=1             // mask = 1 (tests bit 0 first)

    @16
    D=A
    @i
    M=D             // i = 16 (loop counter)

(LOOP)
    @i
    D=M
    @END
    D;JLE           // if i <= 0, done

    // Test: R1 & mask
    @R1
    D=M
    @mask
    D=D&M           // D = R1 & mask
    @SKIP
    D;JEQ           // if bit not set, skip addition

    // Bit is set: R2 += shifted
    @shifted
    D=M
    @R2
    M=D+M           // R2 = R2 + shifted

(SKIP)
    // shifted = shifted + shifted (left shift by 1)
    @shifted
    D=M
    M=D+M           // shifted *= 2

    // mask = mask + mask (advance to next bit)
    @mask
    D=M
    M=D+M           // mask *= 2

    // i--
    @i
    M=M-1

    @LOOP
    0;JMP           // repeat

(END)
    @END
    0;JMP           // infinite loop (halt)
