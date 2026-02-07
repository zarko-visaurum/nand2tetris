//! Recursive descent parser (Compilation Engine) for the Jack language.

use crate::ast::*;
use crate::error::{ErrorAccumulator, JackError};
use crate::token::{Keyword, Span, SpannedToken, Token};

/// Maximum expression nesting depth before the parser bails out.
/// Prevents stack overflow on pathological input (e.g., `(((((...)))))`).
/// 25 is generous for real Jack programs (typical nesting: 3-5 levels).
/// Kept low enough to fit in the default 8 MB thread stack in debug builds.
const MAX_DEPTH: usize = 25;

/// Recursive descent parser for Jack language.
pub struct Parser<'a> {
    tokens: &'a [SpannedToken],
    pos: usize,
    errors: ErrorAccumulator,
    depth: usize,
}

impl<'a> Parser<'a> {
    /// Create a new parser for the given tokens.
    pub fn new(tokens: &'a [SpannedToken]) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: ErrorAccumulator::new(),
            depth: 0,
        }
    }

    /// Parse the tokens into a Class AST.
    pub fn parse(mut self) -> Result<Class, Vec<JackError>> {
        let class = self.parse_class();

        if self.errors.has_errors() {
            Err(self.errors.into_errors())
        } else {
            Ok(class)
        }
    }

    // ========================================================================
    // Helper methods
    // ========================================================================

    fn is_at_end(&self) -> bool {
        self.pos >= self.tokens.len()
    }

    fn current(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.pos)
    }

    fn current_span(&self) -> Span {
        self.current()
            .map(|t| t.span.clone())
            .unwrap_or_else(|| Span::new(0, 0, 1, 1))
    }

    fn peek_token(&self) -> Option<&Token> {
        self.current().map(|t| &t.token)
    }

    fn peek_keyword(&self) -> Option<Keyword> {
        match self.peek_token() {
            Some(Token::Keyword(k)) => Some(*k),
            _ => None,
        }
    }

    fn peek_symbol(&self) -> Option<char> {
        match self.peek_token() {
            Some(Token::Symbol(c)) => Some(*c),
            _ => None,
        }
    }

    fn advance(&mut self) -> Option<&SpannedToken> {
        if self.is_at_end() {
            None
        } else {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            Some(token)
        }
    }

    fn expect_keyword(&mut self, keyword: Keyword) -> Option<Span> {
        if self.peek_keyword() == Some(keyword) {
            Some(self.advance().unwrap().span.clone())
        } else {
            let span = self.current_span();
            let got = self
                .peek_token()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "end of file".to_string());
            self.errors.push(JackError::syntax_expected(
                span.clone(),
                format!("expected keyword '{}', got {}", keyword.as_str(), got),
                vec![keyword.as_str().to_string()],
            ));
            None
        }
    }

    fn expect_symbol(&mut self, symbol: char) -> Option<Span> {
        if self.peek_symbol() == Some(symbol) {
            Some(self.advance().unwrap().span.clone())
        } else {
            let span = self.current_span();
            let got = self
                .peek_token()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "end of file".to_string());
            self.errors.push(JackError::syntax_expected(
                span.clone(),
                format!("expected '{}', got {}", symbol, got),
                vec![symbol.to_string()],
            ));
            None
        }
    }

    fn expect_identifier(&mut self) -> Option<(String, Span)> {
        if let Some(Token::Identifier(name)) = self.peek_token().cloned() {
            let span = self.advance().unwrap().span.clone();
            Some((name, span))
        } else {
            let span = self.current_span();
            let got = self
                .peek_token()
                .map(|t| t.to_string())
                .unwrap_or_else(|| "end of file".to_string());
            self.errors.push(JackError::syntax_expected(
                span.clone(),
                format!("expected identifier, got {}", got),
                vec!["identifier".to_string()],
            ));
            None
        }
    }

    /// Synchronize after an error by advancing to a recovery point.
    fn synchronize(&mut self) {
        while !self.is_at_end() {
            // Sync at statement keywords
            if let Some(
                Keyword::Let
                | Keyword::If
                | Keyword::While
                | Keyword::Do
                | Keyword::Return
                | Keyword::Static
                | Keyword::Field
                | Keyword::Constructor
                | Keyword::Function
                | Keyword::Method,
            ) = self.peek_keyword()
            {
                return;
            }

            // Sync at closing brace
            if self.peek_symbol() == Some('}') {
                return;
            }

            // Sync after semicolon
            if self.peek_symbol() == Some(';') {
                self.advance();
                return;
            }

            self.advance();
        }
    }

    // ========================================================================
    // Grammar rules
    // ========================================================================

    /// class: 'class' className '{' classVarDec* subroutineDec* '}'
    fn parse_class(&mut self) -> Class {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::Class);
        let name = self.expect_identifier().map(|(n, _)| n).unwrap_or_default();
        self.expect_symbol('{');

        let mut class_var_decs = Vec::new();
        while matches!(self.peek_keyword(), Some(Keyword::Static | Keyword::Field)) {
            if let Some(dec) = self.parse_class_var_dec() {
                class_var_decs.push(dec);
            }
        }

        let mut subroutine_decs = Vec::new();
        while matches!(
            self.peek_keyword(),
            Some(Keyword::Constructor | Keyword::Function | Keyword::Method)
        ) {
            if let Some(dec) = self.parse_subroutine_dec() {
                subroutine_decs.push(dec);
            }
        }

        self.expect_symbol('}');

        Class {
            name,
            class_var_decs,
            subroutine_decs,
            span: start_span,
        }
    }

    /// classVarDec: ('static' | 'field') type varName (',' varName)* ';'
    fn parse_class_var_dec(&mut self) -> Option<ClassVarDec> {
        let start_span = self.current_span();

        let kind = match self.peek_keyword() {
            Some(Keyword::Static) => {
                self.advance();
                ClassVarKind::Static
            }
            Some(Keyword::Field) => {
                self.advance();
                ClassVarKind::Field
            }
            _ => {
                self.errors.push(JackError::syntax(
                    self.current_span(),
                    "expected 'static' or 'field'",
                ));
                self.synchronize();
                return None;
            }
        };

        let var_type = self.parse_type()?;

        let mut names = Vec::new();
        if let Some((name, _)) = self.expect_identifier() {
            names.push(name);
        }

        while self.peek_symbol() == Some(',') {
            self.advance();
            if let Some((name, _)) = self.expect_identifier() {
                names.push(name);
            }
        }

        self.expect_symbol(';');

        Some(ClassVarDec {
            kind,
            var_type,
            names,
            span: start_span,
        })
    }

    /// type: 'int' | 'char' | 'boolean' | className
    fn parse_type(&mut self) -> Option<Type> {
        match self.peek_token() {
            Some(Token::Keyword(Keyword::Int)) => {
                self.advance();
                Some(Type::Int)
            }
            Some(Token::Keyword(Keyword::Char)) => {
                self.advance();
                Some(Type::Char)
            }
            Some(Token::Keyword(Keyword::Boolean)) => {
                self.advance();
                Some(Type::Boolean)
            }
            Some(Token::Identifier(name)) => {
                let name = name.clone();
                self.advance();
                Some(Type::ClassName(name))
            }
            _ => {
                let got = self
                    .peek_token()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "end of file".to_string());
                self.errors.push(JackError::syntax(
                    self.current_span(),
                    format!(
                        "expected type (int, char, boolean, or class name), got {}",
                        got
                    ),
                ));
                None
            }
        }
    }

    /// subroutineDec: ('constructor'|'function'|'method') ('void'|type) subroutineName '(' parameterList ')' subroutineBody
    fn parse_subroutine_dec(&mut self) -> Option<SubroutineDec> {
        let start_span = self.current_span();

        let kind = match self.peek_keyword() {
            Some(Keyword::Constructor) => {
                self.advance();
                SubroutineKind::Constructor
            }
            Some(Keyword::Function) => {
                self.advance();
                SubroutineKind::Function
            }
            Some(Keyword::Method) => {
                self.advance();
                SubroutineKind::Method
            }
            _ => {
                self.errors.push(JackError::syntax(
                    self.current_span(),
                    "expected 'constructor', 'function', or 'method'",
                ));
                self.synchronize();
                return None;
            }
        };

        let return_type = if self.peek_keyword() == Some(Keyword::Void) {
            self.advance();
            ReturnType::Void
        } else {
            ReturnType::Type(self.parse_type()?)
        };

        let name = self.expect_identifier().map(|(n, _)| n).unwrap_or_default();

        self.expect_symbol('(');
        let parameters = self.parse_parameter_list();
        self.expect_symbol(')');

        let body = self.parse_subroutine_body();

        Some(SubroutineDec {
            kind,
            return_type,
            name,
            parameters,
            body,
            span: start_span,
        })
    }

    /// parameterList: ((type varName) (',' type varName)*)?
    fn parse_parameter_list(&mut self) -> Vec<Parameter> {
        let mut params = Vec::new();

        if self.peek_symbol() == Some(')') {
            return params;
        }

        if let Some(var_type) = self.parse_type()
            && let Some((name, _)) = self.expect_identifier()
        {
            params.push(Parameter { var_type, name });
        }

        while self.peek_symbol() == Some(',') {
            self.advance();
            if let Some(var_type) = self.parse_type()
                && let Some((name, _)) = self.expect_identifier()
            {
                params.push(Parameter { var_type, name });
            }
        }

        params
    }

    /// subroutineBody: '{' varDec* statements '}'
    fn parse_subroutine_body(&mut self) -> SubroutineBody {
        let start_span = self.current_span();

        self.expect_symbol('{');

        let mut var_decs = Vec::new();
        while self.peek_keyword() == Some(Keyword::Var) {
            if let Some(dec) = self.parse_var_dec() {
                var_decs.push(dec);
            }
        }

        let statements = self.parse_statements();

        self.expect_symbol('}');

        SubroutineBody {
            var_decs,
            statements,
            span: start_span,
        }
    }

    /// varDec: 'var' type varName (',' varName)* ';'
    fn parse_var_dec(&mut self) -> Option<VarDec> {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::Var)?;
        let var_type = self.parse_type()?;

        let mut names = Vec::new();
        if let Some((name, _)) = self.expect_identifier() {
            names.push(name);
        }

        while self.peek_symbol() == Some(',') {
            self.advance();
            if let Some((name, _)) = self.expect_identifier() {
                names.push(name);
            }
        }

        self.expect_symbol(';');

        Some(VarDec {
            var_type,
            names,
            span: start_span,
        })
    }

    /// statements: statement*
    fn parse_statements(&mut self) -> Vec<Statement> {
        let mut statements = Vec::new();

        loop {
            match self.peek_keyword() {
                Some(Keyword::Let) => {
                    if let Some(stmt) = self.parse_let_statement() {
                        statements.push(Statement::Let(stmt));
                    }
                }
                Some(Keyword::If) => {
                    if let Some(stmt) = self.parse_if_statement() {
                        statements.push(Statement::If(stmt));
                    }
                }
                Some(Keyword::While) => {
                    if let Some(stmt) = self.parse_while_statement() {
                        statements.push(Statement::While(stmt));
                    }
                }
                Some(Keyword::Do) => {
                    if let Some(stmt) = self.parse_do_statement() {
                        statements.push(Statement::Do(stmt));
                    }
                }
                Some(Keyword::Return) => {
                    if let Some(stmt) = self.parse_return_statement() {
                        statements.push(Statement::Return(stmt));
                    }
                }
                _ => break,
            }

            if self.errors.is_full() {
                break;
            }
        }

        statements
    }

    /// letStatement: 'let' varName ('[' expression ']')? '=' expression ';'
    fn parse_let_statement(&mut self) -> Option<LetStatement> {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::Let)?;
        let (var_name, _) = self.expect_identifier()?;

        let index = if self.peek_symbol() == Some('[') {
            self.advance();
            let expr = self.parse_expression()?;
            self.expect_symbol(']');
            Some(Box::new(expr))
        } else {
            None
        };

        self.expect_symbol('=');
        let value = self.parse_expression()?;
        self.expect_symbol(';');

        Some(LetStatement {
            var_name,
            index,
            value,
            span: start_span,
        })
    }

    /// ifStatement: 'if' '(' expression ')' '{' statements '}' ('else' '{' statements '}')?
    fn parse_if_statement(&mut self) -> Option<IfStatement> {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::If)?;
        self.expect_symbol('(');
        let condition = self.parse_expression()?;
        self.expect_symbol(')');
        self.expect_symbol('{');
        let then_statements = self.parse_statements();
        self.expect_symbol('}');

        let else_statements = if self.peek_keyword() == Some(Keyword::Else) {
            self.advance();
            self.expect_symbol('{');
            let stmts = self.parse_statements();
            self.expect_symbol('}');
            Some(stmts)
        } else {
            None
        };

        Some(IfStatement {
            condition,
            then_statements,
            else_statements,
            span: start_span,
        })
    }

    /// whileStatement: 'while' '(' expression ')' '{' statements '}'
    fn parse_while_statement(&mut self) -> Option<WhileStatement> {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::While)?;
        self.expect_symbol('(');
        let condition = self.parse_expression()?;
        self.expect_symbol(')');
        self.expect_symbol('{');
        let statements = self.parse_statements();
        self.expect_symbol('}');

        Some(WhileStatement {
            condition,
            statements,
            span: start_span,
        })
    }

    /// doStatement: 'do' subroutineCall ';'
    fn parse_do_statement(&mut self) -> Option<DoStatement> {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::Do)?;
        let call = self.parse_subroutine_call()?;
        self.expect_symbol(';');

        Some(DoStatement {
            call,
            span: start_span,
        })
    }

    /// returnStatement: 'return' expression? ';'
    fn parse_return_statement(&mut self) -> Option<ReturnStatement> {
        let start_span = self.current_span();

        self.expect_keyword(Keyword::Return)?;

        let value = if self.peek_symbol() != Some(';') {
            Some(self.parse_expression()?)
        } else {
            None
        };

        self.expect_symbol(';');

        Some(ReturnStatement {
            value,
            span: start_span,
        })
    }

    /// expression: term (op term)*
    fn parse_expression(&mut self) -> Option<Expression> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.errors.push(JackError::syntax(
                self.current_span(),
                "expression nesting too deep".to_string(),
            ));
            self.depth -= 1;
            return None;
        }
        let result = self.parse_expression_inner();
        self.depth -= 1;
        result
    }

    /// Inner expression parsing logic, separated to guarantee depth decrement.
    fn parse_expression_inner(&mut self) -> Option<Expression> {
        let start_span = self.current_span();

        let term = self.parse_term()?;
        let mut ops = Vec::new();

        while let Some(c) = self.peek_symbol() {
            if let Some(op) = BinaryOp::from_char(c) {
                self.advance();
                if let Some(next_term) = self.parse_term() {
                    ops.push((op, next_term));
                }
            } else {
                break;
            }
        }

        Some(Expression {
            term,
            ops,
            span: start_span,
        })
    }

    /// term: integerConstant | stringConstant | keywordConstant | varName | varName'['expression']' | subroutineCall | '('expression')' | unaryOp term
    fn parse_term(&mut self) -> Option<Term> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            self.errors.push(JackError::syntax(
                self.current_span(),
                "expression nesting too deep".to_string(),
            ));
            self.depth -= 1;
            return None;
        }
        let result = self.parse_term_inner();
        self.depth -= 1;
        result
    }

    /// Inner term parsing logic, separated to guarantee depth decrement on all paths.
    fn parse_term_inner(&mut self) -> Option<Term> {
        let start_span = self.current_span();

        match self.peek_token().cloned() {
            Some(Token::IntegerConstant(n)) => {
                self.advance();
                Some(Term::IntegerConstant(n, start_span))
            }
            Some(Token::StringConstant(s)) => {
                self.advance();
                Some(Term::StringConstant(s, start_span))
            }
            Some(Token::Keyword(k)) => {
                if let Some(kc) = KeywordConstant::from_keyword(k) {
                    self.advance();
                    Some(Term::KeywordConstant(kc, start_span))
                } else {
                    self.errors.push(JackError::syntax(
                        start_span,
                        format!("unexpected keyword '{}'", k.as_str()),
                    ));
                    None
                }
            }
            Some(Token::Symbol('(')) => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect_symbol(')');
                Some(Term::Parenthesized(Box::new(expr), start_span))
            }
            Some(Token::Symbol(c)) if c == '-' || c == '~' => {
                self.advance();
                let op = UnaryOp::from_char(c).unwrap();
                let term = self.parse_term()?;
                Some(Term::UnaryOp(op, Box::new(term), start_span))
            }
            Some(Token::Identifier(name)) => {
                self.advance();

                match self.peek_symbol() {
                    Some('[') => {
                        // Array access
                        self.advance();
                        let index = self.parse_expression()?;
                        self.expect_symbol(']');
                        Some(Term::ArrayAccess(name, Box::new(index), start_span))
                    }
                    Some('(') => {
                        // Subroutine call: name(args)
                        self.advance();
                        let arguments = self.parse_expression_list();
                        self.expect_symbol(')');
                        Some(Term::SubroutineCall(SubroutineCall {
                            receiver: None,
                            name,
                            arguments,
                            span: start_span,
                        }))
                    }
                    Some('.') => {
                        // Method call: receiver.name(args)
                        self.advance();
                        let (method_name, _) = self.expect_identifier()?;
                        self.expect_symbol('(');
                        let arguments = self.parse_expression_list();
                        self.expect_symbol(')');
                        Some(Term::SubroutineCall(SubroutineCall {
                            receiver: Some(name),
                            name: method_name,
                            arguments,
                            span: start_span,
                        }))
                    }
                    _ => {
                        // Simple variable
                        Some(Term::VarName(name, start_span))
                    }
                }
            }
            _ => {
                let got = self
                    .peek_token()
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "end of file".to_string());
                self.errors.push(JackError::syntax(
                    start_span,
                    format!("expected term, got {}", got),
                ));
                self.synchronize();
                None
            }
        }
    }

    /// subroutineCall: subroutineName '(' expressionList ')' | (className | varName) '.' subroutineName '(' expressionList ')'
    fn parse_subroutine_call(&mut self) -> Option<SubroutineCall> {
        let start_span = self.current_span();

        let (first_name, _) = self.expect_identifier()?;

        let (receiver, name) = if self.peek_symbol() == Some('.') {
            self.advance();
            let (method_name, _) = self.expect_identifier()?;
            (Some(first_name), method_name)
        } else {
            (None, first_name)
        };

        self.expect_symbol('(');
        let arguments = self.parse_expression_list();
        self.expect_symbol(')');

        Some(SubroutineCall {
            receiver,
            name,
            arguments,
            span: start_span,
        })
    }

    /// expressionList: (expression (',' expression)*)?
    fn parse_expression_list(&mut self) -> Vec<Expression> {
        let mut exprs = Vec::new();

        if self.peek_symbol() == Some(')') {
            return exprs;
        }

        if let Some(expr) = self.parse_expression() {
            exprs.push(expr);
        }

        while self.peek_symbol() == Some(',') {
            self.advance();
            if let Some(expr) = self.parse_expression() {
                exprs.push(expr);
            }
        }

        exprs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::JackTokenizer;

    fn parse(input: &str) -> Result<Class, Vec<JackError>> {
        let tokens = JackTokenizer::new(input).tokenize().unwrap();
        Parser::new(&tokens).parse()
    }

    #[test]
    fn test_empty_class() {
        let class = parse("class Main { }").unwrap();
        assert_eq!(class.name, "Main");
        assert!(class.class_var_decs.is_empty());
        assert!(class.subroutine_decs.is_empty());
    }

    #[test]
    fn test_class_with_field() {
        let class = parse("class Point { field int x, y; }").unwrap();
        assert_eq!(class.class_var_decs.len(), 1);
        assert_eq!(class.class_var_decs[0].kind, ClassVarKind::Field);
        assert_eq!(class.class_var_decs[0].names, vec!["x", "y"]);
    }

    #[test]
    fn test_simple_function() {
        let class = parse("class Main { function void main() { return; } }").unwrap();
        assert_eq!(class.subroutine_decs.len(), 1);
        let sub = &class.subroutine_decs[0];
        assert_eq!(sub.kind, SubroutineKind::Function);
        assert_eq!(sub.name, "main");
        assert!(matches!(sub.return_type, ReturnType::Void));
    }

    #[test]
    fn test_let_statement() {
        let class = parse("class Main { function void main() { let x = 5; return; } }").unwrap();
        let stmts = &class.subroutine_decs[0].body.statements;
        assert_eq!(stmts.len(), 2);
        assert!(matches!(stmts[0], Statement::Let(_)));
    }

    #[test]
    fn test_expression() {
        let class =
            parse("class Main { function void main() { let x = 1 + 2 * 3; return; } }").unwrap();
        let stmts = &class.subroutine_decs[0].body.statements;
        if let Statement::Let(s) = &stmts[0] {
            assert_eq!(s.value.ops.len(), 2);
        } else {
            panic!("Expected let statement");
        }
    }
}
