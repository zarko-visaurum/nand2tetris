//! VM command parser supporting all 20 commands.
//!
//! Parses VM bytecode into typed command structures with full validation.

use crate::error::{Result, VMError};

/// Arithmetic and logical operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOp {
    Add,
    Sub,
    Neg,
    Eq,
    Lt,
    Gt,
    And,
    Or,
    Not,
}

/// Memory segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Segment {
    Constant,
    Local,
    Argument,
    This,
    That,
    Pointer,
    Temp,
    Static,
}

/// VM command variants.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMCommand {
    // Arithmetic/logical (9 commands)
    Arithmetic(ArithmeticOp),

    // Memory access (push/pop × 8 segments)
    Push { segment: Segment, index: u16 },
    Pop { segment: Segment, index: u16 },

    // Program flow (3 commands)
    Label { name: String },
    Goto { label: String },
    IfGoto { label: String },

    // Function commands (3 commands)
    Function { name: String, num_locals: u16 },
    Call { name: String, num_args: u16 },
    Return,
}

/// Parse a single VM line into a command.
///
/// Returns `Ok(None)` for empty lines and comments.
/// Returns `Ok(Some(cmd))` for valid commands.
/// Returns `Err` for invalid syntax.
pub fn parse_line(line: &str, line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    // Strip comments and whitespace
    let line = line.split("//").next().unwrap_or("").trim();
    if line.is_empty() {
        return Ok(None);
    }

    let parts: Vec<&str> = line.split_whitespace().collect();
    let cmd = parts[0].to_lowercase();

    match cmd.as_str() {
        // Arithmetic/logical commands
        "add" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Add))),
        "sub" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Sub))),
        "neg" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Neg))),
        "eq" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Eq))),
        "lt" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Lt))),
        "gt" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Gt))),
        "and" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::And))),
        "or" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Or))),
        "not" => Ok(Some(VMCommand::Arithmetic(ArithmeticOp::Not))),

        // Memory access commands
        "push" => parse_push(&parts, line_num, filename),
        "pop" => parse_pop(&parts, line_num, filename),

        // Program flow commands
        "label" => parse_label(&parts, line_num, filename),
        "goto" => parse_goto(&parts, line_num, filename),
        "if-goto" => parse_if_goto(&parts, line_num, filename),

        // Function commands
        "function" => parse_function(&parts, line_num, filename),
        "call" => parse_call(&parts, line_num, filename),
        "return" => Ok(Some(VMCommand::Return)),

        _ => Err(VMError::InvalidCommand {
            line: line_num,
            file: filename.to_string(),
            command: cmd,
        }),
    }
}

fn parse_push(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 3 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "push".to_string(),
        });
    }

    let segment = parse_segment(parts[1], line_num, filename)?;
    let index = parse_index(parts[2], line_num, filename)?;
    validate_segment_index(segment, index, line_num, filename)?;

    Ok(Some(VMCommand::Push { segment, index }))
}

fn parse_pop(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 3 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "pop".to_string(),
        });
    }

    let segment = parse_segment(parts[1], line_num, filename)?;

    // Cannot pop to constant
    if segment == Segment::Constant {
        return Err(VMError::PopToConstant {
            line: line_num,
            file: filename.to_string(),
        });
    }

    let index = parse_index(parts[2], line_num, filename)?;
    validate_segment_index(segment, index, line_num, filename)?;

    Ok(Some(VMCommand::Pop { segment, index }))
}

fn parse_label(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 2 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "label".to_string(),
        });
    }

    let name = parts[1].to_string();
    if name.is_empty() {
        return Err(VMError::InvalidLabelName {
            line: line_num,
            file: filename.to_string(),
            name,
        });
    }

    Ok(Some(VMCommand::Label { name }))
}

fn parse_goto(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 2 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "goto".to_string(),
        });
    }

    Ok(Some(VMCommand::Goto {
        label: parts[1].to_string(),
    }))
}

fn parse_if_goto(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 2 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "if-goto".to_string(),
        });
    }

    Ok(Some(VMCommand::IfGoto {
        label: parts[1].to_string(),
    }))
}

fn parse_function(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 3 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "function".to_string(),
        });
    }

    let name = parts[1].to_string();
    if name.is_empty() {
        return Err(VMError::InvalidFunctionName {
            line: line_num,
            file: filename.to_string(),
            name,
        });
    }

    let num_locals = parse_index(parts[2], line_num, filename)?;

    Ok(Some(VMCommand::Function { name, num_locals }))
}

fn parse_call(parts: &[&str], line_num: usize, filename: &str) -> Result<Option<VMCommand>> {
    if parts.len() < 3 {
        return Err(VMError::MissingArgument {
            line: line_num,
            file: filename.to_string(),
            command: "call".to_string(),
        });
    }

    let name = parts[1].to_string();
    let num_args = parse_index(parts[2], line_num, filename)?;

    Ok(Some(VMCommand::Call { name, num_args }))
}

fn parse_segment(s: &str, line_num: usize, filename: &str) -> Result<Segment> {
    match s.to_lowercase().as_str() {
        "constant" => Ok(Segment::Constant),
        "local" => Ok(Segment::Local),
        "argument" => Ok(Segment::Argument),
        "this" => Ok(Segment::This),
        "that" => Ok(Segment::That),
        "pointer" => Ok(Segment::Pointer),
        "temp" => Ok(Segment::Temp),
        "static" => Ok(Segment::Static),
        _ => Err(VMError::InvalidSegment {
            line: line_num,
            file: filename.to_string(),
            segment: s.to_string(),
        }),
    }
}

fn parse_index(s: &str, line_num: usize, filename: &str) -> Result<u16> {
    s.parse::<u16>().map_err(|_| VMError::InvalidNumber {
        line: line_num,
        file: filename.to_string(),
        value: s.to_string(),
    })
}

fn validate_segment_index(
    segment: Segment,
    index: u16,
    line_num: usize,
    filename: &str,
) -> Result<()> {
    match segment {
        Segment::Pointer if index > 1 => Err(VMError::InvalidPointerIndex {
            line: line_num,
            file: filename.to_string(),
            index,
        }),
        Segment::Temp if index > 7 => Err(VMError::InvalidTempIndex {
            line: line_num,
            file: filename.to_string(),
            index,
        }),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arithmetic() {
        assert_eq!(
            parse_line("add", 1, "Test.vm").unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Add))
        );
        assert_eq!(
            parse_line("sub", 1, "Test.vm").unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Sub))
        );
        assert_eq!(
            parse_line("eq", 1, "Test.vm").unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Eq))
        );
    }

    #[test]
    fn test_parse_push() {
        assert_eq!(
            parse_line("push constant 7", 1, "Test.vm").unwrap(),
            Some(VMCommand::Push {
                segment: Segment::Constant,
                index: 7
            })
        );
        assert_eq!(
            parse_line("push local 0", 1, "Test.vm").unwrap(),
            Some(VMCommand::Push {
                segment: Segment::Local,
                index: 0
            })
        );
    }

    #[test]
    fn test_parse_pop() {
        assert_eq!(
            parse_line("pop local 2", 1, "Test.vm").unwrap(),
            Some(VMCommand::Pop {
                segment: Segment::Local,
                index: 2
            })
        );
    }

    #[test]
    fn test_parse_pop_constant_error() {
        let result = parse_line("pop constant 5", 1, "Test.vm");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_label() {
        assert_eq!(
            parse_line("label LOOP", 1, "Test.vm").unwrap(),
            Some(VMCommand::Label {
                name: "LOOP".to_string()
            })
        );
    }

    #[test]
    fn test_parse_goto() {
        assert_eq!(
            parse_line("goto END", 1, "Test.vm").unwrap(),
            Some(VMCommand::Goto {
                label: "END".to_string()
            })
        );
    }

    #[test]
    fn test_parse_if_goto() {
        assert_eq!(
            parse_line("if-goto LOOP", 1, "Test.vm").unwrap(),
            Some(VMCommand::IfGoto {
                label: "LOOP".to_string()
            })
        );
    }

    #[test]
    fn test_parse_function() {
        assert_eq!(
            parse_line("function Foo.bar 3", 1, "Test.vm").unwrap(),
            Some(VMCommand::Function {
                name: "Foo.bar".to_string(),
                num_locals: 3
            })
        );
    }

    #[test]
    fn test_parse_call() {
        assert_eq!(
            parse_line("call Foo.bar 2", 1, "Test.vm").unwrap(),
            Some(VMCommand::Call {
                name: "Foo.bar".to_string(),
                num_args: 2
            })
        );
    }

    #[test]
    fn test_parse_return() {
        assert_eq!(
            parse_line("return", 1, "Test.vm").unwrap(),
            Some(VMCommand::Return)
        );
    }

    #[test]
    fn test_parse_comments() {
        assert_eq!(parse_line("// comment", 1, "Test.vm").unwrap(), None);
        assert_eq!(
            parse_line("add // inline comment", 1, "Test.vm").unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Add))
        );
    }

    #[test]
    fn test_parse_empty() {
        assert_eq!(parse_line("", 1, "Test.vm").unwrap(), None);
        assert_eq!(parse_line("   ", 1, "Test.vm").unwrap(), None);
    }

    #[test]
    fn test_validate_pointer_index() {
        assert!(parse_line("push pointer 0", 1, "Test.vm").is_ok());
        assert!(parse_line("push pointer 1", 1, "Test.vm").is_ok());
        assert!(parse_line("push pointer 2", 1, "Test.vm").is_err());
    }

    #[test]
    fn test_validate_temp_index() {
        assert!(parse_line("push temp 7", 1, "Test.vm").is_ok());
        assert!(parse_line("push temp 8", 1, "Test.vm").is_err());
    }
}
