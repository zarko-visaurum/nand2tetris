//! Property-based fuzzing tests for the Jack Analyzer (Project 10).
//!
//! Uses proptest to generate arbitrary Jack source code and verify the analyzer
//! never panics and handles all input gracefully.

use jack_analyzer::analyze_source;
use proptest::prelude::*;
use proptest::test_runner::TestRunner;

/// Stack size for tests with deeply nested proptest strategy trees.
/// Debug builds don't inline, so proptest's combinator stack frames
/// can exhaust the default 8 MB thread stack.
const PROPTEST_STACK_SIZE: usize = 16 * 1024 * 1024;

/// Generate valid Jack identifiers
fn arb_identifier() -> impl Strategy<Value = String> {
    "[a-zA-Z_][a-zA-Z0-9_]{0,10}".prop_map(|s| s)
}

/// Generate valid Jack class names (start with uppercase by convention)
fn arb_class_name() -> impl Strategy<Value = String> {
    "[A-Z][a-zA-Z0-9]{0,10}".prop_map(|s| s)
}

/// Generate valid subroutine names
fn arb_subroutine_name() -> impl Strategy<Value = String> {
    "[a-z][a-zA-Z0-9]{0,10}".prop_map(|s| s)
}

/// Generate valid integer constants (0-32767)
fn arb_integer() -> impl Strategy<Value = String> {
    (0u16..32768).prop_map(|n| n.to_string())
}

/// Generate valid string constants (no newlines or quotes inside)
fn arb_string_constant() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9 .,!?]{0,20}".prop_map(|s| format!("\"{}\"", s))
}

/// Generate keyword constants
fn arb_keyword_constant() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("true".to_string()),
        Just("false".to_string()),
        Just("null".to_string()),
        Just("this".to_string()),
    ]
}

/// Generate valid types
fn arb_type() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("int".to_string()),
        Just("char".to_string()),
        Just("boolean".to_string()),
        arb_class_name(),
    ]
}

/// Generate unary operators
fn arb_unary_op() -> impl Strategy<Value = String> {
    prop_oneof![Just("-".to_string()), Just("~".to_string()),]
}

/// Generate binary operators
fn arb_binary_op() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("+".to_string()),
        Just("-".to_string()),
        Just("*".to_string()),
        Just("/".to_string()),
        Just("&".to_string()),
        Just("|".to_string()),
        Just("<".to_string()),
        Just(">".to_string()),
        Just("=".to_string()),
    ]
}

/// Generate simple terms (non-recursive)
fn arb_simple_term() -> impl Strategy<Value = String> {
    prop_oneof![
        arb_integer(),
        arb_string_constant(),
        arb_keyword_constant(),
        arb_identifier(),
    ]
}

/// Generate simple expressions
fn arb_simple_expression() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => arb_simple_term(),
        1 => (arb_unary_op(), arb_simple_term()).prop_map(|(op, term)| format!("{}{}", op, term)),
        1 => (arb_simple_term(), arb_binary_op(), arb_simple_term())
            .prop_map(|(t1, op, t2)| format!("{} {} {}", t1, op, t2)),
    ]
}

/// Generate let statements
fn arb_let_statement() -> impl Strategy<Value = String> {
    (arb_identifier(), arb_simple_expression())
        .prop_map(|(var, expr)| format!("let {} = {};", var, expr))
}

/// Generate do statements (subroutine calls)
fn arb_do_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        // Simple function call: do func();
        arb_subroutine_name().prop_map(|name| format!("do {}();", name)),
        // Method call: do obj.method();
        (arb_identifier(), arb_subroutine_name())
            .prop_map(|(obj, method)| format!("do {}.{}();", obj, method)),
        // Static call: do Class.func();
        (arb_class_name(), arb_subroutine_name())
            .prop_map(|(class, method)| format!("do {}.{}();", class, method)),
    ]
}

/// Generate return statements
fn arb_return_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("return;".to_string()),
        arb_simple_expression().prop_map(|expr| format!("return {};", expr)),
    ]
}

/// Generate while statements
fn arb_while_statement() -> impl Strategy<Value = String> {
    (arb_simple_expression(), arb_let_statement())
        .prop_map(|(cond, body)| format!("while ({}) {{ {} }}", cond, body))
}

/// Generate if statements
fn arb_if_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        // if without else
        (arb_simple_expression(), arb_let_statement())
            .prop_map(|(cond, body)| format!("if ({}) {{ {} }}", cond, body)),
        // if with else
        (
            arb_simple_expression(),
            arb_let_statement(),
            arb_let_statement()
        )
            .prop_map(|(cond, if_body, else_body)| format!(
                "if ({}) {{ {} }} else {{ {} }}",
                cond, if_body, else_body
            )),
    ]
}

/// Generate any valid statement
fn arb_statement() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => arb_let_statement(),
        2 => arb_do_statement(),
        2 => arb_return_statement(),
        1 => arb_while_statement(),
        1 => arb_if_statement(),
    ]
}

/// Generate variable declarations
fn arb_var_dec() -> impl Strategy<Value = String> {
    (arb_type(), arb_identifier()).prop_map(|(ty, name)| format!("var {} {};", ty, name))
}

/// Generate field declarations
fn arb_field_dec() -> impl Strategy<Value = String> {
    prop_oneof![
        (arb_type(), arb_identifier()).prop_map(|(ty, name)| format!("field {} {};", ty, name)),
        (arb_type(), arb_identifier()).prop_map(|(ty, name)| format!("static {} {};", ty, name)),
    ]
}

/// Generate parameter lists
fn arb_parameter_list() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("".to_string()),
        (arb_type(), arb_identifier()).prop_map(|(ty, name)| format!("{} {}", ty, name)),
        (arb_type(), arb_identifier(), arb_type(), arb_identifier())
            .prop_map(|(t1, n1, t2, n2)| format!("{} {}, {} {}", t1, n1, t2, n2)),
    ]
}

/// Generate subroutine kind
fn arb_subroutine_kind() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("function".to_string()),
        Just("method".to_string()),
        Just("constructor".to_string()),
    ]
}

/// Generate return type
fn arb_return_type() -> impl Strategy<Value = String> {
    prop_oneof![Just("void".to_string()), arb_type(),]
}

/// Generate a simple subroutine
fn arb_subroutine() -> impl Strategy<Value = String> {
    (
        arb_subroutine_kind(),
        arb_return_type(),
        arb_subroutine_name(),
        arb_parameter_list(),
        prop::collection::vec(arb_var_dec(), 0..3),
        prop::collection::vec(arb_statement(), 1..5),
    )
        .prop_map(|(kind, ret_type, name, params, vars, stmts)| {
            let vars_str = vars.join("\n        ");
            let stmts_str = stmts.join("\n        ");
            format!(
                "{} {} {}({}) {{\n        {}\n        {}\n    }}",
                kind, ret_type, name, params, vars_str, stmts_str
            )
        })
}

/// Generate a complete Jack class
fn arb_class() -> impl Strategy<Value = String> {
    (
        arb_class_name(),
        prop::collection::vec(arb_field_dec(), 0..3),
        prop::collection::vec(arb_subroutine(), 1..3),
    )
        .prop_map(|(name, fields, subs)| {
            let fields_str = fields.join("\n    ");
            let subs_str = subs.join("\n\n    ");
            format!(
                "class {} {{\n    {}\n\n    {}\n}}",
                name, fields_str, subs_str
            )
        })
}

/// Generate arbitrary Jack-like input (may or may not be valid)
fn arb_jack_like_input() -> impl Strategy<Value = String> {
    prop_oneof![
        // Valid class
        3 => arb_class(),
        // Just comments
        1 => "// [a-zA-Z0-9 ]{0,50}".prop_map(|s| s),
        // Multi-line comments
        1 => "/\\* [a-zA-Z0-9 ]{0,50} \\*/".prop_map(|s| s),
        // Empty input
        1 => Just("".to_string()),
        // Whitespace only
        1 => "[ \t\n]{0,20}".prop_map(|s| s),
        // Incomplete class
        1 => arb_class_name().prop_map(|name| format!("class {} {{", name)),
        // Random tokens
        1 => "[a-zA-Z0-9+\\-*/{}();, \n]{0,100}".prop_map(|s| s),
    ]
}

// These two tests use deeply nested proptest strategy trees (arb_class → arb_subroutine
// → arb_statement → arb_simple_expression). In debug builds, proptest's combinator stack
// frames exhaust the default 8 MB thread stack, so we spawn a thread with a larger stack.

#[test]
fn test_no_panic_on_valid_class() {
    std::thread::Builder::new()
        .stack_size(PROPTEST_STACK_SIZE)
        .spawn(|| {
            let mut runner = TestRunner::default();
            runner
                .run(&arb_class(), |source| {
                    let _ = analyze_source(&source, "Test.jack");
                    Ok(())
                })
                .unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn test_no_panic_on_arbitrary_input() {
    std::thread::Builder::new()
        .stack_size(PROPTEST_STACK_SIZE)
        .spawn(|| {
            let mut runner = TestRunner::default();
            runner
                .run(&arb_jack_like_input(), |source| {
                    let _ = analyze_source(&source, "Test.jack");
                    Ok(())
                })
                .unwrap();
        })
        .unwrap()
        .join()
        .unwrap();
}

proptest! {

    /// Test that valid integer constants are tokenized correctly
    #[test]
    fn test_valid_integer_constant(n in 0u16..32768) {
        let source = format!("class Test {{ function void main() {{ let x = {}; return; }} }}", n);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Valid integer {} should parse", n);
        prop_assert!(result.token_xml.contains(&n.to_string()), "Token XML should contain integer");
    }

    /// Test that valid string constants are tokenized correctly
    #[test]
    fn test_valid_string_constant(s in "[a-zA-Z0-9 ]{0,20}") {
        let source = format!("class Test {{ function void main() {{ let x = \"{}\"; return; }} }}", s);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Valid string should parse");
        prop_assert!(result.token_xml.contains("<stringConstant>"), "Should have string constant");
    }

    /// Test that all keywords are recognized
    #[test]
    fn test_keyword_recognition(keyword in prop_oneof![
        Just("class"), Just("constructor"), Just("function"), Just("method"),
        Just("field"), Just("static"), Just("var"), Just("int"), Just("char"),
        Just("boolean"), Just("void"), Just("true"), Just("false"), Just("null"),
        Just("this"), Just("let"), Just("do"), Just("if"), Just("else"),
        Just("while"), Just("return"),
    ]) {
        // Create minimal valid context for each keyword
        let source = match keyword {
            "class" => "class Test { }".to_string(),
            "constructor" => "class Test { constructor Test new() { return this; } }".to_string(),
            "function" => "class Test { function void main() { return; } }".to_string(),
            "method" => "class Test { method void foo() { return; } }".to_string(),
            "field" => "class Test { field int x; }".to_string(),
            "static" => "class Test { static int x; }".to_string(),
            "var" => "class Test { function void main() { var int x; return; } }".to_string(),
            "int" => "class Test { field int x; }".to_string(),
            "char" => "class Test { field char c; }".to_string(),
            "boolean" => "class Test { field boolean b; }".to_string(),
            "void" => "class Test { function void main() { return; } }".to_string(),
            "true" => "class Test { function void main() { let x = true; return; } }".to_string(),
            "false" => "class Test { function void main() { let x = false; return; } }".to_string(),
            "null" => "class Test { function void main() { let x = null; return; } }".to_string(),
            "this" => "class Test { method void foo() { return this; } }".to_string(),
            "let" => "class Test { function void main() { let x = 1; return; } }".to_string(),
            "do" => "class Test { function void main() { do foo(); return; } }".to_string(),
            "if" => "class Test { function void main() { if (true) { } return; } }".to_string(),
            "else" => "class Test { function void main() { if (true) { } else { } return; } }".to_string(),
            "while" => "class Test { function void main() { while (true) { } return; } }".to_string(),
            "return" => "class Test { function void main() { return; } }".to_string(),
            _ => "class Test { function void main() { return; } }".to_string(),
        };
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Keyword '{}' should be recognized", keyword);
        prop_assert!(result.token_xml.contains(&format!("<keyword> {} </keyword>", keyword)),
            "Token XML should contain keyword '{}'", keyword);
    }

    /// Test that all symbols are recognized
    #[test]
    fn test_symbol_recognition(symbol in prop_oneof![
        Just('{'), Just('}'), Just('('), Just(')'), Just('['), Just(']'),
        Just('.'), Just(','), Just(';'), Just('+'), Just('-'), Just('*'),
        Just('/'), Just('&'), Just('|'), Just('<'), Just('>'), Just('='),
        Just('~'),
    ]) {
        // Symbols appear in various contexts
        let source = match symbol {
            '{' | '}' => "class Test { }".to_string(),
            '(' | ')' => "class Test { function void main() { return; } }".to_string(),
            '[' | ']' => "class Test { function void main() { let x = a[0]; return; } }".to_string(),
            '.' => "class Test { function void main() { do Sys.halt(); return; } }".to_string(),
            ',' => "class Test { function void foo(int a, int b) { return; } }".to_string(),
            ';' => "class Test { function void main() { return; } }".to_string(),
            '+' => "class Test { function void main() { let x = 1 + 2; return; } }".to_string(),
            '-' => "class Test { function void main() { let x = 1 - 2; return; } }".to_string(),
            '*' => "class Test { function void main() { let x = 1 * 2; return; } }".to_string(),
            '/' => "class Test { function void main() { let x = 1 / 2; return; } }".to_string(),
            '&' => "class Test { function void main() { let x = 1 & 2; return; } }".to_string(),
            '|' => "class Test { function void main() { let x = 1 | 2; return; } }".to_string(),
            '<' => "class Test { function void main() { let x = 1 < 2; return; } }".to_string(),
            '>' => "class Test { function void main() { let x = 1 > 2; return; } }".to_string(),
            '=' => "class Test { function void main() { let x = 1; return; } }".to_string(),
            '~' => "class Test { function void main() { let x = ~1; return; } }".to_string(),
            _ => "class Test { }".to_string(),
        };
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Symbol '{}' should be recognized", symbol);
    }

    /// Test that comments are properly ignored
    #[test]
    fn test_comments_ignored(comment in "[a-zA-Z0-9]{1,50}") {
        // Single-line comment - use non-empty comment with unique content
        let unique_marker = format!("COMMENT_MARKER_{}", comment);
        let source = format!("// {}\nclass Test {{ }}", unique_marker);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Single-line comment should be ignored");
        prop_assert!(!result.token_xml.contains(&unique_marker), "Comment content should not appear in tokens");
    }

    /// Test that multi-line comments are properly ignored
    #[test]
    fn test_multiline_comments_ignored(comment in "[a-zA-Z0-9 ]{0,30}") {
        let source = format!("/* {} */\nclass Test {{ }}", comment);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Multi-line comment should be ignored");
    }

    /// Test that API doc comments are properly ignored
    #[test]
    fn test_api_doc_comments_ignored(comment in "[a-zA-Z0-9 ]{0,30}") {
        let source = format!("/** {} */\nclass Test {{ }}", comment);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "API doc comment should be ignored");
    }

    /// Test that let statements with array access work
    #[test]
    fn test_array_access(index in 0u16..1000) {
        let source = format!(
            "class Test {{ function void main() {{ let a[{}] = 1; return; }} }}",
            index
        );
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Array access with index {} should work", index);
    }

    /// Test that expression with multiple operators work
    #[test]
    fn test_chained_expressions(op_count in 1usize..5) {
        let ops = vec![" + 1"; op_count];
        let expr = format!("1{}", ops.join(""));
        let source = format!(
            "class Test {{ function void main() {{ let x = {}; return; }} }}",
            expr
        );
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Chained expression should work");
    }

    /// Test that nested parentheses work
    #[test]
    fn test_nested_parens(depth in 1usize..5) {
        let open = "(".repeat(depth);
        let close = ")".repeat(depth);
        let source = format!(
            "class Test {{ function void main() {{ let x = {}1{}; return; }} }}",
            open, close
        );
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Nested parentheses (depth {}) should work", depth);
    }

    /// Test that subroutine calls with varying argument counts work
    #[test]
    fn test_call_arg_count(arg_count in 0usize..5) {
        let args = if arg_count == 0 {
            "".to_string()
        } else {
            (0..arg_count).map(|_| "1").collect::<Vec<_>>().join(", ")
        };
        let source = format!(
            "class Test {{ function void main() {{ do foo({}); return; }} }}",
            args
        );
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Call with {} args should work", arg_count);
    }

    /// Test that malformed input doesn't panic (just errors)
    #[test]
    fn test_malformed_no_panic(garbage in "[a-zA-Z0-9+\\-*/(){}; \n]{0,100}") {
        let _ = analyze_source(&garbage, "Test.jack");
        // Test passes if no panic
    }

    /// Test that unterminated strings fail gracefully
    #[test]
    fn test_unterminated_string(s in "[a-zA-Z0-9 ]{0,20}") {
        let source = format!("class Test {{ function void main() {{ let x = \"{}\n; return; }} }}", s);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(!result.is_ok(), "Unterminated string should fail");
    }

    /// Test that invalid integers fail gracefully
    #[test]
    fn test_invalid_integer(n in 32768u32..100000) {
        let source = format!("class Test {{ function void main() {{ let x = {}; return; }} }}", n);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(!result.is_ok(), "Integer {} out of range should fail", n);
    }

    /// Test that empty class body works
    #[test]
    fn test_empty_class(name in arb_class_name()) {
        let source = format!("class {} {{ }}", name);
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Empty class should work");
    }

    /// Test multiple field declarations
    #[test]
    fn test_multiple_fields(count in 1usize..5) {
        let fields: Vec<String> = (0..count)
            .map(|i| format!("field int f{};", i))
            .collect();
        let source = format!("class Test {{ {} }}", fields.join(" "));
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Multiple fields should work");
    }

    /// Test multiple subroutines
    #[test]
    fn test_multiple_subroutines(count in 1usize..4) {
        let subs: Vec<String> = (0..count)
            .map(|i| format!("function void sub{}() {{ return; }}", i))
            .collect();
        let source = format!("class Test {{ {} }}", subs.join(" "));
        let result = analyze_source(&source, "Test.jack");
        prop_assert!(result.is_ok(), "Multiple subroutines should work");
    }

    /// Test while with nested if
    #[test]
    fn test_nested_control_flow(_seed in 0u64..1000) {
        let source = "class Test { function void main() { while (true) { if (false) { let x = 1; } } return; } }";
        let result = analyze_source(source, "Test.jack");
        prop_assert!(result.is_ok(), "Nested control flow should work");
    }

    /// Test error recovery - multiple errors don't cause panic
    #[test]
    fn test_error_recovery(error_count in 1usize..5) {
        let errors: Vec<&str> = (0..error_count).map(|_| "let = ;").collect();
        let source = format!(
            "class Test {{ function void main() {{ {} return; }} }}",
            errors.join(" ")
        );
        let result = analyze_source(&source, "Test.jack");
        // May have errors, but should not panic
        let _ = result;
    }
}
