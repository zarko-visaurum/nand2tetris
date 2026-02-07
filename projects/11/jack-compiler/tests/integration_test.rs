//! Integration tests for Jack Compiler (Project 11).
//!
//! Tests all 6 nand2tetris test programs with VM output verification.
//! Follows the automated testing pattern from previous projects.

use jack_compiler::{
    CompileOptions, compile_directory, compile_source, compile_source_with_options,
};
use std::fs;
use std::path::Path;

// =============================================================================
// Helper Functions
// =============================================================================

/// Compile a directory and verify expected/forbidden patterns in the output.
/// Patterns are checked across ALL files (any file can contain the pattern).
fn compile_and_verify(dir_name: &str, expected_patterns: &[&str], forbidden_patterns: &[&str]) {
    let dir_path = Path::new("..").join(dir_name);
    let results = compile_directory(&dir_path);

    assert!(
        !results.is_empty(),
        "{} should contain Jack files",
        dir_name
    );

    // Combine all VM code for pattern matching
    let mut combined_vm = String::new();
    for result in &results {
        assert!(
            result.is_ok(),
            "Compilation failed for {}: {:?}",
            result.filename,
            result.errors
        );
        combined_vm.push_str(&result.vm_code);
    }

    // Verify expected patterns exist somewhere in the output
    for pattern in expected_patterns {
        assert!(
            combined_vm.contains(pattern),
            "{} output should contain '{}'\nCombined output preview:\n{}",
            dir_name,
            pattern,
            &combined_vm[..combined_vm.len().min(1000)]
        );
    }

    // Verify forbidden patterns don't appear anywhere
    for pattern in forbidden_patterns {
        assert!(
            !combined_vm.contains(pattern),
            "{} output should NOT contain '{}' (optimization failed)",
            dir_name,
            pattern
        );
    }

    // Write output files
    for result in &results {
        let vm_path = dir_path.join(format!("{}.vm", result.filename));
        fs::write(&vm_path, &result.vm_code)
            .unwrap_or_else(|_| panic!("Failed to write {}", vm_path.display()));
    }
}

// =============================================================================
// Test 1: Seven - Simple arithmetic and function calls
// =============================================================================

#[test]
fn test_seven() {
    let source = r#"
class Main {
    function void main() {
        do Output.printInt(1 + (2 * 3));
        return;
    }
}
"#;

    let result = compile_source(source, "Main");
    assert!(
        result.is_ok(),
        "Seven compilation failed: {:?}",
        result.errors
    );

    let vm = &result.vm_code;

    // Verify function structure
    assert!(
        vm.contains("function Main.main 0"),
        "Should declare Main.main with 0 locals"
    );

    // With constant folding, 1 + (2 * 3) = 7 gets folded
    assert!(
        vm.contains("push constant 7"),
        "Constant folding should fold 1+(2*3) to 7"
    );

    // Verify Output.printInt call
    assert!(
        vm.contains("call Output.printInt 1"),
        "Should call Output.printInt"
    );

    // Verify void return
    assert!(
        vm.contains("push constant 0"),
        "Should push 0 for void return"
    );
    assert!(vm.contains("return"), "Should return");
}

#[test]
fn test_seven_directory() {
    // The actual Seven directory program has the constant expression folded
    compile_and_verify(
        "Seven",
        &[
            "function Main.main 0",
            "push constant 7", // Constant folding folds 1+(2*3) to 7
            "call Output.printInt 1",
            "return",
        ],
        &[],
    );
}

// =============================================================================
// Test 2: ConvertToBin - Procedural features (loops, conditionals, functions)
// =============================================================================

#[test]
fn test_convert_to_bin() {
    compile_and_verify(
        "ConvertToBin",
        &[
            "function Main.main",
            "label WHILE",
            "if-goto",
            "goto",
            "call Main.convert",
            "return",
        ],
        &[],
    );
}

#[test]
fn test_convert_to_bin_while_loop_structure() {
    let dir_path = Path::new("../ConvertToBin");
    let results = compile_directory(dir_path);
    let main_result = results.iter().find(|r| r.filename == "Main").unwrap();

    let vm = &main_result.vm_code;

    // Verify while loop pattern: label, condition, not, if-goto, body, goto
    let label_pos = vm.find("label WHILE").expect("Should have WHILE label");
    let if_goto_pos = vm.find("if-goto WHILE").expect("Should have if-goto WHILE");
    let goto_pos = vm.rfind("goto WHILE").expect("Should have goto WHILE");

    assert!(label_pos < if_goto_pos, "Label should come before if-goto");
    assert!(
        if_goto_pos < goto_pos,
        "if-goto END should come before goto LOOP"
    );
}

// =============================================================================
// Test 3: Square - OOP (constructors, methods, fields)
// =============================================================================

#[test]
fn test_square() {
    compile_and_verify(
        "Square",
        &[
            // Constructor pattern
            "function Square.new",
            "call Memory.alloc 1",
            "pop pointer 0",
            // Method pattern
            "function Square.draw",
            "push argument 0",
            "pop pointer 0",
            // Field access (this segment)
            "push this",
            "pop this",
            // Return this from constructor
            "push pointer 0",
        ],
        &[],
    );
}

#[test]
fn test_square_constructor_field_count() {
    let dir_path = Path::new("../Square");
    let results = compile_directory(dir_path);
    let square_result = results.iter().find(|r| r.filename == "Square").unwrap();

    // Square has 3 fields: x, y, size
    // Constructor should allocate 3 words
    assert!(
        square_result
            .vm_code
            .contains("push constant 3\ncall Memory.alloc 1"),
        "Square constructor should allocate 3 fields"
    );
}

#[test]
fn test_square_method_this_setup() {
    let dir_path = Path::new("../Square");
    let results = compile_directory(dir_path);
    let square_result = results.iter().find(|r| r.filename == "Square").unwrap();

    // Every method should start with: push argument 0, pop pointer 0
    let method_count = square_result
        .vm_code
        .matches("push argument 0\npop pointer 0")
        .count();

    // Square has multiple methods (draw, erase, incSize, decSize, etc.)
    assert!(
        method_count >= 4,
        "Should have at least 4 methods setting up 'this', found {}",
        method_count
    );
}

// =============================================================================
// Test 4: Average - Arrays and strings
// =============================================================================

#[test]
fn test_average() {
    compile_and_verify(
        "Average",
        &[
            // Array creation
            "call Array.new 1",
            // Array access pattern
            "pop pointer 1",
            "push that 0",
            "pop that 0",
            // String creation
            "call String.new 1",
            "call String.appendChar 2",
            // Keyboard input
            "call Keyboard.readInt 1",
        ],
        &[],
    );
}

#[test]
fn test_average_array_write_pattern() {
    let dir_path = Path::new("../Average");
    let results = compile_directory(dir_path);
    let main_result = results.iter().find(|r| r.filename == "Main").unwrap();

    let vm = &main_result.vm_code;

    // Array write: push base, push index, add, push value, pop temp 0, pop pointer 1, push temp 0, pop that 0
    assert!(vm.contains("pop temp 0"), "Array write should use temp 0");
    assert!(vm.contains("pop pointer 1"), "Array write should set THAT");
    assert!(
        vm.contains("pop that 0"),
        "Array write should store via THAT"
    );
}

// =============================================================================
// Test 5: Pong - Complete OOP with static variables
// =============================================================================

#[test]
fn test_pong() {
    compile_and_verify(
        "Pong",
        &[
            // Static variable access
            "push static",
            "pop static",
            // Multiple classes
            "function Ball.",
            "function Bat.",
            "function PongGame.",
            "function Main.main",
            // Method calls
            "call Ball.",
            "call Bat.",
        ],
        &[],
    );
}

#[test]
fn test_pong_static_variables() {
    let dir_path = Path::new("../Pong");
    let results = compile_directory(dir_path);

    // PongGame has static variable 'instance'
    let pong_game = results.iter().find(|r| r.filename == "PongGame").unwrap();
    assert!(
        pong_game.vm_code.contains("pop static 0") || pong_game.vm_code.contains("push static 0"),
        "PongGame should access static variables"
    );
}

// =============================================================================
// Test 6: ComplexArrays - Nested array access
// =============================================================================

#[test]
fn test_complex_arrays() {
    compile_and_verify(
        "ComplexArrays",
        &[
            "function Main.main",
            "function Main.double",
            "function Main.fill",
            // Nested array access patterns
            "pop pointer 1",
            "push that 0",
            "pop that 0",
        ],
        &[],
    );
}

#[test]
fn test_complex_arrays_nested_access() {
    let dir_path = Path::new("../ComplexArrays");
    let results = compile_directory(dir_path);
    let main_result = results.iter().find(|r| r.filename == "Main").unwrap();

    let vm = &main_result.vm_code;

    // a[b[a[3]]] requires multiple pointer 1 / that 0 sequences
    let pointer_1_count = vm.matches("pop pointer 1").count();
    assert!(
        pointer_1_count >= 5,
        "Should have at least 5 array accesses (found {})",
        pointer_1_count
    );

    // Verify temp 0 usage for array writes
    let temp_0_count = vm.matches("pop temp 0").count();
    assert!(temp_0_count >= 3, "Should use temp 0 for array writes");
}

// =============================================================================
// Optimization Tests
// =============================================================================

#[test]
fn test_peephole_double_not_optimized() {
    let source = r#"
class Main {
    function void main() {
        var int x;
        let x = ~~5;
        return;
    }
}
"#;

    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Double not should be eliminated
    let not_count = result.vm_code.matches("\nnot\n").count();
    assert_eq!(
        not_count, 0,
        "Double not should be eliminated, found {} 'not' commands",
        not_count
    );
}

#[test]
fn test_peephole_double_neg_optimized() {
    let source = r#"
class Main {
    function void main() {
        var int x;
        let x = --5;
        return;
    }
}
"#;

    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Double neg should be eliminated
    let neg_count = result.vm_code.matches("\nneg\n").count();
    assert_eq!(
        neg_count, 0,
        "Double neg should be eliminated, found {} 'neg' commands",
        neg_count
    );
}

#[test]
fn test_optimization_disabled() {
    let source = r#"
class Main {
    function void main() {
        var int x;
        let x = 1 + 2;
        return;
    }
}
"#;

    let options = CompileOptions { optimize: false };
    let result = compile_source_with_options(source, "Main", options);
    assert!(result.is_ok());

    // Without optimization, should have separate pushes
    assert!(
        result.vm_code.contains("push constant 1") && result.vm_code.contains("push constant 2"),
        "Without optimization, should have separate constant pushes"
    );
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[test]
fn test_undefined_variable_error() {
    let source = r#"
class Main {
    function void main() {
        let x = 5;
        return;
    }
}
"#;

    let result = compile_source(source, "Main");
    assert!(!result.is_ok());
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.to_string().contains("Undefined")),
        "Should report undefined variable error"
    );
}

#[test]
fn test_all_test_programs_compile_successfully() {
    let test_dirs = [
        "Seven",
        "ConvertToBin",
        "Square",
        "Average",
        "Pong",
        "ComplexArrays",
    ];

    for dir in &test_dirs {
        let dir_path = Path::new("..").join(dir);
        if !dir_path.exists() {
            continue; // Skip if test directory doesn't exist
        }

        let results = compile_directory(&dir_path);
        assert!(!results.is_empty(), "{} should have Jack files", dir);

        for result in &results {
            assert!(
                result.is_ok(),
                "{}/{}.jack compilation failed: {:?}",
                dir,
                result.filename,
                result.errors
            );
            assert!(
                !result.vm_code.is_empty(),
                "{}/{}.jack produced empty output",
                dir,
                result.filename
            );

            // Write output for manual verification
            let vm_path = dir_path.join(format!("{}.vm", result.filename));
            fs::write(&vm_path, &result.vm_code).expect("Failed to write VM output");
        }
    }
}
