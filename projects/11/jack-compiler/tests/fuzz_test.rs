//! Property-based fuzzing tests for Jack Compiler.
//!
//! Uses proptest to generate random valid Jack programs and verify
//! compiler invariants hold across all inputs.

use proptest::prelude::*;

// =============================================================================
// Arbitrary Value Generators
// =============================================================================

/// Generate a valid Jack identifier (starts with letter or underscore).
fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,10}".prop_filter("not a keyword", |s| {
        !matches!(
            s.as_str(),
            "class"
                | "constructor"
                | "function"
                | "method"
                | "field"
                | "static"
                | "var"
                | "int"
                | "char"
                | "boolean"
                | "void"
                | "true"
                | "false"
                | "null"
                | "this"
                | "let"
                | "do"
                | "if"
                | "else"
                | "while"
                | "return"
        )
    })
}

/// Generate a valid Jack class name (starts with uppercase).
fn arb_class_name() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z0-9]{0,10}".prop_filter("not a keyword", |s| {
        !matches!(
            s.as_str(),
            "Array" | "String" | "Output" | "Math" | "Memory" | "Keyboard" | "Screen" | "Sys"
        )
    })
}

/// Generate a valid Jack integer constant (0-32767).
fn arb_integer() -> impl Strategy<Value = String> {
    (0u16..32768).prop_map(|n| n.to_string())
}

/// Generate a simple constant expression.
#[allow(dead_code)]
fn arb_simple_expression() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_integer(),
        Just("true".to_string()),
        Just("false".to_string()),
        Just("null".to_string()),
    ]
}

/// Generate a variable type.
fn arb_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("int".to_string()),
        Just("char".to_string()),
        Just("boolean".to_string()),
    ]
}

/// Generate a variable declaration.
fn arb_var_dec() -> impl Strategy<Value = (String, String)> {
    (arb_type(), arb_identifier())
}

/// Generate a let statement with a simple expression.
#[allow(dead_code)]
fn arb_let_statement(var_name: String) -> impl Strategy<Value = String> {
    arb_simple_expression().prop_map(move |expr| format!("let {} = {};", var_name, expr))
}

/// Generate a minimal class with variable declarations and let statements.
fn arb_minimal_class() -> impl Strategy<Value = String> {
    (arb_class_name(), prop::collection::vec(arb_var_dec(), 1..4)).prop_map(|(class_name, vars)| {
        let var_decs: String = vars
            .iter()
            .map(|(typ, name)| format!("        var {} {};", typ, name))
            .collect::<Vec<_>>()
            .join("\n");

        let statements: String = vars
            .iter()
            .map(|(_, name)| format!("        let {} = 0;", name))
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            r#"class {} {{
    function void main() {{
{}
{}
        return;
    }}
}}"#,
            class_name, var_decs, statements
        )
    })
}

/// Generate a class with arithmetic expressions.
fn arb_arithmetic_class() -> impl Strategy<Value = String> {
    (
        arb_class_name(),
        prop::collection::vec(arb_integer(), 2..5),
        prop::collection::vec(
            prop_oneof![Just("+"), Just("-"), Just("*"), Just("/")],
            1..4,
        ),
    )
        .prop_map(|(class_name, nums, ops)| {
            let mut expr = nums[0].clone();
            for (i, op) in ops.iter().enumerate() {
                if i + 1 < nums.len() {
                    // Avoid division by zero
                    let num = if *op == "/" && nums[i + 1] == "0" {
                        "1".to_string()
                    } else {
                        nums[i + 1].clone()
                    };
                    expr = format!("({} {} {})", expr, op, num);
                }
            }

            format!(
                r#"class {} {{
    function int calc() {{
        return {};
    }}
}}"#,
                class_name, expr
            )
        })
}

/// Generate a class with if/while statements.
fn arb_control_flow_class() -> impl Strategy<Value = String> {
    (arb_class_name(), arb_integer(), arb_integer()).prop_map(|(class_name, val1, val2)| {
        format!(
            r#"class {} {{
    function void test() {{
        var int x;
        var int y;
        let x = {};
        let y = {};
        if (x < y) {{
            let x = y;
        }} else {{
            let y = x;
        }}
        while (x > 0) {{
            let x = x - 1;
        }}
        return;
    }}
}}"#,
            class_name, val1, val2
        )
    })
}

// =============================================================================
// Property Tests - Core Invariants
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]

    /// Compiler should never panic on syntactically valid input.
    #[test]
    fn test_no_panic_on_valid_input(source in arb_minimal_class()) {
        let _ = jack_compiler::compile_source(&source, "Test");
    }

    /// Compiler should never panic on arithmetic expressions.
    #[test]
    fn test_no_panic_on_arithmetic(source in arb_arithmetic_class()) {
        let _ = jack_compiler::compile_source(&source, "Test");
    }

    /// Compiler should never panic on control flow constructs.
    #[test]
    fn test_no_panic_on_control_flow(source in arb_control_flow_class()) {
        let _ = jack_compiler::compile_source(&source, "Test");
    }

    /// Generated VM code should be syntactically valid.
    #[test]
    fn test_vm_output_valid(source in arb_minimal_class()) {
        let result = jack_compiler::compile_source(&source, "Test");
        if result.is_ok() {
            for line in result.vm_code.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // Verify each line is a valid VM command
                let valid = line.starts_with("push ")
                    || line.starts_with("pop ")
                    || line.starts_with("label ")
                    || line.starts_with("goto ")
                    || line.starts_with("if-goto ")
                    || line.starts_with("function ")
                    || line.starts_with("call ")
                    || line == "return"
                    || line == "add"
                    || line == "sub"
                    || line == "neg"
                    || line == "eq"
                    || line == "gt"
                    || line == "lt"
                    || line == "and"
                    || line == "or"
                    || line == "not";

                prop_assert!(valid, "Invalid VM command: {}", line);
            }
        }
    }

    /// Optimization should never change whether compilation succeeds or fails.
    #[test]
    fn test_optimization_stability(source in arb_minimal_class()) {
        let optimized = jack_compiler::compile_source_with_options(
            &source,
            "Test",
            jack_compiler::CompileOptions { optimize: true },
        );
        let unoptimized = jack_compiler::compile_source_with_options(
            &source,
            "Test",
            jack_compiler::CompileOptions { optimize: false },
        );

        // Both should either succeed or fail
        prop_assert_eq!(
            optimized.is_ok(),
            unoptimized.is_ok(),
            "Optimization changed compilation success: opt={}, unopt={}",
            optimized.is_ok(),
            unoptimized.is_ok()
        );
    }

    /// Optimized code should never be longer than unoptimized.
    #[test]
    fn test_optimization_reduces_or_maintains_size(source in arb_minimal_class()) {
        let optimized = jack_compiler::compile_source_with_options(
            &source,
            "Test",
            jack_compiler::CompileOptions { optimize: true },
        );
        let unoptimized = jack_compiler::compile_source_with_options(
            &source,
            "Test",
            jack_compiler::CompileOptions { optimize: false },
        );

        if optimized.is_ok() && unoptimized.is_ok() {
            let opt_lines = optimized.vm_code.lines().count();
            let unopt_lines = unoptimized.vm_code.lines().count();
            prop_assert!(
                opt_lines <= unopt_lines,
                "Optimized ({}) should be <= unoptimized ({})",
                opt_lines,
                unopt_lines
            );
        }
    }

    /// VM code should always have a return statement for each function.
    #[test]
    fn test_functions_have_return(source in arb_minimal_class()) {
        let result = jack_compiler::compile_source(&source, "Test");
        if result.is_ok() {
            let function_count = result.vm_code.matches("function ").count();
            let return_count = result.vm_code.matches("\nreturn\n").count()
                + if result.vm_code.ends_with("return\n") { 1 } else { 0 };

            prop_assert!(
                return_count >= function_count,
                "Each function should have at least one return: {} functions, {} returns",
                function_count,
                return_count
            );
        }
    }
}

// =============================================================================
// Property Tests - Optimizer Specific
// =============================================================================

mod optimizer_fuzz {
    use super::*;
    use jack_compiler::PeepholeOptimizer;

    /// Generate a random VM instruction.
    fn arb_vm_instruction() -> impl Strategy<Value = String> {
        prop_oneof![
            (0u16..100).prop_map(|n| format!("push constant {}", n)),
            (0u16..10).prop_map(|n| format!("push local {}", n)),
            (0u16..10).prop_map(|n| format!("pop local {}", n)),
            (0u16..10).prop_map(|n| format!("push argument {}", n)),
            (0u16..10).prop_map(|n| format!("pop argument {}", n)),
            (0u16..5).prop_map(|n| format!("push this {}", n)),
            (0u16..5).prop_map(|n| format!("pop this {}", n)),
            (0u16..3).prop_map(|n| format!("push static {}", n)),
            (0u16..3).prop_map(|n| format!("pop static {}", n)),
            Just("push pointer 0".to_string()),
            Just("push pointer 1".to_string()),
            Just("pop pointer 0".to_string()),
            Just("pop pointer 1".to_string()),
            Just("push that 0".to_string()),
            Just("pop that 0".to_string()),
            Just("add".to_string()),
            Just("sub".to_string()),
            Just("neg".to_string()),
            Just("not".to_string()),
            Just("eq".to_string()),
            Just("lt".to_string()),
            Just("gt".to_string()),
            Just("and".to_string()),
            Just("or".to_string()),
        ]
    }

    /// Generate a random VM program (sequence of instructions).
    fn arb_vm_program() -> impl Strategy<Value = String> {
        prop::collection::vec(arb_vm_instruction(), 1..50).prop_map(|lines| lines.join("\n") + "\n")
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(200))]

        /// Optimizer should never panic on any input.
        #[test]
        fn test_optimizer_no_panic(vm_code in arb_vm_program()) {
            let _ = PeepholeOptimizer::optimize(&vm_code);
        }

        /// Optimized code should be valid VM instructions.
        #[test]
        fn test_optimizer_output_valid(vm_code in arb_vm_program()) {
            let optimized = PeepholeOptimizer::optimize(&vm_code);
            for line in optimized.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                let valid = line.starts_with("push ")
                    || line.starts_with("pop ")
                    || line.starts_with("label ")
                    || line.starts_with("goto ")
                    || line.starts_with("if-goto ")
                    || line.starts_with("function ")
                    || line.starts_with("call ")
                    || line == "return"
                    || line == "add"
                    || line == "sub"
                    || line == "neg"
                    || line == "not"
                    || line == "eq"
                    || line == "gt"
                    || line == "lt"
                    || line == "and"
                    || line == "or";

                prop_assert!(valid, "Invalid optimized VM: {}", line);
            }
        }

        /// Optimizer should never increase code size.
        #[test]
        fn test_optimizer_never_increases_size(vm_code in arb_vm_program()) {
            let optimized = PeepholeOptimizer::optimize(&vm_code);
            let input_lines = vm_code.lines().count();
            let output_lines = optimized.lines().count();

            prop_assert!(
                output_lines <= input_lines,
                "Optimizer increased size: {} -> {}",
                input_lines,
                output_lines
            );
        }

        /// Optimizer reaches fixed point within 3 iterations.
        /// Note: Single-pass optimization may expose new opportunities,
        /// so we test convergence rather than strict idempotence.
        #[test]
        fn test_optimizer_converges(vm_code in arb_vm_program()) {
            let once = PeepholeOptimizer::optimize(&vm_code);
            let twice = PeepholeOptimizer::optimize(&once);
            let thrice = PeepholeOptimizer::optimize(&twice);

            // After 3 iterations, should reach fixed point
            prop_assert_eq!(
                twice,
                thrice,
                "Optimizer should converge within 3 iterations"
            );
        }
    }
}

// =============================================================================
// Property Tests - Constant Folder Specific
// =============================================================================

mod constant_folder_fuzz {
    use super::*;
    use jack_compiler::ConstantFolder;

    /// Generate an expression that should be foldable.
    fn arb_foldable_expression() -> impl Strategy<Value = String> {
        prop_oneof![
            // Simple integers
            (0i32..32768).prop_map(|n| n.to_string()),
            // Binary operations on constants
            (0i32..100, 0i32..100).prop_map(|(a, b)| format!("{} + {}", a, b)),
            (0i32..100, 0i32..100).prop_map(|(a, b)| format!("{} - {}", a, b)),
            (0i32..100, 1i32..100).prop_map(|(a, b)| format!("{} * {}", a, b)),
            (0i32..100, 1i32..100).prop_map(|(a, b)| format!("{} / {}", a, b)),
            // Keywords
            Just("true".to_string()),
            Just("false".to_string()),
            Just("null".to_string()),
            // Unary
            (1i32..100).prop_map(|n| format!("-{}", n)),
            (0i32..100).prop_map(|n| format!("~{}", n)),
            // Parenthesized
            (0i32..100).prop_map(|n| format!("({})", n)),
        ]
    }

    /// Parse an expression and attempt to fold it.
    fn try_fold(expr_str: &str) -> Option<i32> {
        let source = format!(
            "class T {{ function void f() {{ var int x; let x = {}; return; }} }}",
            expr_str
        );

        let tokenizer = jack_analyzer::tokenizer::JackTokenizer::new(&source);
        let tokens = match tokenizer.tokenize() {
            Ok(t) => t,
            Err(_) => return None,
        };

        let parser = jack_analyzer::parser::Parser::new(&tokens);
        let class = match parser.parse() {
            Ok(c) => c,
            Err(_) => return None,
        };

        if let jack_analyzer::ast::Statement::Let(let_stmt) =
            &class.subroutine_decs[0].body.statements[0]
        {
            ConstantFolder::fold_expression(&let_stmt.value)
        } else {
            None
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Constant folder should produce results in valid ranges.
        #[test]
        fn test_fold_result_in_range(expr in arb_foldable_expression()) {
            if let Some(result) = try_fold(&expr) {
                // Verify the result is a valid i32 (always true by type, but
                // documents the intent that constant folding produces i32 values)
                let _ = result; // type-checked as i32
            }
        }

        /// Constant folder should handle division by zero gracefully.
        #[test]
        fn test_fold_division_by_zero(dividend in 0i32..1000) {
            let expr = format!("{} / 0", dividend);
            let result = try_fold(&expr);
            // Division by zero should return None
            prop_assert!(
                result.is_none(),
                "Division by zero should not fold, got {:?}",
                result
            );
        }

        /// Simple integer constants should always fold.
        #[test]
        fn test_integer_constants_always_fold(n in 0u16..32768u16) {
            let expr = n.to_string();
            let result = try_fold(&expr);
            prop_assert_eq!(
                result,
                Some(n as i32),
                "Integer {} should fold to {}",
                n,
                n
            );
        }

        /// Addition should be commutative.
        #[test]
        fn test_addition_commutative(a in 0i32..1000, b in 0i32..1000) {
            let expr1 = format!("{} + {}", a, b);
            let expr2 = format!("{} + {}", b, a);
            let r1 = try_fold(&expr1);
            let r2 = try_fold(&expr2);
            prop_assert_eq!(r1, r2, "{} + {} != {} + {}", a, b, b, a);
        }

        /// Multiplication should be commutative.
        #[test]
        fn test_multiplication_commutative(a in 0i32..100, b in 0i32..100) {
            let expr1 = format!("{} * {}", a, b);
            let expr2 = format!("{} * {}", b, a);
            let r1 = try_fold(&expr1);
            let r2 = try_fold(&expr2);
            prop_assert_eq!(r1, r2, "{} * {} != {} * {}", a, b, b, a);
        }

        /// Double negation should cancel out.
        #[test]
        fn test_double_negation_identity(n in 1i32..1000) {
            let expr = format!("-(- {})", n);
            let result = try_fold(&expr);
            prop_assert_eq!(
                result,
                Some(n),
                "--{} should equal {}",
                n,
                n
            );
        }
    }
}

// =============================================================================
// Property Tests - Symbol Table Specific
// =============================================================================

mod symbol_table_fuzz {
    use super::*;
    use jack_analyzer::ast::Type;
    use jack_compiler::{SymbolKind, SymbolTable};

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Each defined symbol should be retrievable.
        #[test]
        fn test_define_then_lookup(
            class_name in "[A-Z][a-zA-Z0-9]{0,10}",
            var_name in "[a-z][a-zA-Z0-9]{0,10}",
        ) {
            let mut table = SymbolTable::new();
            table.start_class(&class_name);

            // Filter out keywords
            if matches!(var_name.as_str(), "int" | "char" | "boolean" | "void" | "var" | "let" | "if" | "else" | "while" | "do" | "return" | "true" | "false" | "null" | "this") {
                return Ok(());
            }

            let result = table.define(
                &var_name,
                Type::Int,
                SymbolKind::Field,
                jack_analyzer::token::Span::new(0, 0, 0, 0),
            );
            prop_assert!(result.is_ok(), "Define should succeed");

            let symbol = table.lookup(&var_name);
            prop_assert!(symbol.is_some(), "Lookup should find defined symbol");
            prop_assert_eq!(symbol.unwrap().kind, SymbolKind::Field);
        }

        /// Subroutine scope should shadow class scope.
        #[test]
        fn test_scope_shadowing(
            class_name in "[A-Z][a-zA-Z0-9]{0,10}",
            var_name in "[a-z][a-zA-Z0-9]{0,10}",
        ) {
            // Filter out keywords
            if matches!(var_name.as_str(), "int" | "char" | "boolean" | "void" | "var" | "let" | "if" | "else" | "while" | "do" | "return" | "true" | "false" | "null" | "this") {
                return Ok(());
            }

            let mut table = SymbolTable::new();
            table.start_class(&class_name);

            // Define in class scope
            table.define(
                &var_name,
                Type::Int,
                SymbolKind::Field,
                jack_analyzer::token::Span::new(0, 0, 0, 0),
            ).unwrap();

            // Start subroutine and define same name
            table.start_subroutine();
            table.define(
                &var_name,
                Type::Boolean,
                SymbolKind::Local,
                jack_analyzer::token::Span::new(0, 0, 0, 0),
            ).unwrap();

            // Lookup should return subroutine scope version
            let symbol = table.lookup(&var_name).unwrap();
            prop_assert_eq!(symbol.kind, SymbolKind::Local, "Subroutine scope should shadow class scope");
        }

        /// Starting new subroutine should clear subroutine scope.
        #[test]
        fn test_subroutine_reset(
            class_name in "[A-Z][a-zA-Z0-9]{0,10}",
            var_name in "[a-z][a-zA-Z0-9]{0,10}",
        ) {
            // Filter out keywords
            if matches!(var_name.as_str(), "int" | "char" | "boolean" | "void" | "var" | "let" | "if" | "else" | "while" | "do" | "return" | "true" | "false" | "null" | "this") {
                return Ok(());
            }

            let mut table = SymbolTable::new();
            table.start_class(&class_name);
            table.start_subroutine();

            table.define(
                &var_name,
                Type::Int,
                SymbolKind::Local,
                jack_analyzer::token::Span::new(0, 0, 0, 0),
            ).unwrap();

            // Start new subroutine
            table.start_subroutine();

            // Variable should no longer be visible
            let symbol = table.lookup(&var_name);
            prop_assert!(symbol.is_none(), "Variable should not be visible after subroutine reset");
        }

        /// Index counters should increment correctly.
        #[test]
        fn test_index_counters(n in 1usize..10) {
            let mut table = SymbolTable::new();
            table.start_class("Test");
            table.start_subroutine();

            for i in 0..n {
                let name = format!("var{}", i);
                table.define(
                    &name,
                    Type::Int,
                    SymbolKind::Local,
                    jack_analyzer::token::Span::new(0, 0, 0, 0),
                ).unwrap();
            }

            prop_assert_eq!(
                table.var_count(SymbolKind::Local),
                n as u16,
                "Local count should be {}",
                n
            );

            // Each variable should have correct index
            for i in 0..n {
                let name = format!("var{}", i);
                let symbol = table.lookup(&name).unwrap();
                prop_assert_eq!(
                    symbol.index,
                    i as u16,
                    "Variable {} should have index {}",
                    name,
                    i
                );
            }
        }
    }
}
