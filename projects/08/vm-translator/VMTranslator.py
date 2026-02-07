#!/usr/bin/env python3
"""
VMTranslator - Full Stack VM to Hack Assembly Translator (Project 08)

Production-ready Python implementation for nand2tetris Coursera autograder.
Supports all 20 VM commands including branching and function calls.

Usage:
    python3 VMTranslator.py <file.vm>           # Single file
    python3 VMTranslator.py <directory>         # Directory with multiple .vm files
"""

from __future__ import annotations

import sys
from dataclasses import dataclass, field
from enum import Enum, auto
from pathlib import Path


class ArithmeticOp(Enum):
    """Arithmetic and logical operations."""

    ADD = auto()
    SUB = auto()
    NEG = auto()
    EQ = auto()
    LT = auto()
    GT = auto()
    AND = auto()
    OR = auto()
    NOT = auto()


class Segment(Enum):
    """Memory segments."""

    CONSTANT = "constant"
    LOCAL = "local"
    ARGUMENT = "argument"
    THIS = "this"
    THAT = "that"
    POINTER = "pointer"
    TEMP = "temp"
    STATIC = "static"


@dataclass
class VMCommand:
    """Base class for VM commands."""

    pass


@dataclass
class ArithmeticCommand(VMCommand):
    """Arithmetic/logical command."""

    op: ArithmeticOp


@dataclass
class PushCommand(VMCommand):
    """Push command."""

    segment: Segment
    index: int


@dataclass
class PopCommand(VMCommand):
    """Pop command."""

    segment: Segment
    index: int


@dataclass
class LabelCommand(VMCommand):
    """Label declaration."""

    name: str


@dataclass
class GotoCommand(VMCommand):
    """Unconditional jump."""

    label: str


@dataclass
class IfGotoCommand(VMCommand):
    """Conditional jump."""

    label: str


@dataclass
class FunctionCommand(VMCommand):
    """Function declaration."""

    name: str
    num_locals: int


@dataclass
class CallCommand(VMCommand):
    """Function call."""

    name: str
    num_args: int


@dataclass
class ReturnCommand(VMCommand):
    """Return from function."""

    pass


class VMTranslatorError(Exception):
    """VM translation error with context."""

    def __init__(self, message: str, line: int = 0, filename: str = "") -> None:
        self.line = line
        self.filename = filename
        context = f"{filename}:" if filename else ""
        context += f"line {line}: " if line else ""
        super().__init__(f"{context}{message}")


class Parser:
    """VM command parser."""

    ARITHMETIC_OPS: dict[str, ArithmeticOp] = {
        "add": ArithmeticOp.ADD,
        "sub": ArithmeticOp.SUB,
        "neg": ArithmeticOp.NEG,
        "eq": ArithmeticOp.EQ,
        "lt": ArithmeticOp.LT,
        "gt": ArithmeticOp.GT,
        "and": ArithmeticOp.AND,
        "or": ArithmeticOp.OR,
        "not": ArithmeticOp.NOT,
    }

    SEGMENTS: dict[str, Segment] = {
        "constant": Segment.CONSTANT,
        "local": Segment.LOCAL,
        "argument": Segment.ARGUMENT,
        "this": Segment.THIS,
        "that": Segment.THAT,
        "pointer": Segment.POINTER,
        "temp": Segment.TEMP,
        "static": Segment.STATIC,
    }

    @staticmethod
    def parse_line(line: str, line_num: int, filename: str = "") -> VMCommand | None:
        """Parse a single VM line. Returns None for empty/comment lines."""
        # Strip comments and whitespace
        line = line.split("//")[0].strip()
        if not line:
            return None

        parts = line.split()
        cmd = parts[0].lower()

        # Arithmetic/logical commands
        if cmd in Parser.ARITHMETIC_OPS:
            return ArithmeticCommand(Parser.ARITHMETIC_OPS[cmd])

        # Memory access commands
        if cmd == "push":
            if len(parts) != 3:
                raise VMTranslatorError(
                    "push requires segment and index", line_num, filename
                )
            segment = Parser._parse_segment(parts[1], line_num, filename)
            index = Parser._parse_index(parts[2], line_num, filename)
            Parser._validate_segment_index(segment, index, line_num, filename)
            return PushCommand(segment, index)

        if cmd == "pop":
            if len(parts) != 3:
                raise VMTranslatorError(
                    "pop requires segment and index", line_num, filename
                )
            segment = Parser._parse_segment(parts[1], line_num, filename)
            if segment == Segment.CONSTANT:
                raise VMTranslatorError(
                    "cannot pop to constant segment", line_num, filename
                )
            index = Parser._parse_index(parts[2], line_num, filename)
            Parser._validate_segment_index(segment, index, line_num, filename)
            return PopCommand(segment, index)

        # Program flow commands
        if cmd == "label":
            if len(parts) != 2:
                raise VMTranslatorError("label requires a name", line_num, filename)
            return LabelCommand(parts[1])

        if cmd == "goto":
            if len(parts) != 2:
                raise VMTranslatorError("goto requires a label", line_num, filename)
            return GotoCommand(parts[1])

        if cmd == "if-goto":
            if len(parts) != 2:
                raise VMTranslatorError("if-goto requires a label", line_num, filename)
            return IfGotoCommand(parts[1])

        # Function commands
        if cmd == "function":
            if len(parts) != 3:
                raise VMTranslatorError(
                    "function requires name and num_locals", line_num, filename
                )
            num_locals = Parser._parse_index(parts[2], line_num, filename)
            return FunctionCommand(parts[1], num_locals)

        if cmd == "call":
            if len(parts) != 3:
                raise VMTranslatorError(
                    "call requires name and num_args", line_num, filename
                )
            num_args = Parser._parse_index(parts[2], line_num, filename)
            return CallCommand(parts[1], num_args)

        if cmd == "return":
            return ReturnCommand()

        raise VMTranslatorError(f"unknown command: {cmd}", line_num, filename)

    @staticmethod
    def _parse_segment(s: str, line_num: int, filename: str) -> Segment:
        if s.lower() not in Parser.SEGMENTS:
            raise VMTranslatorError(f"invalid segment: {s}", line_num, filename)
        return Parser.SEGMENTS[s.lower()]

    @staticmethod
    def _parse_index(s: str, line_num: int, filename: str) -> int:
        try:
            index = int(s)
            if index < 0:
                raise VMTranslatorError(
                    f"index must be non-negative: {s}", line_num, filename
                )
            return index
        except ValueError:
            raise VMTranslatorError(f"invalid index: {s}", line_num, filename)

    @staticmethod
    def _validate_segment_index(
        segment: Segment, index: int, line_num: int, filename: str
    ) -> None:
        if segment == Segment.POINTER and index > 1:
            raise VMTranslatorError(
                f"pointer index must be 0 or 1, got {index}", line_num, filename
            )
        if segment == Segment.TEMP and index > 7:
            raise VMTranslatorError(
                f"temp index must be 0-7, got {index}", line_num, filename
            )


@dataclass
class CodeGenerator:
    """Hack assembly code generator."""

    static_filename: str = ""
    current_function: str = ""
    label_counter: int = 0
    call_counter: int = 0

    # Segment base pointer symbols
    SEGMENT_BASES: dict[Segment, str] = field(
        default_factory=lambda: {
            Segment.LOCAL: "LCL",
            Segment.ARGUMENT: "ARG",
            Segment.THIS: "THIS",
            Segment.THAT: "THAT",
        }
    )

    def set_filename(self, filename: str) -> None:
        """Set current filename for static variable naming."""
        self.static_filename = Path(filename).stem

    def set_function(self, name: str) -> None:
        """Set current function context for label scoping."""
        self.current_function = name

    def translate(self, cmd: VMCommand) -> list[str]:
        """Translate a VM command to Hack assembly lines."""
        if isinstance(cmd, ArithmeticCommand):
            return self._translate_arithmetic(cmd.op)
        elif isinstance(cmd, PushCommand):
            return self._translate_push(cmd.segment, cmd.index)
        elif isinstance(cmd, PopCommand):
            return self._translate_pop(cmd.segment, cmd.index)
        elif isinstance(cmd, LabelCommand):
            return self._translate_label(cmd.name)
        elif isinstance(cmd, GotoCommand):
            return self._translate_goto(cmd.label)
        elif isinstance(cmd, IfGotoCommand):
            return self._translate_if_goto(cmd.label)
        elif isinstance(cmd, FunctionCommand):
            return self._translate_function(cmd.name, cmd.num_locals)
        elif isinstance(cmd, CallCommand):
            return self._translate_call(cmd.name, cmd.num_args)
        elif isinstance(cmd, ReturnCommand):
            return self._translate_return()
        else:
            raise VMTranslatorError(f"unknown command type: {type(cmd)}")

    def _scoped_label(self, label: str) -> str:
        """Generate function-scoped label."""
        if self.current_function:
            return f"{self.current_function}${label}"
        return f"{self.static_filename}${label}"

    def _translate_arithmetic(self, op: ArithmeticOp) -> list[str]:
        """Translate arithmetic/logical operations."""
        if op == ArithmeticOp.ADD:
            return ["@SP", "AM=M-1", "D=M", "A=A-1", "M=D+M"]
        elif op == ArithmeticOp.SUB:
            return ["@SP", "AM=M-1", "D=M", "A=A-1", "M=M-D"]
        elif op == ArithmeticOp.NEG:
            return ["@SP", "A=M-1", "M=-M"]
        elif op == ArithmeticOp.AND:
            return ["@SP", "AM=M-1", "D=M", "A=A-1", "M=D&M"]
        elif op == ArithmeticOp.OR:
            return ["@SP", "AM=M-1", "D=M", "A=A-1", "M=D|M"]
        elif op == ArithmeticOp.NOT:
            return ["@SP", "A=M-1", "M=!M"]
        elif op in (ArithmeticOp.EQ, ArithmeticOp.LT, ArithmeticOp.GT):
            return self._translate_comparison(op)
        raise VMTranslatorError(f"unknown arithmetic op: {op}")

    def _translate_comparison(self, op: ArithmeticOp) -> list[str]:
        """Translate comparison operations with unique labels."""
        jump_map = {
            ArithmeticOp.EQ: "JEQ",
            ArithmeticOp.LT: "JLT",
            ArithmeticOp.GT: "JGT",
        }
        true_label = f"{jump_map[op]}_TRUE_{self.label_counter}"
        end_label = f"{jump_map[op]}_END_{self.label_counter}"
        self.label_counter += 1

        return [
            "@SP",
            "AM=M-1",
            "D=M",  # Pop y into D
            "A=A-1",
            "D=M-D",  # D = x - y
            f"@{true_label}",
            f"D;{jump_map[op]}",  # Jump if condition met
            "@SP",
            "A=M-1",
            "M=0",  # Push false
            f"@{end_label}",
            "0;JMP",
            f"({true_label})",
            "@SP",
            "A=M-1",
            "M=-1",  # Push true (-1)
            f"({end_label})",
        ]

    def _translate_push(self, segment: Segment, index: int) -> list[str]:
        """Translate push command."""
        if segment == Segment.CONSTANT:
            return [f"@{index}", "D=A", "@SP", "A=M", "M=D", "@SP", "M=M+1"]

        elif segment in self.SEGMENT_BASES:
            base = self.SEGMENT_BASES[segment]
            return [
                f"@{index}",
                "D=A",
                f"@{base}",
                "A=D+M",
                "D=M",
                "@SP",
                "A=M",
                "M=D",
                "@SP",
                "M=M+1",
            ]

        elif segment == Segment.TEMP:
            addr = 5 + index
            return [f"@{addr}", "D=M", "@SP", "A=M", "M=D", "@SP", "M=M+1"]

        elif segment == Segment.POINTER:
            addr = "THIS" if index == 0 else "THAT"
            return [f"@{addr}", "D=M", "@SP", "A=M", "M=D", "@SP", "M=M+1"]

        elif segment == Segment.STATIC:
            label = f"{self.static_filename}.{index}"
            return [f"@{label}", "D=M", "@SP", "A=M", "M=D", "@SP", "M=M+1"]

        raise VMTranslatorError(f"unknown segment: {segment}")

    def _translate_pop(self, segment: Segment, index: int) -> list[str]:
        """Translate pop command."""
        if segment in self.SEGMENT_BASES:
            base = self.SEGMENT_BASES[segment]
            return [
                f"@{index}",
                "D=A",
                f"@{base}",
                "D=D+M",
                "@R13",
                "M=D",  # Save target address
                "@SP",
                "AM=M-1",
                "D=M",  # Pop value
                "@R13",
                "A=M",
                "M=D",  # Store at target
            ]

        elif segment == Segment.TEMP:
            addr = 5 + index
            return ["@SP", "AM=M-1", "D=M", f"@{addr}", "M=D"]

        elif segment == Segment.POINTER:
            addr = "THIS" if index == 0 else "THAT"
            return ["@SP", "AM=M-1", "D=M", f"@{addr}", "M=D"]

        elif segment == Segment.STATIC:
            label = f"{self.static_filename}.{index}"
            return ["@SP", "AM=M-1", "D=M", f"@{label}", "M=D"]

        raise VMTranslatorError(f"cannot pop to segment: {segment}")

    def _translate_label(self, name: str) -> list[str]:
        """Translate label declaration."""
        return [f"({self._scoped_label(name)})"]

    def _translate_goto(self, label: str) -> list[str]:
        """Translate unconditional goto."""
        return [f"@{self._scoped_label(label)}", "0;JMP"]

    def _translate_if_goto(self, label: str) -> list[str]:
        """Translate conditional goto (jump if stack top != 0)."""
        return [
            "@SP",
            "AM=M-1",
            "D=M",  # Pop value into D
            f"@{self._scoped_label(label)}",
            "D;JNE",  # Jump if D != 0
        ]

    def _translate_function(self, name: str, num_locals: int) -> list[str]:
        """Translate function declaration."""
        self.set_function(name)
        lines = [f"({name})"]

        # Initialize local variables to 0
        for _ in range(num_locals):
            lines.extend(["@SP", "A=M", "M=0", "@SP", "M=M+1"])

        return lines

    def _translate_call(self, name: str, num_args: int) -> list[str]:
        """Translate function call."""
        if self.current_function:
            return_label = f"{self.current_function}$ret.{self.call_counter}"
        else:
            return_label = f"{self.static_filename}$ret.{self.call_counter}"
        self.call_counter += 1

        lines = [
            # Push return address
            f"@{return_label}",
            "D=A",
            "@SP",
            "A=M",
            "M=D",
            "@SP",
            "M=M+1",
            # Push LCL
            "@LCL",
            "D=M",
            "@SP",
            "A=M",
            "M=D",
            "@SP",
            "M=M+1",
            # Push ARG
            "@ARG",
            "D=M",
            "@SP",
            "A=M",
            "M=D",
            "@SP",
            "M=M+1",
            # Push THIS
            "@THIS",
            "D=M",
            "@SP",
            "A=M",
            "M=D",
            "@SP",
            "M=M+1",
            # Push THAT
            "@THAT",
            "D=M",
            "@SP",
            "A=M",
            "M=D",
            "@SP",
            "M=M+1",
            # ARG = SP - num_args - 5
            "@SP",
            "D=M",
            f"@{num_args + 5}",
            "D=D-A",
            "@ARG",
            "M=D",
            # LCL = SP
            "@SP",
            "D=M",
            "@LCL",
            "M=D",
            # goto function
            f"@{name}",
            "0;JMP",
            # Return label
            f"({return_label})",
        ]
        return lines

    def _translate_return(self) -> list[str]:
        """Translate return from function."""
        return [
            # frame = LCL (store in R13)
            "@LCL",
            "D=M",
            "@R13",
            "M=D",
            # retAddr = *(frame - 5) (store in R14)
            "@5",
            "A=D-A",
            "D=M",
            "@R14",
            "M=D",
            # *ARG = pop()
            "@SP",
            "AM=M-1",
            "D=M",
            "@ARG",
            "A=M",
            "M=D",
            # SP = ARG + 1
            "@ARG",
            "D=M+1",
            "@SP",
            "M=D",
            # THAT = *(frame - 1)
            "@R13",
            "AM=M-1",
            "D=M",
            "@THAT",
            "M=D",
            # THIS = *(frame - 2)
            "@R13",
            "AM=M-1",
            "D=M",
            "@THIS",
            "M=D",
            # ARG = *(frame - 3)
            "@R13",
            "AM=M-1",
            "D=M",
            "@ARG",
            "M=D",
            # LCL = *(frame - 4)
            "@R13",
            "AM=M-1",
            "D=M",
            "@LCL",
            "M=D",
            # goto retAddr
            "@R14",
            "A=M",
            "0;JMP",
        ]


def generate_bootstrap() -> list[str]:
    """Generate VM bootstrap code (SP=256, call Sys.init)."""
    return [
        # SP = 256
        "@256",
        "D=A",
        "@SP",
        "M=D",
        # call Sys.init 0
        "@Sys.init$ret.BOOTSTRAP",
        "D=A",
        "@SP",
        "A=M",
        "M=D",
        "@SP",
        "M=M+1",
        "@LCL",
        "D=M",
        "@SP",
        "A=M",
        "M=D",
        "@SP",
        "M=M+1",
        "@ARG",
        "D=M",
        "@SP",
        "A=M",
        "M=D",
        "@SP",
        "M=M+1",
        "@THIS",
        "D=M",
        "@SP",
        "A=M",
        "M=D",
        "@SP",
        "M=M+1",
        "@THAT",
        "D=M",
        "@SP",
        "A=M",
        "M=D",
        "@SP",
        "M=M+1",
        "@SP",
        "D=M",
        "@5",
        "D=D-A",
        "@ARG",
        "M=D",
        "@SP",
        "D=M",
        "@LCL",
        "M=D",
        "@Sys.init",
        "0;JMP",
        "(Sys.init$ret.BOOTSTRAP)",
    ]


def translate_file(vm_path: Path, codegen: CodeGenerator) -> list[str]:
    """Translate a single .vm file."""
    codegen.set_filename(vm_path.stem)
    lines: list[str] = []

    with vm_path.open() as f:
        for line_num, line in enumerate(f, 1):
            cmd = Parser.parse_line(line, line_num, vm_path.name)
            if cmd is not None:
                lines.extend(codegen.translate(cmd))

    return lines


def translate_directory(dir_path: Path) -> str:
    """Translate all .vm files in a directory."""
    vm_files = sorted(dir_path.glob("*.vm"))
    if not vm_files:
        raise VMTranslatorError(f"no .vm files found in {dir_path}")

    # Check if we need bootstrap (Sys.vm with Sys.init exists)
    sys_file = dir_path / "Sys.vm"
    need_bootstrap = sys_file.exists()

    codegen = CodeGenerator()
    all_lines: list[str] = []

    # Generate bootstrap if needed
    if need_bootstrap:
        all_lines.extend(generate_bootstrap())

    # Process Sys.vm first if it exists
    if sys_file.exists():
        all_lines.extend(translate_file(sys_file, codegen))
        vm_files = [f for f in vm_files if f.name != "Sys.vm"]

    # Process remaining files in alphabetical order
    for vm_file in vm_files:
        all_lines.extend(translate_file(vm_file, codegen))

    return "\n".join(all_lines)


def translate_single_file(vm_path: Path) -> str:
    """Translate a single .vm file (no bootstrap)."""
    codegen = CodeGenerator()
    lines = translate_file(vm_path, codegen)
    return "\n".join(lines)


def main() -> None:
    """Main entry point."""
    if len(sys.argv) < 2:
        print("Usage: python3 VMTranslator.py <file.vm | directory>", file=sys.stderr)
        sys.exit(1)

    input_path = Path(sys.argv[1])

    if not input_path.exists():
        print(f"Error: {input_path} not found", file=sys.stderr)
        sys.exit(1)

    try:
        if input_path.is_dir():
            # Directory mode
            asm_code = translate_directory(input_path)
            output_path = input_path / f"{input_path.name}.asm"
        else:
            # Single file mode
            if input_path.suffix != ".vm":
                print(
                    f"Error: expected .vm file, got {input_path.suffix}",
                    file=sys.stderr,
                )
                sys.exit(1)
            asm_code = translate_single_file(input_path)
            output_path = input_path.with_suffix(".asm")

        output_path.write_text(asm_code)
        print(f"Translated -> {output_path}")

    except VMTranslatorError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
