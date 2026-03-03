# JIT Compiler - Complete Feature Implementation Guide

## Quick Reference: What Needs to be Done

### IMMEDIATE PRIORITIES (Next 5 hours):
1. **Fix fforge hanging issue** - Currently times out during execution
   - Root cause: likely infinite loop in compile() or patch_branches()
   - Solution: Add detailed tracing/logging to identify exact point
   - Test with minimal 1-line program

2. **Implement Floating Point Arithmetic**
   - Add FMOV, FADD, FSUB, FMUL, FDIV instructions to encoder
   - Modify RegisterMap to track D0-D7 (float registers separately)
   - Update compiler to emit float opcodes
   - Test: `var x: 3.14; var y: 2.71; say: x + y`

3. **Complete String Implementation**
   - Implement string interning in JitContext
   - Add string allocation/concatenation in compiler
   - Support string comparisons
   - Test: `say: "Hello " + "World"`

### NEAR TERM (Next 10 hours):
4. **Array/List Full Support**
   - Dynamic allocation for lists
   - Index operations with bounds checking
   - Length/capacity tracking
   - Test: `var x: [1, 2, 3]; say: x[1]`

5. **Class/Object Support**
   - Class definition compilation
   - Constructor code generation
   - Method dispatch (vtable)
   - Field access/mutation
   - Test: `class Point { x: i64, y: i64 }; var p: Point{10, 20}; say: p.x`

6. **Exception Handling (try/catch/throw)**
   - Exception stack management
   - throw instruction compilation
   - try/catch block generation
   - Unwinding and recovery
   - Test: `try { throw "error" } catch e { say: e }`

### MEDIUM TERM (Next 20 hours):
7. **Pattern Matching**
   - Match expression compilation
   - Guard conditions
   - Exhaustiveness checking
   - Test: `match x { 1 => "one", 2 => "two", _ => "other" }`

8. **Async/Await**
   - State machine conversion
   - Future type implementation
   - Spawn/await operations
   - Event loop integration
   - Test: `async fn fetch() { ... }; await fetch()`

9. **Complete Optimization (Phase 10)**
   - Linear scan register allocator
   - Peephole optimizations
   - Loop optimizations
   - Dead code elimination

10. **Module System Integration**
    - Module compilation
    - Import resolution
    - Visibility handling
    - Test: `import math { sin, cos }; say: sin(0)`

---

## Detailed Implementation Specifications

### Feature 1: Floating Point Arithmetic

**Files to Modify:**
- `src/jit/encoder.rs` - Add float instructions
- `src/jit/regalloc.rs` - Track D0-D7 separately  
- `src/jit/compiler.rs` - Emit float instructions
- `src/ir.rs` - Add Float variant to IrValue

**Encoder Changes:**
```rust
// Add to encoder.rs:
fn encode_fmov_imm(rd: u8, imm: u8) -> u32 {
    // FMOV Dd, #imm (F16 format)
    // 0001 1110 0010 0000 iiii iiii rd
    let imm5 = (imm & 0x1F) as u32;
    let imm8 = ((imm >> 5) & 0x07) as u32;
    0x1E201000 | ((imm8 << 13) | (imm5 << 5) | rd)
}

fn encode_fadd(rd: u8, rn: u8, rm: u8) -> u32 {
    // FADD Dd, Dn, Dm (double precision)
    // 0001 1110 0010 0000 0000 00rm rn rd
    0x1E200800 | ((rm as u32) << 16) | ((rn as u32) << 5) | rd
}

// Similar for FSUB, FMUL, FDIV, FCMP
```

**RegisterMap Changes:**
```rust
pub struct RegisterMap {
    // Integer registers
    x_regs: [bool; 31], // X0-X30
    
    // Float registers (NEW)
    d_regs: [bool; 8],  // D0-D7
    
    // Mappings
    var_to_x: HashMap<String, u8>,
    var_to_d: HashMap<String, u8>, // NEW
}

pub fn alloc_float(&mut self, var: &str) -> Result<u8, String> {
    for i in 0..8 {
        if !self.d_regs[i] {
            self.d_regs[i] = true;
            self.var_to_d.insert(var.to_string(), i as u8);
            return Ok(i as u8);
        }
    }
    Err("Out of float registers".to_string())
}
```

**Test File** (`test_jit_floats.fr`):
```
var pi: 3.14159
var e: 2.71828
var result: pi + e
say: result
```

---

### Feature 2: String Support

**Files to Modify:**
- `src/jit/compiler.rs` - String compilation
- `src/jit/context.rs` - String interning
- `src/jit/ffi.rs` - String runtime functions

**String Interning:**
```rust
pub struct StringPool {
    // String value -> pointer (address in JIT memory)
    strings: HashMap<String, u64>,
}

impl StringPool {
    pub fn intern(&mut self, s: &str) -> u64 {
        if let Some(&ptr) = self.strings.get(s) {
            return ptr;
        }
        
        // Allocate new string
        let layout = std::alloc::Layout::from_size_align(s.len() + 16, 8).unwrap();
        let ptr = unsafe { std::alloc::alloc(layout) as u64 };
        
        // Write header: [len: u64, data: u8*]
        unsafe {
            *(ptr as *mut u64) = s.len() as u64;
            std::ptr::copy_nonoverlapping(s.as_ptr(), (ptr + 8) as *mut u8, s.len());
        }
        
        self.strings.insert(s.to_string(), ptr);
        ptr
    }
}
```

**Compiler Integration:**
```rust
IrInstr::LoadConst { dest, value: IrValue::String(s) } => {
    let str_ptr = self.context.string_pool.intern(s);
    emit.emit_mov_imm(Location::Register(0), (str_ptr & 0xFFFF) as u16);
    // For 64-bit pointers, use MOVZ/MOVK
    self.var_types.insert(dest.clone(), TypeTag::String);
}
```

**Test File** (`test_jit_strings.fr`):
```
var greeting: "Hello"
var name: "World"
say: greeting + " " + name
```

---

### Feature 3: Array/List Implementation

**Memory Layout:**
```
struct List {
    len: u64,       // @offset 0
    capacity: u64,  // @offset 8
    data: u64,      // @offset 16 (pointer to array data)
}

Array data:
[element0, element1, element2, ...]
```

**Compiler Implementation:**
```rust
IrInstr::ListAlloc { dest, capacity } => {
    // Allocate: malloc(capacity * 8 + 24)
    emit.emit_mov_imm(Location::Register(0), capacity);
    emit.emit_call(RuntimeFunctions::malloc().addr());
    
    // Initialize header
    emit.emit_mov_imm(Location::Register(1), 0);  // len = 0
    emit.store_field(Location::Register(0), 0, Location::Register(1));
    
    emit.emit_mov_imm(Location::Register(1), capacity);  // capacity
    emit.store_field(Location::Register(0), 8, Location::Register(1));
    
    // data = malloc(capacity * 8)
    emit.emit_call(RuntimeFunctions::malloc().addr());
    emit.store_field(Location::Register(0), 16, Location::Register(0));
}
```

**Index Access:**
```rust
IrInstr::ListIndex { dest, list, index } => {
    let list_loc = self.regmap.get(list).unwrap();
    let index_loc = self.regmap.get(index).unwrap();
    
    // Calculate address: list.data + index * 8
    emit.emit_ldr(Location::Register(10), list_loc, 16); // load data ptr
    emit.emit_add(Location::Register(10), Location::Register(10), index_loc);
    emit.emit_ldr(dest_loc, Location::Register(10), 0);  // load element
}
```

**Test File** (`test_jit_arrays.fr`):
```
var list: [10, 20, 30]
say: list[0]
say: list[1]
say: list[2]
```

---

### Feature 4: Class/Object Support

**Class Layout:**
```
Class definition:
{
    name: "Point",
    methods: [
        { name: "new", offset: 0x1000 },
        { name: "distance", offset: 0x2000 },
    ],
    fields: [
        { name: "x", offset: 0, type: "i64" },
        { name: "y", offset: 8, type: "i64" },
    ]
}

Instance layout:
[vtable_ptr, field0_value, field1_value, ...]
```

**Compiler Implementation:**
```rust
IrInstr::ClassAlloc { dest, class_name, init_values } => {
    // Get class definition
    let class_def = self.context.classes.get(class_name)?;
    let inst_size = 8 + (class_def.fields.len() * 8); // vtable + fields
    
    // Allocate instance
    emit.emit_mov_imm(Location::Register(0), inst_size as u16);
    emit.emit_call(RuntimeFunctions::malloc().addr());
    
    // Write vtable pointer
    let vtable_addr = class_def.vtable_addr;
    emit.emit_mov_imm(Location::Register(1), (vtable_addr & 0xFFFF) as u16);
    emit.store(Location::Register(0), 0, Location::Register(1));
    
    // Initialize fields
    for (i, value) in init_values.iter().enumerate() {
        let offset = 8 + (i * 8);
        let val_loc = self.regmap.get(value).unwrap();
        emit.store(Location::Register(0), offset as i32, val_loc);
    }
}

IrInstr::FieldAccess { dest, obj, field_name } => {
    let obj_loc = self.regmap.get(obj).unwrap();
    let field_offset = class_def.field_offset(field_name)?;
    
    emit.emit_ldr(dest_loc, obj_loc, field_offset as i32);
}

IrInstr::MethodCall { dest, obj, method, args } => {
    // Load vtable pointer from object
    emit.emit_ldr(Location::Register(9), obj_loc, 0);  // vtable
    
    // Load method pointer from vtable
    let method_idx = class_def.method_index(method)?;
    emit.emit_ldr(Location::Register(10), Location::Register(9), 
                  (method_idx * 8) as i32);
    
    // Set up arguments
    // obj (this) goes in X0
    emit.move_from_reg(Location::Register(0), obj_loc);
    
    // Other arguments in X1-X7
    for (i, arg) in args.iter().enumerate() {
        let arg_loc = self.regmap.get(arg).unwrap();
        emit.move_from_reg(Location::Register((i + 1) as u8), arg_loc);
    }
    
    // Call via BLR
    emit.emit_blr(Location::Register(10));
    
    // Result in X0
    emit.move_to_phys_reg(dest_loc, 0);
}
```

**Test File** (`test_jit_classes.fr`):
```
class Point {
    x: i64
    y: i64
    
    fn distance {
        var sum: self.x + self.y
        return sum
    }
}

var p: Point{3, 4}
say: p.distance()
```

---

### Feature 5: Exception Handling

**Exception Stack:**
```rust
pub struct ExceptionHandler {
    // Stack of try blocks
    stack: Vec<ExceptionFrame>,
}

pub struct ExceptionFrame {
    handler_addr: u64,  // Address of catch block
    sp: u64,            // Stack pointer at try
    fp: u64,            // Frame pointer at try
    exception: Option<String>,
}
```

**Compiler Implementation:**
```rust
IrInstr::Try { body, catch_var, handler } => {
    // Record current stack position
    emit.emit_mov_imm(Location::Register(10), 0); // SP
    emit.emit_call(RuntimeFunctions::push_try().addr());
    
    // Compile try body
    self.compile(body)?;
    
    // Pop handler on success
    emit.emit_call(RuntimeFunctions::pop_try().addr());
    
    // Jump over catch
    emit.emit_b(0);
    let catch_offset = emit.len() - 4;
    
    // Catch block starts here
    emit.emit_call(RuntimeFunctions::get_last_error().addr());
    // Result in X0 is exception string pointer
    
    // Store in catch variable
    let catch_var_loc = self.regmap.alloc(catch_var)?;
    emit.move_from_phys_reg(catch_var_loc, 0);
    
    // Compile catch body
    self.compile(handler)?;
}

IrInstr::Throw { value } => {
    let val_loc = self.regmap.get(value).unwrap();
    emit.move_from_reg(Location::Register(0), val_loc);
    emit.emit_call(RuntimeFunctions::throw().addr());
}
```

**Test File** (`test_jit_exceptions.fr`):
```
try {
    var x: 10
    if x > 5 {
        throw "x is too large"
    }
    say: "OK"
} catch err {
    say: "Error: "
    say: err
}
```

---

## Integration Checklist

For each feature, ensure:
- [ ] Unit tests pass
- [ ] Integration test file runs without errors
- [ ] Performance is acceptable (< 100ms total)
- [ ] No memory leaks (valgrind clean)
- [ ] No register corruption (preserved regs intact)
- [ ] Stack alignment maintained (16-byte aligned)
- [ ] W^X protection enforced
- [ ] Cache invalidation done

## Performance Measurements

Add benchmarks for each feature:
```bash
cargo bench --features jit -- --nocapture | tee bench_results.txt
```

Expected metrics:
- Compile time: < 10ms for typical function
- Execution time: 5-10x faster than interpreter
- Memory overhead: < 2x for code + metadata

## Final Integration

Once all features are complete:
1. Run full test suite: `cargo test --release`
2. Benchmark all pathways: `forge`, `fforge`, `forger`, `forge -a`
3. Profile for optimization opportunities
4. Document performance improvements
5. Create user guide for JIT features

