//! XML output generation for tokenizer and parser results.
//!
//! This module uses zero-allocation techniques where possible:
//! - Pre-sized String buffers based on token count estimates
//! - Direct push_str() instead of format!() macros
//! - Static string slices for tag names

use crate::ast::*;
use crate::token::SpannedToken;

/// Estimated bytes per token in XML output (for buffer pre-allocation).
const BYTES_PER_TOKEN: usize = 40;

/// Estimated bytes per indent level.
const BYTES_PER_INDENT: usize = 2;

/// Generate token XML output (*T.xml format).
///
/// Uses zero-allocation techniques with pre-sized buffer.
pub fn tokens_to_xml(tokens: &[SpannedToken]) -> String {
    // Pre-allocate: <tokens>\n + tokens + </tokens>\n
    let capacity = 10 + (tokens.len() * BYTES_PER_TOKEN) + 11;
    let mut output = String::with_capacity(capacity);

    output.push_str("<tokens>\n");

    for token in tokens {
        let tag = token.token.xml_tag();
        let value = token.token.xml_value();
        // Write directly without format!()
        output.push('<');
        output.push_str(tag);
        output.push_str("> ");
        output.push_str(&value);
        output.push_str(" </");
        output.push_str(tag);
        output.push_str(">\n");
    }

    output.push_str("</tokens>\n");
    output
}

/// XML writer for AST nodes (*.xml format).
///
/// Uses zero-allocation techniques:
/// - Pre-sized buffer based on token count
/// - Direct string operations instead of format!()
/// - Indent string reuse
pub struct XmlWriter {
    output: String,
    indent: usize,
}

impl XmlWriter {
    /// Create a new XML writer with pre-allocated buffer.
    pub fn new() -> Self {
        Self {
            output: String::new(),
            indent: 0,
        }
    }

    /// Create a new XML writer with capacity hint based on token count.
    pub fn with_capacity(token_count: usize) -> Self {
        // Estimate: each token generates ~40 bytes, plus indent overhead
        let capacity = token_count * BYTES_PER_TOKEN + token_count * BYTES_PER_INDENT * 4;
        Self {
            output: String::with_capacity(capacity),
            indent: 0,
        }
    }

    /// Write a class to XML.
    pub fn write_class(mut self, class: &Class, tokens: &[SpannedToken]) -> String {
        // Resize buffer based on actual token count
        if self.output.capacity() == 0 {
            let capacity = tokens.len() * BYTES_PER_TOKEN + tokens.len() * BYTES_PER_INDENT * 4;
            self.output.reserve(capacity);
        }

        let mut ctx = XmlContext::new(tokens);
        self.write_class_impl(class, &mut ctx);
        self.output
    }

    /// Write indentation directly (no allocation).
    #[inline]
    fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    /// Open an XML tag (zero-allocation).
    #[inline]
    fn open_tag(&mut self, tag: &str) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(tag);
        self.output.push_str(">\n");
        self.indent += 1;
    }

    /// Close an XML tag (zero-allocation).
    #[inline]
    fn close_tag(&mut self, tag: &str) {
        self.indent -= 1;
        self.write_indent();
        self.output.push_str("</");
        self.output.push_str(tag);
        self.output.push_str(">\n");
    }

    /// Write a terminal element (zero-allocation).
    #[inline]
    fn write_terminal(&mut self, tag: &str, value: &str) {
        self.write_indent();
        self.output.push('<');
        self.output.push_str(tag);
        self.output.push_str("> ");
        self.output.push_str(value);
        self.output.push_str(" </");
        self.output.push_str(tag);
        self.output.push_str(">\n");
    }

    /// Write a token from the context.
    #[inline]
    fn write_token(&mut self, ctx: &mut XmlContext) {
        if let Some(token) = ctx.advance() {
            let tag = token.token.xml_tag();
            let value = token.token.xml_value();
            self.write_terminal(tag, &value);
        }
    }

    fn write_class_impl(&mut self, class: &Class, ctx: &mut XmlContext) {
        self.open_tag("class");

        // 'class'
        self.write_token(ctx);
        // className
        self.write_token(ctx);
        // '{'
        self.write_token(ctx);

        for dec in &class.class_var_decs {
            self.write_class_var_dec(dec, ctx);
        }

        for sub in &class.subroutine_decs {
            self.write_subroutine_dec(sub, ctx);
        }

        // '}'
        self.write_token(ctx);

        self.close_tag("class");
    }

    fn write_class_var_dec(&mut self, dec: &ClassVarDec, ctx: &mut XmlContext) {
        self.open_tag("classVarDec");

        // 'static' | 'field'
        self.write_token(ctx);
        // type
        self.write_token(ctx);
        // varName
        self.write_token(ctx);

        // (',' varName)*
        for _ in 1..dec.names.len() {
            self.write_token(ctx); // ','
            self.write_token(ctx); // varName
        }

        // ';'
        self.write_token(ctx);

        self.close_tag("classVarDec");
    }

    fn write_subroutine_dec(&mut self, sub: &SubroutineDec, ctx: &mut XmlContext) {
        self.open_tag("subroutineDec");

        // 'constructor' | 'function' | 'method'
        self.write_token(ctx);
        // 'void' | type
        self.write_token(ctx);
        // subroutineName
        self.write_token(ctx);
        // '('
        self.write_token(ctx);

        self.write_parameter_list(&sub.parameters, ctx);

        // ')'
        self.write_token(ctx);

        self.write_subroutine_body(&sub.body, ctx);

        self.close_tag("subroutineDec");
    }

    fn write_parameter_list(&mut self, params: &[Parameter], ctx: &mut XmlContext) {
        self.open_tag("parameterList");

        if !params.is_empty() {
            // type varName
            self.write_token(ctx);
            self.write_token(ctx);

            for _ in 1..params.len() {
                // ',' type varName
                self.write_token(ctx);
                self.write_token(ctx);
                self.write_token(ctx);
            }
        }

        self.close_tag("parameterList");
    }

    fn write_subroutine_body(&mut self, body: &SubroutineBody, ctx: &mut XmlContext) {
        self.open_tag("subroutineBody");

        // '{'
        self.write_token(ctx);

        for dec in &body.var_decs {
            self.write_var_dec(dec, ctx);
        }

        self.write_statements(&body.statements, ctx);

        // '}'
        self.write_token(ctx);

        self.close_tag("subroutineBody");
    }

    fn write_var_dec(&mut self, dec: &VarDec, ctx: &mut XmlContext) {
        self.open_tag("varDec");

        // 'var'
        self.write_token(ctx);
        // type
        self.write_token(ctx);
        // varName
        self.write_token(ctx);

        for _ in 1..dec.names.len() {
            // ',' varName
            self.write_token(ctx);
            self.write_token(ctx);
        }

        // ';'
        self.write_token(ctx);

        self.close_tag("varDec");
    }

    fn write_statements(&mut self, statements: &[Statement], ctx: &mut XmlContext) {
        self.open_tag("statements");

        for stmt in statements {
            match stmt {
                Statement::Let(s) => self.write_let_statement(s, ctx),
                Statement::If(s) => self.write_if_statement(s, ctx),
                Statement::While(s) => self.write_while_statement(s, ctx),
                Statement::Do(s) => self.write_do_statement(s, ctx),
                Statement::Return(s) => self.write_return_statement(s, ctx),
            }
        }

        self.close_tag("statements");
    }

    fn write_let_statement(&mut self, stmt: &LetStatement, ctx: &mut XmlContext) {
        self.open_tag("letStatement");

        // 'let'
        self.write_token(ctx);
        // varName
        self.write_token(ctx);

        if let Some(index) = &stmt.index {
            // '['
            self.write_token(ctx);
            self.write_expression(index, ctx);
            // ']'
            self.write_token(ctx);
        }

        // '='
        self.write_token(ctx);
        self.write_expression(&stmt.value, ctx);
        // ';'
        self.write_token(ctx);

        self.close_tag("letStatement");
    }

    fn write_if_statement(&mut self, stmt: &IfStatement, ctx: &mut XmlContext) {
        self.open_tag("ifStatement");

        // 'if'
        self.write_token(ctx);
        // '('
        self.write_token(ctx);
        self.write_expression(&stmt.condition, ctx);
        // ')'
        self.write_token(ctx);
        // '{'
        self.write_token(ctx);
        self.write_statements(&stmt.then_statements, ctx);
        // '}'
        self.write_token(ctx);

        if let Some(else_stmts) = &stmt.else_statements {
            // 'else'
            self.write_token(ctx);
            // '{'
            self.write_token(ctx);
            self.write_statements(else_stmts, ctx);
            // '}'
            self.write_token(ctx);
        }

        self.close_tag("ifStatement");
    }

    fn write_while_statement(&mut self, stmt: &WhileStatement, ctx: &mut XmlContext) {
        self.open_tag("whileStatement");

        // 'while'
        self.write_token(ctx);
        // '('
        self.write_token(ctx);
        self.write_expression(&stmt.condition, ctx);
        // ')'
        self.write_token(ctx);
        // '{'
        self.write_token(ctx);
        self.write_statements(&stmt.statements, ctx);
        // '}'
        self.write_token(ctx);

        self.close_tag("whileStatement");
    }

    fn write_do_statement(&mut self, stmt: &DoStatement, ctx: &mut XmlContext) {
        self.open_tag("doStatement");

        // 'do'
        self.write_token(ctx);
        self.write_subroutine_call(&stmt.call, ctx);
        // ';'
        self.write_token(ctx);

        self.close_tag("doStatement");
    }

    fn write_return_statement(&mut self, stmt: &ReturnStatement, ctx: &mut XmlContext) {
        self.open_tag("returnStatement");

        // 'return'
        self.write_token(ctx);

        if let Some(ref value) = stmt.value {
            self.write_expression(value, ctx);
        }

        // ';'
        self.write_token(ctx);

        self.close_tag("returnStatement");
    }

    fn write_expression(&mut self, expr: &Expression, ctx: &mut XmlContext) {
        self.open_tag("expression");

        self.write_term(&expr.term, ctx);

        for (_, term) in &expr.ops {
            // op
            self.write_token(ctx);
            self.write_term(term, ctx);
        }

        self.close_tag("expression");
    }

    fn write_term(&mut self, term: &Term, ctx: &mut XmlContext) {
        self.open_tag("term");

        match term {
            Term::IntegerConstant(_, _) => {
                self.write_token(ctx);
            }
            Term::StringConstant(_, _) => {
                self.write_token(ctx);
            }
            Term::KeywordConstant(_, _) => {
                self.write_token(ctx);
            }
            Term::VarName(_, _) => {
                self.write_token(ctx);
            }
            Term::ArrayAccess(_, expr, _) => {
                // varName
                self.write_token(ctx);
                // '['
                self.write_token(ctx);
                self.write_expression(expr, ctx);
                // ']'
                self.write_token(ctx);
            }
            Term::SubroutineCall(call) => {
                self.write_subroutine_call(call, ctx);
            }
            Term::Parenthesized(expr, _) => {
                // '('
                self.write_token(ctx);
                self.write_expression(expr, ctx);
                // ')'
                self.write_token(ctx);
            }
            Term::UnaryOp(_, inner, _) => {
                // unaryOp
                self.write_token(ctx);
                self.write_term(inner, ctx);
            }
        }

        self.close_tag("term");
    }

    fn write_subroutine_call(&mut self, call: &SubroutineCall, ctx: &mut XmlContext) {
        if call.receiver.is_some() {
            // className | varName
            self.write_token(ctx);
            // '.'
            self.write_token(ctx);
        }

        // subroutineName
        self.write_token(ctx);
        // '('
        self.write_token(ctx);
        self.write_expression_list(&call.arguments, ctx);
        // ')'
        self.write_token(ctx);
    }

    fn write_expression_list(&mut self, exprs: &[Expression], ctx: &mut XmlContext) {
        self.open_tag("expressionList");

        if !exprs.is_empty() {
            self.write_expression(&exprs[0], ctx);

            for expr in &exprs[1..] {
                // ','
                self.write_token(ctx);
                self.write_expression(expr, ctx);
            }
        }

        self.close_tag("expressionList");
    }
}

impl Default for XmlWriter {
    fn default() -> Self {
        Self::new()
    }
}

/// Context for tracking token position during XML generation.
struct XmlContext<'a> {
    tokens: &'a [SpannedToken],
    pos: usize,
}

impl<'a> XmlContext<'a> {
    fn new(tokens: &'a [SpannedToken]) -> Self {
        Self { tokens, pos: 0 }
    }

    #[inline]
    fn advance(&mut self) -> Option<&'a SpannedToken> {
        if self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use crate::tokenizer::JackTokenizer;

    #[test]
    fn test_tokens_to_xml() {
        let tokens = JackTokenizer::new("class Main { }").tokenize().unwrap();
        let xml = tokens_to_xml(&tokens);
        assert!(xml.contains("<keyword> class </keyword>"));
        assert!(xml.contains("<identifier> Main </identifier>"));
        assert!(xml.contains("<symbol> { </symbol>"));
    }

    #[test]
    fn test_class_to_xml() {
        let input = "class Main { }";
        let tokens = JackTokenizer::new(input).tokenize().unwrap();
        let class = Parser::new(&tokens).parse().unwrap();
        let xml = XmlWriter::new().write_class(&class, &tokens);
        assert!(xml.contains("<class>"));
        assert!(xml.contains("</class>"));
        assert!(xml.contains("<keyword> class </keyword>"));
    }

    #[test]
    fn test_xml_escaping() {
        let tokens = JackTokenizer::new(
            "class Main { field int x; function void test() { if (x < 5) { return; } return; } }",
        )
        .tokenize()
        .unwrap();
        let xml = tokens_to_xml(&tokens);
        assert!(xml.contains("<symbol> &lt; </symbol>"));
    }

    #[test]
    fn test_with_capacity() {
        let input = "class Main { function void main() { return; } }";
        let tokens = JackTokenizer::new(input).tokenize().unwrap();
        let class = Parser::new(&tokens).parse().unwrap();
        // Use with_capacity for better pre-allocation
        let xml = XmlWriter::with_capacity(tokens.len()).write_class(&class, &tokens);
        assert!(xml.contains("<class>"));
        assert!(xml.contains("<subroutineDec>"));
    }

    #[test]
    fn test_pre_allocation() {
        // Verify that pre-allocation reduces reallocations
        let tokens = JackTokenizer::new("class Main { }").tokenize().unwrap();
        let xml = tokens_to_xml(&tokens);
        // Output should fit in pre-allocated buffer (no reallocation needed)
        assert!(xml.len() < tokens.len() * BYTES_PER_TOKEN + 21);
    }
}
