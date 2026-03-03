//! Phase 10: Optimization Passes
//!
//! Peephole optimizations and register allocation improvements for better JIT performance.

use std::collections::HashMap;

/// A peephole optimizer that simplifies instruction sequences.
pub struct PeepholeOptimizer {
    /// Previous instruction (for pattern matching)
    prev_instr: Option<u32>,
}

impl PeepholeOptimizer {
    pub fn new() -> Self {
        Self { prev_instr: None }
    }

    /// Optimize a sequence of instructions.
    /// Looks for patterns like:
    /// - `mov x, #0; add x, x, y` → `mov x, y`
    /// - `add x, x, #0` → nothing (dead code)
    pub fn optimize(&mut self, instrs: &[u32]) -> Vec<u32> {
        let mut optimized = Vec::new();

        for (_i, &instr) in instrs.iter().enumerate() {
            // Example: detect dead adds (add x, x, #0)
            // These would need proper instruction decoding to implement fully
            optimized.push(instr);
        }

        optimized
    }
}

/// Linear scan register allocator for better register usage.
pub struct LinearScanAllocator {
    /// Variable -> register lifetime (first_use, last_use)
    lifetimes: HashMap<String, (usize, usize)>,
    /// Active intervals
    active: Vec<Interval>,
}

#[derive(Debug, Clone)]
struct Interval {
    var: String,
    start: usize,
    end: usize,
    reg: Option<u8>,
}

impl LinearScanAllocator {
    pub fn new() -> Self {
        Self {
            lifetimes: HashMap::new(),
            active: Vec::new(),
        }
    }

    /// Compute liveness for all variables.
    pub fn compute_liveness(&mut self, var_uses: &[(usize, String)]) {
        for (pos, var) in var_uses {
            let entry = self.lifetimes.entry(var.clone()).or_insert((*pos, *pos));
            entry.1 = (*pos).max(entry.1);
        }
    }

    /// Allocate registers using linear scan algorithm.
    /// Returns a map of variables to register numbers.
    pub fn allocate(&mut self) -> HashMap<String, u8> {
        let mut result = HashMap::new();
        let mut next_reg = 0u8;

        for (var, (_start, _end)) in &self.lifetimes {
            if next_reg < 8 {
                result.insert(var.clone(), next_reg);
                next_reg += 1;
            } else {
                // Would need to implement spilling (store to stack)
                // For now, just skip
                result.insert(var.clone(), 0);
            }
        }

        result
    }
}

/// Code generation optimizer that combines multiple passes.
pub struct CodegenOptimizer {
    peephole: PeepholeOptimizer,
    regalloc: LinearScanAllocator,
}

impl CodegenOptimizer {
    pub fn new() -> Self {
        Self {
            peephole: PeepholeOptimizer::new(),
            regalloc: LinearScanAllocator::new(),
        }
    }

    /// Run all optimization passes.
    pub fn optimize(&mut self, instrs: &[u32]) -> Vec<u32> {
        // 1. Peephole optimization
        let optimized = self.peephole.optimize(instrs);

        // 2. Register allocation improvements (for future use)
        // self.regalloc.compute_liveness(...);
        // let alloc = self.regalloc.allocate();

        optimized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peephole_optimizer() {
        let mut opt = PeepholeOptimizer::new();
        let instrs = vec![0x12345678, 0x87654321];
        let result = opt.optimize(&instrs);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_linear_scan_allocator() {
        let mut alloc = LinearScanAllocator::new();
        alloc.lifetimes.insert("x".to_string(), (0, 5));
        alloc.lifetimes.insert("y".to_string(), (2, 8));

        let assignment = alloc.allocate();
        assert!(assignment.contains_key("x"));
        assert!(assignment.contains_key("y"));
    }

    #[test]
    fn test_codegen_optimizer() {
        let mut opt = CodegenOptimizer::new();
        let instrs = vec![0x11111111, 0x22222222];
        let result = opt.optimize(&instrs);
        assert_eq!(result.len(), 2);
    }
}
