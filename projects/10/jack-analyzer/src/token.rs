//! Token types and source spans for the Jack tokenizer.

use std::fmt;

/// Source location span for error reporting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, column: usize) -> Self {
        Self {
            start,
            end,
            line,
            column,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

/// A token with its source location.
#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Span,
}

impl SpannedToken {
    pub fn new(token: Token, span: Span) -> Self {
        Self { token, span }
    }
}

/// Jack language token types.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Keyword(Keyword),
    Symbol(char),
    IntegerConstant(u16),
    StringConstant(String),
    Identifier(String),
}

impl Token {
    /// Returns the XML tag name for this token type.
    pub fn xml_tag(&self) -> &'static str {
        match self {
            Token::Keyword(_) => "keyword",
            Token::Symbol(_) => "symbol",
            Token::IntegerConstant(_) => "integerConstant",
            Token::StringConstant(_) => "stringConstant",
            Token::Identifier(_) => "identifier",
        }
    }

    /// Returns the XML-escaped value of this token.
    pub fn xml_value(&self) -> String {
        match self {
            Token::Keyword(k) => k.as_str().to_string(),
            Token::Symbol(c) => xml_escape_char(*c),
            Token::IntegerConstant(n) => n.to_string(),
            Token::StringConstant(s) => xml_escape(s),
            Token::Identifier(s) => xml_escape(s),
        }
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::Keyword(k) => write!(f, "keyword '{}'", k.as_str()),
            Token::Symbol(c) => write!(f, "symbol '{}'", c),
            Token::IntegerConstant(n) => write!(f, "integer {}", n),
            Token::StringConstant(s) => write!(f, "string \"{}\"", s),
            Token::Identifier(s) => write!(f, "identifier '{}'", s),
        }
    }
}

/// XML escape a single character.
fn xml_escape_char(c: char) -> String {
    match c {
        '<' => "&lt;".to_string(),
        '>' => "&gt;".to_string(),
        '&' => "&amp;".to_string(),
        '"' => "&quot;".to_string(),
        _ => c.to_string(),
    }
}

/// XML escape a string.
fn xml_escape(s: &str) -> String {
    s.chars().map(xml_escape_char).collect()
}

/// Jack language keywords.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keyword {
    Class,
    Constructor,
    Function,
    Method,
    Field,
    Static,
    Var,
    Int,
    Char,
    Boolean,
    Void,
    True,
    False,
    Null,
    This,
    Let,
    Do,
    If,
    Else,
    While,
    Return,
}

impl Keyword {
    /// Try to parse a string as a keyword.
    pub fn parse_keyword(s: &str) -> Option<Self> {
        match s {
            "class" => Some(Keyword::Class),
            "constructor" => Some(Keyword::Constructor),
            "function" => Some(Keyword::Function),
            "method" => Some(Keyword::Method),
            "field" => Some(Keyword::Field),
            "static" => Some(Keyword::Static),
            "var" => Some(Keyword::Var),
            "int" => Some(Keyword::Int),
            "char" => Some(Keyword::Char),
            "boolean" => Some(Keyword::Boolean),
            "void" => Some(Keyword::Void),
            "true" => Some(Keyword::True),
            "false" => Some(Keyword::False),
            "null" => Some(Keyword::Null),
            "this" => Some(Keyword::This),
            "let" => Some(Keyword::Let),
            "do" => Some(Keyword::Do),
            "if" => Some(Keyword::If),
            "else" => Some(Keyword::Else),
            "while" => Some(Keyword::While),
            "return" => Some(Keyword::Return),
            _ => None,
        }
    }

    /// Returns the string representation of the keyword.
    pub fn as_str(&self) -> &'static str {
        match self {
            Keyword::Class => "class",
            Keyword::Constructor => "constructor",
            Keyword::Function => "function",
            Keyword::Method => "method",
            Keyword::Field => "field",
            Keyword::Static => "static",
            Keyword::Var => "var",
            Keyword::Int => "int",
            Keyword::Char => "char",
            Keyword::Boolean => "boolean",
            Keyword::Void => "void",
            Keyword::True => "true",
            Keyword::False => "false",
            Keyword::Null => "null",
            Keyword::This => "this",
            Keyword::Let => "let",
            Keyword::Do => "do",
            Keyword::If => "if",
            Keyword::Else => "else",
            Keyword::While => "while",
            Keyword::Return => "return",
        }
    }
}

/// Jack language symbols.
pub const SYMBOLS: &[char] = &[
    '{', '}', '(', ')', '[', ']', '.', ',', ';', '+', '-', '*', '/', '&', '|', '<', '>', '=', '~',
];

/// Check if a character is a Jack symbol.
pub fn is_symbol(c: char) -> bool {
    SYMBOLS.contains(&c)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_from_str() {
        assert_eq!(Keyword::parse_keyword("class"), Some(Keyword::Class));
        assert_eq!(Keyword::parse_keyword("return"), Some(Keyword::Return));
        assert_eq!(Keyword::parse_keyword("notakeyword"), None);
    }

    #[test]
    fn test_xml_escape() {
        assert_eq!(xml_escape("<>&\""), "&lt;&gt;&amp;&quot;");
        assert_eq!(xml_escape("hello"), "hello");
    }

    #[test]
    fn test_is_symbol() {
        assert!(is_symbol('{'));
        assert!(is_symbol('+'));
        assert!(!is_symbol('a'));
        assert!(!is_symbol(' '));
    }
}
