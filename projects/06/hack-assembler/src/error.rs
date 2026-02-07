use thiserror::Error;

#[derive(Error, Debug)]
pub enum AsmError {
    #[error("line {line}: invalid A-instruction value: {value}")]
    InvalidAValue { line: usize, value: String },

    #[error("line {line}: duplicate label: {label}")]
    DuplicateLabel { line: usize, label: String },

    #[error("line {line}: invalid C-instruction syntax: {text}")]
    InvalidSyntax { line: usize, text: String },

    #[error("line {line}: invalid dest field: {dest}")]
    InvalidDest { line: usize, dest: String },

    #[error("line {line}: invalid comp field: {comp}")]
    InvalidComp { line: usize, comp: String },

    #[error("line {line}: invalid jump field: {jump}")]
    InvalidJump { line: usize, jump: String },

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AsmError>;
