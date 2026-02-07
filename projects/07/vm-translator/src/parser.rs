use crate::error::{Result, VMError};

/// VM Command representation
#[derive(Debug, Clone, PartialEq)]
pub enum VMCommand {
    Arithmetic(ArithmeticOp),
    Push { segment: Segment, index: u16 },
    Pop { segment: Segment, index: u16 },
}

/// Arithmetic/Logical operations (9 total)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ArithmeticOp {
    Add, // x + y
    Sub, // x - y
    Neg, // -x
    Eq,  // x == y
    Lt,  // x < y
    Gt,  // x > y
    And, // x & y
    Or,  // x | y
    Not, // !x
}

/// Memory segments (8 total)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Segment {
    Constant, // Push only, immediate value
    Local,    // RAM[LCL + index]
    Argument, // RAM[ARG + index]
    This,     // RAM[THIS + index]
    That,     // RAM[THAT + index]
    Pointer,  // RAM[3] (THIS) or RAM[4] (THAT)
    Temp,     // RAM[5-12]
    Static,   // RAM[16+], file-scoped
}

impl ArithmeticOp {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "add" => Some(ArithmeticOp::Add),
            "sub" => Some(ArithmeticOp::Sub),
            "neg" => Some(ArithmeticOp::Neg),
            "eq" => Some(ArithmeticOp::Eq),
            "lt" => Some(ArithmeticOp::Lt),
            "gt" => Some(ArithmeticOp::Gt),
            "and" => Some(ArithmeticOp::And),
            "or" => Some(ArithmeticOp::Or),
            "not" => Some(ArithmeticOp::Not),
            _ => None,
        }
    }
}

impl Segment {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "constant" => Some(Segment::Constant),
            "local" => Some(Segment::Local),
            "argument" => Some(Segment::Argument),
            "this" => Some(Segment::This),
            "that" => Some(Segment::That),
            "pointer" => Some(Segment::Pointer),
            "temp" => Some(Segment::Temp),
            "static" => Some(Segment::Static),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Segment::Constant => "constant",
            Segment::Local => "local",
            Segment::Argument => "argument",
            Segment::This => "this",
            Segment::That => "that",
            Segment::Pointer => "pointer",
            Segment::Temp => "temp",
            Segment::Static => "static",
        }
    }

    /// Get maximum valid index for this segment
    pub fn max_index(&self) -> Option<u16> {
        match self {
            Segment::Pointer => Some(1),  // 0 or 1 only
            Segment::Temp => Some(7),     // 0-7 only
            Segment::Static => Some(239), // 0-239
            _ => None,                    // No fixed limit for other segments
        }
    }
}

/// Strip comments and whitespace from a line
fn clean_line(line: &str) -> &str {
    line.split("//").next().unwrap_or("").trim()
}

/// Parse a single line of VM code
/// Returns None for empty/comment-only lines
pub fn parse_line(line: &str, line_num: usize) -> Result<Option<VMCommand>> {
    let cleaned = clean_line(line);

    if cleaned.is_empty() {
        return Ok(None);
    }

    let tokens: Vec<&str> = cleaned.split_whitespace().collect();

    if tokens.is_empty() {
        return Ok(None);
    }

    let command = tokens[0];

    // Try parsing as arithmetic command (no operands)
    if let Some(op) = ArithmeticOp::from_str(command) {
        return Ok(Some(VMCommand::Arithmetic(op)));
    }

    // Parse memory access commands (require 2 operands)
    match command {
        "push" | "pop" => {
            if tokens.len() < 3 {
                return Err(VMError::MissingOperand {
                    line: line_num,
                    command: command.to_string(),
                });
            }

            let segment_str = tokens[1];
            let segment =
                Segment::from_str(segment_str).ok_or_else(|| VMError::InvalidSegment {
                    line: line_num,
                    segment: segment_str.to_string(),
                })?;

            let index_str = tokens[2];
            let index: u16 = index_str.parse().map_err(|_| VMError::InvalidIndex {
                line: line_num,
                value: index_str.to_string(),
            })?;

            // Validate index range for segments with fixed limits
            if let Some(max) = segment.max_index()
                && index > max
            {
                return Err(VMError::IndexOutOfRange {
                    line: line_num,
                    index,
                    segment: segment.name().to_string(),
                    max,
                });
            }

            // Special validation: cannot pop to constant segment
            if command == "pop" && matches!(segment, Segment::Constant) {
                return Err(VMError::PopToConstant { line: line_num });
            }

            // Validate pointer index (must be 0 or 1)
            if matches!(segment, Segment::Pointer) && index > 1 {
                return Err(VMError::InvalidPointerIndex {
                    line: line_num,
                    index,
                });
            }

            // Safe: outer match guarantees command is "push" or "pop"
            if command == "push" {
                Ok(Some(VMCommand::Push { segment, index }))
            } else {
                Ok(Some(VMCommand::Pop { segment, index }))
            }
        }
        _ => Err(VMError::InvalidCommand {
            line: line_num,
            command: command.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_arithmetic() {
        assert_eq!(
            parse_line("add", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Add))
        );
        assert_eq!(
            parse_line("sub", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Sub))
        );
        assert_eq!(
            parse_line("neg", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Neg))
        );
        assert_eq!(
            parse_line("eq", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Eq))
        );
        assert_eq!(
            parse_line("lt", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Lt))
        );
        assert_eq!(
            parse_line("gt", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Gt))
        );
        assert_eq!(
            parse_line("and", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::And))
        );
        assert_eq!(
            parse_line("or", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Or))
        );
        assert_eq!(
            parse_line("not", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Not))
        );
    }

    #[test]
    fn test_parse_push() {
        assert_eq!(
            parse_line("push constant 7", 1).unwrap(),
            Some(VMCommand::Push {
                segment: Segment::Constant,
                index: 7
            })
        );
        assert_eq!(
            parse_line("push local 2", 1).unwrap(),
            Some(VMCommand::Push {
                segment: Segment::Local,
                index: 2
            })
        );
        assert_eq!(
            parse_line("push temp 5", 1).unwrap(),
            Some(VMCommand::Push {
                segment: Segment::Temp,
                index: 5
            })
        );
    }

    #[test]
    fn test_parse_pop() {
        assert_eq!(
            parse_line("pop local 0", 1).unwrap(),
            Some(VMCommand::Pop {
                segment: Segment::Local,
                index: 0
            })
        );
        assert_eq!(
            parse_line("pop argument 3", 1).unwrap(),
            Some(VMCommand::Pop {
                segment: Segment::Argument,
                index: 3
            })
        );
    }

    #[test]
    fn test_parse_comments() {
        assert_eq!(parse_line("// This is a comment", 1).unwrap(), None);
        assert_eq!(
            parse_line("add // inline comment", 1).unwrap(),
            Some(VMCommand::Arithmetic(ArithmeticOp::Add))
        );
    }

    #[test]
    fn test_parse_empty_lines() {
        assert_eq!(parse_line("", 1).unwrap(), None);
        assert_eq!(parse_line("   ", 1).unwrap(), None);
        assert_eq!(parse_line("\t\t", 1).unwrap(), None);
    }

    #[test]
    fn test_parse_errors() {
        // Invalid command
        assert!(parse_line("invalid", 1).is_err());

        // Pop to constant
        assert!(matches!(
            parse_line("pop constant 5", 1),
            Err(VMError::PopToConstant { line: 1 })
        ));

        // Invalid segment
        assert!(matches!(
            parse_line("push invalid 5", 1),
            Err(VMError::InvalidSegment { line: 1, .. })
        ));

        // Temp index out of range
        assert!(matches!(
            parse_line("push temp 8", 1),
            Err(VMError::IndexOutOfRange {
                line: 1,
                index: 8,
                max: 7,
                ..
            })
        ));

        // Pointer index out of range (caught by max_index check)
        assert!(matches!(
            parse_line("push pointer 2", 1),
            Err(VMError::IndexOutOfRange {
                line: 1,
                index: 2,
                max: 1,
                ..
            })
        ));
    }
}
