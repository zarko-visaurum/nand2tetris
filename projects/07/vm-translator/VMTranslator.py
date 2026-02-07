#!/usr/bin/env python3
"""
VMTranslator - Stack VM to Hack Assembly Translator
Minimal Python implementation for nand2tetris Coursera autograder

This is a simplified port of the production Rust implementation.
For the full production version, see the Rust implementation.
"""

import sys
from pathlib import Path


class VMTranslator:
    def __init__(self, filename):
        self.filename = Path(filename).stem
        self.label_counter = 0

    def translate(self, vm_code):
        """Translate VM code to Hack assembly"""
        lines = vm_code.strip().split("\n")
        asm_lines = []

        for line in lines:
            # Remove comments and whitespace
            line = line.split("//")[0].strip()
            if not line:
                continue

            # Parse command
            parts = line.split()
            command = parts[0]

            # Arithmetic/logical commands
            if command in ["add", "sub", "and", "or"]:
                asm_lines.extend(self.translate_binary_op(command))
            elif command in ["neg", "not"]:
                asm_lines.extend(self.translate_unary_op(command))
            elif command in ["eq", "lt", "gt"]:
                asm_lines.extend(self.translate_comparison(command))
            # Memory access commands
            elif command == "push":
                segment, index = parts[1], int(parts[2])
                asm_lines.extend(self.translate_push(segment, index))
            elif command == "pop":
                segment, index = parts[1], int(parts[2])
                asm_lines.extend(self.translate_pop(segment, index))

        return "\n".join(asm_lines)

    def translate_binary_op(self, op):
        """Translate binary arithmetic/logical operations (5 instructions via AM=M-1 fusion)"""
        op_map = {"add": "D+M", "sub": "M-D", "and": "D&M", "or": "D|M"}
        return [
            "@SP",
            "AM=M-1",  # SP--, A points to y
            "D=M",  # D = y
            "A=A-1",  # A points to x (peek, SP already correct)
            f"M={op_map[op]}",  # x op y, result in place
        ]

    def translate_unary_op(self, op):
        """Translate unary operations (3 instructions via A=M-1 peek)"""
        op_map = {"neg": "-M", "not": "!M"}
        return [
            "@SP",
            "A=M-1",  # Peek at top (SP unchanged)
            f"M={op_map[op]}",  # Operate in place
        ]

    def translate_comparison(self, op):
        """Translate comparison operations (fused pattern)"""
        jump_map = {"eq": "JEQ", "lt": "JLT", "gt": "JGT"}
        true_label = f"{op.upper()}_TRUE_{self.label_counter}"
        end_label = f"{op.upper()}_END_{self.label_counter}"
        self.label_counter += 1

        return [
            "@SP",
            "AM=M-1",  # SP--, A points to y
            "D=M",  # D = y
            "A=A-1",  # A points to x (peek)
            "D=M-D",  # D = x - y
            f"@{true_label}",
            f"D;{jump_map[op]}",  # Jump if condition met
            "@SP",
            "A=M-1",
            "M=0",  # Push false (0), SP already correct
            f"@{end_label}",
            "0;JMP",
            f"({true_label})",
            "@SP",
            "A=M-1",
            "M=-1",  # Push true (-1), SP already correct
            f"({end_label})",
        ]

    def translate_push(self, segment, index):
        """Translate push commands"""
        if segment == "constant":
            return [f"@{index}", "D=A", "@SP", "A=M", "M=D", "@SP", "M=M+1"]
        elif segment in ["local", "argument", "this", "that"]:
            base_map = {
                "local": "LCL",
                "argument": "ARG",
                "this": "THIS",
                "that": "THAT",
            }
            return [
                f"@{index}",
                "D=A",
                f"@{base_map[segment]}",
                "A=D+M",
                "D=M",
                "@SP",
                "A=M",
                "M=D",
                "@SP",
                "M=M+1",
            ]
        elif segment == "temp":
            addr = 5 + index
            return [f"@{addr}", "D=M", "@SP", "A=M", "M=D", "@SP", "M=M+1"]
        elif segment == "pointer":
            addr = "THIS" if index == 0 else "THAT"
            return [f"@{addr}", "D=M", "@SP", "A=M", "M=D", "@SP", "M=M+1"]
        elif segment == "static":
            return [
                f"@{self.filename}.{index}",
                "D=M",
                "@SP",
                "A=M",
                "M=D",
                "@SP",
                "M=M+1",
            ]
        return []

    def translate_pop(self, segment, index):
        """Translate pop commands (fused AM=M-1 for direct segments)"""
        if segment in ["local", "argument", "this", "that"]:
            base_map = {
                "local": "LCL",
                "argument": "ARG",
                "this": "THIS",
                "that": "THAT",
            }
            return [
                f"@{index}",
                "D=A",
                f"@{base_map[segment]}",
                "D=D+M",
                "@R13",
                "M=D",  # Save target address
                "@SP",
                "AM=M-1",
                "D=M",  # Pop value (fused)
                "@R13",
                "A=M",
                "M=D",  # Store at target
            ]
        elif segment == "temp":
            addr = 5 + index
            return ["@SP", "AM=M-1", "D=M", f"@{addr}", "M=D"]
        elif segment == "pointer":
            addr = "THIS" if index == 0 else "THAT"
            return ["@SP", "AM=M-1", "D=M", f"@{addr}", "M=D"]
        elif segment == "static":
            return ["@SP", "AM=M-1", "D=M", f"@{self.filename}.{index}", "M=D"]
        return []


def main():
    if len(sys.argv) < 2:
        print("Usage: VMTranslator.py <file.vm>")
        sys.exit(1)

    vm_file = Path(sys.argv[1])
    if not vm_file.exists():
        print(f"Error: File not found: {vm_file}")
        sys.exit(1)

    # Read VM code
    vm_code = vm_file.read_text()

    # Translate
    translator = VMTranslator(vm_file.stem)
    asm_code = translator.translate(vm_code)

    # Write output
    asm_file = vm_file.with_suffix(".asm")
    asm_file.write_text(asm_code)

    print(f"Translated {vm_file} -> {asm_file}")


if __name__ == "__main__":
    main()
