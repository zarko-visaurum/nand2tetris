//! Comprehensive error types for VM translation.
//!
//! All errors include context (line number, filename) for actionable messages.

use thiserror::Error;

/// VM translation error with full context.
#[derive(Error, Debug)]
pub enum VMError {
    // Parse errors
    #[error("{file}:{line}: invalid command: {command}")]
    InvalidCommand {
        line: usize,
        file: String,
        command: String,
    },

    #[error("{file}:{line}: invalid segment: {segment}")]
    InvalidSegment {
        line: usize,
        file: String,
        segment: String,
    },

    #[error("{file}:{line}: index {index} out of range for segment {segment}")]
    IndexOutOfRange {
        line: usize,
        file: String,
        index: u16,
        segment: String,
    },

    #[error("{file}:{line}: cannot pop to constant segment")]
    PopToConstant { line: usize, file: String },

    #[error("{file}:{line}: invalid pointer index {index} (must be 0 or 1)")]
    InvalidPointerIndex {
        line: usize,
        file: String,
        index: u16,
    },

    #[error("{file}:{line}: invalid temp index {index} (must be 0-7)")]
    InvalidTempIndex {
        line: usize,
        file: String,
        index: u16,
    },

    #[error("{file}:{line}: missing argument for {command}")]
    MissingArgument {
        line: usize,
        file: String,
        command: String,
    },

    #[error("{file}:{line}: invalid number: {value}")]
    InvalidNumber {
        line: usize,
        file: String,
        value: String,
    },

    // Program flow errors
    #[error("{file}:{line}: invalid label name: {name}")]
    InvalidLabelName {
        line: usize,
        file: String,
        name: String,
    },

    // Function errors
    #[error("{file}:{line}: invalid function name: {name}")]
    InvalidFunctionName {
        line: usize,
        file: String,
        name: String,
    },

    // I/O errors
    #[error("failed to read file {path}: {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to write file {path}: {source}")]
    FileWrite {
        path: String,
        #[source]
        source: std::io::Error,
    },

    #[error("no .vm files found in directory: {path}")]
    NoVmFiles { path: String },

    #[error("path is not a file or directory: {path}")]
    InvalidPath { path: String },
}

/// Result type alias for VM operations.
pub type Result<T> = std::result::Result<T, VMError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = VMError::InvalidCommand {
            line: 42,
            file: "Test.vm".to_string(),
            command: "foo".to_string(),
        };
        assert_eq!(format!("{}", err), "Test.vm:42: invalid command: foo");
    }

    #[test]
    fn test_pop_constant_error() {
        let err = VMError::PopToConstant {
            line: 10,
            file: "Main.vm".to_string(),
        };
        assert!(format!("{}", err).contains("cannot pop to constant"));
    }

    #[test]
    fn test_index_out_of_range() {
        let err = VMError::IndexOutOfRange {
            line: 5,
            file: "Foo.vm".to_string(),
            index: 99,
            segment: "temp".to_string(),
        };
        assert!(format!("{}", err).contains("99"));
        assert!(format!("{}", err).contains("temp"));
    }
}
