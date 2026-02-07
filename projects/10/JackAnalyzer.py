#!/usr/bin/env python3
"""
JackAnalyzer - Syntax Analyzer for the Jack Programming Language
nand2tetris Project 10

Usage:
    python3 JackAnalyzer.py <input>

Where <input> is either:
    - A single .jack file
    - A directory containing .jack files
"""

import sys
from pathlib import Path
from dataclasses import dataclass
from typing import Optional, List, Tuple
from enum import Enum, auto
from concurrent.futures import ProcessPoolExecutor, as_completed

# ============================================================================
# Token Types
# ============================================================================

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
    line: int
    column: int


# ============================================================================
# Tokenizer
# ============================================================================


class JackTokenizer:
    """Lexical analyzer for Jack language."""

    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.line = 1
        self.column = 1
        self.errors: List[str] = []

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
                    self.column = 1
                else:
                    self.column += 1
                self.pos += 1
            elif self.source[self.pos : self.pos + 2] == "//":
                while self.pos < len(self.source) and self.source[self.pos] != "\n":
                    self.pos += 1
            elif self.source[self.pos : self.pos + 2] in ("/*", "/**"):
                self.pos += 2
                self.column += 2
                while self.pos < len(self.source) - 1:
                    if self.source[self.pos : self.pos + 2] == "*/":
                        self.pos += 2
                        self.column += 2
                        break
                    if self.source[self.pos] == "\n":
                        self.line += 1
                        self.column = 1
                    else:
                        self.column += 1
                    self.pos += 1
            else:
                break

    def _next_token(self) -> Optional[Token]:
        ch = self.source[self.pos]
        start_line, start_col = self.line, self.column

        if ch in SYMBOLS:
            self.pos += 1
            self.column += 1
            return Token(TokenType.SYMBOL, ch, start_line, start_col)

        if ch.isdigit():
            start = self.pos
            while self.pos < len(self.source) and self.source[self.pos].isdigit():
                self.pos += 1
                self.column += 1
            value = self.source[start : self.pos]
            if int(value) > 32767:
                self.errors.append(f"Integer overflow at line {start_line}: {value}")
            return Token(TokenType.INT_CONST, value, start_line, start_col)

        if ch == '"':
            self.pos += 1
            self.column += 1
            start = self.pos
            while self.pos < len(self.source) and self.source[self.pos] != '"':
                if self.source[self.pos] == "\n":
                    self.errors.append(f"Unterminated string at line {start_line}")
                    break
                self.pos += 1
                self.column += 1
            value = self.source[start : self.pos]
            if self.pos < len(self.source) and self.source[self.pos] == '"':
                self.pos += 1
                self.column += 1
            return Token(TokenType.STRING_CONST, value, start_line, start_col)

        if ch.isalpha() or ch == "_":
            start = self.pos
            while self.pos < len(self.source) and (
                self.source[self.pos].isalnum() or self.source[self.pos] == "_"
            ):
                self.pos += 1
                self.column += 1
            value = self.source[start : self.pos]
            if value in KEYWORDS:
                return Token(TokenType.KEYWORD, value, start_line, start_col)
            return Token(TokenType.IDENTIFIER, value, start_line, start_col)

        self.errors.append(
            f"Unknown character '{ch}' at line {start_line}, column {start_col}"
        )
        self.pos += 1
        self.column += 1
        return None


# ============================================================================
# XML Writer
# ============================================================================


def xml_escape(s: str) -> str:
    return (
        s.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
    )


def tokens_to_xml(tokens: List[Token]) -> str:
    tag_map = {
        TokenType.KEYWORD: "keyword",
        TokenType.SYMBOL: "symbol",
        TokenType.INT_CONST: "integerConstant",
        TokenType.STRING_CONST: "stringConstant",
        TokenType.IDENTIFIER: "identifier",
    }
    lines = ["<tokens>"]
    for token in tokens:
        tag = tag_map[token.type]
        lines.append(f"<{tag}> {xml_escape(token.value)} </{tag}>")
    lines.append("</tokens>")
    return "\n".join(lines) + "\n"


# ============================================================================
# Parser (Compilation Engine)
# ============================================================================


class CompilationEngine:
    """Recursive descent parser for Jack language."""

    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.pos = 0
        self.indent = 0
        self.output: List[str] = []
        self.errors: List[str] = []

    def compile(self) -> str:
        self.compile_class()
        return "\n".join(self.output) + "\n"

    def _current(self) -> Optional[Token]:
        return self.tokens[self.pos] if self.pos < len(self.tokens) else None

    def _peek_value(self) -> Optional[str]:
        t = self._current()
        return t.value if t else None

    def _peek_type(self) -> Optional[TokenType]:
        t = self._current()
        return t.type if t else None

    def _advance(self) -> Token:
        token = self.tokens[self.pos]
        self.pos += 1
        return token

    def _expect(self, value: str) -> Token:
        token = self._current()
        if token is None or token.value != value:
            got = token.value if token else "EOF"
            line = token.line if token else "?"
            self.errors.append(f"Expected '{value}', got '{got}' at line {line}")
            if token and token.value != value:
                return token
        return self._advance()

    def _write(self, line: str):
        self.output.append("  " * self.indent + line)

    def _write_terminal(self, token: Token):
        tag_map = {
            TokenType.KEYWORD: "keyword",
            TokenType.SYMBOL: "symbol",
            TokenType.INT_CONST: "integerConstant",
            TokenType.STRING_CONST: "stringConstant",
            TokenType.IDENTIFIER: "identifier",
        }
        tag = tag_map[token.type]
        self._write(f"<{tag}> {xml_escape(token.value)} </{tag}>")

    def _open_tag(self, tag: str):
        self._write(f"<{tag}>")
        self.indent += 1

    def _close_tag(self, tag: str):
        self.indent -= 1
        self._write(f"</{tag}>")

    def compile_class(self):
        self._open_tag("class")
        self._write_terminal(self._expect("class"))
        self._write_terminal(self._advance())  # className
        self._write_terminal(self._expect("{"))

        while self._peek_value() in ("static", "field"):
            self.compile_class_var_dec()

        while self._peek_value() in ("constructor", "function", "method"):
            self.compile_subroutine()

        self._write_terminal(self._expect("}"))
        self._close_tag("class")

    def compile_class_var_dec(self):
        self._open_tag("classVarDec")
        self._write_terminal(self._advance())  # static | field
        self._write_terminal(self._advance())  # type
        self._write_terminal(self._advance())  # varName

        while self._peek_value() == ",":
            self._write_terminal(self._advance())
            self._write_terminal(self._advance())

        self._write_terminal(self._expect(";"))
        self._close_tag("classVarDec")

    def compile_subroutine(self):
        self._open_tag("subroutineDec")
        self._write_terminal(self._advance())  # constructor | function | method
        self._write_terminal(self._advance())  # void | type
        self._write_terminal(self._advance())  # subroutineName
        self._write_terminal(self._expect("("))
        self.compile_parameter_list()
        self._write_terminal(self._expect(")"))
        self.compile_subroutine_body()
        self._close_tag("subroutineDec")

    def compile_parameter_list(self):
        self._open_tag("parameterList")
        if self._peek_value() != ")":
            self._write_terminal(self._advance())  # type
            self._write_terminal(self._advance())  # varName
            while self._peek_value() == ",":
                self._write_terminal(self._advance())
                self._write_terminal(self._advance())
                self._write_terminal(self._advance())
        self._close_tag("parameterList")

    def compile_subroutine_body(self):
        self._open_tag("subroutineBody")
        self._write_terminal(self._expect("{"))

        while self._peek_value() == "var":
            self.compile_var_dec()

        self.compile_statements()
        self._write_terminal(self._expect("}"))
        self._close_tag("subroutineBody")

    def compile_var_dec(self):
        self._open_tag("varDec")
        self._write_terminal(self._expect("var"))
        self._write_terminal(self._advance())  # type
        self._write_terminal(self._advance())  # varName

        while self._peek_value() == ",":
            self._write_terminal(self._advance())
            self._write_terminal(self._advance())

        self._write_terminal(self._expect(";"))
        self._close_tag("varDec")

    def compile_statements(self):
        self._open_tag("statements")
        while self._peek_value() in ("let", "if", "while", "do", "return"):
            if self._peek_value() == "let":
                self.compile_let()
            elif self._peek_value() == "if":
                self.compile_if()
            elif self._peek_value() == "while":
                self.compile_while()
            elif self._peek_value() == "do":
                self.compile_do()
            elif self._peek_value() == "return":
                self.compile_return()
        self._close_tag("statements")

    def compile_let(self):
        self._open_tag("letStatement")
        self._write_terminal(self._expect("let"))
        self._write_terminal(self._advance())  # varName

        if self._peek_value() == "[":
            self._write_terminal(self._advance())
            self.compile_expression()
            self._write_terminal(self._expect("]"))

        self._write_terminal(self._expect("="))
        self.compile_expression()
        self._write_terminal(self._expect(";"))
        self._close_tag("letStatement")

    def compile_if(self):
        self._open_tag("ifStatement")
        self._write_terminal(self._expect("if"))
        self._write_terminal(self._expect("("))
        self.compile_expression()
        self._write_terminal(self._expect(")"))
        self._write_terminal(self._expect("{"))
        self.compile_statements()
        self._write_terminal(self._expect("}"))

        if self._peek_value() == "else":
            self._write_terminal(self._advance())
            self._write_terminal(self._expect("{"))
            self.compile_statements()
            self._write_terminal(self._expect("}"))

        self._close_tag("ifStatement")

    def compile_while(self):
        self._open_tag("whileStatement")
        self._write_terminal(self._expect("while"))
        self._write_terminal(self._expect("("))
        self.compile_expression()
        self._write_terminal(self._expect(")"))
        self._write_terminal(self._expect("{"))
        self.compile_statements()
        self._write_terminal(self._expect("}"))
        self._close_tag("whileStatement")

    def compile_do(self):
        self._open_tag("doStatement")
        self._write_terminal(self._expect("do"))
        self._compile_subroutine_call()
        self._write_terminal(self._expect(";"))
        self._close_tag("doStatement")

    def compile_return(self):
        self._open_tag("returnStatement")
        self._write_terminal(self._expect("return"))
        if self._peek_value() != ";":
            self.compile_expression()
        self._write_terminal(self._expect(";"))
        self._close_tag("returnStatement")

    def _compile_subroutine_call(self):
        self._write_terminal(self._advance())  # subroutineName | className | varName

        if self._peek_value() == ".":
            self._write_terminal(self._advance())
            self._write_terminal(self._advance())  # subroutineName

        self._write_terminal(self._expect("("))
        self.compile_expression_list()
        self._write_terminal(self._expect(")"))

    def compile_expression(self):
        self._open_tag("expression")
        self.compile_term()

        pv = self._peek_value()
        while pv is not None and pv in "+-*/&|<>=":
            self._write_terminal(self._advance())
            self.compile_term()
            pv = self._peek_value()

        self._close_tag("expression")

    def compile_term(self):
        self._open_tag("term")
        token = self._current()

        if token is None:
            self.errors.append("Unexpected end of input")
            self._close_tag("term")
            return

        if token.type == TokenType.INT_CONST:
            self._write_terminal(self._advance())
        elif token.type == TokenType.STRING_CONST:
            self._write_terminal(self._advance())
        elif token.value in ("true", "false", "null", "this"):
            self._write_terminal(self._advance())
        elif token.value == "(":
            self._write_terminal(self._advance())
            self.compile_expression()
            self._write_terminal(self._expect(")"))
        elif token.value in "-~":
            self._write_terminal(self._advance())
            self.compile_term()
        elif token.type == TokenType.IDENTIFIER:
            self._write_terminal(self._advance())

            if self._peek_value() == "[":
                self._write_terminal(self._advance())
                self.compile_expression()
                self._write_terminal(self._expect("]"))
            elif self._peek_value() == "(":
                self._write_terminal(self._advance())
                self.compile_expression_list()
                self._write_terminal(self._expect(")"))
            elif self._peek_value() == ".":
                self._write_terminal(self._advance())
                self._write_terminal(self._advance())  # subroutineName
                self._write_terminal(self._expect("("))
                self.compile_expression_list()
                self._write_terminal(self._expect(")"))
        else:
            self.errors.append(f"Unexpected token '{token.value}' at line {token.line}")
            self._advance()

        self._close_tag("term")

    def compile_expression_list(self):
        self._open_tag("expressionList")
        if self._peek_value() != ")":
            self.compile_expression()
            while self._peek_value() == ",":
                self._write_terminal(self._advance())
                self.compile_expression()
        self._close_tag("expressionList")


# ============================================================================
# Main
# ============================================================================


def analyze_file(path: Path) -> Tuple[str, List[str]]:
    """Analyze a single .jack file and return (filename, errors)."""
    source = path.read_text()

    tokenizer = JackTokenizer(source)
    tokens = tokenizer.tokenize()

    token_xml = tokens_to_xml(tokens)
    token_path = path.with_name(path.stem + "T.xml")
    token_path.write_text(token_xml)

    parser = CompilationEngine(tokens)
    parse_xml = parser.compile()

    parse_path = path.with_suffix(".xml")
    parse_path.write_text(parse_xml)

    errors = tokenizer.errors + parser.errors
    return (path.name, errors)


def main():
    if len(sys.argv) != 2:
        print(f"Usage: {sys.argv[0]} <input.jack | directory>", file=sys.stderr)
        sys.exit(1)

    input_path = Path(sys.argv[1])

    if input_path.is_file():
        name, errors = analyze_file(input_path)
        if errors:
            for e in errors:
                print(f"[{name}] {e}", file=sys.stderr)
            sys.exit(1)
    elif input_path.is_dir():
        jack_files = list(input_path.glob("*.jack"))
        if not jack_files:
            print(f"No .jack files found in {input_path}", file=sys.stderr)
            sys.exit(1)

        all_errors = []
        with ProcessPoolExecutor() as executor:
            futures = {executor.submit(analyze_file, f): f for f in jack_files}
            for future in as_completed(futures):
                name, errors = future.result()
                if errors:
                    all_errors.extend(f"[{name}] {e}" for e in errors)

        if all_errors:
            for e in all_errors:
                print(e, file=sys.stderr)
            sys.exit(1)
    else:
        print(f"Input not found: {input_path}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
