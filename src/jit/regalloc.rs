//! Phase 5 & 4: Register Allocation & Spilling
//!
//! Manages the mapping of variables to physical registers (x0-x7) or stack slots.

use std::collections::HashMap;

use crate::jit::branching::{
    encode_b, encode_b_eq, encode_b_gt, encode_b_lt, encode_b_ne, encode_cmp_reg,
};
use crate::jit::encoder::{
    encode_add_imm, encode_add_reg, encode_blr, encode_ldur, encode_mov64, encode_mov_imm,
    encode_mul_reg, encode_sdiv_reg, encode_stur, encode_sub_reg, Reg,
};

#[derive(Debug, Clone, Copy)]
pub enum ConditionCode {
    Eq, // Equal (Z set)
    Ne, // Not equal (Z clear)
    Lt, // Less than (N != V)
    Le, // Less than or equal (Z set or N != V)
    Gt, // Greater than (Z clear and N == V)
    Ge, // Greater than or equal (N == V)
}

impl ConditionCode {
    pub fn to_bits(self) -> u32 {
        match self {
            ConditionCode::Eq => 0b0000,
            ConditionCode::Ne => 0b0001,
            ConditionCode::Lt => 0b1011,
            ConditionCode::Le => 0b1101,
            ConditionCode::Gt => 0b1100,
            ConditionCode::Ge => 0b1010,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Location {
    Register(u8),
    Stack(i32), // Offset from FP (e.g., -8, -16)
}

/// Tracks which physical register holds which variable.
pub struct RegisterMap {
    /// variable name -> location
    pub var_map: HashMap<String, Location>,
    /// register number -> is available? (x0-x7)
    reg_free: [bool; 8],
    /// Current stack offset (starts at -16, grows down)
    stack_offset: i32,
}

impl RegisterMap {
    pub fn new() -> Self {
        Self {
            var_map: HashMap::new(),
            reg_free: [true; 8],
            stack_offset: -16, // Start after FP/LR save area
        }
    }

    /// Allocate a location for a variable.
    /// Tries registers first, then spills to stack.
    pub fn alloc(&mut self, var: &str) -> Result<Location, String> {
        if let Some(&loc) = self.var_map.get(var) {
            return Ok(loc);
        }

        // Try to find a free register
        for i in 0..8 {
            if self.reg_free[i] {
                self.reg_free[i] = false;
                let loc = Location::Register(i as u8);
                self.var_map.insert(var.to_string(), loc);
                return Ok(loc);
            }
        }

        // Spill to stack
        let offset = self.stack_offset;
        self.stack_offset -= 8; // 8 bytes per slot
        let loc = Location::Stack(offset);
        self.var_map.insert(var.to_string(), loc);
        Ok(loc)
    }

    /// Deallocate a register.
    pub fn free(&mut self, loc: Location) {
        if let Location::Register(reg) = loc {
            if (reg as usize) < 8 {
                self.reg_free[reg as usize] = true;
            }
        }
    }

    /// Get the location for a variable.
    pub fn get(&self, var: &str) -> Option<Location> {
        self.var_map.get(var).copied()
    }

    /// Clear all mappings (for, e.g., function boundaries)
    pub fn clear(&mut self) {
        self.var_map.clear();
        self.reg_free = [true; 8];
        self.stack_offset = -16;
    }

    pub fn stack_frame_bytes(&self) -> u16 {
        if self.stack_offset >= -16 {
            return 0;
        }

        // stack_offset tracks the next free slot below FP. With offsets like -16, -24, -32...
        // The total space needed so that the lowest slot is within SP is: (-stack_offset) - 8.
        let used_i32 = (-self.stack_offset) - 8;
        if used_i32 <= 0 {
            return 0;
        }

        let mut bytes = if used_i32 > u16::MAX as i32 {
            u16::MAX
        } else {
            used_i32 as u16
        };

        // Keep 16-byte alignment for AAPCS64.
        let rem = bytes % 16;
        if rem != 0 {
            bytes = bytes.saturating_add(16 - rem);
        }

        bytes
    }
}

/// Extended encoder that handles spilling transparently.
pub struct ArithmeticEncoder {
    buf: Vec<u8>,
}

impl ArithmeticEncoder {
    pub fn new() -> Self {
        Self { buf: Vec::new() }
    }

    pub fn emit_u32_le(&mut self, value: u32) {
        self.buf.extend_from_slice(&value.to_le_bytes());
    }

    pub fn emit_bytes(&mut self, bytes: &[u8]) {
        self.buf.extend_from_slice(bytes);
    }

    // --- Raw Instruction Emitters ---

    fn raw_ldr(&mut self, rt: u8, base: u8, offset: i32) {
        let base_reg = if base == 31 { Reg::SP } else { Reg::X(base) };
        match i16::try_from(offset) {
            Ok(off) if (-256..=255).contains(&off) => {
                self.emit_u32_le(encode_ldur(Reg::X(rt), base_reg, off))
            }
            Ok(off) => {
                // Offset doesn't fit LDUR imm9; compute address in scratch reg x12.
                for instr in encode_mov64(Reg::X(12), off as i64 as u64) {
                    self.emit_u32_le(instr);
                }
                self.emit_u32_le(encode_add_reg(Reg::X(12), Reg::X(12), base_reg));
                self.emit_u32_le(encode_ldur(Reg::X(rt), Reg::X(12), 0));
            }
            Err(_) => self.emit_u32_le(encode_mov_imm(Reg::X(rt), 0)),
        }
    }

    fn raw_str(&mut self, rt: u8, base: u8, offset: i32) {
        let base_reg = if base == 31 { Reg::SP } else { Reg::X(base) };
        if let Ok(off) = i16::try_from(offset) {
            if (-256..=255).contains(&off) {
                self.emit_u32_le(encode_stur(Reg::X(rt), base_reg, off));
            } else {
                // Offset doesn't fit STUR imm9; compute address in scratch reg x12.
                for instr in encode_mov64(Reg::X(12), off as i64 as u64) {
                    self.emit_u32_le(instr);
                }
                self.emit_u32_le(encode_add_reg(Reg::X(12), Reg::X(12), base_reg));
                self.emit_u32_le(encode_stur(Reg::X(rt), Reg::X(12), 0));
            }
        }
    }

    // --- High-Level Emitters with Spilling Support ---
    // We use x9, x10 as scratch registers for spilling if needed.
    // x29 is FP.

    /// Load a value from a Location into a physical register.
    /// If loc is Register, moves it to `target_reg`.
    /// If loc is Stack, loads it from the stack to `target_reg`.
    pub fn load_to_reg(&mut self, target_reg: u8, loc: Location) {
        match loc {
            Location::Register(r) => {
                if r != target_reg {
                    // MOV target, r
                    self.emit_u32_le(encode_add_imm(Reg::X(target_reg), Reg::X(r), 0));
                }
            }
            Location::Stack(offset) => {
                // LDR target, [FP, #offset]
                self.raw_ldr(target_reg, 29, offset);
            }
        }
    }

    /// Store a value from a physical register into a Location.
    pub fn store_from_reg(&mut self, src_reg: u8, loc: Location) {
        match loc {
            Location::Register(r) => {
                if r != src_reg {
                    // MOV r, src
                    self.emit_u32_le(encode_add_imm(Reg::X(r), Reg::X(src_reg), 0));
                }
            }
            Location::Stack(offset) => {
                // STR src, [FP, #offset]
                self.raw_str(src_reg, 29, offset);
            }
        }
    }

    // --- Operations ---

    pub fn emit_mov_imm(&mut self, dest: Location, imm: u16) {
        // Load imm into x9 (scratch)
        self.emit_u32_le(encode_mov_imm(Reg::X(9), imm));
        // Store x9 to dest
        self.store_from_reg(9, dest);
    }

    pub fn emit_mov(&mut self, dest: Location, src: Location) {
        // Move from src to dest
        // Load src into x9
        self.load_to_reg(9, src);
        // Store x9 to dest
        self.store_from_reg(9, dest);
    }

    // Add helper for raw register move (needed for manual register manipulation)
    pub fn emit_add_imm(&mut self, dest: u8, src: u8, imm: u16) {
        self.emit_u32_le(encode_add_imm(Reg::X(dest), Reg::X(src), imm));
    }

    pub fn emit_add(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left); // x9 = left
        self.load_to_reg(10, right); // x10 = right
        self.emit_u32_le(encode_add_reg(Reg::X(9), Reg::X(9), Reg::X(10))); // x9 = x9 + x10
        self.store_from_reg(9, dest); // dest = x9
    }

    pub fn emit_sub(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_sub_reg(Reg::X(9), Reg::X(9), Reg::X(10)));
        self.store_from_reg(9, dest);
    }

    pub fn emit_mul(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_mul_reg(Reg::X(9), Reg::X(9), Reg::X(10)));
        self.store_from_reg(9, dest);
    }

    pub fn emit_div(&mut self, dest: Location, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_sdiv_reg(Reg::X(9), Reg::X(9), Reg::X(10)));
        self.store_from_reg(9, dest);
    }

    pub fn emit_cmp(&mut self, left: Location, right: Location) {
        self.load_to_reg(9, left);
        self.load_to_reg(10, right);
        self.emit_u32_le(encode_cmp_reg(Reg::X(9), Reg::X(10)));
    }

    // CSET - Conditional Set
    // Sets register to 1 if condition is true, 0 otherwise
    pub fn emit_cset(&mut self, dest: Location, cond: ConditionCode) {
        // CSET Xd, cond is actually CSINC Xd, XZR, XZR, (invert cond)
        // Encoding: 0x9A9F_07E0 | (cond << 12) | (dest_reg << 0)
        // But simpler: use the CSET pseudo-instruction encoding
        // CSET Rd, cond = CSINC Rd, XZR, XZR, !cond
        // Base: 0x9A9F07E0, but we need proper encoding

        // For simplicity, use a compare result approach:
        // We already did CMP, flags are set
        // CSET Xd, cond:
        //   if cond true: Xd = 1
        //   else: Xd = 0

        // ARM64 CSET encoding:
        // CSINC Xd, XZR, XZR, invert(cond)
        // Which is: Xd = (cond) ? XZR + 1 : XZR = (cond) ? 1 : 0

        let inverted_cond = match cond {
            ConditionCode::Eq => 0b0001, // NE
            ConditionCode::Ne => 0b0000, // EQ
            ConditionCode::Lt => 0b1010, // GE
            ConditionCode::Le => 0b1100, // GT
            ConditionCode::Gt => 0b1101, // LE
            ConditionCode::Ge => 0b1011, // LT
        };

        // CSINC: sf=1, op=0, S=0, Rm=11111, cond, op2=00, Rn=11111, Rd
        // 1_0_0_11010100_11111_cond_00_11111_Rd
        let instr = 0x9A9F07E0 | (inverted_cond << 12) | (9u32 << 0); // Use x9 as temp
        self.emit_u32_le(instr);
        self.store_from_reg(9, dest);
    }

    // Control Flow
    pub fn emit_b(&mut self, offset: i32) {
        self.emit_u32_le(encode_b(offset));
    }
    pub fn emit_b_eq(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_eq(offset));
    }
    pub fn emit_b_ne(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_ne(offset));
    }
    pub fn emit_b_lt(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_lt(offset));
    }
    pub fn emit_b_gt(&mut self, offset: i32) {
        self.emit_u32_le(encode_b_gt(offset));
    }

    // FFI
    pub fn emit_call(&mut self, addr: u64) {
        let instructions = encode_mov64(Reg::X(9), addr);
        for instr in instructions {
            self.emit_u32_le(instr);
        }
        self.emit_u32_le(encode_blr(Reg::X(9)));
    }

    pub fn emit_call_arg2(&mut self, addr: u64, arg0: u64, arg1: u64) {
        let instructions0 = encode_mov64(Reg::X(0), arg0);
        for instr in instructions0 {
            self.emit_u32_le(instr);
        }
        let instructions1 = encode_mov64(Reg::X(1), arg1);
        for instr in instructions1 {
            self.emit_u32_le(instr);
        }
        self.emit_call(addr);
    }

    // Helper to move a Location to a specific register (e.g., for args)
    pub fn move_to_phys_reg(&mut self, dest_phys: u8, src: Location) {
        self.load_to_reg(dest_phys, src);
    }

    // Helper to move from a specific register to a Location (e.g. return val)
    pub fn move_from_phys_reg(&mut self, dest: Location, src_phys: u8) {
        self.store_from_reg(src_phys, dest);
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
    }
    pub fn len(&self) -> usize {
        self.buf.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_map_alloc() {
        let mut map = RegisterMap::new();
        let r1 = map.alloc("x").unwrap();
        let r2 = map.alloc("y").unwrap();
        assert_ne!(r1, r2);
    }

    #[test]
    fn test_arithmetic_encoder_emit() {
        let mut enc = ArithmeticEncoder::new();
        enc.emit_mov_imm(Location::Register(0), 42);
        assert_eq!(enc.len(), 8); // mov x9, #42; mov x0, x9
    }
}
