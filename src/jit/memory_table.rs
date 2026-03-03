//! Memory Table - Tracks heap allocations and lifetimes for GC integration
//!
//! Manages memory allocation metadata for JIT-compiled code

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AllocationType {
    String,
    List,
    Map,
    Object,
    Closure,
}

#[derive(Debug, Clone)]
pub struct Allocation {
    pub id: u64,
    pub alloc_type: AllocationType,
    pub size: usize,
    pub references: usize,
    pub is_live: bool,
    pub created_at: u64, // Instruction counter
}

pub struct MemoryTable {
    allocations: HashMap<u64, Allocation>,
    next_id: u64,

    // Stack frame tracking
    stack_frames: Vec<StackFrame>,

    // GC roots
    roots: Vec<u64>,

    // Instruction counter for allocation tracking
    instruction_counter: u64,
}

#[derive(Debug, Clone)]
pub struct StackFrame {
    pub frame_id: u64,
    pub local_allocations: Vec<u64>,
    pub parent_frame: Option<u64>,
}

impl MemoryTable {
    pub fn new() -> Self {
        Self {
            allocations: HashMap::new(),
            next_id: 1,
            stack_frames: Vec::new(),
            roots: Vec::new(),
            instruction_counter: 0,
        }
    }

    pub fn allocate(&mut self, alloc_type: AllocationType, size: usize) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        let allocation = Allocation {
            id,
            alloc_type,
            size,
            references: 0,
            is_live: true,
            created_at: self.instruction_counter,
        };

        self.allocations.insert(id, allocation);

        // Add to current stack frame if exists
        if let Some(frame) = self.stack_frames.last_mut() {
            frame.local_allocations.push(id);
        }

        id
    }

    pub fn increment_ref(&mut self, id: u64) {
        if let Some(alloc) = self.allocations.get_mut(&id) {
            alloc.references += 1;
        }
    }

    pub fn decrement_ref(&mut self, id: u64) -> bool {
        if let Some(alloc) = self.allocations.get_mut(&id) {
            if alloc.references == 0 {
                return false;
            }
            alloc.references -= 1;
            alloc.references == 0 && !self.roots.contains(&id)
        } else {
            false
        }
    }

    pub fn mark_root(&mut self, id: u64) {
        if !self.roots.contains(&id) {
            self.roots.push(id);
        }
    }

    pub fn unmark_root(&mut self, id: u64) {
        self.roots.retain(|&x| x != id);
    }

    pub fn push_frame(&mut self, frame_id: u64, parent: Option<u64>) {
        self.stack_frames.push(StackFrame {
            frame_id,
            local_allocations: Vec::new(),
            parent_frame: parent,
        });
    }

    pub fn pop_frame(&mut self) -> Option<Vec<u64>> {
        self.stack_frames.pop().map(|frame| frame.local_allocations)
    }

    pub fn increment_instruction(&mut self) {
        self.instruction_counter += 1;
    }

    pub fn get_instruction_counter(&self) -> u64 {
        self.instruction_counter
    }

    pub fn get_allocation(&self, id: u64) -> Option<&Allocation> {
        self.allocations.get(&id)
    }

    // Collect garbage (allocations with 0 references and not roots)
    pub fn collect_garbage(&mut self) -> Vec<u64> {
        let mut collected = Vec::new();

        self.allocations.retain(|&id, alloc| {
            if alloc.references == 0 && !self.roots.contains(&id) {
                collected.push(id);
                false
            } else {
                true
            }
        });

        collected
    }

    // Get allocation statistics
    pub fn stats(&self) -> MemoryStats {
        let total_allocations = self.allocations.len();
        let total_size: usize = self.allocations.values().map(|a| a.size).sum();
        let live_allocations = self.allocations.values().filter(|a| a.is_live).count();

        MemoryStats {
            total_allocations,
            live_allocations,
            total_size,
            stack_depth: self.stack_frames.len(),
            root_count: self.roots.len(),
        }
    }

    // Mark all allocations reachable from roots (for mark-sweep GC)
    pub fn mark_reachable(&mut self) {
        let root_ids: Vec<u64> = self.roots.iter().copied().collect();
        for root_id in root_ids {
            self.mark_recursive(root_id);
        }
    }

    fn mark_recursive(&mut self, id: u64) {
        if let Some(alloc) = self.allocations.get_mut(&id) {
            if !alloc.is_live {
                alloc.is_live = true;
                // Would traverse references to other allocations here
            }
        }
    }

    // Sweep unmarked allocations
    pub fn sweep(&mut self) -> Vec<u64> {
        let mut swept = Vec::new();

        self.allocations.retain(|&id, alloc| {
            if !alloc.is_live {
                swept.push(id);
                false
            } else {
                alloc.is_live = false; // Reset for next GC cycle
                true
            }
        });

        swept
    }
}

#[derive(Debug, Clone)]
pub struct MemoryStats {
    pub total_allocations: usize,
    pub live_allocations: usize,
    pub total_size: usize,
    pub stack_depth: usize,
    pub root_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_allocation() {
        let mut table = MemoryTable::new();
        let id = table.allocate(AllocationType::String, 100);
        assert!(table.get_allocation(id).is_some());
    }

    #[test]
    fn test_reference_counting() {
        let mut table = MemoryTable::new();
        let id = table.allocate(AllocationType::List, 50);

        table.increment_ref(id);
        assert_eq!(table.get_allocation(id).unwrap().references, 1);
        assert!(table.decrement_ref(id));
        assert!(!table.decrement_ref(id));
    }

    #[test]
    fn test_garbage_collection() {
        let mut table = MemoryTable::new();
        let id1 = table.allocate(AllocationType::String, 100);
        let id2 = table.allocate(AllocationType::List, 200);

        table.increment_ref(id1);

        let collected = table.collect_garbage();
        assert!(collected.contains(&id2));
        assert!(!collected.contains(&id1));
    }

    #[test]
    fn test_stack_frames() {
        let mut table = MemoryTable::new();
        table.push_frame(1, None);

        let id = table.allocate(AllocationType::Object, 300);

        let frame_allocs = table.pop_frame().unwrap();
        assert!(frame_allocs.contains(&id));
    }
}
