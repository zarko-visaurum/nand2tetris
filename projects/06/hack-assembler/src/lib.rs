pub mod codegen;
pub mod error;
pub mod parser;
pub mod symbols;

use codegen::HackCodeGen;
use error::{AsmError, Result};
use parser::{Instruction, Line, parse_line};
use symbols::SymbolTable;

/// Assemble Hack assembly source to binary
pub fn assemble(source: &str) -> Result<String> {
    let lines: Vec<&str> = source.lines().collect();

    // Pre-allocate output (estimate ~16 chars per line)
    let mut output = String::with_capacity(lines.len() * 17);

    // Pass 1: Parse and build symbol table
    let mut symbol_table = SymbolTable::new();
    let mut parsed_lines = Vec::with_capacity(lines.len());
    let mut rom_address = 0u16;

    for (line_num, line) in lines.iter().enumerate() {
        let parsed = parse_line(line, line_num + 1)?;

        match &parsed {
            Line::Label(label) => {
                symbol_table
                    .add_label(label.clone(), rom_address)
                    .map_err(|dup| AsmError::DuplicateLabel {
                        line: line_num + 1,
                        label: dup,
                    })?;
            }
            Line::Instruction(_) => {
                rom_address += 1;
            }
            Line::Empty => {}
        }

        parsed_lines.push(parsed);
    }

    // Pass 2: Resolve symbols and generate code
    let codegen = HackCodeGen::hack();

    for parsed in parsed_lines.iter() {
        match parsed {
            Line::Instruction(inst) => {
                // Resolve symbols to addresses
                let resolved = match inst {
                    Instruction::ASymbol(symbol) => {
                        let addr = symbol_table.get_or_allocate(symbol);
                        inst.clone().resolve(addr)
                    }
                    Instruction::AValue(v) => inst.clone().resolve(*v),
                    Instruction::CInstruction { .. } => inst.clone().resolve(0), // addr unused for C-instructions
                };

                // Zero-allocation encoding: write directly to output buffer
                codegen.encode(&resolved, &mut output);
                output.push('\n');
            }
            Line::Label(_) | Line::Empty => {}
        }
    }

    Ok(output.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_program() {
        let source = r#"
            @2
            D=A
            @3
            D=D+A
            @0
            M=D
        "#;

        let result = assemble(source).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        assert_eq!(lines.len(), 6);
        assert_eq!(lines[0], "0000000000000010"); // @2
        assert_eq!(lines[1], "1110110000010000"); // D=A
        assert_eq!(lines[2], "0000000000000011"); // @3
        assert_eq!(lines[3], "1110000010010000"); // D=D+A
        assert_eq!(lines[4], "0000000000000000"); // @0
        assert_eq!(lines[5], "1110001100001000"); // M=D
    }

    #[test]
    fn test_with_labels() {
        let source = r#"
            @i
            M=1
        (LOOP)
            @i
            D=M
            @10
            D=D-A
            @END
            D;JGT
            @i
            M=M+1
            @LOOP
            0;JMP
        (END)
            @END
            0;JMP
        "#;

        let result = assemble(source);
        assert!(result.is_ok());

        let output = result.unwrap();
        let lines: Vec<&str> = output.lines().collect();

        // Should have 14 instructions (2 labels don't generate code)
        assert_eq!(lines.len(), 14);
    }

    #[test]
    fn test_predefined_symbols() {
        let source = r#"
            @R0
            D=M
            @SP
            M=D
            @SCREEN
            D=A
            @KBD
            D=A
        "#;

        let result = assemble(source).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        assert_eq!(lines[0], "0000000000000000"); // @R0 (0)
        assert_eq!(lines[2], "0000000000000000"); // @SP (0)
        assert_eq!(lines[4], "0100000000000000"); // @SCREEN (16384)
        assert_eq!(lines[6], "0110000000000000"); // @KBD (24576)
    }

    #[test]
    fn test_variable_allocation() {
        let source = r#"
            @i
            M=1
            @j
            M=1
            @i
            D=M
        "#;

        let result = assemble(source).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        // @i should be 16, @j should be 17
        assert_eq!(lines[0], "0000000000010000"); // @i (16)
        assert_eq!(lines[2], "0000000000010001"); // @j (17)
        assert_eq!(lines[4], "0000000000010000"); // @i (16) again
    }

    #[test]
    fn test_comments_and_whitespace() {
        let source = r#"
            // This is a comment
            @2     // inline comment
            D=A    // another comment
            
            // Empty line above
        "#;

        let result = assemble(source).unwrap();
        let lines: Vec<&str> = result.lines().collect();

        assert_eq!(lines.len(), 2); // Only 2 instructions
    }

    #[test]
    fn test_duplicate_label_error() {
        let source = r#"
        (LOOP)
            @i
            M=1
        (LOOP)
            @i
            M=2
        "#;

        let result = assemble(source);
        assert!(result.is_err());

        match result.unwrap_err() {
            AsmError::DuplicateLabel { label, .. } => {
                assert_eq!(label, "LOOP");
            }
            _ => panic!("Expected DuplicateLabel error"),
        }
    }
}
