//! Hotpath Optimizer - Identifies and optimizes frequently executed code paths
//!
//! Tracks execution frequency and applies aggressive optimizations to hot code

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HotpathTracker {
    // Function call counts
    function_counts: HashMap<String, u64>,

    // Basic block execution counts
    block_counts: HashMap<BlockId, u64>,

    // Loop iteration counts
    loop_counts: HashMap<LoopId, u64>,

    // Variable access patterns
    var_access: HashMap<String, AccessPattern>,

    // Thresholds for hotness
    hot_function_threshold: u64,
    hot_block_threshold: u64,
    hot_loop_threshold: u64,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct BlockId(pub usize);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub struct LoopId(pub usize);

#[derive(Debug, Clone)]
pub struct AccessPattern {
    pub read_count: u64,
    pub write_count: u64,
    pub last_access_time: u64,
    pub access_stride: Option<i32>, // For array access patterns
}

#[derive(Debug, Clone)]
pub struct HotpathInfo {
    pub hot_functions: Vec<String>,
    pub hot_blocks: Vec<BlockId>,
    pub hot_loops: Vec<LoopId>,
    pub hot_variables: Vec<String>,
}

impl HotpathTracker {
    pub fn new(
        hot_function_threshold: u64,
        hot_block_threshold: u64,
        hot_loop_threshold: u64,
    ) -> Self {
        Self {
            function_counts: HashMap::new(),
            block_counts: HashMap::new(),
            loop_counts: HashMap::new(),
            var_access: HashMap::new(),
            hot_function_threshold,
            hot_block_threshold,
            hot_loop_threshold,
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(100, 1000, 10000)
    }

    // Track function call
    pub fn record_function_call(&mut self, name: &str) {
        *self.function_counts.entry(name.to_string()).or_insert(0) += 1;
    }

    // Track block execution
    pub fn record_block_execution(&mut self, block_id: BlockId) {
        *self.block_counts.entry(block_id).or_insert(0) += 1;
    }

    // Track loop iteration
    pub fn record_loop_iteration(&mut self, loop_id: LoopId) {
        *self.loop_counts.entry(loop_id).or_insert(0) += 1;
    }

    // Track variable access
    pub fn record_var_read(&mut self, var: &str, time: u64) {
        let pattern = self
            .var_access
            .entry(var.to_string())
            .or_insert(AccessPattern {
                read_count: 0,
                write_count: 0,
                last_access_time: 0,
                access_stride: None,
            });
        pattern.read_count += 1;
        pattern.last_access_time = time;
    }

    pub fn record_var_write(&mut self, var: &str, time: u64) {
        let pattern = self
            .var_access
            .entry(var.to_string())
            .or_insert(AccessPattern {
                read_count: 0,
                write_count: 0,
                last_access_time: 0,
                access_stride: None,
            });
        pattern.write_count += 1;
        pattern.last_access_time = time;
    }

    // Get hotpath information
    pub fn get_hotpaths(&self) -> HotpathInfo {
        let hot_functions: Vec<String> = self
            .function_counts
            .iter()
            .filter(|(_, &count)| count >= self.hot_function_threshold)
            .map(|(name, _)| name.clone())
            .collect();

        let hot_blocks: Vec<BlockId> = self
            .block_counts
            .iter()
            .filter(|(_, &count)| count >= self.hot_block_threshold)
            .map(|(&id, _)| id)
            .collect();

        let hot_loops: Vec<LoopId> = self
            .loop_counts
            .iter()
            .filter(|(_, &count)| count >= self.hot_loop_threshold)
            .map(|(&id, _)| id)
            .collect();

        let hot_variables: Vec<String> = self
            .var_access
            .iter()
            .filter(|(_, pattern)| {
                (pattern.read_count + pattern.write_count) >= self.hot_function_threshold
            })
            .map(|(name, _)| name.clone())
            .collect();

        HotpathInfo {
            hot_functions,
            hot_blocks,
            hot_loops,
            hot_variables,
        }
    }

    // Check if function should be recompiled with optimizations
    pub fn should_optimize_function(&self, name: &str) -> bool {
        self.function_counts
            .get(name)
            .map(|&count| count >= self.hot_function_threshold)
            .unwrap_or(false)
    }

    // Check if variable should be kept in register
    pub fn should_pin_to_register(&self, var: &str) -> bool {
        self.var_access
            .get(var)
            .map(|pattern| (pattern.read_count + pattern.write_count) >= self.hot_block_threshold)
            .unwrap_or(false)
    }

    // Reset counters (for testing or periodic reset)
    pub fn reset(&mut self) {
        self.function_counts.clear();
        self.block_counts.clear();
        self.loop_counts.clear();
        self.var_access.clear();
    }

    // Get statistics
    pub fn stats(&self) -> HotpathStats {
        HotpathStats {
            total_functions: self.function_counts.len(),
            total_blocks: self.block_counts.len(),
            total_loops: self.loop_counts.len(),
            hot_functions: self.get_hotpaths().hot_functions.len(),
            hot_blocks: self.get_hotpaths().hot_blocks.len(),
            hot_loops: self.get_hotpaths().hot_loops.len(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct HotpathStats {
    pub total_functions: usize,
    pub total_blocks: usize,
    pub total_loops: usize,
    pub hot_functions: usize,
    pub hot_blocks: usize,
    pub hot_loops: usize,
}

/// Hotpath-specific optimizations
pub struct HotpathOptimizer {
    tracker: HotpathTracker,
}

impl HotpathOptimizer {
    pub fn new(tracker: HotpathTracker) -> Self {
        Self { tracker }
    }

    /// Determine optimal register allocation for hot variables
    pub fn optimize_register_allocation(&self, variables: &[String]) -> Vec<(String, bool)> {
        variables
            .iter()
            .map(|var| {
                let should_pin = self.tracker.should_pin_to_register(var);
                (var.clone(), should_pin)
            })
            .collect()
    }

    /// Suggest inline candidates based on call frequency
    pub fn suggest_inline_candidates(&self) -> Vec<String> {
        let hotpaths = self.tracker.get_hotpaths();

        // Functions called frequently but not too large should be inlined
        hotpaths
            .hot_functions
            .iter()
            .filter(|name| {
                // Could add size check here
                self.tracker
                    .function_counts
                    .get(*name)
                    .map(|&count| count >= self.tracker.hot_function_threshold * 2)
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    /// Determine if loop should be unrolled
    pub fn should_unroll_loop(&self, loop_id: LoopId, trip_count: Option<usize>) -> bool {
        let is_hot = self
            .tracker
            .loop_counts
            .get(&loop_id)
            .map(|&count| count >= self.tracker.hot_loop_threshold)
            .unwrap_or(false);

        // Unroll if hot and has known small trip count
        is_hot && trip_count.map(|c| c <= 8).unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotpath_tracking() {
        let mut tracker = HotpathTracker::with_defaults();

        for _ in 0..150 {
            tracker.record_function_call("main");
        }

        assert!(tracker.should_optimize_function("main"));
    }

    #[test]
    fn test_hot_variables() {
        let mut tracker = HotpathTracker::with_defaults();

        for i in 0..1500 {
            tracker.record_var_read("x", i);
        }

        assert!(tracker.should_pin_to_register("x"));
    }

    #[test]
    fn test_hotpath_info() {
        let mut tracker = HotpathTracker::new(10, 50, 100);

        for _ in 0..15 {
            tracker.record_function_call("foo");
        }

        for _ in 0..60 {
            tracker.record_block_execution(BlockId(0));
        }

        let info = tracker.get_hotpaths();
        assert!(info.hot_functions.contains(&"foo".to_string()));
        assert!(info.hot_blocks.contains(&BlockId(0)));
    }
}
