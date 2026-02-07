//! VM code generator for the Jack compiler.
//!
//! Traverses the AST and emits VM code using the VMWriter.

use crate::error::CompileError;
use crate::optimizer::{ConstantFolder, StrengthReduction};
use crate::symbol_table::{SymbolKind, SymbolTable};
use crate::vm_writer::VMWriter;
use jack_analyzer::ast::*;

/// Write a u32 value to a string buffer without allocation.
#[inline]
fn write_u32(n: u32, buf: &mut String) {
    if n == 0 {
        buf.push('0');
        return;
    }
    let mut digits = [0u8; 10]; // Max 10 digits for u32
    let mut i = 0;
    let mut num = n;
    while num > 0 {
        digits[i] = (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        buf.push((b'0' + digits[i]) as char);
    }
}

/// Write a u16 value to a string buffer without allocation.
#[inline]
fn write_u16(n: u16, buf: &mut String) {
    if n == 0 {
        buf.push('0');
        return;
    }
    let mut digits = [0u8; 5]; // Max 5 digits for u16
    let mut i = 0;
    let mut num = n;
    while num > 0 {
        digits[i] = (num % 10) as u8;
        num /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        buf.push((b'0' + digits[i]) as char);
    }
}

/// Code generator that compiles Jack AST to VM code.
pub struct CodeGenerator {
    /// Symbol table for variable lookup.
    symbols: SymbolTable,
    /// VM code emitter.
    vm: VMWriter,
    /// Counter for generating unique labels.
    label_counter: u32,
    /// Current class name.
    class_name: String,
    /// Current subroutine kind (for `this` handling).
    current_subroutine_kind: Option<SubroutineKind>,
    /// Collected compilation errors.
    errors: Vec<CompileError>,
    /// Whether to apply constant folding optimization.
    optimize: bool,
}

impl CodeGenerator {
    /// Create a new code generator with optimizations enabled.
    pub fn new() -> Self {
        Self::with_options(true)
    }

    /// Create a new code generator with specified optimization setting.
    pub fn with_options(optimize: bool) -> Self {
        Self {
            symbols: SymbolTable::new(),
            vm: VMWriter::new(),
            label_counter: 0,
            class_name: String::new(),
            current_subroutine_kind: None,
            errors: Vec::new(),
            optimize,
        }
    }

    /// Compile a class to VM code with optimizations enabled.
    ///
    /// Returns the generated VM code or a list of errors.
    pub fn compile(class: &Class) -> Result<String, Vec<CompileError>> {
        Self::compile_with_options(class, true)
    }

    /// Compile a class to VM code with specified optimization setting.
    ///
    /// Returns the generated VM code or a list of errors.
    pub fn compile_with_options(
        class: &Class,
        optimize: bool,
    ) -> Result<String, Vec<CompileError>> {
        let mut compiler = CodeGenerator::with_options(optimize);
        compiler.compile_class(class);

        if compiler.errors.is_empty() {
            Ok(compiler.vm.into_output())
        } else {
            Err(compiler.errors)
        }
    }

    /// Generate a unique label with the given prefix.
    /// Uses pre-allocated capacity to reduce allocations.
    #[inline]
    fn unique_label(&mut self, prefix: &str) -> String {
        let mut label = String::with_capacity(prefix.len() + 11); // prefix + '_' + max 10 digits
        label.push_str(prefix);
        label.push('_');
        write_u32(self.label_counter, &mut label);
        self.label_counter += 1;
        label
    }

    /// Record a compilation error.
    fn error(&mut self, error: CompileError) {
        self.errors.push(error);
    }

    // ========================================================================
    // Class Compilation
    // ========================================================================

    fn compile_class(&mut self, class: &Class) {
        self.class_name = class.name.clone();
        self.symbols.start_class(&class.name);

        // Define class-level variables
        for var_dec in &class.class_var_decs {
            self.compile_class_var_dec(var_dec);
        }

        // Compile subroutines
        for sub in &class.subroutine_decs {
            self.compile_subroutine(sub);
        }
    }

    fn compile_class_var_dec(&mut self, dec: &ClassVarDec) {
        let kind = match dec.kind {
            ClassVarKind::Static => SymbolKind::Static,
            ClassVarKind::Field => SymbolKind::Field,
        };

        for name in &dec.names {
            if let Err(e) = self
                .symbols
                .define(name, dec.var_type.clone(), kind, dec.span.clone())
            {
                self.error(e);
            }
        }
    }

    // ========================================================================
    // Subroutine Compilation
    // ========================================================================

    fn compile_subroutine(&mut self, sub: &SubroutineDec) {
        self.symbols.start_subroutine();
        self.current_subroutine_kind = Some(sub.kind);

        // For methods, `this` is argument 0
        if sub.kind == SubroutineKind::Method
            && let Err(e) = self.symbols.define(
                "this",
                Type::ClassName(self.class_name.clone()),
                SymbolKind::Argument,
                sub.span.clone(),
            )
        {
            self.error(e);
        }

        // Define parameters
        for param in &sub.parameters {
            if let Err(e) = self.symbols.define(
                &param.name,
                param.var_type.clone(),
                SymbolKind::Argument,
                sub.span.clone(),
            ) {
                self.error(e);
            }
        }

        // Define local variables
        for var_dec in &sub.body.var_decs {
            for name in &var_dec.names {
                if let Err(e) = self.symbols.define(
                    name,
                    var_dec.var_type.clone(),
                    SymbolKind::Local,
                    var_dec.span.clone(),
                ) {
                    self.error(e);
                }
            }
        }

        // Emit function declaration (zero-allocation)
        let num_locals = self.symbols.var_count(SymbolKind::Local);
        {
            let buf = self.vm.output_mut();
            buf.push_str("function ");
            buf.push_str(&self.class_name);
            buf.push('.');
            buf.push_str(&sub.name);
            buf.push(' ');
            write_u16(num_locals, buf);
            buf.push('\n');
        }

        // Handle constructor/method preamble
        match sub.kind {
            SubroutineKind::Constructor => {
                // Allocate memory for object fields
                let field_count = self.symbols.field_count();
                self.vm.write_push("constant", field_count);
                self.vm.write_call("Memory.alloc", 1);
                self.vm.write_pop("pointer", 0);
            }
            SubroutineKind::Method => {
                // Set `this` to argument 0
                self.vm.write_push("argument", 0);
                self.vm.write_pop("pointer", 0);
            }
            SubroutineKind::Function => {
                // No special setup needed
            }
        }

        // Compile statements
        self.compile_statements(&sub.body.statements);
    }

    // ========================================================================
    // Statement Compilation
    // ========================================================================

    #[inline]
    fn compile_statements(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.compile_statement(stmt);
        }
    }

    #[inline]
    fn compile_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let(s) => self.compile_let(s),
            Statement::If(s) => self.compile_if(s),
            Statement::While(s) => self.compile_while(s),
            Statement::Do(s) => self.compile_do(s),
            Statement::Return(s) => self.compile_return(s),
        }
    }

    fn compile_let(&mut self, stmt: &LetStatement) {
        let symbol = match self.symbols.lookup(&stmt.var_name) {
            Some(s) => s.clone(),
            None => {
                self.error(CompileError::undefined_variable(
                    &stmt.var_name,
                    stmt.span.clone(),
                ));
                return;
            }
        };

        if let Some(index_expr) = &stmt.index {
            // Array assignment: let arr[i] = expr
            // Push base address
            self.vm.write_push(symbol.segment(), symbol.index);
            // Compile and add index
            self.compile_expression(index_expr);
            self.vm.write_arithmetic("add");
            // Compile value
            self.compile_expression(&stmt.value);
            // Store via THAT
            self.vm.write_pop("temp", 0);
            self.vm.write_pop("pointer", 1);
            self.vm.write_push("temp", 0);
            self.vm.write_pop("that", 0);
        } else {
            // Simple assignment
            self.compile_expression(&stmt.value);
            self.vm.write_pop(symbol.segment(), symbol.index);
        }
    }

    fn compile_if(&mut self, stmt: &IfStatement) {
        let false_label = self.unique_label("IF_FALSE");
        let end_label = self.unique_label("IF_END");

        // Compile condition
        self.compile_expression(&stmt.condition);
        self.vm.write_arithmetic("not");
        self.vm.write_if_goto(&false_label);

        // Compile then-branch
        self.compile_statements(&stmt.then_statements);
        self.vm.write_goto(&end_label);

        // Compile else-branch (if present)
        self.vm.write_label(&false_label);
        if let Some(else_stmts) = &stmt.else_statements {
            self.compile_statements(else_stmts);
        }

        self.vm.write_label(&end_label);
    }

    fn compile_while(&mut self, stmt: &WhileStatement) {
        let exp_label = self.unique_label("WHILE_EXP");
        let end_label = self.unique_label("WHILE_END");

        self.vm.write_label(&exp_label);

        // Compile condition
        self.compile_expression(&stmt.condition);
        self.vm.write_arithmetic("not");
        self.vm.write_if_goto(&end_label);

        // Compile body
        self.compile_statements(&stmt.statements);
        self.vm.write_goto(&exp_label);

        self.vm.write_label(&end_label);
    }

    fn compile_do(&mut self, stmt: &DoStatement) {
        self.compile_subroutine_call(&stmt.call);
        // Discard return value
        self.vm.write_pop("temp", 0);
    }

    fn compile_return(&mut self, stmt: &ReturnStatement) {
        if let Some(expr) = &stmt.value {
            self.compile_expression(expr);
        } else {
            // Void return - push 0
            self.vm.write_push("constant", 0);
        }
        self.vm.write_return();
    }

    // ========================================================================
    // Expression Compilation
    // ========================================================================

    #[inline]
    fn compile_expression(&mut self, expr: &Expression) {
        // Try constant folding first (only if optimization is enabled)
        if self.optimize
            && let Some(value) = ConstantFolder::fold_expression(expr)
        {
            if (0..=32767).contains(&value) {
                self.vm.write_push("constant", value as u16);
                return;
            } else if (-32768..0).contains(&value) {
                // Handle negative constants: push |value| then negate
                self.vm.write_push("constant", (-value) as u16);
                self.vm.write_arithmetic("neg");
                return;
            }
        }

        // Strength reduction: const_pow2 * expr (left-side constant)
        // Pattern: first term is IntegerConstant(pow2), first op is Mul
        if self.optimize
            && !expr.ops.is_empty()
            && let (BinaryOp::Mul, ref right_term) = expr.ops[0]
            && let Term::IntegerConstant(n, _) = &expr.term
            && let Some(shifts) = StrengthReduction::optimize_multiply(*n)
        {
            // Compile the right term first, then shift left
            self.compile_term(right_term);
            self.emit_shift_left(shifts);
            // Continue with remaining ops (if any)
            for (op, term) in expr.ops.iter().skip(1) {
                self.compile_term(term);
                self.compile_binary_op(*op);
            }
            return;
        }

        // Normal compilation
        self.compile_term(&expr.term);

        for (op, term) in &expr.ops {
            // Strength reduction: expr * const_pow2 (right-side constant)
            if self.optimize
                && *op == BinaryOp::Mul
                && let Term::IntegerConstant(n, _) = term
                && let Some(shifts) = StrengthReduction::optimize_multiply(*n)
            {
                // Value is already on stack; emit shift-left instead of Math.multiply
                self.emit_shift_left(shifts);
                continue;
            }
            self.compile_term(term);
            self.compile_binary_op(*op);
        }
    }

    /// Emit a shift-left sequence (multiply by 2^shifts) for the value on top of stack.
    ///
    /// Each shift doubles the value: x * 2 = x + x.
    /// To duplicate the top-of-stack value, we use temp 0 as scratch:
    ///   pop temp 0 / push temp 0 / push temp 0 / add
    #[inline]
    fn emit_shift_left(&mut self, shifts: u32) {
        for _ in 0..shifts {
            // Duplicate top of stack and add (x + x = x * 2)
            self.vm.write_pop("temp", 0);
            self.vm.write_push("temp", 0);
            self.vm.write_push("temp", 0);
            self.vm.write_arithmetic("add");
        }
    }

    #[inline]
    fn compile_term(&mut self, term: &Term) {
        match term {
            Term::IntegerConstant(value, _) => {
                self.vm.write_push("constant", *value);
            }

            Term::StringConstant(s, _) => {
                self.compile_string_constant(s);
            }

            Term::KeywordConstant(kw, _) => {
                self.compile_keyword_constant(*kw);
            }

            Term::VarName(name, span) => match self.symbols.lookup(name) {
                Some(symbol) => {
                    self.vm.write_push(symbol.segment(), symbol.index);
                }
                None => {
                    self.error(CompileError::undefined_variable(name, span.clone()));
                }
            },

            Term::ArrayAccess(name, index_expr, span) => {
                match self.symbols.lookup(name) {
                    Some(symbol) => {
                        // Push base address
                        self.vm.write_push(symbol.segment(), symbol.index);
                        // Compile and add index
                        self.compile_expression(index_expr);
                        self.vm.write_arithmetic("add");
                        // Access via THAT
                        self.vm.write_pop("pointer", 1);
                        self.vm.write_push("that", 0);
                    }
                    None => {
                        self.error(CompileError::undefined_variable(name, span.clone()));
                    }
                }
            }

            Term::SubroutineCall(call) => {
                self.compile_subroutine_call(call);
            }

            Term::Parenthesized(expr, _) => {
                self.compile_expression(expr);
            }

            Term::UnaryOp(op, inner, _) => {
                self.compile_term(inner);
                match op {
                    UnaryOp::Neg => self.vm.write_arithmetic("neg"),
                    UnaryOp::Not => self.vm.write_arithmetic("not"),
                }
            }
        }
    }

    #[inline]
    fn compile_string_constant(&mut self, s: &str) {
        // Create string object
        let len = s.len() as u16;
        self.vm.write_push("constant", len);
        self.vm.write_call("String.new", 1);

        // Append each character
        for ch in s.chars() {
            self.vm.write_push("constant", ch as u16);
            self.vm.write_call("String.appendChar", 2);
        }
    }

    #[inline]
    fn compile_keyword_constant(&mut self, kw: KeywordConstant) {
        match kw {
            KeywordConstant::True => {
                // true = -1 = ~0
                self.vm.write_push("constant", 0);
                self.vm.write_arithmetic("not");
            }
            KeywordConstant::False | KeywordConstant::Null => {
                self.vm.write_push("constant", 0);
            }
            KeywordConstant::This => {
                self.vm.write_push("pointer", 0);
            }
        }
    }

    #[inline]
    fn compile_binary_op(&mut self, op: BinaryOp) {
        match op {
            BinaryOp::Add => self.vm.write_arithmetic("add"),
            BinaryOp::Sub => self.vm.write_arithmetic("sub"),
            BinaryOp::And => self.vm.write_arithmetic("and"),
            BinaryOp::Or => self.vm.write_arithmetic("or"),
            BinaryOp::Lt => self.vm.write_arithmetic("lt"),
            BinaryOp::Gt => self.vm.write_arithmetic("gt"),
            BinaryOp::Eq => self.vm.write_arithmetic("eq"),
            BinaryOp::Mul => self.vm.write_call("Math.multiply", 2),
            BinaryOp::Div => self.vm.write_call("Math.divide", 2),
        }
    }

    fn compile_subroutine_call(&mut self, call: &SubroutineCall) {
        // Determine class name for the call and push receiver if method
        // We need to clone the class name to avoid borrow issues
        let (class_name_owned, num_args) = if let Some(receiver) = &call.receiver {
            // Either ClassName.function() or varName.method()
            if let Some(symbol) = self.symbols.lookup(receiver) {
                // Method call on object variable - push receiver
                self.vm.write_push(symbol.segment(), symbol.index);
                let cn = match &symbol.symbol_type {
                    Type::ClassName(name) => name.clone(),
                    _ => receiver.clone(), // Fallback
                };
                (cn, call.arguments.len() as u16 + 1)
            } else {
                // Function or constructor call: ClassName.func()
                (receiver.clone(), call.arguments.len() as u16)
            }
        } else {
            // Method call on `this`: method()
            self.vm.write_push("pointer", 0);
            (self.class_name.clone(), call.arguments.len() as u16 + 1)
        };

        // Compile arguments
        for arg in &call.arguments {
            self.compile_expression(arg);
        }

        // Write call command (zero-allocation for the write itself)
        {
            let buf = self.vm.output_mut();
            buf.push_str("call ");
            buf.push_str(&class_name_owned);
            buf.push('.');
            buf.push_str(&call.name);
            buf.push(' ');
            write_u16(num_args, buf);
            buf.push('\n');
        }
    }
}

impl Default for CodeGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jack_analyzer::parser::Parser;
    use jack_analyzer::tokenizer::JackTokenizer;

    /// Helper to compile Jack source and return VM code.
    fn compile_source(source: &str) -> Result<String, Vec<CompileError>> {
        let tokenizer = JackTokenizer::new(source);
        let tokens = tokenizer.tokenize().expect("tokenization failed");
        let parser = Parser::new(&tokens);
        let class = parser.parse().expect("parsing failed");
        CodeGenerator::compile(&class)
    }

    #[test]
    fn test_empty_function() {
        let source = r#"
class Main {
    function void main() {
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("function Main.main 0"));
        assert!(vm.contains("push constant 0"));
        assert!(vm.contains("return"));
    }

    #[test]
    fn test_integer_constant() {
        let source = r#"
class Main {
    function int seven() {
        return 7;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push constant 7"));
        assert!(vm.contains("return"));
    }

    #[test]
    fn test_simple_arithmetic() {
        let source = r#"
class Main {
    function int add() {
        return 1 + 2;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // Constant folding should fold 1 + 2 to 3
        assert!(
            vm.contains("push constant 3"),
            "Constant folding should fold 1+2 to 3"
        );
    }

    #[test]
    fn test_arithmetic_with_variable() {
        // Test that arithmetic with variables still generates proper code
        let source = r#"
class Main {
    function int add() {
        var int x;
        let x = 1;
        return x + 2;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push local 0"));
        assert!(vm.contains("push constant 2"));
        assert!(vm.contains("add"));
    }

    #[test]
    fn test_multiplication() {
        let source = r#"
class Main {
    function int mul() {
        return 3 * 4;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // Constant folding should fold 3 * 4 to 12
        assert!(
            vm.contains("push constant 12"),
            "Constant folding should fold 3*4 to 12"
        );
    }

    #[test]
    fn test_multiplication_with_variable_power_of_two() {
        // Test that multiplication by power of 2 uses strength reduction (shift)
        let source = r#"
class Main {
    function int mul() {
        var int x;
        let x = 3;
        return x * 4;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // Strength reduction: x * 4 should use shift (add) instead of Math.multiply
        assert!(
            !vm.contains("call Math.multiply 2"),
            "Should use strength reduction for x * 4, not Math.multiply"
        );
        assert!(
            vm.contains("add"),
            "Should use add for shift-based multiply"
        );
    }

    #[test]
    fn test_multiplication_with_variable_non_power_of_two() {
        // Test that multiplication by non-power-of-2 still calls Math.multiply
        let source = r#"
class Main {
    function int mul() {
        var int x;
        let x = 3;
        return x * 5;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(
            vm.contains("call Math.multiply 2"),
            "Should use Math.multiply for non-power-of-2"
        );
    }

    #[test]
    fn test_strength_reduction_left_constant() {
        // Test strength reduction when power-of-2 is on the left: 2 * x
        let source = r#"
class Main {
    function int mul() {
        var int x;
        let x = 5;
        return 2 * x;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // 2 * x should use strength reduction
        assert!(
            !vm.contains("call Math.multiply 2"),
            "Should use strength reduction for 2 * x"
        );
        assert!(
            vm.contains("add"),
            "Should use add for shift-based multiply"
        );
    }

    #[test]
    fn test_local_variable() {
        let source = r#"
class Main {
    function int test() {
        var int x;
        let x = 5;
        return x;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("function Main.test 1"));
        assert!(vm.contains("push constant 5"));
        assert!(vm.contains("pop local 0"));
        assert!(vm.contains("push local 0"));
    }

    #[test]
    fn test_multiple_locals() {
        let source = r#"
class Main {
    function int test() {
        var int x, y;
        let x = 1;
        let y = 2;
        return x + y;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("function Main.test 2"));
        assert!(vm.contains("pop local 0"));
        assert!(vm.contains("pop local 1"));
    }

    #[test]
    fn test_true_false_null() {
        let source = r#"
class Main {
    function int test() {
        var boolean a, b;
        var int c;
        let a = true;
        let b = false;
        let c = null;
        return 0;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // true = -1, constant folding folds to push constant then neg
        // OR it could be push constant 0 then not - depends on folding
        // Actually true = ~0 = -1, which gets folded to push constant 1 / neg
        // Or since -1 fits in range -32768 to -1, it becomes: push constant 1, neg
        // Let's check for the patterns
        assert!(
            vm.contains("push constant 1\nneg") || vm.contains("push constant 0\nnot"),
            "true should be represented as -1"
        );
        // false and null = 0
        assert!(
            vm.matches("push constant 0").count() >= 2,
            "false and null should be 0"
        );
    }

    #[test]
    fn test_unary_negation() {
        let source = r#"
class Main {
    function int test() {
        return -5;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push constant 5"));
        assert!(vm.contains("neg"));
    }

    #[test]
    fn test_unary_not() {
        let source = r#"
class Main {
    function boolean test() {
        return ~true;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // ~true = ~(-1) = 0, which gets constant-folded to just push constant 0
        assert!(vm.contains("push constant 0"), "~true should fold to 0");
    }

    #[test]
    fn test_unary_not_with_variable() {
        // Test that ~ on variable still generates not instruction
        let source = r#"
class Main {
    function boolean test() {
        var boolean x;
        let x = true;
        return ~x;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("not"));
    }

    #[test]
    fn test_if_statement() {
        let source = r#"
class Main {
    function void test() {
        var int x;
        if (true) {
            let x = 1;
        }
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("if-goto IF_FALSE_"));
        assert!(vm.contains("goto IF_END_"));
        assert!(vm.contains("label IF_FALSE_"));
        assert!(vm.contains("label IF_END_"));
    }

    #[test]
    fn test_if_else_statement() {
        let source = r#"
class Main {
    function void test() {
        var int x;
        if (false) {
            let x = 1;
        } else {
            let x = 2;
        }
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("if-goto IF_FALSE_"));
        assert!(vm.contains("goto IF_END_"));
        assert!(vm.contains("label IF_FALSE_"));
        assert!(vm.contains("push constant 2"));
    }

    #[test]
    fn test_while_statement() {
        let source = r#"
class Main {
    function void test() {
        var int x;
        let x = 0;
        while (x < 10) {
            let x = x + 1;
        }
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("label WHILE_EXP_"));
        assert!(vm.contains("if-goto WHILE_END_"));
        assert!(vm.contains("goto WHILE_EXP_"));
        assert!(vm.contains("label WHILE_END_"));
    }

    #[test]
    fn test_do_statement() {
        let source = r#"
class Main {
    function void test() {
        do Output.printInt(7);
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push constant 7"));
        assert!(vm.contains("call Output.printInt 1"));
        assert!(vm.contains("pop temp 0")); // Discard return value
    }

    #[test]
    fn test_constructor() {
        let source = r#"
class Point {
    field int x, y;

    constructor Point new(int ax, int ay) {
        let x = ax;
        let y = ay;
        return this;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // Constructor allocates 2 fields
        assert!(vm.contains("push constant 2"));
        assert!(vm.contains("call Memory.alloc 1"));
        assert!(vm.contains("pop pointer 0"));
        // Return this
        assert!(vm.contains("push pointer 0\nreturn"));
    }

    #[test]
    fn test_method() {
        let source = r#"
class Point {
    field int x;

    method int getX() {
        return x;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // Method sets up this pointer
        assert!(vm.contains("push argument 0"));
        assert!(vm.contains("pop pointer 0"));
        // Access field via this segment
        assert!(vm.contains("push this 0"));
    }

    #[test]
    fn test_method_call_on_this() {
        let source = r#"
class Test {
    method void foo() {
        do bar();
        return;
    }

    method void bar() {
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // Method call on this pushes pointer 0
        assert!(vm.contains("push pointer 0\ncall Test.bar 1"));
    }

    #[test]
    fn test_static_variable() {
        let source = r#"
class Counter {
    static int count;

    function void increment() {
        let count = count + 1;
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push static 0"));
        assert!(vm.contains("pop static 0"));
    }

    #[test]
    fn test_string_constant() {
        let source = r#"
class Main {
    function String test() {
        return "hi";
    }
}
"#;
        let vm = compile_source(source).unwrap();
        // String creation
        assert!(vm.contains("push constant 2")); // length
        assert!(vm.contains("call String.new 1"));
        // Append chars
        assert!(vm.contains("push constant 104")); // 'h'
        assert!(vm.contains("call String.appendChar 2"));
        assert!(vm.contains("push constant 105")); // 'i'
    }

    #[test]
    fn test_array_access_read() {
        let source = r#"
class Main {
    function int test() {
        var Array a;
        return a[5];
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push local 0")); // base
        assert!(vm.contains("push constant 5")); // index
        assert!(vm.contains("add"));
        assert!(vm.contains("pop pointer 1"));
        assert!(vm.contains("push that 0"));
    }

    #[test]
    fn test_array_access_write() {
        let source = r#"
class Main {
    function void test() {
        var Array a;
        let a[3] = 42;
        return;
    }
}
"#;
        let vm = compile_source(source).unwrap();
        assert!(vm.contains("push local 0")); // base
        assert!(vm.contains("push constant 3")); // index
        assert!(vm.contains("add"));
        assert!(vm.contains("push constant 42")); // value
        assert!(vm.contains("pop temp 0"));
        assert!(vm.contains("pop pointer 1"));
        assert!(vm.contains("push temp 0"));
        assert!(vm.contains("pop that 0"));
    }

    #[test]
    fn test_undefined_variable_error() {
        let source = r#"
class Main {
    function void test() {
        let x = 5;
        return;
    }
}
"#;
        let result = compile_source(source);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(
            errors
                .iter()
                .any(|e| matches!(e, CompileError::UndefinedVariable { .. }))
        );
    }
}
