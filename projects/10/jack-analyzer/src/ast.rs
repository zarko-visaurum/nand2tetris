//! Abstract Syntax Tree definitions for the Jack language.
//!
//! These AST nodes are designed to support:
//! 1. XML output generation (Project 10)
//! 2. Visitor pattern for code generation (Project 11)

use crate::token::{Keyword, Span};

/// A complete Jack class.
#[derive(Debug, Clone)]
pub struct Class {
    pub name: String,
    pub class_var_decs: Vec<ClassVarDec>,
    pub subroutine_decs: Vec<SubroutineDec>,
    pub span: Span,
}

/// Class variable declaration (static or field).
#[derive(Debug, Clone)]
pub struct ClassVarDec {
    pub kind: ClassVarKind,
    pub var_type: Type,
    pub names: Vec<String>,
    pub span: Span,
}

/// Kind of class variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassVarKind {
    Static,
    Field,
}

impl ClassVarKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ClassVarKind::Static => "static",
            ClassVarKind::Field => "field",
        }
    }
}

/// Type specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Char,
    Boolean,
    ClassName(String),
}

impl Type {
    pub fn as_str(&self) -> String {
        match self {
            Type::Int => "int".to_string(),
            Type::Char => "char".to_string(),
            Type::Boolean => "boolean".to_string(),
            Type::ClassName(name) => name.clone(),
        }
    }
}

/// Subroutine declaration (constructor, function, or method).
#[derive(Debug, Clone)]
pub struct SubroutineDec {
    pub kind: SubroutineKind,
    pub return_type: ReturnType,
    pub name: String,
    pub parameters: Vec<Parameter>,
    pub body: SubroutineBody,
    pub span: Span,
}

/// Kind of subroutine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubroutineKind {
    Constructor,
    Function,
    Method,
}

impl SubroutineKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SubroutineKind::Constructor => "constructor",
            SubroutineKind::Function => "function",
            SubroutineKind::Method => "method",
        }
    }
}

/// Return type (void or a type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnType {
    Void,
    Type(Type),
}

impl ReturnType {
    pub fn as_str(&self) -> String {
        match self {
            ReturnType::Void => "void".to_string(),
            ReturnType::Type(t) => t.as_str(),
        }
    }
}

/// Subroutine parameter.
#[derive(Debug, Clone)]
pub struct Parameter {
    pub var_type: Type,
    pub name: String,
}

/// Subroutine body.
#[derive(Debug, Clone)]
pub struct SubroutineBody {
    pub var_decs: Vec<VarDec>,
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Local variable declaration.
#[derive(Debug, Clone)]
pub struct VarDec {
    pub var_type: Type,
    pub names: Vec<String>,
    pub span: Span,
}

/// Statement types.
#[derive(Debug, Clone)]
pub enum Statement {
    Let(LetStatement),
    If(IfStatement),
    While(WhileStatement),
    Do(DoStatement),
    Return(ReturnStatement),
}

/// Let statement: let varName[expr]? = expr;
#[derive(Debug, Clone)]
pub struct LetStatement {
    pub var_name: String,
    pub index: Option<Box<Expression>>,
    pub value: Expression,
    pub span: Span,
}

/// If statement: if (expr) { statements } (else { statements })?
#[derive(Debug, Clone)]
pub struct IfStatement {
    pub condition: Expression,
    pub then_statements: Vec<Statement>,
    pub else_statements: Option<Vec<Statement>>,
    pub span: Span,
}

/// While statement: while (expr) { statements }
#[derive(Debug, Clone)]
pub struct WhileStatement {
    pub condition: Expression,
    pub statements: Vec<Statement>,
    pub span: Span,
}

/// Do statement: do subroutineCall;
#[derive(Debug, Clone)]
pub struct DoStatement {
    pub call: SubroutineCall,
    pub span: Span,
}

/// Return statement: return expr?;
#[derive(Debug, Clone)]
pub struct ReturnStatement {
    pub value: Option<Expression>,
    pub span: Span,
}

/// Expression: term (op term)*
#[derive(Debug, Clone)]
pub struct Expression {
    pub term: Term,
    pub ops: Vec<(BinaryOp, Term)>,
    pub span: Span,
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    And, // &
    Or,  // |
    Lt,  // <
    Gt,  // >
    Eq,  // =
}

impl BinaryOp {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '+' => Some(BinaryOp::Add),
            '-' => Some(BinaryOp::Sub),
            '*' => Some(BinaryOp::Mul),
            '/' => Some(BinaryOp::Div),
            '&' => Some(BinaryOp::And),
            '|' => Some(BinaryOp::Or),
            '<' => Some(BinaryOp::Lt),
            '>' => Some(BinaryOp::Gt),
            '=' => Some(BinaryOp::Eq),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_char(&self) -> char {
        match self {
            BinaryOp::Add => '+',
            BinaryOp::Sub => '-',
            BinaryOp::Mul => '*',
            BinaryOp::Div => '/',
            BinaryOp::And => '&',
            BinaryOp::Or => '|',
            BinaryOp::Lt => '<',
            BinaryOp::Gt => '>',
            BinaryOp::Eq => '=',
        }
    }
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg, // -
    Not, // ~
}

impl UnaryOp {
    pub fn from_char(c: char) -> Option<Self> {
        match c {
            '-' => Some(UnaryOp::Neg),
            '~' => Some(UnaryOp::Not),
            _ => None,
        }
    }

    #[allow(dead_code)]
    pub fn as_char(&self) -> char {
        match self {
            UnaryOp::Neg => '-',
            UnaryOp::Not => '~',
        }
    }
}

/// Term in an expression.
#[derive(Debug, Clone)]
pub enum Term {
    IntegerConstant(u16, Span),
    StringConstant(String, Span),
    KeywordConstant(KeywordConstant, Span),
    VarName(String, Span),
    ArrayAccess(String, Box<Expression>, Span),
    SubroutineCall(SubroutineCall),
    Parenthesized(Box<Expression>, Span),
    UnaryOp(UnaryOp, Box<Term>, Span),
}

impl Term {
    pub fn span(&self) -> &Span {
        match self {
            Term::IntegerConstant(_, span) => span,
            Term::StringConstant(_, span) => span,
            Term::KeywordConstant(_, span) => span,
            Term::VarName(_, span) => span,
            Term::ArrayAccess(_, _, span) => span,
            Term::SubroutineCall(call) => &call.span,
            Term::Parenthesized(_, span) => span,
            Term::UnaryOp(_, _, span) => span,
        }
    }
}

/// Keyword constants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeywordConstant {
    True,
    False,
    Null,
    This,
}

impl KeywordConstant {
    pub fn from_keyword(k: Keyword) -> Option<Self> {
        match k {
            Keyword::True => Some(KeywordConstant::True),
            Keyword::False => Some(KeywordConstant::False),
            Keyword::Null => Some(KeywordConstant::Null),
            Keyword::This => Some(KeywordConstant::This),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            KeywordConstant::True => "true",
            KeywordConstant::False => "false",
            KeywordConstant::Null => "null",
            KeywordConstant::This => "this",
        }
    }
}

/// Subroutine call.
#[derive(Debug, Clone)]
pub struct SubroutineCall {
    /// Optional class/variable name for method calls.
    pub receiver: Option<String>,
    pub name: String,
    pub arguments: Vec<Expression>,
    pub span: Span,
}

/// Visitor trait for AST traversal (Project 11 extension point).
#[allow(dead_code)]
pub trait AstVisitor {
    fn visit_class(&mut self, class: &Class);
    fn visit_class_var_dec(&mut self, dec: &ClassVarDec);
    fn visit_subroutine(&mut self, sub: &SubroutineDec);
    fn visit_parameter(&mut self, param: &Parameter);
    fn visit_var_dec(&mut self, dec: &VarDec);
    fn visit_statements(&mut self, statements: &[Statement]);
    fn visit_statement(&mut self, stmt: &Statement);
    fn visit_expression(&mut self, expr: &Expression);
    fn visit_term(&mut self, term: &Term);
}

/// Default visitor implementation that walks the entire tree.
#[allow(dead_code)]
pub trait AstWalker: AstVisitor {
    fn walk_class(&mut self, class: &Class) {
        for dec in &class.class_var_decs {
            self.visit_class_var_dec(dec);
        }
        for sub in &class.subroutine_decs {
            self.visit_subroutine(sub);
        }
    }

    fn walk_subroutine(&mut self, sub: &SubroutineDec) {
        for param in &sub.parameters {
            self.visit_parameter(param);
        }
        for dec in &sub.body.var_decs {
            self.visit_var_dec(dec);
        }
        self.visit_statements(&sub.body.statements);
    }

    fn walk_statements(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.visit_statement(stmt);
        }
    }

    fn walk_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let(s) => {
                if let Some(idx) = &s.index {
                    self.visit_expression(idx);
                }
                self.visit_expression(&s.value);
            }
            Statement::If(s) => {
                self.visit_expression(&s.condition);
                self.visit_statements(&s.then_statements);
                if let Some(else_stmts) = &s.else_statements {
                    self.visit_statements(else_stmts);
                }
            }
            Statement::While(s) => {
                self.visit_expression(&s.condition);
                self.visit_statements(&s.statements);
            }
            Statement::Do(s) => {
                for arg in &s.call.arguments {
                    self.visit_expression(arg);
                }
            }
            Statement::Return(s) => {
                if let Some(val) = &s.value {
                    self.visit_expression(val);
                }
            }
        }
    }

    fn walk_expression(&mut self, expr: &Expression) {
        self.visit_term(&expr.term);
        for (_, term) in &expr.ops {
            self.visit_term(term);
        }
    }

    fn walk_term(&mut self, term: &Term) {
        match term {
            Term::ArrayAccess(_, expr, _) => {
                self.visit_expression(expr);
            }
            Term::SubroutineCall(call) => {
                for arg in &call.arguments {
                    self.visit_expression(arg);
                }
            }
            Term::Parenthesized(expr, _) => {
                self.visit_expression(expr);
            }
            Term::UnaryOp(_, inner, _) => {
                self.visit_term(inner);
            }
            _ => {}
        }
    }
}
