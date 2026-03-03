//! Symbol Table - Tracks variables, functions, and types across compilation
//!
//! Provides efficient lookup and management of symbols during JIT compilation

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolType {
    Variable,
    Function,
    Parameter,
    Global,
    Local,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ValueType {
    Int,
    Float,
    String,
    Bool,
    List,
    Map,
    Object,
    Function,
    Unknown,
}

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub sym_type: SymbolType,
    pub value_type: ValueType,
    pub scope_level: usize,
    pub location: Option<SymbolLocation>,
    pub is_mutable: bool,
    pub reference_count: usize, // For hotpath tracking
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SymbolLocation {
    Register(u8),
    Stack(i32),
    Global(u64), // Address for global variables
}

pub struct SymbolTable {
    // Scoped symbol storage
    scopes: Vec<HashMap<String, Symbol>>,
    current_scope: usize,

    // Global function registry
    functions: HashMap<String, FunctionSymbol>,

    // Type inference cache
    type_cache: HashMap<String, ValueType>,
}

#[derive(Debug, Clone)]
pub struct FunctionSymbol {
    pub name: String,
    pub param_count: usize,
    pub param_types: Vec<ValueType>,
    pub return_type: ValueType,
    pub address: Option<u64>,
    pub is_compiled: bool,
    pub call_count: usize, // For hotpath optimization
}

impl SymbolTable {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()], // Global scope
            current_scope: 0,
            functions: HashMap::new(),
            type_cache: HashMap::new(),
        }
    }

    pub fn enter_scope(&mut self) {
        self.current_scope += 1;
        if self.current_scope >= self.scopes.len() {
            self.scopes.push(HashMap::new());
        } else {
            self.scopes[self.current_scope].clear();
        }
    }

    pub fn exit_scope(&mut self) {
        if self.current_scope > 0 {
            self.scopes[self.current_scope].clear();
            self.current_scope -= 1;
        }
    }

    pub fn declare_variable(
        &mut self,
        name: String,
        value_type: ValueType,
        location: Option<SymbolLocation>,
    ) -> Result<(), String> {
        if self.scopes[self.current_scope].contains_key(&name) {
            return Err(format!(
                "Variable '{}' already declared in this scope",
                name
            ));
        }

        let symbol = Symbol {
            name: name.clone(),
            sym_type: if self.current_scope == 0 {
                SymbolType::Global
            } else {
                SymbolType::Local
            },
            value_type,
            scope_level: self.current_scope,
            location,
            is_mutable: true,
            reference_count: 0,
        };

        self.scopes[self.current_scope].insert(name.clone(), symbol);
        self.type_cache.insert(name, value_type);
        Ok(())
    }

    pub fn lookup(&self, name: &str) -> Option<&Symbol> {
        // Search from current scope up to global
        for scope_idx in (0..=self.current_scope).rev() {
            if let Some(symbol) = self.scopes[scope_idx].get(name) {
                return Some(symbol);
            }
        }
        None
    }

    pub fn lookup_mut(&mut self, name: &str) -> Option<&mut Symbol> {
        for scope_idx in (0..=self.current_scope).rev() {
            if self.scopes[scope_idx].contains_key(name) {
                return self.scopes[scope_idx].get_mut(name);
            }
        }
        None
    }

    pub fn update_location(&mut self, name: &str, location: SymbolLocation) -> Result<(), String> {
        if let Some(symbol) = self.lookup_mut(name) {
            symbol.location = Some(location);
            Ok(())
        } else {
            Err(format!("Variable '{}' not found", name))
        }
    }

    pub fn increment_reference(&mut self, name: &str) {
        if let Some(symbol) = self.lookup_mut(name) {
            symbol.reference_count += 1;
        }
    }

    pub fn get_type(&self, name: &str) -> Option<ValueType> {
        self.type_cache.get(name).copied()
    }

    // Function management
    pub fn declare_function(&mut self, func: FunctionSymbol) -> Result<(), String> {
        if self.functions.contains_key(&func.name) {
            return Err(format!("Function '{}' already declared", func.name));
        }
        self.functions.insert(func.name.clone(), func);
        Ok(())
    }

    pub fn get_function(&self, name: &str) -> Option<&FunctionSymbol> {
        self.functions.get(name)
    }

    pub fn get_function_mut(&mut self, name: &str) -> Option<&mut FunctionSymbol> {
        self.functions.get_mut(name)
    }

    pub fn mark_function_compiled(&mut self, name: &str, address: u64) {
        if let Some(func) = self.functions.get_mut(name) {
            func.is_compiled = true;
            func.address = Some(address);
        }
    }

    pub fn increment_call_count(&mut self, name: &str) {
        if let Some(func) = self.functions.get_mut(name) {
            func.call_count += 1;
        }
    }

    // Get hot variables (frequently accessed)
    pub fn get_hot_variables(&self, threshold: usize) -> Vec<String> {
        let mut hot_vars = Vec::new();
        for scope in &self.scopes {
            for (name, symbol) in scope {
                if symbol.reference_count >= threshold {
                    hot_vars.push(name.clone());
                }
            }
        }
        hot_vars
    }

    // Get hot functions (frequently called)
    pub fn get_hot_functions(&self, threshold: usize) -> Vec<String> {
        self.functions
            .iter()
            .filter(|(_, func)| func.call_count >= threshold)
            .map(|(name, _)| name.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symbol_table_basic() {
        let mut table = SymbolTable::new();
        table
            .declare_variable("x".to_string(), ValueType::Int, None)
            .unwrap();
        assert!(table.lookup("x").is_some());
    }

    #[test]
    fn test_scoped_lookup() {
        let mut table = SymbolTable::new();
        table
            .declare_variable("global".to_string(), ValueType::Int, None)
            .unwrap();

        table.enter_scope();
        table
            .declare_variable("local".to_string(), ValueType::Int, None)
            .unwrap();

        assert!(table.lookup("global").is_some());
        assert!(table.lookup("local").is_some());

        table.exit_scope();
        assert!(table.lookup("global").is_some());
        assert!(table.lookup("local").is_none());
    }

    #[test]
    fn test_hot_variables() {
        let mut table = SymbolTable::new();
        table
            .declare_variable("x".to_string(), ValueType::Int, None)
            .unwrap();

        for _ in 0..10 {
            table.increment_reference("x");
        }

        let hot = table.get_hot_variables(5);
        assert!(hot.contains(&"x".to_string()));
    }
}
