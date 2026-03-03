//! Phase 11: Speculative Optimization & Deoptimization
//!
//! Scaffolding for speculative guards, PIC, OSR, tiered compilation, and escape analysis.
//! This module provides data structures and minimal logic that can be wired into
//! the JIT compiler once runtime type tags and profiling hooks are finalized.

use std::collections::{HashMap, HashSet};

/// Runtime type tags for guard checks and inline caches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypeTag {
    Int,
    Float,
    Str,
    Obj,
    Unknown,
}

/// Tiered compilation levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Baseline,
    Optimized,
}

impl Default for Tier {
    fn default() -> Self {
        Tier::Baseline
    }
}

/// Simple hot counter for calls/loops.
#[derive(Debug, Default)]
pub struct HotCounter {
    count: u64,
    threshold: u64,
}

impl HotCounter {
    pub fn new(threshold: u64) -> Self {
        Self {
            count: 0,
            threshold,
        }
    }

    pub fn tick(&mut self) -> bool {
        self.count = self.count.saturating_add(1);
        self.count >= self.threshold
    }

    pub fn count(&self) -> u64 {
        self.count
    }
}

/// Profile data for a function or loop.
#[derive(Debug)]
pub struct JitProfile {
    call_counter: HotCounter,
    loop_counters: HashMap<String, HotCounter>,
    tier: Tier,
}

impl Default for JitProfile {
    fn default() -> Self {
        Self {
            call_counter: HotCounter::new(0),
            loop_counters: HashMap::new(),
            tier: Tier::Baseline,
        }
    }
}

impl JitProfile {
    pub fn new(call_threshold: u64, loop_threshold: u64) -> Self {
        let _loop_threshold = loop_threshold;
        Self {
            call_counter: HotCounter::new(call_threshold),
            loop_counters: HashMap::new(),
            tier: Tier::Baseline,
        }
    }

    pub fn tick_call(&mut self) -> bool {
        self.call_counter.tick()
    }

    pub fn tick_loop(&mut self, loop_id: &str, loop_threshold: u64) -> bool {
        let counter = self
            .loop_counters
            .entry(loop_id.to_string())
            .or_insert_with(|| HotCounter::new(loop_threshold));
        counter.tick()
    }

    pub fn tier(&self) -> Tier {
        self.tier
    }

    pub fn promote(&mut self) {
        self.tier = Tier::Optimized;
    }
}

/// Speculative guard for type checks.
#[derive(Debug, Clone)]
pub struct SpecGuard {
    pub expected: TypeTag,
    pub deopt: DeoptAction,
}

/// Deoptimization action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeoptAction {
    FallbackToVm,
}

/// Inline cache handler kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheHandler {
    IntAdd,
    StrConcat,
    SlowFallback,
}

/// A single PIC entry.
#[derive(Debug, Clone)]
pub struct PicEntry {
    pub ty: TypeTag,
    pub handler: CacheHandler,
}

/// Polymorphic Inline Cache (PIC) for a call site.
#[derive(Debug, Default)]
pub struct PolymorphicInlineCache {
    max_entries: usize,
    entries: Vec<PicEntry>,
}

impl PolymorphicInlineCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            max_entries,
            entries: Vec::new(),
        }
    }

    pub fn record(&mut self, ty: TypeTag, handler: CacheHandler) {
        if self.entries.iter().any(|e| e.ty == ty) {
            return;
        }
        if self.entries.len() < self.max_entries {
            self.entries.push(PicEntry { ty, handler });
        }
    }

    pub fn resolve(&self, ty: TypeTag) -> CacheHandler {
        self.entries
            .iter()
            .find(|e| e.ty == ty)
            .map(|e| e.handler)
            .unwrap_or(CacheHandler::SlowFallback)
    }

    pub fn entries(&self) -> &[PicEntry] {
        &self.entries
    }
}

/// On-Stack Replacement (OSR) plan for a hot loop.
#[derive(Debug, Default)]
pub struct OsrPlanner {
    loop_offsets: HashMap<String, usize>,
}

impl OsrPlanner {
    pub fn register_loop(&mut self, loop_id: &str, byte_offset: usize) {
        self.loop_offsets.insert(loop_id.to_string(), byte_offset);
    }

    pub fn lookup(&self, loop_id: &str) -> Option<usize> {
        self.loop_offsets.get(loop_id).copied()
    }
}

/// Escape analysis result for a function scope.
#[derive(Debug, Default)]
pub struct EscapeResult {
    pub escapes: HashSet<String>,
}

impl EscapeResult {
    pub fn escapes(&self, name: &str) -> bool {
        self.escapes.contains(name)
    }
}

/// Escape analysis stub.
#[derive(Debug, Default)]
pub struct EscapeAnalysis;

impl EscapeAnalysis {
    pub fn analyze(
        locals: &[String],
        returned: &[String],
        stored_globally: &[String],
    ) -> EscapeResult {
        let mut escapes = HashSet::new();
        for name in locals {
            if returned.contains(name) || stored_globally.contains(name) {
                escapes.insert(name.clone());
            }
        }
        EscapeResult { escapes }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hot_counter() {
        let mut counter = HotCounter::new(3);
        assert!(!counter.tick());
        assert!(!counter.tick());
        assert!(counter.tick());
    }

    #[test]
    fn test_pic_resolve() {
        let mut pic = PolymorphicInlineCache::new(2);
        pic.record(TypeTag::Int, CacheHandler::IntAdd);
        pic.record(TypeTag::Str, CacheHandler::StrConcat);
        assert_eq!(pic.resolve(TypeTag::Int), CacheHandler::IntAdd);
        assert_eq!(pic.resolve(TypeTag::Str), CacheHandler::StrConcat);
        assert_eq!(pic.resolve(TypeTag::Float), CacheHandler::SlowFallback);
    }

    #[test]
    fn test_osr_planner() {
        let mut osr = OsrPlanner::default();
        osr.register_loop("loop0", 128);
        assert_eq!(osr.lookup("loop0"), Some(128));
        assert_eq!(osr.lookup("loop1"), None);
    }

    #[test]
    fn test_escape_analysis() {
        let locals = vec!["a".to_string(), "b".to_string()];
        let returned = vec!["b".to_string()];
        let stored = vec![];
        let result = EscapeAnalysis::analyze(&locals, &returned, &stored);
        assert!(!result.escapes("a"));
        assert!(result.escapes("b"));
    }
}
