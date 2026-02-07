//! Compiler optimizations for the Jack compiler.
//!
//! Includes:
//! - Constant folding (AST-level)
//! - Peephole optimization (VM-level)
//! - Strength reduction (codegen-level)

use jack_analyzer::ast::{BinaryOp, Expression, Term, UnaryOp};

/// Constant folder for compile-time expression evaluation.
pub struct ConstantFolder;

impl ConstantFolder {
    /// Attempt to fold a constant expression at compile time.
    ///
    /// Returns `Some(value)` if the expression can be fully evaluated,
    /// `None` if it contains variables or cannot be folded.
    pub fn fold_expression(expr: &Expression) -> Option<i32> {
        let mut result = Self::fold_term(&expr.term)?;

        for (op, term) in &expr.ops {
            let right = Self::fold_term(term)?;
            result = Self::apply_op(result, *op, right)?;
        }

        Some(result)
    }

    /// Attempt to fold a term.
    fn fold_term(term: &Term) -> Option<i32> {
        match term {
            Term::IntegerConstant(n, _) => Some(*n as i32),

            Term::UnaryOp(UnaryOp::Neg, inner, _) => Self::fold_term(inner).map(|n| -n),

            Term::UnaryOp(UnaryOp::Not, inner, _) => Self::fold_term(inner).map(|n| !n),

            Term::Parenthesized(inner, _) => Self::fold_expression(inner),

            Term::KeywordConstant(kw, _) => {
                use jack_analyzer::ast::KeywordConstant::*;
                match kw {
                    True => Some(-1), // ~0 in 16-bit
                    False | Null => Some(0),
                    This => None, // Runtime value
                }
            }

            // Variables, arrays, function calls cannot be folded
            _ => None,
        }
    }

    /// Apply a binary operation at compile time.
    fn apply_op(left: i32, op: BinaryOp, right: i32) -> Option<i32> {
        match op {
            BinaryOp::Add => Some(left.wrapping_add(right)),
            BinaryOp::Sub => Some(left.wrapping_sub(right)),
            BinaryOp::Mul => Some(left.wrapping_mul(right)),
            BinaryOp::Div if right != 0 => Some(left / right),
            BinaryOp::Div => None, // Division by zero
            BinaryOp::And => Some(left & right),
            BinaryOp::Or => Some(left | right),
            BinaryOp::Lt => Some(if left < right { -1 } else { 0 }),
            BinaryOp::Gt => Some(if left > right { -1 } else { 0 }),
            BinaryOp::Eq => Some(if left == right { -1 } else { 0 }),
        }
    }

    /// Check if a value fits in Jack's integer range (0-32767 for constants).
    pub fn in_range(value: i32) -> bool {
        (0..=32767).contains(&value)
    }
}

/// Peephole optimizer for VM code.
pub struct PeepholeOptimizer;

impl PeepholeOptimizer {
    /// Optimize VM code using peephole patterns.
    pub fn optimize(vm_code: &str) -> String {
        let lines: Vec<&str> = vm_code.lines().collect();
        let mut optimized = Vec::with_capacity(lines.len());
        let mut i = 0;

        while i < lines.len() {
            // Pattern: push X / pop X (same non-constant location) → remove both
            if i + 1 < lines.len() && Self::is_redundant_push_pop(lines[i], lines[i + 1]) {
                i += 2;
                continue;
            }

            // Pattern: push constant 0 / add → remove both (identity)
            if i + 1 < lines.len() && lines[i] == "push constant 0" && lines[i + 1] == "add" {
                i += 2;
                continue;
            }

            // Pattern: push constant 0 / sub → just neg the previous
            // (Not applied here as it would require lookback)

            // Pattern: push constant 1 / add → increment optimization
            // (Keep as-is for now; VM doesn't have inc instruction)

            // Pattern: not / not → remove both (double negation)
            if i + 1 < lines.len() && lines[i] == "not" && lines[i + 1] == "not" {
                i += 2;
                continue;
            }

            // Pattern: neg / neg → remove both (double negation)
            if i + 1 < lines.len() && lines[i] == "neg" && lines[i + 1] == "neg" {
                i += 2;
                continue;
            }

            // Pattern: push constant 0 / not → push constant -1 (true)
            if i + 1 < lines.len() && lines[i] == "push constant 0" && lines[i + 1] == "not" {
                optimized.push("push constant 0");
                optimized.push("not");
                i += 2;
                continue;
            }

            optimized.push(lines[i]);
            i += 1;
        }

        if optimized.is_empty() {
            String::new()
        } else {
            optimized.join("\n") + "\n"
        }
    }

    /// Check if push/pop pair is redundant (same location, not constant).
    fn is_redundant_push_pop(line1: &str, line2: &str) -> bool {
        if let (Some(push_rest), Some(pop_rest)) =
            (line1.strip_prefix("push "), line2.strip_prefix("pop "))
        {
            // Same location and not a constant (constants have side effects on stack)
            push_rest == pop_rest && !push_rest.starts_with("constant")
        } else {
            false
        }
    }
}

/// Strength reduction utilities for code generation.
pub struct StrengthReduction;

impl StrengthReduction {
    /// Check if a number is a power of 2 (for multiplication optimization).
    pub fn is_power_of_two(n: u16) -> bool {
        n > 0 && (n & (n - 1)) == 0
    }

    /// Get the number of left shifts needed to multiply by n (if power of 2).
    pub fn shift_count(n: u16) -> Option<u32> {
        if Self::is_power_of_two(n) {
            Some(n.trailing_zeros())
        } else {
            None
        }
    }

    /// Check if multiplication can be replaced with shifts.
    /// Returns the number of shifts if applicable.
    pub fn optimize_multiply(n: u16) -> Option<u32> {
        if n <= 16384 && Self::is_power_of_two(n) {
            Self::shift_count(n)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jack_analyzer::parser::Parser;
    use jack_analyzer::tokenizer::JackTokenizer;

    // ========================================================================
    // Constant Folding Tests
    // ========================================================================

    fn parse_expr(source: &str) -> Expression {
        // Wrap in a minimal class to parse
        let full_source = format!(
            "class T {{ function void f() {{ var int x; let x = {}; return; }} }}",
            source
        );
        let tokenizer = JackTokenizer::new(&full_source);
        let tokens = tokenizer.tokenize().unwrap();
        let parser = Parser::new(&tokens);
        let class = parser.parse().unwrap();

        // Extract expression from let statement
        if let jack_analyzer::ast::Statement::Let(let_stmt) =
            &class.subroutine_decs[0].body.statements[0]
        {
            let_stmt.value.clone()
        } else {
            panic!("Expected let statement");
        }
    }

    #[test]
    fn test_fold_integer_constant() {
        let expr = parse_expr("42");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(42));
    }

    #[test]
    fn test_fold_simple_addition() {
        let expr = parse_expr("1 + 2");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(3));
    }

    #[test]
    fn test_fold_simple_subtraction() {
        let expr = parse_expr("10 - 3");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(7));
    }

    #[test]
    fn test_fold_multiplication() {
        let expr = parse_expr("4 * 5");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(20));
    }

    #[test]
    fn test_fold_division() {
        let expr = parse_expr("20 / 4");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(5));
    }

    #[test]
    fn test_fold_chain_operations() {
        // Jack evaluates left-to-right: 1 + 2 + 3 = ((1 + 2) + 3) = 6
        let expr = parse_expr("1 + 2 + 3");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(6));
    }

    #[test]
    fn test_fold_parenthesized() {
        let expr = parse_expr("(1 + 2)");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(3));
    }

    #[test]
    fn test_fold_negation() {
        let expr = parse_expr("-5");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(-5));
    }

    #[test]
    fn test_fold_double_negation() {
        let expr = parse_expr("-(-5)");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(5));
    }

    #[test]
    fn test_fold_not() {
        let expr = parse_expr("~0");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(-1));
    }

    #[test]
    fn test_fold_comparison_lt() {
        let expr = parse_expr("1 < 2");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(-1)); // true
    }

    #[test]
    fn test_fold_comparison_gt() {
        let expr = parse_expr("5 > 3");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(-1)); // true
    }

    #[test]
    fn test_fold_comparison_eq() {
        let expr = parse_expr("5 = 5");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(-1)); // true
    }

    #[test]
    fn test_fold_comparison_false() {
        let expr = parse_expr("3 > 5");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(0)); // false
    }

    #[test]
    fn test_fold_true_keyword() {
        let expr = parse_expr("true");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(-1));
    }

    #[test]
    fn test_fold_false_keyword() {
        let expr = parse_expr("false");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(0));
    }

    #[test]
    fn test_fold_null_keyword() {
        let expr = parse_expr("null");
        assert_eq!(ConstantFolder::fold_expression(&expr), Some(0));
    }

    #[test]
    fn test_in_range() {
        assert!(ConstantFolder::in_range(0));
        assert!(ConstantFolder::in_range(32767));
        assert!(!ConstantFolder::in_range(-1));
        assert!(!ConstantFolder::in_range(32768));
    }

    // ========================================================================
    // Peephole Optimizer Tests
    // ========================================================================

    #[test]
    fn test_peephole_push_pop_elimination() {
        let input = "push local 0\npop local 0\npush constant 5\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push constant 5\n");
    }

    #[test]
    fn test_peephole_push_pop_different_locations() {
        let input = "push local 0\npop local 1\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push local 0\npop local 1\n");
    }

    #[test]
    fn test_peephole_constant_not_eliminated() {
        // push constant / pop should not be eliminated (side effect on stack)
        let input = "push constant 5\npop constant 5\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push constant 5\npop constant 5\n");
    }

    #[test]
    fn test_peephole_double_not() {
        let input = "push local 0\nnot\nnot\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push local 0\n");
    }

    #[test]
    fn test_peephole_double_neg() {
        let input = "push local 0\nneg\nneg\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push local 0\n");
    }

    #[test]
    fn test_peephole_identity_add() {
        let input = "push local 0\npush constant 0\nadd\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push local 0\n");
    }

    #[test]
    fn test_peephole_no_optimization_needed() {
        let input = "push constant 1\npush constant 2\nadd\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push constant 1\npush constant 2\nadd\n");
    }

    #[test]
    fn test_peephole_multiple_patterns() {
        let input = "push local 0\npop local 0\nnot\nnot\npush constant 0\nadd\npush constant 5\n";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "push constant 5\n");
    }

    #[test]
    fn test_peephole_empty_input() {
        let input = "";
        let optimized = PeepholeOptimizer::optimize(input);
        assert_eq!(optimized, "");
    }

    // ========================================================================
    // Strength Reduction Tests
    // ========================================================================

    #[test]
    fn test_is_power_of_two() {
        assert!(StrengthReduction::is_power_of_two(1));
        assert!(StrengthReduction::is_power_of_two(2));
        assert!(StrengthReduction::is_power_of_two(4));
        assert!(StrengthReduction::is_power_of_two(8));
        assert!(StrengthReduction::is_power_of_two(16));
        assert!(StrengthReduction::is_power_of_two(32));
        assert!(StrengthReduction::is_power_of_two(16384));

        assert!(!StrengthReduction::is_power_of_two(0));
        assert!(!StrengthReduction::is_power_of_two(3));
        assert!(!StrengthReduction::is_power_of_two(5));
        assert!(!StrengthReduction::is_power_of_two(6));
        assert!(!StrengthReduction::is_power_of_two(7));
    }

    #[test]
    fn test_shift_count() {
        assert_eq!(StrengthReduction::shift_count(1), Some(0));
        assert_eq!(StrengthReduction::shift_count(2), Some(1));
        assert_eq!(StrengthReduction::shift_count(4), Some(2));
        assert_eq!(StrengthReduction::shift_count(8), Some(3));
        assert_eq!(StrengthReduction::shift_count(16), Some(4));
        assert_eq!(StrengthReduction::shift_count(16384), Some(14));

        assert_eq!(StrengthReduction::shift_count(3), None);
        assert_eq!(StrengthReduction::shift_count(0), None);
    }

    #[test]
    fn test_optimize_multiply() {
        assert_eq!(StrengthReduction::optimize_multiply(2), Some(1));
        assert_eq!(StrengthReduction::optimize_multiply(4), Some(2));
        assert_eq!(StrengthReduction::optimize_multiply(8), Some(3));
        assert_eq!(StrengthReduction::optimize_multiply(16384), Some(14));

        // 32768 is too large for Jack integers
        assert_eq!(StrengthReduction::optimize_multiply(32768), None);
        // Non-power-of-2
        assert_eq!(StrengthReduction::optimize_multiply(3), None);
    }
}
