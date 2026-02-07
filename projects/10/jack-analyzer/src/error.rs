//! Error types and diagnostics for the Jack analyzer.

use crate::token::Span;
use std::fmt;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for the Jack analyzer.
#[derive(Debug, Error)]
pub enum JackError {
    #[error("Lexical error at {span}: {message}")]
    Lexical {
        span: Span,
        message: String,
        #[source]
        cause: Option<Box<JackError>>,
    },

    #[error("Syntax error at {span}: {message}")]
    Syntax {
        span: Span,
        message: String,
        expected: Vec<String>,
        #[source]
        cause: Option<Box<JackError>>,
    },

    #[error("IO error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl JackError {
    /// Create a lexical error.
    pub fn lexical(span: Span, message: impl Into<String>) -> Self {
        JackError::Lexical {
            span,
            message: message.into(),
            cause: None,
        }
    }

    /// Create a syntax error.
    pub fn syntax(span: Span, message: impl Into<String>) -> Self {
        JackError::Syntax {
            span,
            message: message.into(),
            expected: Vec::new(),
            cause: None,
        }
    }

    /// Create a syntax error with expected tokens.
    pub fn syntax_expected(span: Span, message: impl Into<String>, expected: Vec<String>) -> Self {
        JackError::Syntax {
            span,
            message: message.into(),
            expected,
            cause: None,
        }
    }

    /// Create an IO error.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        JackError::Io {
            path: path.into(),
            source,
        }
    }

    /// Get the span of this error, if any.
    pub fn span(&self) -> Option<&Span> {
        match self {
            JackError::Lexical { span, .. } => Some(span),
            JackError::Syntax { span, .. } => Some(span),
            JackError::Io { .. } => None,
        }
    }

    /// Chain this error with a cause.
    pub fn with_cause(self, cause: JackError) -> Self {
        match self {
            JackError::Lexical { span, message, .. } => JackError::Lexical {
                span,
                message,
                cause: Some(Box::new(cause)),
            },
            JackError::Syntax {
                span,
                message,
                expected,
                ..
            } => JackError::Syntax {
                span,
                message,
                expected,
                cause: Some(Box::new(cause)),
            },
            other => other,
        }
    }
}

/// A collection of errors with multi-error reporting support.
#[derive(Debug, Default)]
pub struct ErrorAccumulator {
    errors: Vec<JackError>,
    max_errors: usize,
}

impl ErrorAccumulator {
    /// Create a new error accumulator with default max errors (20).
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
            max_errors: 20,
        }
    }

    /// Create with a custom max error limit.
    pub fn with_max(max_errors: usize) -> Self {
        Self {
            errors: Vec::new(),
            max_errors,
        }
    }

    /// Add an error to the accumulator.
    pub fn push(&mut self, error: JackError) {
        if self.errors.len() < self.max_errors {
            self.errors.push(error);
        }
    }

    /// Check if we've hit the error limit.
    pub fn is_full(&self) -> bool {
        self.errors.len() >= self.max_errors
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get the number of errors.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Consume and return all errors.
    pub fn into_errors(self) -> Vec<JackError> {
        self.errors
    }

    /// Get a reference to all errors.
    pub fn errors(&self) -> &[JackError] {
        &self.errors
    }

    /// Extend with errors from another accumulator.
    pub fn extend(&mut self, other: ErrorAccumulator) {
        for error in other.errors {
            if !self.is_full() {
                self.errors.push(error);
            }
        }
    }
}

/// Diagnostic formatter for rich error output.
pub struct Diagnostic<'a> {
    error: &'a JackError,
    source: Option<&'a str>,
    filename: Option<&'a str>,
}

impl<'a> Diagnostic<'a> {
    pub fn new(error: &'a JackError) -> Self {
        Self {
            error,
            source: None,
            filename: None,
        }
    }

    pub fn with_source(mut self, source: &'a str) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_filename(mut self, filename: &'a str) -> Self {
        self.filename = Some(filename);
        self
    }
}

impl fmt::Display for Diagnostic<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filename = self.filename.unwrap_or("<input>");

        match self.error {
            JackError::Lexical {
                span,
                message,
                cause,
            } => {
                writeln!(f, "error: {}", message)?;
                writeln!(f, "  --> {}:{}:{}", filename, span.line, span.column)?;

                if let Some(source) = self.source
                    && let Some(line) = source.lines().nth(span.line - 1)
                {
                    writeln!(f, "   |")?;
                    writeln!(f, "{:3} | {}", span.line, line)?;
                    writeln!(f, "   | {:>width$}^", "", width = span.column - 1)?;
                }

                if let Some(cause) = cause {
                    writeln!(f, "   = caused by: {}", cause)?;
                }
            }
            JackError::Syntax {
                span,
                message,
                expected,
                cause,
            } => {
                writeln!(f, "error: {}", message)?;
                writeln!(f, "  --> {}:{}:{}", filename, span.line, span.column)?;

                if let Some(source) = self.source
                    && let Some(line) = source.lines().nth(span.line - 1)
                {
                    writeln!(f, "   |")?;
                    writeln!(f, "{:3} | {}", span.line, line)?;
                    writeln!(f, "   | {:>width$}^", "", width = span.column - 1)?;
                }

                if !expected.is_empty() {
                    writeln!(f, "   = expected: {}", expected.join(", "))?;
                }

                if let Some(cause) = cause {
                    writeln!(f, "   = caused by: {}", cause)?;
                }
            }
            JackError::Io { path, source } => {
                writeln!(f, "error: IO error for {}: {}", path.display(), source)?;
            }
        }

        Ok(())
    }
}

/// Format multiple errors with context.
pub fn format_errors(errors: &[JackError], source: &str, filename: &str) -> String {
    let mut output = String::new();
    let total = errors.len();

    for (i, error) in errors.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(&format!("Error {} of {}:\n", i + 1, total));
        output.push_str(
            &Diagnostic::new(error)
                .with_source(source)
                .with_filename(filename)
                .to_string(),
        );
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_accumulator() {
        let mut acc = ErrorAccumulator::with_max(3);
        assert!(!acc.has_errors());

        acc.push(JackError::lexical(Span::new(0, 1, 1, 1), "error 1"));
        acc.push(JackError::lexical(Span::new(0, 1, 1, 1), "error 2"));
        assert!(!acc.is_full());

        acc.push(JackError::lexical(Span::new(0, 1, 1, 1), "error 3"));
        assert!(acc.is_full());

        // Should not add more after limit
        acc.push(JackError::lexical(Span::new(0, 1, 1, 1), "error 4"));
        assert_eq!(acc.len(), 3);
    }
}
