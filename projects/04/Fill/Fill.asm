// This file is part of www.nand2tetris.org
// and the book "The Elements of Computing Systems"
// by Nisan and Schocken, MIT Press.
// File name: projects/4/Fill.asm

// Runs an infinite loop that listens to the keyboard input. 
// When a key is pressed (any key), the program blackens the screen,
// i.e. writes "black" in every pixel. When no key is pressed, 
// the screen should be cleared.

// | Memory Map | Address | Purpose |
// |------------|---------|---------|
// | **SCREEN** | 16384 | Screen memory start |
// | **KBD** | 24576 | Keyboard register |
// | Screen size | 8192 words | 256 rows × 512 pixels ÷ 16 bits/word |
// 
// ## Algorithm Flow
// 
// 1. Check KBD register
// 2. If KBD ≠ 0 → set color = -1 (black)
//    If KBD = 0 → set color = 0 (white)
// 3. Fill all 8192 screen words with color value
// 4. Goto step 1 (infinite loop)

(MAIN)
    @KBD
    D=M         // D = key stroke code
    @SETWHITE
    D;JEQ       // if no key (KBD == 0), goto SETWHITE

(SETBLACK)
    @colour
    M=-1        // colour = -1 (0xFFFF = all pixels black)
    @FILL
    0;JMP

(SETWHITE)
    @colour
    M=0         // colour = 0 (0x0000 = all pixels white)

(FILL)
    @8192       // screen has 8192 16-bit words (256 rows * 512 columns / 16 bits)
    D=A
    @i
    M=D         // i = 8192 (loop counter)

    @SCREEN
    D=A
    @addr
    M=D         // addr = 16384 (screen base address)

(FILLLOOP)
    @i
    D=M
    @MAIN
    D;JLE       // if i <= 0, restart keyboard check

    @colour
    D=M         // D = colour value (0 or -1)
    @addr
    A=M
    M=D         // RAM[addr] = color (fill current word)

    @addr
    M=M+1       // addr++ (next screen word)
    @i
    M=M-1       // i-- (decrement counter)
    
    @FILLLOOP
    0;JMP       // continue filling