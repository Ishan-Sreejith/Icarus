pub mod branching;
pub mod compiler;
pub mod context;
pub mod encoder;
pub mod ffi;
pub mod heap;
pub mod hotpath;
pub mod memory;
pub mod memory_table;
pub mod optimize;
pub mod phase11;
pub mod regalloc;
pub mod runtime;
pub mod stackmap;
pub mod symbol_table;
pub mod trampoline;

#[cfg(test)]
mod safety_tests;
