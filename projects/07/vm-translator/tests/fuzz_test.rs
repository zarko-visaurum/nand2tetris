use proptest::prelude::*;
use vm_translator::translate;

/// Generate arbitrary VM-like commands for fuzzing
fn arb_vm_line() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid arithmetic commands
        Just("add".to_string()),
        Just("sub".to_string()),
        Just("neg".to_string()),
        Just("eq".to_string()),
        Just("lt".to_string()),
        Just("gt".to_string()),
        Just("and".to_string()),
        Just("or".to_string()),
        Just("not".to_string()),
        // Valid push commands with various segments
        (0u16..32768).prop_map(|n| format!("push constant {}", n)),
        (0u16..8).prop_map(|n| format!("push temp {}", n)),
        (0u16..2).prop_map(|n| format!("push pointer {}", n)),
        (0u16..100).prop_map(|n| format!("push local {}", n)),
        (0u16..100).prop_map(|n| format!("push argument {}", n)),
        (0u16..100).prop_map(|n| format!("push this {}", n)),
        (0u16..100).prop_map(|n| format!("push that {}", n)),
        (0u16..240).prop_map(|n| format!("push static {}", n)),
        // Valid pop commands
        (0u16..8).prop_map(|n| format!("pop temp {}", n)),
        (0u16..2).prop_map(|n| format!("pop pointer {}", n)),
        (0u16..100).prop_map(|n| format!("pop local {}", n)),
        (0u16..100).prop_map(|n| format!("pop argument {}", n)),
        (0u16..100).prop_map(|n| format!("pop this {}", n)),
        (0u16..100).prop_map(|n| format!("pop that {}", n)),
        (0u16..240).prop_map(|n| format!("pop static {}", n)),
        // Comments and empty lines
        Just("// This is a comment".to_string()),
        Just("".to_string()),
        Just("   ".to_string()),
        // Invalid commands (should produce errors, not panics)
        "[a-z]{3,10}".prop_map(|s| s),
        "push [a-z]+ [0-9]+".prop_map(|s| s),
        "[A-Z]+".prop_map(|s| s),
    ]
}

fn arb_vm_program() -> impl Strategy<Value = String> {
    prop::collection::vec(arb_vm_line(), 0..50).prop_map(|lines| lines.join("\n"))
}

proptest! {
    /// Test that translator never panics on arbitrary input
    #[test]
    fn test_no_panic_on_arbitrary_input(input in arb_vm_program()) {
        let _ = translate(&input, "Test");
    }

    /// Test that valid arithmetic commands always succeed
    #[test]
    fn test_valid_arithmetic_succeeds(
        op in prop_oneof![
            Just("add"), Just("sub"), Just("neg"),
            Just("eq"), Just("lt"), Just("gt"),
            Just("and"), Just("or"), Just("not")
        ]
    ) {
        let result = translate(op, "Test");
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
            // Each comparison generates 2 labels (TRUE_n and END_n+1)
            // Total label references should be 2 * comparison_count
            let true_count = asm.matches("JEQ_TRUE_").count();
            let end_count = asm.matches("JEQ_END_").count();

            // Each comparison should generate labels
            prop_assert!(true_count >= comparison_count, "Should have at least {} TRUE labels", comparison_count);
            prop_assert!(end_count >= comparison_count, "Should have at least {} END labels", comparison_count);

            // Verify labels are unique by checking if we have different numbers
            if comparison_count > 1 {
                prop_assert!(asm.contains("JEQ_TRUE_0"), "Should have label 0");
                prop_assert!(asm.contains("JEQ_TRUE_2") || asm.contains("JEQ_END_3"),
                    "Should have labels with different numbers");
            }
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
}
