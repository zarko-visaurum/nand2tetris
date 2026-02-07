//! Error types for the Jack compiler.

use jack_analyzer::error::JackError;
use jack_analyzer::token::Span;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during Jack compilation.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Variable used but not declared.
    #[error("Undefined variable '{name}' at {span}")]
    UndefinedVariable { name: String, span: Span },

    /// Variable declared twice in the same scope.
    #[error("Duplicate definition of '{name}' at {span}")]
    DuplicateDefinition { name: String, span: Span },

    /// Lexical or syntax error from parser.
    #[error("Parse error: {0}")]
    Parse(#[from] JackError),

    /// File I/O error.
    #[error("IO error for {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl CompileError {
    /// Create an IO error.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Create an undefined variable error.
    pub fn undefined_variable(name: impl Into<String>, span: Span) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            span,
        }
    }

    /// Create a duplicate definition error.
    pub fn duplicate_definition(name: impl Into<String>, span: Span) -> Self {
        Self::DuplicateDefinition {
            name: name.into(),
            span,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let span = Span::new(0, 5, 1, 1);
        let err = CompileError::undefined_variable("foo", span);
        assert!(err.to_string().contains("foo"));
        assert!(err.to_string().contains("Undefined"));
    }
}
