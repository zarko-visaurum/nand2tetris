//! Two-level symbol table for the Jack compiler.
//!
//! The symbol table maintains two scopes:
//! - **Class scope**: `static` and `field` variables, persists across subroutines
//! - **Subroutine scope**: `argument` and `local` variables, reset per subroutine
//!
//! Lookup is subroutine-first, allowing local variables to shadow class-level ones.

use crate::error::CompileError;
use jack_analyzer::ast::Type;
use jack_analyzer::token::Span;
use std::collections::HashMap;

/// The kind of symbol, determining its VM segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// Class-level static variable → `static` segment
    Static,
    /// Class-level field variable → `this` segment
    Field,
    /// Subroutine argument → `argument` segment
    Argument,
    /// Subroutine local variable → `local` segment
    Local,
}

impl SymbolKind {
    /// Convert to VM segment name.
    #[inline]
    pub fn to_segment(self) -> &'static str {
        match self {
            SymbolKind::Static => "static",
            SymbolKind::Field => "this",
            SymbolKind::Argument => "argument",
            SymbolKind::Local => "local",
        }
    }

    /// Check if this is a class-level symbol.
    #[inline]
    pub fn is_class_level(self) -> bool {
        matches!(self, SymbolKind::Static | SymbolKind::Field)
    }
}

/// A symbol entry in the symbol table.
#[derive(Debug, Clone)]
pub struct Symbol {
    /// The symbol name.
    pub name: String,
    /// The symbol's type (int, char, boolean, or class name).
    pub symbol_type: Type,
    /// The kind of symbol (determines VM segment).
    pub kind: SymbolKind,
    /// The index within its segment.
    pub index: u16,
}

impl Symbol {
    /// Get the VM segment for this symbol.
    #[inline]
    pub fn segment(&self) -> &'static str {
        self.kind.to_segment()
    }
}

/// Two-level symbol table for Jack compilation.
///
/// Manages class-scope (static, field) and subroutine-scope (argument, local) symbols
/// with proper index counting per kind.
#[derive(Debug)]
pub struct SymbolTable {
    /// Class-level symbols (static and field).
    class_scope: HashMap<String, Symbol>,
    /// Subroutine-level symbols (argument and local).
    subroutine_scope: HashMap<String, Symbol>,
    /// Count of static variables.
    static_count: u16,
    /// Count of field variables.
    field_count: u16,
    /// Count of argument variables.
    argument_count: u16,
    /// Count of local variables.
    local_count: u16,
    /// Current class name.
    class_name: String,
}

impl SymbolTable {
    /// Create a new empty symbol table.
    pub fn new() -> Self {
        Self {
            class_scope: HashMap::new(),
            subroutine_scope: HashMap::new(),
            static_count: 0,
            field_count: 0,
            argument_count: 0,
            local_count: 0,
            class_name: String::new(),
        }
    }

    /// Start compiling a new class.
    ///
    /// Clears class-level symbols and resets static/field counters.
    pub fn start_class(&mut self, name: &str) {
        self.class_scope.clear();
        self.subroutine_scope.clear();
        self.static_count = 0;
        self.field_count = 0;
        self.argument_count = 0;
        self.local_count = 0;
        self.class_name = name.to_string();
    }

    /// Start compiling a new subroutine.
    ///
    /// Clears subroutine-level symbols and resets argument/local counters.
    /// Class-level symbols remain accessible.
    pub fn start_subroutine(&mut self) {
        self.subroutine_scope.clear();
        self.argument_count = 0;
        self.local_count = 0;
    }

    /// Define a new symbol in the appropriate scope.
    ///
    /// Returns an error if the symbol is already defined in the same scope.
    pub fn define(
        &mut self,
        name: &str,
        symbol_type: Type,
        kind: SymbolKind,
        span: Span,
    ) -> Result<(), CompileError> {
        // Check for duplicates in the appropriate scope
        let scope = if kind.is_class_level() {
            &self.class_scope
        } else {
            &self.subroutine_scope
        };

        if scope.contains_key(name) {
            return Err(CompileError::duplicate_definition(name, span));
        }

        // Get and increment the appropriate counter
        let index = match kind {
            SymbolKind::Static => {
                let idx = self.static_count;
                self.static_count += 1;
                idx
            }
            SymbolKind::Field => {
                let idx = self.field_count;
                self.field_count += 1;
                idx
            }
            SymbolKind::Argument => {
                let idx = self.argument_count;
                self.argument_count += 1;
                idx
            }
            SymbolKind::Local => {
                let idx = self.local_count;
                self.local_count += 1;
                idx
            }
        };

        let symbol = Symbol {
            name: name.to_string(),
            symbol_type,
            kind,
            index,
        };

        // Insert into appropriate scope
        if kind.is_class_level() {
            self.class_scope.insert(name.to_string(), symbol);
        } else {
            self.subroutine_scope.insert(name.to_string(), symbol);
        }

        Ok(())
    }

    /// Look up a symbol by name.
    ///
    /// Searches subroutine scope first, then class scope (allowing shadowing).
    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        self.subroutine_scope
            .get(name)
            .or_else(|| self.class_scope.get(name))
    }

    /// Get the count of symbols of a given kind.
    pub fn var_count(&self, kind: SymbolKind) -> u16 {
        match kind {
            SymbolKind::Static => self.static_count,
            SymbolKind::Field => self.field_count,
            SymbolKind::Argument => self.argument_count,
            SymbolKind::Local => self.local_count,
        }
    }

    /// Get the number of field variables (needed for Memory.alloc in constructors).
    #[inline]
    pub fn field_count(&self) -> u16 {
        self.field_count
    }

    /// Get the current class name.
    #[inline]
    pub fn class_name(&self) -> &str {
        &self.class_name
    }
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_span() -> Span {
        Span::new(0, 1, 1, 1)
    }

    #[test]
    fn test_new_symbol_table_is_empty() {
        let table = SymbolTable::new();
        assert_eq!(table.var_count(SymbolKind::Static), 0);
        assert_eq!(table.var_count(SymbolKind::Field), 0);
        assert_eq!(table.var_count(SymbolKind::Argument), 0);
        assert_eq!(table.var_count(SymbolKind::Local), 0);
        assert!(table.lookup("x").is_none());
    }

    #[test]
    fn test_start_class() {
        let mut table = SymbolTable::new();
        table.start_class("Test");
        assert_eq!(table.class_name(), "Test");
    }

    #[test]
    fn test_define_static_variables() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table
            .define("a", Type::Int, SymbolKind::Static, test_span())
            .unwrap();
        table
            .define("b", Type::Int, SymbolKind::Static, test_span())
            .unwrap();

        assert_eq!(table.var_count(SymbolKind::Static), 2);

        let a = table.lookup("a").unwrap();
        assert_eq!(a.name, "a");
        assert_eq!(a.kind, SymbolKind::Static);
        assert_eq!(a.index, 0);
        assert_eq!(a.segment(), "static");

        let b = table.lookup("b").unwrap();
        assert_eq!(b.index, 1);
    }

    #[test]
    fn test_define_field_variables() {
        let mut table = SymbolTable::new();
        table.start_class("Point");

        table
            .define("x", Type::Int, SymbolKind::Field, test_span())
            .unwrap();
        table
            .define("y", Type::Int, SymbolKind::Field, test_span())
            .unwrap();

        assert_eq!(table.var_count(SymbolKind::Field), 2);
        assert_eq!(table.field_count(), 2);

        let x = table.lookup("x").unwrap();
        assert_eq!(x.kind, SymbolKind::Field);
        assert_eq!(x.index, 0);
        assert_eq!(x.segment(), "this");

        let y = table.lookup("y").unwrap();
        assert_eq!(y.index, 1);
    }

    #[test]
    fn test_index_counters_are_independent() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table
            .define("a", Type::Int, SymbolKind::Static, test_span())
            .unwrap();
        table
            .define("b", Type::Int, SymbolKind::Static, test_span())
            .unwrap();
        table
            .define("c", Type::Int, SymbolKind::Field, test_span())
            .unwrap();

        assert_eq!(table.lookup("a").unwrap().index, 0);
        assert_eq!(table.lookup("b").unwrap().index, 1);
        assert_eq!(table.lookup("c").unwrap().index, 0); // Field index is separate
    }

    #[test]
    fn test_subroutine_scope() {
        let mut table = SymbolTable::new();
        table.start_class("Test");
        table.start_subroutine();

        table
            .define("x", Type::Int, SymbolKind::Argument, test_span())
            .unwrap();
        table
            .define("y", Type::Int, SymbolKind::Local, test_span())
            .unwrap();

        assert_eq!(table.var_count(SymbolKind::Argument), 1);
        assert_eq!(table.var_count(SymbolKind::Local), 1);

        let x = table.lookup("x").unwrap();
        assert_eq!(x.kind, SymbolKind::Argument);
        assert_eq!(x.segment(), "argument");

        let y = table.lookup("y").unwrap();
        assert_eq!(y.kind, SymbolKind::Local);
        assert_eq!(y.segment(), "local");
    }

    #[test]
    fn test_subroutine_reset() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table.start_subroutine();
        table
            .define("x", Type::Int, SymbolKind::Local, test_span())
            .unwrap();
        assert!(table.lookup("x").is_some());

        table.start_subroutine(); // Reset
        assert!(table.lookup("x").is_none()); // Local cleared
        assert_eq!(table.var_count(SymbolKind::Local), 0);
    }

    #[test]
    fn test_class_scope_persists_across_subroutines() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table
            .define("field1", Type::Int, SymbolKind::Field, test_span())
            .unwrap();

        table.start_subroutine();
        // Field should still be accessible
        assert!(table.lookup("field1").is_some());

        table.start_subroutine();
        // Still accessible after another reset
        assert!(table.lookup("field1").is_some());
    }

    #[test]
    fn test_two_level_scope_shadowing() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table
            .define("x", Type::Int, SymbolKind::Field, test_span())
            .unwrap();

        table.start_subroutine();
        table
            .define("x", Type::Boolean, SymbolKind::Local, test_span())
            .unwrap();

        let sym = table.lookup("x").unwrap();
        assert_eq!(sym.kind, SymbolKind::Local); // Subroutine shadows class
        assert_eq!(sym.symbol_type, Type::Boolean);
    }

    #[test]
    fn test_duplicate_definition_error_same_scope() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table
            .define("x", Type::Int, SymbolKind::Field, test_span())
            .unwrap();
        let result = table.define("x", Type::Int, SymbolKind::Field, test_span());

        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_in_different_scopes_allowed() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        // Static and Field are both class-level, so this should fail
        table
            .define("x", Type::Int, SymbolKind::Static, test_span())
            .unwrap();
        // But field x should be in the same scope, so it should fail too
        // Actually, they're in the same HashMap (class_scope), so duplicate
        let result = table.define("x", Type::Int, SymbolKind::Field, test_span());
        assert!(result.is_err());
    }

    #[test]
    fn test_symbol_type_preserved() {
        let mut table = SymbolTable::new();
        table.start_class("Test");

        table
            .define("count", Type::Int, SymbolKind::Static, test_span())
            .unwrap();
        table
            .define("flag", Type::Boolean, SymbolKind::Static, test_span())
            .unwrap();
        table
            .define("letter", Type::Char, SymbolKind::Static, test_span())
            .unwrap();
        table
            .define(
                "point",
                Type::ClassName("Point".to_string()),
                SymbolKind::Field,
                test_span(),
            )
            .unwrap();

        assert_eq!(table.lookup("count").unwrap().symbol_type, Type::Int);
        assert_eq!(table.lookup("flag").unwrap().symbol_type, Type::Boolean);
        assert_eq!(table.lookup("letter").unwrap().symbol_type, Type::Char);
        assert_eq!(
            table.lookup("point").unwrap().symbol_type,
            Type::ClassName("Point".to_string())
        );
    }

    #[test]
    fn test_kind_to_segment() {
        assert_eq!(SymbolKind::Static.to_segment(), "static");
        assert_eq!(SymbolKind::Field.to_segment(), "this");
        assert_eq!(SymbolKind::Argument.to_segment(), "argument");
        assert_eq!(SymbolKind::Local.to_segment(), "local");
    }

    #[test]
    fn test_kind_is_class_level() {
        assert!(SymbolKind::Static.is_class_level());
        assert!(SymbolKind::Field.is_class_level());
        assert!(!SymbolKind::Argument.is_class_level());
        assert!(!SymbolKind::Local.is_class_level());
    }

    #[test]
    fn test_multiple_arguments_indexing() {
        let mut table = SymbolTable::new();
        table.start_class("Test");
        table.start_subroutine();

        table
            .define(
                "this",
                Type::ClassName("Test".to_string()),
                SymbolKind::Argument,
                test_span(),
            )
            .unwrap();
        table
            .define("x", Type::Int, SymbolKind::Argument, test_span())
            .unwrap();
        table
            .define("y", Type::Int, SymbolKind::Argument, test_span())
            .unwrap();

        assert_eq!(table.lookup("this").unwrap().index, 0);
        assert_eq!(table.lookup("x").unwrap().index, 1);
        assert_eq!(table.lookup("y").unwrap().index, 2);
        assert_eq!(table.var_count(SymbolKind::Argument), 3);
    }
}
