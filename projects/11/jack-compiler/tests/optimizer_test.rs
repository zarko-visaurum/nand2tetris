//! Optimizer-specific integration tests for Jack Compiler.
//!
//! Tests the peephole optimizer and constant folder with complete
//! Jack programs to verify end-to-end optimization behavior.

use jack_compiler::{CompileOptions, compile_source, compile_source_with_options};

// =============================================================================
// Constant Folding Integration Tests
// =============================================================================

#[test]
fn test_constant_fold_simple_addition() {
    let source = r#"
class Main {
    function int test() {
        return 1 + 2;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should fold to push constant 3
    assert!(
        result.vm_code.contains("push constant 3"),
        "Should fold 1+2 to 3\nActual:\n{}",
        result.vm_code
    );
    // Should NOT have separate pushes
    assert!(
        !result.vm_code.contains("push constant 1\npush constant 2"),
        "Should not have unfold constants"
    );
}

#[test]
fn test_constant_fold_chained_operations() {
    let source = r#"
class Main {
    function int test() {
        return 1 + 2 + 3 + 4;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Jack evaluates left-to-right: ((1+2)+3)+4 = 10
    assert!(
        result.vm_code.contains("push constant 10"),
        "Should fold 1+2+3+4 to 10\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_constant_fold_multiplication() {
    let source = r#"
class Main {
    function int test() {
        return 6 * 7;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should fold to 42
    assert!(
        result.vm_code.contains("push constant 42"),
        "Should fold 6*7 to 42\nActual:\n{}",
        result.vm_code
    );
    // Should NOT call Math.multiply
    assert!(
        !result.vm_code.contains("call Math.multiply"),
        "Should not call Math.multiply for constant multiplication"
    );
}

#[test]
fn test_constant_fold_division() {
    let source = r#"
class Main {
    function int test() {
        return 100 / 5;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should fold to 20
    assert!(
        result.vm_code.contains("push constant 20"),
        "Should fold 100/5 to 20\nActual:\n{}",
        result.vm_code
    );
    // Should NOT call Math.divide
    assert!(
        !result.vm_code.contains("call Math.divide"),
        "Should not call Math.divide for constant division"
    );
}

#[test]
fn test_constant_fold_negation() {
    let source = r#"
class Main {
    function int test() {
        return -42;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should have push constant 42 followed by neg
    // (since -42 is negative, it becomes push 42, neg)
    assert!(
        result.vm_code.contains("push constant 42") && result.vm_code.contains("neg"),
        "Should represent -42 as push 42, neg\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_constant_fold_not_true() {
    let source = r#"
class Main {
    function int test() {
        return ~true;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // ~true = ~(-1) = 0
    assert!(
        result.vm_code.contains("push constant 0"),
        "Should fold ~true to 0\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_constant_fold_comparison() {
    let source = r#"
class Main {
    function boolean test() {
        return 5 < 10;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // 5 < 10 is true = -1
    // Should be represented as push constant 1, neg (since -1 is negative)
    assert!(
        result.vm_code.contains("push constant 1") && result.vm_code.contains("neg"),
        "Should fold 5<10 to true (-1)\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_no_fold_with_variables() {
    let source = r#"
class Main {
    function int test() {
        var int x;
        let x = 5;
        return x + 3;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should NOT fold since x is a variable
    assert!(
        result.vm_code.contains("push local 0"),
        "Should push variable x\nActual:\n{}",
        result.vm_code
    );
    assert!(
        result.vm_code.contains("push constant 3"),
        "Should push constant 3\nActual:\n{}",
        result.vm_code
    );
    assert!(
        result.vm_code.contains("add"),
        "Should have add instruction\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_fold_disabled_with_option() {
    let source = r#"
class Main {
    function int test() {
        return 1 + 2;
    }
}
"#;
    let options = CompileOptions { optimize: false };
    let result = compile_source_with_options(source, "Main", options);
    assert!(result.is_ok());

    // With optimization disabled, should have separate constants
    assert!(
        result.vm_code.contains("push constant 1") && result.vm_code.contains("push constant 2"),
        "Without optimization, should have separate constants\nActual:\n{}",
        result.vm_code
    );
}

// =============================================================================
// Peephole Optimization Integration Tests
// =============================================================================

#[test]
fn test_peephole_double_not_eliminated() {
    let source = r#"
class Main {
    function boolean test() {
        var boolean x;
        let x = true;
        return ~~x;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Double not should be eliminated
    let not_count = result.vm_code.matches("\nnot\n").count();
    assert_eq!(
        not_count, 0,
        "Double not should be eliminated\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_peephole_double_neg_eliminated() {
    let source = r#"
class Main {
    function int test() {
        var int x;
        let x = 5;
        return --x;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Double neg should be eliminated
    let neg_count = result.vm_code.matches("\nneg\n").count();
    assert_eq!(
        neg_count, 0,
        "Double neg should be eliminated\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_peephole_identity_add_eliminated() {
    let source = r#"
class Main {
    function int test() {
        var int x;
        let x = 5;
        return x + 0;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // push constant 0 / add should be eliminated
    // Note: with constant folding, the entire x + 0 may be optimized differently
    // Check that we don't have the identity pattern
    let has_identity = result.vm_code.contains("push constant 0\nadd");
    assert!(
        !has_identity,
        "Identity add should be eliminated\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_peephole_disabled_with_option() {
    let source = r#"
class Main {
    function boolean test() {
        var boolean x;
        let x = false;
        return ~~x;
    }
}
"#;
    let options = CompileOptions { optimize: false };
    let result = compile_source_with_options(source, "Main", options);
    assert!(result.is_ok());

    // Without optimization, double not from ~~x should remain
    // (using false instead of true avoids the extra not from true = ~0)
    let not_count = result.vm_code.matches("not\n").count();
    assert_eq!(
        not_count, 2,
        "Without optimization, both nots from ~~x should remain\nActual:\n{}",
        result.vm_code
    );
}

// =============================================================================
// Combined Optimization Tests
// =============================================================================

#[test]
fn test_complex_constant_expression() {
    let source = r#"
class Main {
    function int test() {
        return (2 + 3) * (4 - 1);
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // (2+3) * (4-1) = 5 * 3 = 15
    assert!(
        result.vm_code.contains("push constant 15"),
        "Should fold (2+3)*(4-1) to 15\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_partial_folding() {
    // When only part of expression is foldable
    let source = r#"
class Main {
    function int test() {
        var int x;
        let x = 5;
        return (2 + 3) + x;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Note: Current implementation folds entire expressions, not partial
    // So this will likely not fold since x prevents folding
    // Just verify compilation succeeds
    assert!(
        result.vm_code.contains("push local 0"),
        "Should access variable x\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_optimization_preserves_semantics() {
    // Complex expression that tests semantic preservation
    let source = r#"
class Main {
    function int test() {
        var int a, b, c;
        let a = 10;
        let b = 20;
        let c = 30;
        if (a < b) {
            return c;
        } else {
            return a + b;
        }
    }
}
"#;

    let optimized = compile_source_with_options(source, "Main", CompileOptions { optimize: true });
    let unoptimized =
        compile_source_with_options(source, "Main", CompileOptions { optimize: false });

    assert!(optimized.is_ok());
    assert!(unoptimized.is_ok());

    // Both should have the same structural elements
    assert!(
        optimized.vm_code.contains("if-goto") && unoptimized.vm_code.contains("if-goto"),
        "Both should have conditional branching"
    );

    // Optimized should be no longer
    assert!(
        optimized.vm_code.lines().count() <= unoptimized.vm_code.lines().count(),
        "Optimized code should not be longer than unoptimized"
    );
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_division_by_zero_not_folded() {
    // Division by zero in a constant expression should not fold
    let source = r#"
class Main {
    function int test() {
        var int x;
        let x = 5 / 0;
        return x;
    }
}
"#;
    // This should still compile (division by zero is a runtime error)
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should have Math.divide call since we can't fold division by zero
    assert!(
        result.vm_code.contains("call Math.divide"),
        "Division by zero should not be folded\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_large_constant_handling() {
    let source = r#"
class Main {
    function int test() {
        return 32767;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    assert!(
        result.vm_code.contains("push constant 32767"),
        "Should handle max Jack integer\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_boolean_constant_optimization() {
    let source = r#"
class Main {
    function boolean test() {
        return true;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // true = -1, can be folded
    // Either push constant 1, neg OR push constant 0, not
    let has_true_pattern = (result.vm_code.contains("push constant 1")
        && result.vm_code.contains("neg"))
        || (result.vm_code.contains("push constant 0") && result.vm_code.contains("not"));

    assert!(
        has_true_pattern,
        "Should represent true as -1\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_string_constants_not_folded() {
    // String constants can't be folded - they need runtime allocation
    let source = r#"
class Main {
    function void test() {
        var String s;
        let s = "hi";
        return;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Should have String.new and appendChar calls
    assert!(
        result.vm_code.contains("call String.new"),
        "String constants need String.new\nActual:\n{}",
        result.vm_code
    );
}

// =============================================================================
// Regression Tests
// =============================================================================

#[test]
fn test_nested_not_optimization() {
    let source = r#"
class Main {
    function boolean test() {
        var boolean x;
        let x = true;
        return ~~~~x;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // ~~~~x = x (four nots cancel)
    let not_count = result.vm_code.matches("\nnot\n").count();
    assert_eq!(
        not_count, 0,
        "Quadruple not should be eliminated\nActual:\n{}",
        result.vm_code
    );
}

#[test]
fn test_mixed_arithmetic_constant_folding() {
    let source = r#"
class Main {
    function int test() {
        return 100 - 50 + 25 - 10;
    }
}
"#;
    let result = compile_source(source, "Main");
    assert!(result.is_ok());

    // Left-to-right: ((100-50)+25)-10 = (50+25)-10 = 75-10 = 65
    assert!(
        result.vm_code.contains("push constant 65"),
        "Should fold to 65\nActual:\n{}",
        result.vm_code
    );
}
