use crate::error::{AsmError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Instruction {
    AValue(u16),
    ASymbol(String),
    CInstruction { dest: u8, comp: u8, jump: u8 },
}

/// Resolved instruction with all symbols converted to addresses
/// This type makes it impossible to have unresolved symbols at codegen time
#[derive(Debug, Clone, PartialEq)]
pub enum ResolvedInstruction {
    AValue(u16),
    CInstruction { dest: u8, comp: u8, jump: u8 },
}

impl Instruction {
    /// Resolve an instruction by converting symbols to addresses
    pub fn resolve(self, addr: u16) -> ResolvedInstruction {
        match self {
            Instruction::AValue(v) => ResolvedInstruction::AValue(v),
            Instruction::ASymbol(_) => ResolvedInstruction::AValue(addr),
            Instruction::CInstruction { dest, comp, jump } => {
                ResolvedInstruction::CInstruction { dest, comp, jump }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Instruction(Instruction),
    Label(String),
    Empty,
}

/// Strip comments and whitespace
fn clean_line(line: &str) -> &str {
    line.split("//").next().unwrap_or("").trim()
}

/// Parse A-instruction (@value or @symbol)
fn parse_a_instruction(line: &str, line_num: usize) -> Result<Instruction> {
    let value_str = &line[1..]; // Skip '@'

    if value_str.is_empty() {
        return Err(AsmError::InvalidSyntax {
            line: line_num,
            text: line.to_string(),
        });
    }

    // Try parse as number
    if let Ok(value) = value_str.parse::<u16>() {
        if value > 32767 {
            return Err(AsmError::InvalidAValue {
                line: line_num,
                value: value_str.to_string(),
            });
        }
        Ok(Instruction::AValue(value))
    } else {
        // Symbol
        Ok(Instruction::ASymbol(value_str.to_string()))
    }
}

/// Parse C-instruction (dest=comp;jump)
fn parse_c_instruction(line: &str, line_num: usize) -> Result<Instruction> {
    let (dest_str, rest) = if let Some(eq_pos) = line.find('=') {
        (&line[..eq_pos], &line[eq_pos + 1..])
    } else {
        ("", line)
    };

    let (comp_str, jump_str) = if let Some(semi_pos) = rest.find(';') {
        (&rest[..semi_pos], &rest[semi_pos + 1..])
    } else {
        (rest, "")
    };

    let dest = parse_dest(dest_str).ok_or_else(|| AsmError::InvalidDest {
        line: line_num,
        dest: dest_str.to_string(),
    })?;

    let comp = parse_comp(comp_str).ok_or_else(|| AsmError::InvalidComp {
        line: line_num,
        comp: comp_str.to_string(),
    })?;

    let jump = parse_jump(jump_str).ok_or_else(|| AsmError::InvalidJump {
        line: line_num,
        jump: jump_str.to_string(),
    })?;

    Ok(Instruction::CInstruction { dest, comp, jump })
}

/// Parse dest field (3 bits: A D M)
fn parse_dest(s: &str) -> Option<u8> {
    match s {
        "" => Some(0b000),
        "M" => Some(0b001),
        "D" => Some(0b010),
        "MD" | "DM" => Some(0b011),
        "A" => Some(0b100),
        "AM" | "MA" => Some(0b101),
        "AD" | "DA" => Some(0b110),
        "AMD" | "ADM" | "MAD" | "MDA" | "DAM" | "DMA" => Some(0b111),
        _ => None,
    }
}

/// Parse comp field (7 bits: a + 6 c-bits)
/// The 'a' bit determines if M (a=1) or A (a=0) is used
fn parse_comp(s: &str) -> Option<u8> {
    match s {
        // === Constants (a=0) ===
        "0" => Some(0b0101010),
        "1" => Some(0b0111111),
        "-1" => Some(0b0111010),

        // === D-register operations (a=0) ===
        "D" => Some(0b0001100),
        "!D" => Some(0b0001101),
        "-D" => Some(0b0001111),
        "D+1" | "1+D" => Some(0b0011111),
        "D-1" => Some(0b0001110),

        // === A-register operations (a=0) ===
        "A" => Some(0b0110000),
        "!A" => Some(0b0110001),
        "-A" => Some(0b0110011),
        "A+1" | "1+A" => Some(0b0110111),
        "A-1" => Some(0b0110010),

        // === ALU operations with A-register (a=0) ===
        "D+A" | "A+D" => Some(0b0000010),
        "D-A" => Some(0b0010011),
        "A-D" => Some(0b0000111),
        "D&A" | "A&D" => Some(0b0000000),
        "D|A" | "A|D" => Some(0b0010101),

        // === M-register operations (a=1) ===
        "M" => Some(0b1110000),
        "!M" => Some(0b1110001),
        "-M" => Some(0b1110011),
        "M+1" | "1+M" => Some(0b1110111),
        "M-1" => Some(0b1110010),

        // === ALU operations with M-register (a=1) ===
        "D+M" | "M+D" => Some(0b1000010),
        "D-M" => Some(0b1010011),
        "M-D" => Some(0b1000111),
        "D&M" | "M&D" => Some(0b1000000),
        "D|M" | "M|D" => Some(0b1010101),

        _ => None,
    }
}

/// Parse jump field (3 bits)
fn parse_jump(s: &str) -> Option<u8> {
    match s {
        "" => Some(0b000),
        "JGT" => Some(0b001),
        "JEQ" => Some(0b010),
        "JGE" => Some(0b011),
        "JLT" => Some(0b100),
        "JNE" => Some(0b101),
        "JLE" => Some(0b110),
        "JMP" => Some(0b111),
        _ => None,
    }
}

/// Parse single line
pub fn parse_line(line: &str, line_num: usize) -> Result<Line> {
    let clean = clean_line(line);

    if clean.is_empty() {
        return Ok(Line::Empty);
    }

    // Label
    if clean.starts_with('(') {
        if !clean.ends_with(')') {
            return Err(AsmError::InvalidSyntax {
                line: line_num,
                text: line.to_string(),
            });
        }
        let label = clean[1..clean.len() - 1].to_string();
        return Ok(Line::Label(label));
    }

    // A-instruction
    if clean.starts_with('@') {
        return Ok(Line::Instruction(parse_a_instruction(clean, line_num)?));
    }

    // C-instruction
    Ok(Line::Instruction(parse_c_instruction(clean, line_num)?))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_line() {
        assert_eq!(clean_line("  @123  "), "@123");
        assert_eq!(clean_line("D=M // comment"), "D=M");
        assert_eq!(clean_line("// only comment"), "");
    }

    #[test]
    fn test_parse_a_value() {
        let inst = parse_line("@17", 1).unwrap();
        assert_eq!(inst, Line::Instruction(Instruction::AValue(17)));
    }

    #[test]
    fn test_parse_a_symbol() {
        let inst = parse_line("@LOOP", 1).unwrap();
        assert_eq!(
            inst,
            Line::Instruction(Instruction::ASymbol("LOOP".to_string()))
        );
    }

    #[test]
    fn test_parse_label() {
        let line = parse_line("(LOOP)", 1).unwrap();
        assert_eq!(line, Line::Label("LOOP".to_string()));
    }

    #[test]
    fn test_parse_c_instruction() {
        let inst = parse_line("D=M+1", 1).unwrap();
        match inst {
            Line::Instruction(Instruction::CInstruction { dest, comp, jump }) => {
                assert_eq!(dest, 0b010); // D
                assert_eq!(comp, 0b1110111); // M+1
                assert_eq!(jump, 0b000); // no jump
            }
            _ => panic!("Expected C-instruction"),
        }
    }

    #[test]
    fn test_parse_c_with_jump() {
        let inst = parse_line("D;JGT", 1).unwrap();
        match inst {
            Line::Instruction(Instruction::CInstruction { dest, comp, jump }) => {
                assert_eq!(dest, 0b000);
                assert_eq!(comp, 0b0001100); // D
                assert_eq!(jump, 0b001); // JGT
            }
            _ => panic!("Expected C-instruction"),
        }
    }
}
