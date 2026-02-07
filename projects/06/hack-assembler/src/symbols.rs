use phf::phf_map;
use std::collections::HashMap;

/// Predefined symbols (compile-time perfect hash map)
pub static PREDEFINED: phf::Map<&'static str, u16> = phf_map! {
    "R0" => 0, "R1" => 1, "R2" => 2, "R3" => 3,
    "R4" => 4, "R5" => 5, "R6" => 6, "R7" => 7,
    "R8" => 8, "R9" => 9, "R10" => 10, "R11" => 11,
    "R12" => 12, "R13" => 13, "R14" => 14, "R15" => 15,
    "SP" => 0, "LCL" => 1, "ARG" => 2, "THIS" => 3, "THAT" => 4,
    "SCREEN" => 16384, "KBD" => 24576,
};

pub struct SymbolTable {
    symbols: HashMap<String, u16>,
    next_var_address: u16,
}

impl Default for SymbolTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            symbols: HashMap::with_capacity(64),
            next_var_address: 16,
        }
    }

    pub fn add_label(&mut self, label: String, address: u16) -> Result<(), String> {
        if self.symbols.contains_key(&label) {
            return Err(label);
        }
        self.symbols.insert(label, address);
        Ok(())
    }

    pub fn get_or_allocate(&mut self, symbol: &str) -> u16 {
        // Check predefined symbols first
        if let Some(&addr) = PREDEFINED.get(symbol) {
            return addr;
        }

        // Check user-defined symbols
        if let Some(&addr) = self.symbols.get(symbol) {
            return addr;
        }

        // Allocate new variable
        let addr = self.next_var_address;
        self.symbols.insert(symbol.to_string(), addr);
        self.next_var_address += 1;
        addr
    }

    pub fn get(&self, symbol: &str) -> Option<u16> {
        PREDEFINED
            .get(symbol)
            .copied()
            .or_else(|| self.symbols.get(symbol).copied())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predefined_symbols() {
        let table = SymbolTable::new();
        assert_eq!(table.get("R0"), Some(0));
        assert_eq!(table.get("SP"), Some(0));
        assert_eq!(table.get("SCREEN"), Some(16384));
        assert_eq!(table.get("KBD"), Some(24576));
    }

    #[test]
    fn test_label_addition() {
        let mut table = SymbolTable::new();
        assert!(table.add_label("LOOP".to_string(), 10).is_ok());
        assert_eq!(table.get("LOOP"), Some(10));
        assert!(table.add_label("LOOP".to_string(), 20).is_err());
    }

    #[test]
    fn test_variable_allocation() {
        let mut table = SymbolTable::new();
        assert_eq!(table.get_or_allocate("i"), 16);
        assert_eq!(table.get_or_allocate("j"), 17);
        assert_eq!(table.get_or_allocate("i"), 16); // Same variable
    }
}
