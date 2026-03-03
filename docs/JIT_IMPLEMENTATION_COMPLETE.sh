#!/usr/bin/env bash
# Complete JIT Implementation Plan
# Execute phases sequentially to implement all language features

# ========== PHASE 7: RUNTIME CALLS (FFI) ==========
# TODO: Implement extern "C" function calling from JIT code
#
# Key tasks:
# 1. Load function addresses (64-bit pointers)
#    - Use MOVZ (Move Wide with Zero) for first 16 bits
#    - Use MOVK (Move Wide with Keep) for other 16-bit chunks
#
# 2. ARM64 Calling Convention
#    - Arguments: X0-X7 (integers), D0-D7 (floats)
#    - Return value: X0 (integer), D0 (float)
#    - Preserved registers: X19-X28, SP, FP
#    - BLR (Branch with Link to Register): X30 = PC + 4, jump to register
#
# 3. Implementation:
#    a) Add encode_movz_imm() and encode_movk_imm()
#    b) Add FfiCompiler with load_fn_address() method
#    c) Integrate with emit_call() in ArithmeticEncoder
#    d) Test with simple Rust function calls

# ========== PHASE 8: HEAP ALLOCATION ==========
# TODO: Integrate malloc/free and custom allocator with JIT
#
# Key tasks:
# 1. Pointer representation
#    - Lower 2 bits: tag (00=int, 01=string, 10=list, 11=object)
#    - Upper 62 bits: address
#
# 2. List implementation
#    - Header: [len: i64, cap: i64, ptr: u64]
#    - Access: [base + 16 + index * 8] for element
#
# 3. Implement HeapAllocator code generation:
#    - emit_malloc(size) -> X0 = address
#    - emit_list_alloc(capacity) -> full list setup
#    - emit_free(pointer) -> deallocate
#
# 4. GC integration
#    - Reference counting or mark-sweep
#    - Track roots during execution

# ========== PHASE 9: GARBAGE COLLECTION ==========
# TODO: Generate stack maps for precise GC
#
# Key tasks:
# 1. Stack map format
#    - Offset into function
#    - Register mask (which regs hold pointers)
#    - Stack slot map (which stack positions hold pointers)
#
# 2. Safepoint generation
#    - Mark allocation calls as safepoints
#    - Generate metadata at each safepoint
#    - Emit stack maps alongside code
#
# 3. GC Coordination
#    - Pause JIT code execution
#    - Walk stack frames
#    - Mark reachable objects
#    - Sweep unreachable allocations

# ========== PHASE 10: OPTIMIZATION ==========
# TODO: Linear scan register allocation & peephole optimization
#
# Key tasks:
# 1. Linear Scan Register Allocator
#    - Build live intervals for variables
#    - Sort by start point
#    - Assign registers greedily
#    - Spill to stack when needed
#
# 2. Peephole Optimizations
#    - MOV r, r -> eliminate
#    - ADD r, 0 -> eliminate
#    - Consecutive operations fusion
#    - Dead code elimination
#
# 3. Loop optimizations
#    - Loop unrolling (for fixed trip counts)
#    - Strength reduction (i*2 -> i<<1)
#    - Invariant hoisting

# ========== LANGUAGE FEATURES IMPLEMENTATION ==========

# --- FLOATING POINT SUPPORT ---
# In: src/jit/encoder.rs
#
# Add instructions:
# - FMOV Dn, #imm  (F16 immediate float move)
# - FADD D0, D1, D2 (floating add)
# - FSUB, FMUL, FDIV
# - FCMP (compare floats)
#
# Format:
# fmov: 0x1E201000 | (imm8 << 13) | (rd << 0)
# fadd: 0x1E200800 | (rm << 16) | (rn << 5) | (rd << 0)
#
# In: src/jit/regalloc.rs
# - Add separate D0-D7 register tracking
# - Implement float allocation in RegisterMap
#
# In: src/jit/compiler.rs
# - Handle IrInstr::Float variants
# - Generate float load/store/arithmetic

# --- STRING SUPPORT ---
# In: src/jit/compiler.rs
#
# String representation:
# - Interned strings via StringPool
# - Stored on heap with reference counting
# - Format: [len: u64, data: u8*]
#
# Operations:
# - emit_string_literal(s) -> allocate, return pointer
# - emit_string_concat(s1, s2) -> allocate new, copy, return
# - emit_string_compare(s1, s2) -> call strcmp, return bool
#
# Implementation:
# 1. Register string pool in JitContext
# 2. Add string interning to avoid duplicates
# 3. Call runtime functions for operations
# 4. Generate proper reference counting instructions

# --- ARRAY/LIST SUPPORT ---
# In: src/jit/heap.rs and src/jit/compiler.rs
#
# Operations needed:
# - emit_list_alloc(capacity) -> [len=0, cap, ptr]
# - emit_list_push(list, item) -> set list[len] = item, len++
# - emit_list_index(list, index) -> load list[16 + index*8]
# - emit_list_length(list) -> load list[0]
#
# Implementation:
# 1. Emit code to call malloc(capacity * 8 + 16)
# 2. Write header (len, cap, ptr)
# 3. For indexing: address = list_ptr + 16 + index * 8
# 4. Generate bounds checks before access

# --- OBJECT/CLASS SUPPORT ---
# In: src/jit/compiler.rs (new section)
#
# Class representation:
# - Vtable: [method0_ptr, method1_ptr, ...]
# - Instance: [vtable_ptr, field0, field1, ...]
#
# Operations:
# - emit_class_alloc(class_def) -> allocate + init
# - emit_field_load(obj, field_id) -> load obj[field_id]
# - emit_field_store(obj, field_id, value) -> store obj[field_id] = value
# - emit_method_call(obj, method_id, args) -> load vtable, call method
#
# Implementation:
# 1. Store class layouts in JitContext
# 2. Generate field offset table at compile time
# 3. Emit field access as [ptr + offset] loads/stores
# 4. Generate vtable dispatch code

# --- EXCEPTION HANDLING ---
# In: src/jit/runtime.rs and src/jit/compiler.rs
#
# Implementation:
# 1. Exception stack (thread-local)
# 2. try: mark stack position, set handler
# 3. throw: search stack for handler, longjmp
# 4. catch: restore state, continue
#
# Code generation:
# - try block: save SP, FP to exception record
# - throw: call runtime unwind function
# - catch: restore stack, resume execution

# --- PATTERN MATCHING ---
# In: src/jit/compiler.rs (new section)
#
# Implementation:
# 1. Translate match to if-else chain
# 2. Each pattern becomes comparison + jump
# 3. Arm bodies compiled as separate blocks
#
# Code gen:
# - Discriminant check (usually enum tag)
# - Pattern guard checks (if-condition)
# - Arm body execution
# - Jump to end

# --- ASYNC/AWAIT ---
# In: src/codegen/direct.rs and src/jit/async.rs
#
# Implementation:
# 1. Convert async fn to state machine
# 2. Each await point = state transition
# 3. Resume from saved state
#
# Code gen:
# - Allocate state struct on heap
# - State enum (Pending, Running, Done)
# - Switch on state, execute relevant code
# - On await, save registers to state, return Future
# - On resume, load state, continue execution

# ========== TESTING STRATEGY ==========

# For each phase:
# 1. Unit test (encoder, basic functionality)
# 2. Integration test (full program compilation)
# 3. Performance test (vs interpreter)
# 4. Regression test (previous phases still work)

# Test files:
# test_jit_arithmetic.fr - basic add/sub/mul
# test_jit_functions.fr - function calls
# test_jit_arrays.fr - list operations
# test_jit_objects.fr - class instantiation
# test_jit_exceptions.fr - try/catch
# test_jit_loops.fr - while/for loops
# test_jit_patterns.fr - pattern matching
# test_jit_async.fr - async/await

# ========== BUILD COMMANDS ==========

# Test specific phase:
# cargo test --lib jit::encoder -- --nocapture
# cargo test --lib jit::memory -- --nocapture
# cargo test --lib jit::branching -- --nocapture
# cargo test --lib jit::regalloc -- --nocapture
# cargo test --lib jit::compiler -- --nocapture

# Run full JIT tests:
# cargo test --release jit::

# Benchmark:
# cargo bench --jit

# Profile:
# cargo build --release && perf record ./target/release/fforge test.fr && perf report

# ========== INTEGRATION WITH LANGUAGE ==========

# Command aliases:
# forge main.fr          -> VM (default)
# forge --vm main.fr     -> VM
# forge --jit main.fr    -> JIT
# forge --native main.fr -> Native (AOT)
# forge --rust main.fr   -> Rust interpreter
# forge --ast main.fr    -> Show AST
# forge --ir main.fr     -> Show IR

# Metroman plugins:
# MetroMan is a plugin system for extend functionality
# Plugins implement: load(), execute(), cleanup()
# Plugins can hook into: parse, codegen, execute phases

# ========== CRITICAL SAFETY REQUIREMENTS ==========

# 1. W^X Protection (mandatory on macOS)
#    - Memory must be either writable OR executable, never both
#    - Solution: pthread_jit_write_protect_np()
#
# 2. Stack Alignment (ARM64 ABI requirement)
#    - SP must be 16-byte aligned before memory access instructions
#    - All stack operations must maintain alignment
#
# 3. Cache Coherency
#    - After writing code, must invalidate I-cache
#    - Solution: sys_icache_invalidate()
#
# 4. Exception Safety
#    - JIT code must follow C calling convention
#    - Must preserve callee-saved registers
#    - Must maintain valid stack frames

# ========== PERFORMANCE TARGETS ==========

# JIT vs Interpreter speedup goals:
# - Arithmetic: 5-10x faster
# - Function calls: 3-5x faster
# - Loops: 10-20x faster (with optimization)
# - Overall average: 5-7x faster

# Compilation overhead targets:
# - Simple function: < 1ms
# - Medium function: < 10ms
# - Large function: < 100ms
#
# Total program execution should still be < 1 second for test files

echo "JIT Implementation Plan loaded."
echo "Next steps:"
echo "1. Fix hanging issue in fforge"
echo "2. Complete Phase 7 (FFI/Runtime Calls)"
echo "3. Implement Phase 8 (Heap Allocation)"
echo "4. Complete Phase 9 (GC Integration)"
echo "5. Optimize with Phase 10"
echo "6. Add language features (floats, strings, arrays, objects)"
echo "7. Extend to async/await, pattern matching, exceptions"
echo "8. Performance optimization pass"

