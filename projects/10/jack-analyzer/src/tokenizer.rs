//! Lexical analyzer (tokenizer) for the Jack language.

use crate::error::{ErrorAccumulator, JackError};
use crate::token::{Keyword, Span, SpannedToken, Token, is_symbol};

/// Jack language tokenizer.
pub struct JackTokenizer<'a> {
    #[allow(dead_code)]
    input: &'a str,
    chars: Vec<char>,
    pos: usize,
    byte_offset: usize,
    line: usize,
    column: usize,
    errors: ErrorAccumulator,
}

impl<'a> JackTokenizer<'a> {
    /// Create a new tokenizer for the given input.
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            pos: 0,
            byte_offset: 0,
            line: 1,
            column: 1,
            errors: ErrorAccumulator::new(),
        }
    }

    /// Tokenize the input and return tokens or errors.
    pub fn tokenize(mut self) -> Result<Vec<SpannedToken>, Vec<JackError>> {
        let mut tokens = Vec::new();

        while !self.is_at_end() {
            self.skip_whitespace_and_comments();
            if self.is_at_end() {
                break;
            }

            if let Some(token) = self.next_token() {
                tokens.push(token);
            }

            if self.errors.is_full() {
                break;
            }
        }

        if self.errors.has_errors() {
            Err(self.errors.into_errors())
        } else {
            Ok(tokens)
        }
    }

    /// Check if we've reached the end of input.
    fn is_at_end(&self) -> bool {
        self.pos >= self.chars.len()
    }

    /// Peek at the current character.
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    /// Peek at the next character.
    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    /// Advance to the next character, updating byte offset incrementally.
    fn advance(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.pos += 1;
        self.byte_offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(c)
    }

    /// Skip whitespace and comments.
    fn skip_whitespace_and_comments(&mut self) {
        loop {
            // Skip whitespace
            while let Some(c) = self.peek() {
                if c.is_whitespace() {
                    self.advance();
                } else {
                    break;
                }
            }

            // Check for comments
            if self.peek() == Some('/') {
                if self.peek_next() == Some('/') {
                    // Single-line comment
                    self.advance(); // /
                    self.advance(); // /
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                    continue;
                } else if self.peek_next() == Some('*') {
                    // Multi-line comment
                    self.advance(); // /
                    self.advance(); // *
                    let mut depth = 1;
                    while depth > 0 && !self.is_at_end() {
                        if self.peek() == Some('*') && self.peek_next() == Some('/') {
                            self.advance();
                            self.advance();
                            depth -= 1;
                        } else if self.peek() == Some('/') && self.peek_next() == Some('*') {
                            self.advance();
                            self.advance();
                            depth += 1;
                        } else {
                            self.advance();
                        }
                    }
                    continue;
                }
            }

            break;
        }
    }

    /// Parse the next token.
    fn next_token(&mut self) -> Option<SpannedToken> {
        let start_pos = self.byte_offset;
        let start_line = self.line;
        let start_column = self.column;

        let c = self.peek()?;

        // Symbol
        if is_symbol(c) {
            self.advance();
            let span = Span::new(start_pos, self.byte_offset, start_line, start_column);
            return Some(SpannedToken::new(Token::Symbol(c), span));
        }

        // Integer constant
        if c.is_ascii_digit() {
            return Some(self.read_integer(start_pos, start_line, start_column));
        }

        // String constant
        if c == '"' {
            return self.read_string(start_pos, start_line, start_column);
        }

        // Keyword or identifier
        if c.is_alphabetic() || c == '_' {
            return Some(self.read_identifier(start_pos, start_line, start_column));
        }

        // Unknown character
        self.advance();
        let span = Span::new(start_pos, self.byte_offset, start_line, start_column);
        self.errors.push(JackError::lexical(
            span,
            format!("unexpected character '{}'", c),
        ));
        None
    }

    /// Read an integer constant.
    fn read_integer(
        &mut self,
        start_pos: usize,
        start_line: usize,
        start_column: usize,
    ) -> SpannedToken {
        let mut value: u32 = 0;
        let mut overflow = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
                let digit = c.to_digit(10).unwrap();
                value = value.saturating_mul(10).saturating_add(digit);
                if value > 32767 {
                    overflow = true;
                }
            } else {
                break;
            }
        }

        let span = Span::new(start_pos, self.byte_offset, start_line, start_column);

        if overflow {
            self.errors.push(JackError::lexical(
                span.clone(),
                format!("integer constant {} exceeds maximum value 32767", value),
            ));
        }

        SpannedToken::new(Token::IntegerConstant(value.min(32767) as u16), span)
    }

    /// Read a string constant.
    fn read_string(
        &mut self,
        start_pos: usize,
        start_line: usize,
        start_column: usize,
    ) -> Option<SpannedToken> {
        self.advance(); // Opening quote

        let mut value = String::new();
        let mut terminated = false;

        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                terminated = true;
                break;
            } else if c == '\n' {
                // Newline in string - unterminated
                break;
            } else {
                value.push(c);
                self.advance();
            }
        }

        let span = Span::new(start_pos, self.byte_offset, start_line, start_column);

        if !terminated {
            self.errors.push(JackError::lexical(
                span.clone(),
                "unterminated string constant",
            ));
        }

        Some(SpannedToken::new(Token::StringConstant(value), span))
    }

    /// Read a keyword or identifier.
    fn read_identifier(
        &mut self,
        start_pos: usize,
        start_line: usize,
        start_column: usize,
    ) -> SpannedToken {
        let mut value = String::new();

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let span = Span::new(start_pos, self.byte_offset, start_line, start_column);

        let token = if let Some(keyword) = Keyword::parse_keyword(&value) {
            Token::Keyword(keyword)
        } else {
            Token::Identifier(value)
        };

        SpannedToken::new(token, span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize(input: &str) -> Vec<Token> {
        JackTokenizer::new(input)
            .tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.token)
            .collect()
    }

    #[test]
    fn test_keywords() {
        assert_eq!(tokenize("class"), vec![Token::Keyword(Keyword::Class)]);
        assert_eq!(tokenize("return"), vec![Token::Keyword(Keyword::Return)]);
        assert_eq!(
            tokenize("if else while"),
            vec![
                Token::Keyword(Keyword::If),
                Token::Keyword(Keyword::Else),
                Token::Keyword(Keyword::While),
            ]
        );
    }

    #[test]
    fn test_symbols() {
        assert_eq!(tokenize("{"), vec![Token::Symbol('{')]);
        assert_eq!(
            tokenize("{}()"),
            vec![
                Token::Symbol('{'),
                Token::Symbol('}'),
                Token::Symbol('('),
                Token::Symbol(')'),
            ]
        );
    }

    #[test]
    fn test_integers() {
        assert_eq!(tokenize("0"), vec![Token::IntegerConstant(0)]);
        assert_eq!(tokenize("123"), vec![Token::IntegerConstant(123)]);
        assert_eq!(tokenize("32767"), vec![Token::IntegerConstant(32767)]);
    }

    #[test]
    fn test_strings() {
        assert_eq!(
            tokenize("\"hello\""),
            vec![Token::StringConstant("hello".to_string())]
        );
        assert_eq!(
            tokenize("\"hello world\""),
            vec![Token::StringConstant("hello world".to_string())]
        );
    }

    #[test]
    fn test_identifiers() {
        assert_eq!(tokenize("foo"), vec![Token::Identifier("foo".to_string())]);
        assert_eq!(
            tokenize("_bar"),
            vec![Token::Identifier("_bar".to_string())]
        );
        assert_eq!(
            tokenize("x123"),
            vec![Token::Identifier("x123".to_string())]
        );
    }

    #[test]
    fn test_comments() {
        assert_eq!(
            tokenize("// comment\nclass"),
            vec![Token::Keyword(Keyword::Class)]
        );
        assert_eq!(
            tokenize("/* comment */ class"),
            vec![Token::Keyword(Keyword::Class)]
        );
        assert_eq!(
            tokenize("/** doc */ class"),
            vec![Token::Keyword(Keyword::Class)]
        );
    }

    #[test]
    fn test_complex() {
        let input = "class Main { function void main() { return; } }";
        let tokens = tokenize(input);
        assert_eq!(tokens.len(), 13);
        assert_eq!(tokens[0], Token::Keyword(Keyword::Class));
        assert_eq!(tokens[1], Token::Identifier("Main".to_string()));
    }
}
