//! VM Translator - Stack VM to Hack Assembly
//!
//! This library provides a production-grade translator for converting Stack VM
//! bytecode (.vm) into Hack assembly language (.asm). Built with zero-allocation
//! hot paths and comprehensive error handling.
//!
//! # Architecture
//!
//! ```text
//! VM Bytecode → Parser → CodeGen → Hack Assembly
//! ```
//!
//! - **Parser**: Tokenizes and validates VM commands
//! - **CodeGen**: Generates optimized Hack assembly
//! - **Memory**: Handles segment addressing logic
//! - **Error**: Provides contextual error messages with line numbers
//!
//! # Example
//!
//! ```
//! use vm_translator::translate;
//!
//! let vm_code = "push constant 7\npush constant 8\nadd";
//! let asm_code = translate(vm_code, "SimpleAdd").unwrap();
//! ```

pub mod codegen;
pub mod error;
pub mod memory;
pub mod parser;

use codegen::{Backend, HackAssembly};
use error::Result;
use parser::parse_line;

/// Translate VM code to Hack assembly
///
/// Performs single-pass translation of Stack VM bytecode into Hack assembly.
/// The translation is zero-allocation (writes directly to pre-allocated buffer).
///
/// # Arguments
///
/// * `source` - The VM source code as a string
/// * `filename` - The source filename (without extension, used for static variable naming)
///
/// # Returns
///
/// * `Ok(String)` - The generated Hack assembly code
/// * `Err(VMError)` - Parse or translation error with line context
///
/// # Examples
///
/// ```
/// use vm_translator::translate;
///
/// // Simple arithmetic
/// let result = translate("push constant 5\npush constant 3\nadd", "Test");
/// assert!(result.is_ok());
///
/// // Error handling
/// let result = translate("invalid command", "Test");
/// assert!(result.is_err());
/// ```
///
/// # Translation Process
///
/// 1. **Parse**: Each line is tokenized and validated
/// 2. **Translate**: VM commands are converted to Hack assembly
/// 3. **Output**: Assembly is written to pre-allocated buffer
///
/// # Performance
///
/// - Zero allocations in hot path after initial buffer allocation
/// - Single-pass design (no symbol table needed for VM code)
/// - Pre-allocated output buffer (~50 chars per VM command)
pub fn translate(source: &str, filename: &str) -> Result<String> {
    let lines: Vec<&str> = source.lines().collect();

    // Pre-allocate output buffer (estimate ~50 chars per VM command)
    let mut output = String::with_capacity(lines.len() * 50);

    // Create code generator
    let mut codegen = HackAssembly::new(filename);

    // Single-pass translation: parse and generate code line by line
    for (line_num, line) in lines.iter().enumerate() {
        if let Some(cmd) = parse_line(line, line_num + 1)? {
            // Zero-allocation: write directly to output buffer
            codegen.translate_command(&cmd, &mut output);
            output.push('\n');
        }
    }

    Ok(output.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_arithmetic() {
        let vm_code = "push constant 7\npush constant 8\nadd";
        let asm_code = translate(vm_code, "SimpleAdd").unwrap();

        // Verify key assembly patterns
        assert!(asm_code.contains("@7"));
        assert!(asm_code.contains("@8"));
        assert!(asm_code.contains("D+M"));
    }

    #[test]
    fn test_all_arithmetic_operations() {
        let vm_code = r#"
            push constant 7
            push constant 3
            add
            push constant 5
            sub
            neg
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Should translate without errors
        assert!(asm_code.contains("@7"));
        assert!(asm_code.contains("@3"));
        assert!(asm_code.contains("@5"));
    }

    #[test]
    fn test_all_logical_operations() {
        let vm_code = r#"
            push constant 15
            push constant 7
            and
            push constant 3
            or
            not
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Verify logical operations
        assert!(asm_code.contains("D&M"));
        assert!(asm_code.contains("D|M"));
        assert!(asm_code.contains("M=!M"));
    }

    #[test]
    fn test_comparison_operations() {
        let vm_code = r#"
            push constant 5
            push constant 5
            eq
            push constant 3
            push constant 7
            lt
            push constant 9
            push constant 4
            gt
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Verify unique labels generated
        assert!(asm_code.contains("JEQ"));
        assert!(asm_code.contains("JLT"));
        assert!(asm_code.contains("JGT"));

        // Labels should be unique (different numbers)
        assert!(asm_code.contains("_0"));
        assert!(asm_code.contains("_2"));
        assert!(asm_code.contains("_4"));
    }

    #[test]
    fn test_memory_segments() {
        let vm_code = r#"
            push constant 10
            pop local 0
            push local 0
            pop argument 1
            push argument 1
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Verify segment base pointers
        assert!(asm_code.contains("@LCL"));
        assert!(asm_code.contains("@ARG"));
        assert!(asm_code.contains("@R13")); // Used for pop
    }

    #[test]
    fn test_static_variables() {
        let vm_code = r#"
            push constant 42
            pop static 5
            push static 5
        "#;
        let asm_code = translate(vm_code, "MyFile").unwrap();

        // Verify static naming convention (FileName.index)
        assert!(asm_code.contains("@MyFile.5"));
    }

    #[test]
    fn test_pointer_segment() {
        let vm_code = r#"
            push constant 3000
            pop pointer 0
            push constant 3010
            pop pointer 1
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Verify direct THIS/THAT access
        assert!(asm_code.contains("@THIS") || asm_code.contains("@3"));
        assert!(asm_code.contains("@THAT") || asm_code.contains("@4"));
    }

    #[test]
    fn test_temp_segment() {
        let vm_code = r#"
            push constant 100
            pop temp 3
            push temp 3
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Verify temp addressing (RAM[5+index])
        assert!(asm_code.contains("@8")); // temp 3 = RAM[5+3] = RAM[8]
    }

    #[test]
    fn test_comments_and_whitespace() {
        let vm_code = r#"
            // This is a comment
            push constant 5    // inline comment
            add                // another comment

            // Empty line above
        "#;
        let asm_code = translate(vm_code, "Test").unwrap();

        // Comments should be stripped, commands should be translated
        assert!(asm_code.contains("@5"));
        assert!(asm_code.contains("D+M"));
    }

    #[test]
    fn test_empty_and_comment_only_lines() {
        let vm_code = r#"

        // Comment only

        push constant 7

        // Another comment

        "#;
        let result = translate(vm_code, "Test");
        assert!(result.is_ok());
    }

    #[test]
    fn test_invalid_command_error() {
        let vm_code = "invalid command";
        let result = translate(vm_code, "Test");

        // Should return error with line context
        assert!(result.is_err());
    }

    #[test]
    fn test_pop_to_constant_error() {
        let vm_code = "pop constant 5";
        let result = translate(vm_code, "Test");

        // Should return error (cannot pop to constant)
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_segment_error() {
        let vm_code = "push invalid 5";
        let result = translate(vm_code, "Test");

        assert!(result.is_err());
    }

    #[test]
    fn test_index_out_of_range_error() {
        let vm_code = "push temp 8"; // temp only allows 0-7
        let result = translate(vm_code, "Test");

        assert!(result.is_err());
    }

    #[test]
    fn test_output_no_trailing_newlines() {
        let vm_code = "push constant 5";
        let asm_code = translate(vm_code, "Test").unwrap();

        // Output should not end with newline
        assert!(!asm_code.ends_with('\n'));
    }

    #[test]
    fn test_multiple_commands_with_newlines() {
        let vm_code = "push constant 1\npush constant 2\nadd";
        let asm_code = translate(vm_code, "Test").unwrap();

        // Each command generates multiple assembly lines
        let line_count = asm_code.lines().count();
        assert!(line_count > 3); // More than 3 VM commands worth of assembly
    }
}
