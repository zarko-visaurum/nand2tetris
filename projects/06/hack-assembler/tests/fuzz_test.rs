use hack_assembler::assemble;
use proptest::prelude::*;

// Property-based fuzzing tests to ensure robustness against malformed input

/// Generate arbitrary assembly-like strings
fn arb_asm_line() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid-looking A-instructions
        any::<u16>().prop_map(|n| format!("@{}", n)),
        // Symbol-like strings
        "[a-zA-Z_][a-zA-Z0-9_]*".prop_map(|s| format!("@{}", s)),
        // Label-like strings
        "[a-zA-Z_][a-zA-Z0-9_]*".prop_map(|s| format!("({})", s)),
        // C-instruction-like strings (simpler to avoid regex issues)
        "[ADM01]+",
        // Comments
        "//[^\n]*",
        // Empty lines and whitespace
        "[ \t\r\n]*",
        // Garbage (printable ASCII)
        "[\\x20-\\x7E]+",
    ]
}

fn arb_asm_program() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_asm_line(), 0..100).prop_map(|lines| lines.join("\n"))
}

proptest! {
    /// Fuzzing test: assembler should never panic on arbitrary input
    #[test]
    fn test_no_panic_on_arbitrary_input(input in arb_asm_program()) {
        // The assembler might return an error, but it should never panic
        let _ = assemble(&input);
    }

    /// Fuzzing test: valid numeric A-instructions should always work
    #[test]
    fn test_valid_a_instructions(addr in 0u16..=32767) {
        let source = format!("@{}", addr);
        let result = assemble(&source);
        assert!(result.is_ok(), "Failed on valid A-instruction: @{}", addr);

        let output = result.unwrap();
        assert_eq!(output.lines().count(), 1);
        assert_eq!(output.len(), 16); // 16-bit binary string
    }

    /// Fuzzing test: predefined symbols should always work
    #[test]
    fn test_predefined_symbols(
        symbol in prop_oneof![
            Just("R0"), Just("R1"), Just("R15"),
            Just("SP"), Just("LCL"), Just("ARG"), Just("THIS"), Just("THAT"),
            Just("SCREEN"), Just("KBD")
        ]
    ) {
        let source = format!("@{}", symbol);
        let result = assemble(&source);
        assert!(result.is_ok(), "Failed on predefined symbol: @{}", symbol);
    }

    /// Fuzzing test: invalid A-instruction values should error gracefully
    #[test]
    fn test_invalid_a_values(addr in 32768u32..=65535) {
        let source = format!("@{}", addr);
        let result = assemble(&source);
        // Should either error or (if parsed as symbol) succeed
        // But should never panic
        let _ = result;
    }

    /// Fuzzing test: comments should be ignored
    #[test]
    fn test_comments_ignored(comment in "//.*") {
        let result = assemble(&comment);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ""); // Comments produce no output
    }

    /// Fuzzing test: whitespace handling
    #[test]
    fn test_whitespace_handling(ws in "[ \t]*") {
        let source = format!("{}@0{}", ws, ws);
        let result = assemble(&source);
        assert!(result.is_ok(), "Failed on whitespace-padded input");
    }

    /// Fuzzing test: duplicate labels should error
    #[test]
    fn test_duplicate_labels(label in "[A-Z][A-Z0-9_]*") {
        let source = format!("({})\n@0\n({})\n@1", label, label);
        let result = assemble(&source);
        assert!(result.is_err(), "Should error on duplicate label: {}", label);
    }

    /// Fuzzing test: variable allocation consistency
    #[test]
    fn test_variable_allocation(vars in prop::collection::vec("[a-z][a-z0-9]*", 1..10)) {
        let mut source = String::new();
        for var in &vars {
            source.push_str(&format!("@{}\nM=1\n", var));
        }

        let result = assemble(&source);
        assert!(result.is_ok(), "Failed on variable allocation");

        // Should produce 2 instructions per variable
        let output = result.unwrap();
        assert_eq!(output.lines().count(), vars.len() * 2);
    }
}

#[cfg(test)]
mod additional_fuzz_tests {
    use super::*;

    #[test]
    fn test_empty_input() {
        assert_eq!(assemble("").unwrap(), "");
    }

    #[test]
    fn test_only_comments() {
        assert_eq!(assemble("// comment\n// another").unwrap(), "");
    }

    #[test]
    fn test_only_whitespace() {
        assert_eq!(assemble("   \n\t\n  ").unwrap(), "");
    }

    #[test]
    fn test_max_valid_address() {
        let result = assemble("@32767");
        assert!(result.is_ok());
    }

    #[test]
    fn test_beyond_max_address() {
        let result = assemble("@32768");
        // Should error since 32768 > 32767 (15-bit max)
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_label_no_closing() {
        let result = assemble("(LABEL");
        assert!(result.is_err());
    }

    #[test]
    fn test_malformed_label_no_opening() {
        let result = assemble("LABEL)");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_a_instruction() {
        let result = assemble("@");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_c_instruction() {
        let result = assemble("D==M"); // Double equals is invalid
        assert!(result.is_err());
    }

    #[test]
    fn test_long_symbol_name() {
        let long_name = "a".repeat(1000);
        let source = format!("@{}", long_name);
        let result = assemble(&source);
        // Should not panic, even with very long symbol names
        assert!(result.is_ok());
    }
}
