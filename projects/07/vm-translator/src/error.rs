use thiserror::Error;

pub type Result<T> = std::result::Result<T, VMError>;

#[derive(Error, Debug)]
pub enum VMError {
    #[error("line {line}: invalid command: {command}")]
    InvalidCommand { line: usize, command: String },

    #[error("line {line}: invalid segment: {segment}")]
    InvalidSegment { line: usize, segment: String },

    #[error("line {line}: index {index} out of range for segment {segment} (max: {max})")]
    IndexOutOfRange {
        line: usize,
        index: u16,
        segment: String,
        max: u16,
    },

    #[error("line {line}: cannot pop to constant segment")]
    PopToConstant { line: usize },

    #[error("line {line}: invalid pointer index {index} (must be 0 or 1)")]
    InvalidPointerIndex { line: usize, index: u16 },

    #[error("line {line}: missing operand for command {command}")]
    MissingOperand { line: usize, command: String },

    #[error("line {line}: invalid index value: {value}")]
    InvalidIndex { line: usize, value: String },
}
