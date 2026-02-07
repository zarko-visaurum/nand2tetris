//! Property-based fuzzing tests for the Full VM Translator (Project 08).
//!
//! Uses proptest to generate arbitrary VM commands and verify the translator
//! never panics and handles all input gracefully.

use proptest::prelude::*;
use vm_translator::translate;

/// Generate arbitrary arithmetic commands
fn arb_arithmetic() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("add".to_string()),
        Just("sub".to_string()),
        Just("neg".to_string()),
        Just("eq".to_string()),
        Just("lt".to_string()),
        Just("gt".to_string()),
        Just("and".to_string()),
        Just("or".to_string()),
        Just("not".to_string()),
    ]
}

/// Generate valid push commands
fn arb_push() -> impl Strategy<Value = String> {
    prop_oneof![
        // Push constant (most common)
        (0u16..32768).prop_map(|n| format!("push constant {}", n)),
        // Push temp (0-7)
        (0u16..8).prop_map(|n| format!("push temp {}", n)),
        // Push pointer (0-1)
        (0u16..2).prop_map(|n| format!("push pointer {}", n)),
        // Push indirect segments
        (0u16..100).prop_map(|n| format!("push local {}", n)),
        (0u16..100).prop_map(|n| format!("push argument {}", n)),
        (0u16..100).prop_map(|n| format!("push this {}", n)),
        (0u16..100).prop_map(|n| format!("push that {}", n)),
        // Push static (0-239)
        (0u16..240).prop_map(|n| format!("push static {}", n)),
    ]
}

/// Generate valid pop commands
fn arb_pop() -> impl Strategy<Value = String> {
    prop_oneof![
        // Pop temp (0-7)
        (0u16..8).prop_map(|n| format!("pop temp {}", n)),
        // Pop pointer (0-1)
        (0u16..2).prop_map(|n| format!("pop pointer {}", n)),
        // Pop indirect segments
        (0u16..100).prop_map(|n| format!("pop local {}", n)),
        (0u16..100).prop_map(|n| format!("pop argument {}", n)),
        (0u16..100).prop_map(|n| format!("pop this {}", n)),
        (0u16..100).prop_map(|n| format!("pop that {}", n)),
        // Pop static (0-239)
        (0u16..240).prop_map(|n| format!("pop static {}", n)),
    ]
}

/// Generate valid label names
fn arb_label_name() -> impl Strategy<Value = String> {
    "[A-Z][A-Z0-9_]{0,10}".prop_map(|s| s)
}

/// Generate branching commands
fn arb_branching() -> impl Strategy<Value = String> {
    arb_label_name().prop_flat_map(|name| {
        prop_oneof![
            Just(format!("label {}", name)),
            Just(format!("goto {}", name)),
            Just(format!("if-goto {}", name)),
        ]
    })
}

/// Generate function names
fn arb_function_name() -> impl Strategy<Value = String> {
    ("[A-Z][a-zA-Z0-9]*", "[a-z][a-zA-Z0-9]*")
        .prop_map(|(class, method)| format!("{}.{}", class, method))
}

/// Generate function commands
fn arb_function_cmd() -> impl Strategy<Value = String> {
    (arb_function_name(), 0u16..10).prop_map(|(name, n)| format!("function {} {}", name, n))
}

/// Generate call commands
fn arb_call_cmd() -> impl Strategy<Value = String> {
    (arb_function_name(), 0u16..10).prop_map(|(name, n)| format!("call {} {}", name, n))
}

/// Generate any valid VM command
fn arb_valid_vm_line() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => arb_arithmetic(),
        4 => arb_push(),
        3 => arb_pop(),
        2 => arb_branching(),
        1 => arb_function_cmd(),
        1 => arb_call_cmd(),
        1 => Just("return".to_string()),
    ]
}

/// Generate arbitrary VM lines including invalid ones
fn arb_vm_line() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid commands
        8 => arb_valid_vm_line(),
        // Comments
        1 => Just("// This is a comment".to_string()),
        // Empty lines
        1 => Just("".to_string()),
        1 => Just("   ".to_string()),
        // Invalid commands (should produce errors, not panics)
        1 => "[a-z]{3,10}".prop_map(|s| s),
        1 => "push [a-z]+ [0-9]+".prop_map(|s| s),
    ]
}

/// Generate a VM program (multiple lines)
fn arb_vm_program() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_vm_line(), 0..50).prop_map(|lines| lines.join("\n"))
}

/// Generate a valid function body
fn arb_valid_function() -> impl Strategy<Value = String> {
    (
        arb_function_name(),
        0u16..5,
        prop::collection::vec(prop_oneof![arb_arithmetic(), arb_push(), arb_pop(),], 1..10),
    )
        .prop_map(|(name, locals, body)| {
            let mut lines = vec![format!("function {} {}", name, locals)];
            lines.extend(body);
            lines.push("return".to_string());
            lines.join("\n")
        })
}

proptest! {
    /// Test that translator never panics on arbitrary input
    #[test]
    fn test_no_panic_on_arbitrary_input(input in arb_vm_program()) {
        let _ = translate(&input, "Test");
    }

    /// Test that valid arithmetic commands always succeed
    #[test]
    fn test_valid_arithmetic_succeeds(op in arb_arithmetic()) {
        let result = translate(&op, "Test");
        prop_assert!(result.is_ok(), "Arithmetic operation should succeed");
    }

    /// Test that valid push constant commands always succeed
    #[test]
    fn test_valid_push_constant(n in 0u16..32768) {
        let vm_code = format!("push constant {}", n);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Push constant should succeed");
    }

    /// Test that valid temp operations succeed
    #[test]
    fn test_valid_temp_operations(index in 0u16..8) {
        let push_code = format!("push temp {}", index);
        let pop_code = format!("pop temp {}", index);

        let push_result = translate(&push_code, "Test");
        let pop_result = translate(&pop_code, "Test");

        prop_assert!(push_result.is_ok(), "Push temp should succeed");
        prop_assert!(pop_result.is_ok(), "Pop temp should succeed");
    }

    /// Test that valid pointer operations succeed
    #[test]
    fn test_valid_pointer_operations(index in 0u16..2) {
        let push_code = format!("push pointer {}", index);
        let pop_code = format!("pop pointer {}", index);

        let push_result = translate(&push_code, "Test");
        let pop_result = translate(&pop_code, "Test");

        prop_assert!(push_result.is_ok(), "Push pointer should succeed");
        prop_assert!(pop_result.is_ok(), "Pop pointer should succeed");
    }

    /// Test that invalid temp indices fail gracefully
    #[test]
    fn test_invalid_temp_index(index in 8u16..100) {
        let vm_code = format!("push temp {}", index);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_err(), "Invalid temp index should fail");
    }

    /// Test that invalid pointer indices fail gracefully
    #[test]
    fn test_invalid_pointer_index(index in 2u16..100) {
        let vm_code = format!("push pointer {}", index);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_err(), "Invalid pointer index should fail");
    }

    /// Test that pop to constant fails gracefully
    #[test]
    fn test_pop_to_constant_fails(n in 0u16..32768) {
        let vm_code = format!("pop constant {}", n);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_err(), "Pop to constant should fail");
    }

    /// Test that invalid segments fail gracefully
    #[test]
    fn test_invalid_segment(segment in "[a-z]{5,10}") {
        let vm_code = format!("push {} 5", segment);
        let result = translate(&vm_code, "Test");
        // Should either succeed (if it's a valid segment) or fail gracefully (invalid segment)
        // Test passes if no panic occurs
        let _ = result;
    }

    /// Test that malformed commands fail gracefully
    #[test]
    fn test_malformed_commands(cmd in "[a-z ]{1,20}") {
        let result = translate(&cmd, "Test");
        // Should either succeed or fail gracefully, never panic
        // Test passes if no panic occurs
        let _ = result;
    }

    /// Test that multiple valid commands produce consistent output
    #[test]
    fn test_multiple_commands_consistency(count in 1usize..20) {
        let commands = vec!["push constant 5"; count];
        let vm_code = commands.join("\n");
        let result = translate(&vm_code, "Test");

        prop_assert!(result.is_ok(), "Multiple commands should succeed");
        if let Ok(asm) = result {
            // Count occurrences of push operations
            let push_count = asm.matches("@5").count();
            prop_assert!(push_count >= count, "Should have at least {} pushes", count);
        }
    }

    /// Test that comments are properly stripped
    #[test]
    fn test_comments_stripped(comment in "// [a-zA-Z0-9 ]{0,50}") {
        let vm_code = format!("{}\npush constant 10", comment);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Comments should be stripped");
    }

    /// Test that empty lines are handled
    #[test]
    fn test_empty_lines(empty_count in 0usize..10) {
        let empties = vec!["\n"; empty_count];
        let vm_code = format!("{}push constant 5", empties.join(""));
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Empty lines should be handled");
    }

    /// Test that label generation is unique across comparisons
    #[test]
    fn test_label_uniqueness(comparison_count in 1usize..10) {
        let commands = vec!["push constant 5\npush constant 5\neq"; comparison_count];
        let vm_code = commands.join("\n");
        let result = translate(&vm_code, "Test");

        if let Ok(asm) = result {
            // Each comparison generates 2 labels (TRUE_n and END_n)
            let true_count = asm.matches("JEQ_TRUE_").count();
            let end_count = asm.matches("JEQ_END_").count();

            // Each comparison should generate labels
            prop_assert!(true_count >= comparison_count, "Should have at least {} TRUE labels", comparison_count);
            prop_assert!(end_count >= comparison_count, "Should have at least {} END labels", comparison_count);
        }
    }

    /// Test that static variables use correct filename prefix
    #[test]
    fn test_static_naming(index in 0u16..240) {
        let vm_code = format!("push static {}", index);
        let result = translate(&vm_code, "TestFile");

        if let Ok(asm) = result {
            prop_assert!(asm.contains(&format!("@TestFile.{}", index)),
                "Static variable should use filename prefix");
        }
    }

    // =========================================================================
    // Project 08 Specific Tests - Branching
    // =========================================================================

    /// Test that label commands work correctly
    #[test]
    fn test_label_commands(name in arb_label_name()) {
        let vm_code = format!("function Test.main 0\nlabel {}\nreturn", name);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Label command should succeed");
        if let Ok(asm) = result {
            prop_assert!(asm.contains(&format!("(Test.main${})", name)),
                "Should contain scoped label");
        }
    }

    /// Test that goto commands work correctly
    #[test]
    fn test_goto_commands(name in arb_label_name()) {
        let vm_code = format!("function Test.main 0\nlabel {}\ngoto {}\nreturn", name, name);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Goto command should succeed");
        if let Ok(asm) = result {
            prop_assert!(asm.contains(&format!("@Test.main${}\n0;JMP", name)),
                "Should contain goto instruction");
        }
    }

    /// Test that if-goto commands work correctly
    #[test]
    fn test_if_goto_commands(name in arb_label_name()) {
        let vm_code = format!("function Test.main 0\npush constant 1\nif-goto {}\nlabel {}\nreturn", name, name);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "If-goto command should succeed");
        if let Ok(asm) = result {
            prop_assert!(asm.contains(&format!("@Test.main${}\nD;JNE", name)),
                "Should contain if-goto instruction");
        }
    }

    // =========================================================================
    // Project 08 Specific Tests - Function Commands
    // =========================================================================

    /// Test that function declarations work with varying local counts
    #[test]
    fn test_function_locals(num_locals in 0u16..10) {
        let vm_code = format!("function Test.main {}\nreturn", num_locals);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Function with {} locals should succeed", num_locals);
        if let Ok(asm) = result {
            // Count local initializations
            let init_count = asm.matches("M=0\n@SP\nM=M+1").count();
            prop_assert_eq!(init_count, num_locals as usize, "Should initialize {} locals", num_locals);
        }
    }

    /// Test that call commands work with varying argument counts
    #[test]
    fn test_call_args(num_args in 0u16..10) {
        let vm_code = format!("function Test.main 0\ncall Other.func {}\nreturn\nfunction Other.func 0\nreturn", num_args);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Call with {} args should succeed", num_args);
        if let Ok(asm) = result {
            // ARG = SP - num_args - 5
            let expected_offset = num_args + 5;
            prop_assert!(asm.contains(&format!("@{}\nD=D-A\n@ARG\nM=D", expected_offset)),
                "Should calculate correct ARG offset for {} args", num_args);
        }
    }

    /// Test that return labels are unique across multiple calls
    #[test]
    fn test_return_label_uniqueness(call_count in 1usize..5) {
        let calls = (0..call_count).map(|_| "call Other.func 0").collect::<Vec<_>>().join("\n");
        let vm_code = format!("function Test.main 0\n{}\nreturn\nfunction Other.func 0\nreturn", calls);
        let result = translate(&vm_code, "Test");
        prop_assert!(result.is_ok(), "Multiple calls should succeed");
        if let Ok(asm) = result {
            // Check that each call has a unique return label
            for i in 0..call_count {
                prop_assert!(asm.contains(&format!("$ret.{}", i)),
                    "Should have return label {}", i);
            }
        }
    }

    /// Test that valid function bodies translate correctly
    #[test]
    fn test_valid_function_body(function_code in arb_valid_function()) {
        let result = translate(&function_code, "Test");
        prop_assert!(result.is_ok(), "Valid function body should succeed");
    }

    /// Test that return restores all segments correctly
    #[test]
    fn test_return_restores_segments(_seed in 0u64..1000) {
        let vm_code = "function Test.main 0\nreturn";
        let result = translate(vm_code, "Test");
        prop_assert!(result.is_ok(), "Return should succeed");
        if let Ok(asm) = result {
            // Verify return sequence restores all segments
            prop_assert!(asm.contains("@R13\nM=D"), "Should save frame to R13");
            prop_assert!(asm.contains("@R14\nM=D"), "Should save retAddr to R14");
            prop_assert!(asm.contains("@THAT\nM=D"), "Should restore THAT");
            prop_assert!(asm.contains("@THIS\nM=D"), "Should restore THIS");
            prop_assert!(asm.contains("@ARG\nM=D"), "Should restore ARG");
            prop_assert!(asm.contains("@LCL\nM=D"), "Should restore LCL");
            prop_assert!(asm.contains("@R14\nA=M\n0;JMP"), "Should jump to retAddr");
        }
    }
}
