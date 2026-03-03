# Function Return Value Fix - Session Documentation

## Problem Identified
Functions were returning garbage values instead of their computed results. The root cause was **double epilogue emission**:

1. **First epilogue**: In the `Return` instruction handler (line 524-526)
   - Moves return value to x0
   - Emits function epilogue (LDP FP, LR, [SP], #16)
   - Emits RET instruction

2. **Second epilogue**: At the end of `compile()` (line 563-564) 
   - Unconditionally emits another epilogue
   - Emits another RET instruction
   - This causes stack frame corruption

## Solution Implemented

Added a `has_explicit_return` flag to track whether an explicit Return statement was encountered:

### Code Change in `src/jit/compiler.rs`:

1. **Line 121** - Add flag initialization:
   ```rust
   let mut has_explicit_return = false;
   ```

2. **Line 512** - Mark when Return is processed:
   ```rust
   IrInstr::Return { value } => {
       has_explicit_return = true;  // ← NEW LINE
       if let Some(val) = value {
   ```

3. **Lines 537-564** - Skip second epilogue if explicit return exists:
   ```rust
   if !has_explicit_return {  // ← NEW CONDITION
       // ...epilogue emission...
       emit.emit_u32_le(encode_ldp_fp_lr());
       emit.emit_u32_le(encode_ret());
   }
   ```

## How It Works

- **Functions WITH explicit Return**: 
  - Emit prologue at function start
  - Process instructions  
  - When Return is hit: move value to x0, emit epilogue, return
  - Skip the second epilogue at function end ✓

- **Functions WITHOUT explicit Return** (implicit return of last value):
  - Emit prologue at function start
  - Process instructions
  - At function end: find last computed value, move to x0, emit epilogue, return
  - flag prevents duplicate emission ✓

## Testing Instructions

```bash
cd "/Users/ishan/IdeaProjects/CoRe Main/CoRe Backup V1.0 copy"

# Compile
cargo build

# Test 1: Simple function
cat > /tmp/test1.fr << 'EOF'
fn get_five {
    return 5
}
var x: get_five
say: x
EOF
./target/debug/fforge /tmp/test1.fr

# Expected output: 5

# Test 2: Function with parameters
cat > /tmp/test2.fr << 'EOF'
fn add: a, b {
    return a + b
}
var result: add: 3, 4
say: result
EOF
./target/debug/fforge /tmp/test2.fr

# Expected output: 7

# Test 3: Function without explicit return
cat > /tmp/test3.fr << 'EOF'
fn compute {
    var x: 10
    var y: 20
    x + y
}
var result: compute
say: result
EOF
./target/debug/fforge /tmp/test3.fr

# Expected output: 30
```

## Technical Details

### ARM64 ABI Compliance
- **Prologue** (STP): `stp x29, x30, [sp, #-16]!`
  - Saves frame pointer and return address
  - Adjusts stack pointer (pre-index)
  - Called at function start

- **Epilogue** (LDP): `ldp x29, x30, [sp], #16`
  - Restores frame pointer and return address  
  - Adjusts stack pointer (post-index)
  - Called before RET

### Return Value Convention
- **x0**: Contains the 64-bit integer return value
- **Other registers**: Clobbered (caller-saved)
- **x29, x30**: Preserved (callee-saved)

## Status
✅ **IMPLEMENTED** - Flag-based epilogue deduplication
⚠️ **PENDING TEST** - Need to verify all function types work correctly

## Next Steps
1. Run comprehensive test suite
2. Test nested function calls
3. Test recursion
4. Verify parameter passing for all argument counts
5. Implement caller-saved register cleanup if needed

