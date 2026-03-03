//! Phase 9: Garbage Collector Stack Maps
//!
//! Generates precise stack metadata that tells the GC where pointers live
//! in registers and on the stack during JIT code execution.

use std::collections::HashMap;

/// A safepoint is a location in compiled code where the GC can safely pause.
/// Typically occurs at:
/// - Function calls (including malloc)
/// - Loop back-edges
/// - Exception points
#[derive(Debug, Clone)]
pub struct Safepoint {
    /// Byte offset in the compiled code
    pub offset: usize,
    /// Which registers contain pointers (bit mask, bits 0-30 for x0-x30)
    pub register_mask: u32,
    /// Stack slots that contain pointers (relative offsets from FP)
    pub stack_slots: Vec<i32>,
}

/// Stack map metadata for a single compiled function.
#[derive(Debug, Clone)]
pub struct StackMap {
    /// Function name
    pub name: String,
    /// Safepoints in this function
    pub safepoints: Vec<Safepoint>,
    /// Frame size in bytes
    pub frame_size: usize,
}

impl StackMap {
    /// Create a new stack map for a function.
    pub fn new(name: &str, frame_size: usize) -> Self {
        Self {
            name: name.to_string(),
            safepoints: Vec::new(),
            frame_size,
        }
    }

    /// Register a safepoint in the function.
    pub fn register_safepoint(&mut self, offset: usize, register_mask: u32, stack_slots: Vec<i32>) {
        self.safepoints.push(Safepoint {
            offset,
            register_mask,
            stack_slots,
        });
    }

    /// Serialize the stack map for storage or transmission.
    pub fn serialize(&self) -> String {
        format!("StackMap({},frame_size={})", self.name, self.frame_size)
    }
}

/// Tracks which variables contain pointers for GC purposes.
pub struct GCMetadata {
    /// Variable name -> is pointer?
    var_types: HashMap<String, bool>,
}

impl GCMetadata {
    pub fn new() -> Self {
        Self {
            var_types: HashMap::new(),
        }
    }

    /// Mark a variable as containing a pointer.
    pub fn mark_pointer(&mut self, var: &str) {
        self.var_types.insert(var.to_string(), true);
    }

    /// Mark a variable as a non-pointer value.
    pub fn mark_value(&mut self, var: &str) {
        self.var_types.insert(var.to_string(), false);
    }

    /// Check if a variable is a pointer.
    pub fn is_pointer(&self, var: &str) -> bool {
        self.var_types.get(var).copied().unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safepoint_creation() {
        let sp = Safepoint {
            offset: 100,
            register_mask: 0b11, // x0 and x1 are pointers
            stack_slots: vec![0, 8],
        };
        assert_eq!(sp.offset, 100);
    }

    #[test]
    fn test_stack_map_creation() {
        let mut sm = StackMap::new("test_func", 64);
        sm.register_safepoint(32, 0b11, vec![0, 8]);
        assert_eq!(sm.safepoints.len(), 1);
    }

    #[test]
    fn test_gc_metadata() {
        let mut gc = GCMetadata::new();
        gc.mark_pointer("list");
        gc.mark_value("count");
        assert!(gc.is_pointer("list"));
        assert!(!gc.is_pointer("count"));
    }
}
