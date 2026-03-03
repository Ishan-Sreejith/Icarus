//! JIT Context: Global Symbol Table
//!
//! Stores the memory addresses of compiled functions to allow cross-function calls.
//! Also manages the lifetime of JIT-compiled code blocks.

use crate::jit::memory::JitMemory;
use std::collections::HashMap;

pub struct JitContext {
    /// Function name -> Memory address (u64)
    functions: HashMap<String, u64>,
    /// Keep memory blocks alive
    code_blocks: Vec<JitMemory>,
    /// Keep literal byte buffers alive (string literals, etc.)
    literal_bytes: Vec<Vec<u8>>,
}

impl JitContext {
    pub fn new() -> Self {
        Self {
            functions: HashMap::new(),
            code_blocks: Vec::new(),
            literal_bytes: Vec::new(),
        }
    }

    /// Register a compiled function's address.
    pub fn register_function(&mut self, name: &str, addr: u64) {
        self.functions.insert(name.to_string(), addr);
    }

    /// Get the address of a compiled function.
    pub fn get_function_addr(&self, name: &str) -> Option<u64> {
        self.functions.get(name).copied()
    }

    /// Add a code block to be managed by the context.
    pub fn add_code_block(&mut self, block: JitMemory) {
        self.code_blocks.push(block);
    }

    /// Intern a byte buffer and return its stable pointer and length.
    /// The returned pointer remains valid for the lifetime of this context.
    pub fn intern_bytes(&mut self, bytes: Vec<u8>) -> (*const u8, usize) {
        self.literal_bytes.push(bytes);
        let last = self.literal_bytes.last().expect("just pushed");
        (last.as_ptr(), last.len())
    }
}
