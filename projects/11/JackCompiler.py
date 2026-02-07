#!/usr/bin/env python3
"""
Jack Compiler - Full Jack to VM code compiler with optimizations.
Single-file implementation for Coursera submission.

Usage:
    python3 JackCompiler.py <input.jack | directory>
    python3 JackCompiler.py --no-optimize <input.jack | directory>
"""

import sys
import os
from dataclasses import dataclass
from enum import Enum, auto
from typing import Optional, Dict, List, Tuple

# =============================================================================
# Token Types (from JackAnalyzer)
# =============================================================================

KEYWORDS = frozenset(
    [
        "class",
        "constructor",
        "function",
        "method",
        "field",
        "static",
        "var",
        "int",
        "char",
        "boolean",
        "void",
        "true",
        "false",
        "null",
        "this",
        "let",
        "do",
        "if",
        "else",
        "while",
        "return",
    ]
)

SYMBOLS = frozenset("{}()[].,;+-*/&|<>=~")


class TokenType(Enum):
    KEYWORD = auto()
    SYMBOL = auto()
    INT_CONST = auto()
    STRING_CONST = auto()
    IDENTIFIER = auto()


@dataclass
class Token:
    type: TokenType
    value: str
    line: int = 0


# =============================================================================
# Tokenizer
# =============================================================================


class JackTokenizer:
    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1

    def tokenize(self) -> List[Token]:
        tokens = []
        while self.pos < len(self.source):
            self._skip_whitespace_and_comments()
            if self.pos >= len(self.source):
                break
            token = self._next_token()
            if token:
                tokens.append(token)
        return tokens

    def _skip_whitespace_and_comments(self):
        while self.pos < len(self.source):
            if self.source[self.pos].isspace():
                if self.source[self.pos] == "\n":
                    self.line += 1
                self.pos += 1
            elif self.source[self.pos : self.pos + 2] == "//":
                while self.pos < len(self.source) and self.source[self.pos] != "\n":
                    self.pos += 1
            elif self.source[self.pos : self.pos + 2] in ("/*", "/**"):
                self.pos += 2
                while self.pos < len(self.source) - 1:
                    if self.source[self.pos : self.pos + 2] == "*/":
                        self.pos += 2
                        break
                    if self.source[self.pos] == "\n":
                        self.line += 1
                    self.pos += 1
            else:
                break

    def _next_token(self) -> Optional[Token]:
        ch = self.source[self.pos]
        line = self.line

        if ch in SYMBOLS:
            self.pos += 1
            return Token(TokenType.SYMBOL, ch, line)

        if ch.isdigit():
            start = self.pos
            while self.pos < len(self.source) and self.source[self.pos].isdigit():
                self.pos += 1
            return Token(TokenType.INT_CONST, self.source[start : self.pos], line)

        if ch == '"':
            self.pos += 1
            start = self.pos
            while self.pos < len(self.source) and self.source[self.pos] != '"':
                self.pos += 1
            value = self.source[start : self.pos]
            self.pos += 1
            return Token(TokenType.STRING_CONST, value, line)

        if ch.isalpha() or ch == "_":
            start = self.pos
            while self.pos < len(self.source) and (
                self.source[self.pos].isalnum() or self.source[self.pos] == "_"
            ):
                self.pos += 1
            value = self.source[start : self.pos]
            if value in KEYWORDS:
                return Token(TokenType.KEYWORD, value, line)
            return Token(TokenType.IDENTIFIER, value, line)

        self.pos += 1
        return None


# =============================================================================
# Symbol Table
# =============================================================================


class SymbolKind(Enum):
    STATIC = "static"
    FIELD = "this"
    ARGUMENT = "argument"
    LOCAL = "local"


@dataclass
class Symbol:
    name: str
    symbol_type: str
    kind: SymbolKind
    index: int


class SymbolTable:
    def __init__(self):
        self.class_scope: Dict[str, Symbol] = {}
        self.subroutine_scope: Dict[str, Symbol] = {}
        self.counts: Dict[SymbolKind, int] = {k: 0 for k in SymbolKind}
        self.class_name = ""

    def start_class(self, name: str):
        self.class_scope.clear()
        self.counts[SymbolKind.STATIC] = 0
        self.counts[SymbolKind.FIELD] = 0
        self.class_name = name

    def start_subroutine(self):
        self.subroutine_scope.clear()
        self.counts[SymbolKind.ARGUMENT] = 0
        self.counts[SymbolKind.LOCAL] = 0

    def define(self, name: str, symbol_type: str, kind: SymbolKind):
        symbol = Symbol(name, symbol_type, kind, self.counts[kind])
        self.counts[kind] += 1
        if kind in (SymbolKind.STATIC, SymbolKind.FIELD):
            self.class_scope[name] = symbol
        else:
            self.subroutine_scope[name] = symbol

    def lookup(self, name: str) -> Optional[Symbol]:
        return self.subroutine_scope.get(name) or self.class_scope.get(name)

    def var_count(self, kind: SymbolKind) -> int:
        return self.counts[kind]

    def field_count(self) -> int:
        return self.counts[SymbolKind.FIELD]


# =============================================================================
# VM Writer
# =============================================================================


class VMWriter:
    def __init__(self):
        self.output: List[str] = []

    def write_push(self, segment: str, index: int):
        self.output.append(f"push {segment} {index}")

    def write_pop(self, segment: str, index: int):
        self.output.append(f"pop {segment} {index}")

    def write_arithmetic(self, cmd: str):
        self.output.append(cmd)

    def write_label(self, label: str):
        self.output.append(f"label {label}")

    def write_goto(self, label: str):
        self.output.append(f"goto {label}")

    def write_if_goto(self, label: str):
        self.output.append(f"if-goto {label}")

    def write_function(self, name: str, num_locals: int):
        self.output.append(f"function {name} {num_locals}")

    def write_call(self, name: str, num_args: int):
        self.output.append(f"call {name} {num_args}")

    def write_return(self):
        self.output.append("return")

    def get_output(self) -> str:
        return "\n".join(self.output) + "\n"


# =============================================================================
# Parser + Code Generator (Combined for simplicity)
# =============================================================================


class JackCompiler:
    BINARY_OPS = {
        "+": "add",
        "-": "sub",
        "&": "and",
        "|": "or",
        "<": "lt",
        ">": "gt",
        "=": "eq",
    }

    def __init__(self, tokens: List[Token], optimize: bool = True):
        self.tokens = tokens
        self.pos = 0
        self.symbols = SymbolTable()
        self.vm = VMWriter()
        self.label_counter = 0
        self.class_name = ""
        self.subroutine_kind = ""
        self.optimize = optimize

    def compile(self) -> str:
        self._compile_class()
        vm_code = self.vm.get_output()
        if self.optimize:
            vm_code = self._peephole_optimize(vm_code)
        return vm_code

    def _current(self) -> Optional[Token]:
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def _advance(self) -> Token:
        token = self.tokens[self.pos]
        self.pos += 1
        return token

    def _expect(self, value: str):
        token = self._advance()
        if token.value != value:
            raise SyntaxError(
                f"Expected '{value}', got '{token.value}' at line {token.line}"
            )

    def _unique_label(self, prefix: str) -> str:
        label = f"{prefix}_{self.label_counter}"
        self.label_counter += 1
        return label

    # --- Class compilation ---

    def _compile_class(self):
        self._expect("class")
        self.class_name = self._advance().value
        self.symbols.start_class(self.class_name)
        self._expect("{")

        while self._current() and self._current().value in ("static", "field"):
            self._compile_class_var_dec()

        while self._current() and self._current().value in (
            "constructor",
            "function",
            "method",
        ):
            self._compile_subroutine()

        self._expect("}")

    def _compile_class_var_dec(self):
        kind = (
            SymbolKind.STATIC if self._advance().value == "static" else SymbolKind.FIELD
        )
        var_type = self._advance().value
        name = self._advance().value
        self.symbols.define(name, var_type, kind)

        while self._current() and self._current().value == ",":
            self._advance()
            name = self._advance().value
            self.symbols.define(name, var_type, kind)

        self._expect(";")

    def _compile_subroutine(self):
        self.subroutine_kind = self._advance().value
        self.symbols.start_subroutine()

        self._advance()  # return type (unused, but required by grammar)
        name = self._advance().value

        if self.subroutine_kind == "method":
            self.symbols.define("this", self.class_name, SymbolKind.ARGUMENT)

        self._expect("(")
        self._compile_parameter_list()
        self._expect(")")

        self._compile_subroutine_body(name)

    def _compile_parameter_list(self):
        if self._current() and self._current().value != ")":
            var_type = self._advance().value
            name = self._advance().value
            self.symbols.define(name, var_type, SymbolKind.ARGUMENT)

            while self._current() and self._current().value == ",":
                self._advance()
                var_type = self._advance().value
                name = self._advance().value
                self.symbols.define(name, var_type, SymbolKind.ARGUMENT)

    def _compile_subroutine_body(self, name: str):
        self._expect("{")

        while self._current() and self._current().value == "var":
            self._compile_var_dec()

        num_locals = self.symbols.var_count(SymbolKind.LOCAL)
        self.vm.write_function(f"{self.class_name}.{name}", num_locals)

        if self.subroutine_kind == "constructor":
            self.vm.write_push("constant", self.symbols.field_count())
            self.vm.write_call("Memory.alloc", 1)
            self.vm.write_pop("pointer", 0)
        elif self.subroutine_kind == "method":
            self.vm.write_push("argument", 0)
            self.vm.write_pop("pointer", 0)

        self._compile_statements()
        self._expect("}")

    def _compile_var_dec(self):
        self._expect("var")
        var_type = self._advance().value
        name = self._advance().value
        self.symbols.define(name, var_type, SymbolKind.LOCAL)

        while self._current() and self._current().value == ",":
            self._advance()
            name = self._advance().value
            self.symbols.define(name, var_type, SymbolKind.LOCAL)

        self._expect(";")

    # --- Statements ---

    def _compile_statements(self):
        while self._current() and self._current().value in (
            "let",
            "if",
            "while",
            "do",
            "return",
        ):
            if self._current().value == "let":
                self._compile_let()
            elif self._current().value == "if":
                self._compile_if()
            elif self._current().value == "while":
                self._compile_while()
            elif self._current().value == "do":
                self._compile_do()
            elif self._current().value == "return":
                self._compile_return()

    def _compile_let(self):
        self._expect("let")
        var_name = self._advance().value
        symbol = self.symbols.lookup(var_name)

        if self._current() and self._current().value == "[":
            # Array assignment
            self._advance()
            self.vm.write_push(symbol.kind.value, symbol.index)
            self._compile_expression()
            self._expect("]")
            self.vm.write_arithmetic("add")

            self._expect("=")
            self._compile_expression()
            self._expect(";")

            self.vm.write_pop("temp", 0)
            self.vm.write_pop("pointer", 1)
            self.vm.write_push("temp", 0)
            self.vm.write_pop("that", 0)
        else:
            self._expect("=")
            self._compile_expression()
            self._expect(";")
            self.vm.write_pop(symbol.kind.value, symbol.index)

    def _compile_if(self):
        self._expect("if")
        false_label = self._unique_label("IF_FALSE")
        end_label = self._unique_label("IF_END")

        self._expect("(")
        self._compile_expression()
        self._expect(")")

        self.vm.write_arithmetic("not")
        self.vm.write_if_goto(false_label)

        self._expect("{")
        self._compile_statements()
        self._expect("}")

        self.vm.write_goto(end_label)
        self.vm.write_label(false_label)

        if self._current() and self._current().value == "else":
            self._advance()
            self._expect("{")
            self._compile_statements()
            self._expect("}")

        self.vm.write_label(end_label)

    def _compile_while(self):
        self._expect("while")
        exp_label = self._unique_label("WHILE_EXP")
        end_label = self._unique_label("WHILE_END")

        self.vm.write_label(exp_label)

        self._expect("(")
        self._compile_expression()
        self._expect(")")

        self.vm.write_arithmetic("not")
        self.vm.write_if_goto(end_label)

        self._expect("{")
        self._compile_statements()
        self._expect("}")

        self.vm.write_goto(exp_label)
        self.vm.write_label(end_label)

    def _compile_do(self):
        self._expect("do")
        self._compile_subroutine_call()
        self._expect(";")
        self.vm.write_pop("temp", 0)

    def _compile_return(self):
        self._expect("return")
        if self._current() and self._current().value != ";":
            self._compile_expression()
        else:
            self.vm.write_push("constant", 0)
        self._expect(";")
        self.vm.write_return()

    # --- Expressions ---

    def _compile_expression(self):
        # Try constant folding first (only if optimization is enabled)
        if self.optimize:
            value = self._try_fold_expression()
            if value is not None:
                if 0 <= value <= 32767:
                    self.vm.write_push("constant", value)
                    return
                elif -32768 <= value < 0:
                    # Handle negative constants: push |value| then negate
                    self.vm.write_push("constant", -value)
                    self.vm.write_arithmetic("neg")
                    return

        # Normal compilation
        self._compile_term()

        while self._current() and self._current().value in "+-*/&|<>=":
            op = self._advance().value
            self._compile_term()

            if op == "*":
                self.vm.write_call("Math.multiply", 2)
            elif op == "/":
                self.vm.write_call("Math.divide", 2)
            else:
                self.vm.write_arithmetic(self.BINARY_OPS[op])

    def _try_fold_expression(self) -> Optional[int]:
        """Attempt to fold a constant expression at compile time.

        Returns the folded value if successful, None otherwise.
        Saves and restores position if folding fails.
        """
        save_pos = self.pos
        try:
            result = self._fold_term()
            if result is None:
                self.pos = save_pos
                return None

            while self._current() and self._current().value in "+-*/&|<>=":
                op = self._advance().value
                right = self._fold_term()
                if right is None:
                    self.pos = save_pos
                    return None
                result = self._apply_fold_op(result, op, right)
                if result is None:
                    self.pos = save_pos
                    return None

            return result
        except (IndexError, ValueError, TypeError):
            self.pos = save_pos
            return None

    def _fold_term(self) -> Optional[int]:
        """Attempt to fold a term. Returns None if not foldable."""
        token = self._current()
        if token is None:
            return None

        if token.type == TokenType.INT_CONST:
            self._advance()
            return int(token.value)

        if token.value == "true":
            self._advance()
            return -1  # true = ~0 = -1 in 16-bit

        if token.value in ("false", "null"):
            self._advance()
            return 0

        if token.value == "(":
            self._advance()
            result = self._fold_expression_inner()
            if result is None:
                return None
            if self._current() and self._current().value == ")":
                self._advance()
                return result
            return None

        if token.value == "-":
            self._advance()
            inner = self._fold_term()
            return -inner if inner is not None else None

        if token.value == "~":
            self._advance()
            inner = self._fold_term()
            return ~inner if inner is not None else None

        # Variables, arrays, function calls cannot be folded
        return None

    def _fold_expression_inner(self) -> Optional[int]:
        """Fold expression inside parentheses."""
        result = self._fold_term()
        if result is None:
            return None

        while self._current() and self._current().value in "+-*/&|<>=":
            op = self._advance().value
            right = self._fold_term()
            if right is None:
                return None
            result = self._apply_fold_op(result, op, right)
            if result is None:
                return None

        return result

    def _apply_fold_op(self, left: int, op: str, right: int) -> Optional[int]:
        """Apply a binary operation at compile time."""
        if op == "+":
            return (left + right) & 0xFFFF  # Wrap to 16-bit
        if op == "-":
            return (left - right) & 0xFFFF
        if op == "*":
            return (left * right) & 0xFFFF
        if op == "/":
            return left // right if right != 0 else None  # Avoid division by zero
        if op == "&":
            return left & right
        if op == "|":
            return left | right
        if op == "<":
            return -1 if left < right else 0
        if op == ">":
            return -1 if left > right else 0
        if op == "=":
            return -1 if left == right else 0
        return None

    def _compile_term(self):
        token = self._current()

        if token.type == TokenType.INT_CONST:
            self._advance()
            self.vm.write_push("constant", int(token.value))

        elif token.type == TokenType.STRING_CONST:
            self._advance()
            self.vm.write_push("constant", len(token.value))
            self.vm.write_call("String.new", 1)
            for ch in token.value:
                self.vm.write_push("constant", ord(ch))
                self.vm.write_call("String.appendChar", 2)

        elif token.value == "true":
            self._advance()
            self.vm.write_push("constant", 0)
            self.vm.write_arithmetic("not")

        elif token.value in ("false", "null"):
            self._advance()
            self.vm.write_push("constant", 0)

        elif token.value == "this":
            self._advance()
            self.vm.write_push("pointer", 0)

        elif token.value == "(":
            self._advance()
            self._compile_expression()
            self._expect(")")

        elif token.value in "-~":
            op = self._advance().value
            self._compile_term()
            self.vm.write_arithmetic("neg" if op == "-" else "not")

        elif token.type == TokenType.IDENTIFIER:
            name = self._advance().value

            if self._current() and self._current().value == "[":
                # Array access
                symbol = self.symbols.lookup(name)
                self._advance()
                self.vm.write_push(symbol.kind.value, symbol.index)
                self._compile_expression()
                self._expect("]")
                self.vm.write_arithmetic("add")
                self.vm.write_pop("pointer", 1)
                self.vm.write_push("that", 0)

            elif self._current() and self._current().value in "(.":
                # Subroutine call
                self.pos -= 1  # Put back the identifier
                self._compile_subroutine_call()

            else:
                # Variable access
                symbol = self.symbols.lookup(name)
                self.vm.write_push(symbol.kind.value, symbol.index)

    def _compile_subroutine_call(self):
        name = self._advance().value
        num_args = 0

        if self._current() and self._current().value == ".":
            self._advance()
            method_name = self._advance().value

            symbol = self.symbols.lookup(name)
            if symbol:
                # Method call on object
                self.vm.write_push(symbol.kind.value, symbol.index)
                full_name = f"{symbol.symbol_type}.{method_name}"
                num_args = 1
            else:
                # Function/constructor call
                full_name = f"{name}.{method_name}"
        else:
            # Method call on this
            self.vm.write_push("pointer", 0)
            full_name = f"{self.class_name}.{name}"
            num_args = 1

        self._expect("(")
        num_args += self._compile_expression_list()
        self._expect(")")

        self.vm.write_call(full_name, num_args)

    def _compile_expression_list(self) -> int:
        count = 0
        if self._current() and self._current().value != ")":
            self._compile_expression()
            count = 1

            while self._current() and self._current().value == ",":
                self._advance()
                self._compile_expression()
                count += 1

        return count

    # --- Peephole Optimization ---

    def _peephole_optimize(self, vm_code: str) -> str:
        lines = vm_code.strip().split("\n")
        optimized = []
        i = 0

        while i < len(lines):
            # Pattern: not / not -> remove both
            if i + 1 < len(lines) and lines[i] == "not" and lines[i + 1] == "not":
                i += 2
                continue

            # Pattern: neg / neg -> remove both
            if i + 1 < len(lines) and lines[i] == "neg" and lines[i + 1] == "neg":
                i += 2
                continue

            # Pattern: push X / pop X (same location) -> remove both
            if i + 1 < len(lines):
                if lines[i].startswith("push ") and lines[i + 1].startswith("pop "):
                    push_rest = lines[i][5:]
                    pop_rest = lines[i + 1][4:]
                    if push_rest == pop_rest and not push_rest.startswith("constant"):
                        i += 2
                        continue

            # Pattern: push constant 0 / add -> remove both
            if (
                i + 1 < len(lines)
                and lines[i] == "push constant 0"
                and lines[i + 1] == "add"
            ):
                i += 2
                continue

            optimized.append(lines[i])
            i += 1

        return "\n".join(optimized) + "\n" if optimized else ""


# =============================================================================
# Main
# =============================================================================


def compile_file(filepath: str, optimize: bool = True) -> Tuple[str, str]:
    with open(filepath, "r") as f:
        source = f.read()

    basename = os.path.splitext(os.path.basename(filepath))[0]
    tokenizer = JackTokenizer(source)
    tokens = tokenizer.tokenize()
    compiler = JackCompiler(tokens, optimize)
    vm_code = compiler.compile()

    return basename, vm_code


def main():
    if len(sys.argv) < 2:
        print(
            "Usage: JackCompiler.py [--no-optimize] <file.jack | directory>",
            file=sys.stderr,
        )
        sys.exit(1)

    # Parse arguments
    args = sys.argv[1:]
    optimize = True
    if "--no-optimize" in args:
        optimize = False
        args.remove("--no-optimize")

    if not args:
        print("Error: No input file or directory specified", file=sys.stderr)
        sys.exit(1)

    path = args[0]

    if os.path.isfile(path):
        jack_files = [path]
        output_dir = os.path.dirname(path) or "."
    elif os.path.isdir(path):
        jack_files = [
            os.path.join(path, f) for f in os.listdir(path) if f.endswith(".jack")
        ]
        output_dir = path
    else:
        print(f"Error: {path} not found", file=sys.stderr)
        sys.exit(2)

    if not jack_files:
        print(f"Error: No .jack files found in {path}", file=sys.stderr)
        sys.exit(2)

    for jack_file in jack_files:
        try:
            basename, vm_code = compile_file(jack_file, optimize)
            output_path = os.path.join(output_dir, f"{basename}.vm")
            with open(output_path, "w") as f:
                f.write(vm_code)
            print(f"Compiled {basename}.jack -> {basename}.vm")
        except Exception as e:
            print(f"Error compiling {jack_file}: {e}", file=sys.stderr)
            sys.exit(1)


if __name__ == "__main__":
    main()
