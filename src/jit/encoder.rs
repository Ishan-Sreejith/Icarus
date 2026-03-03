//! Phase 2: The Binary Encoder (The "Assembler")
//!
//! This module provides functions to encode a minimal set of ARM64 instructions
//! into their 32-bit binary representation.
//!
//! ARM64 instructions are 32 bits wide. The encoding depends on the instruction
//! type (e.g., data processing, load/store, branch).
//!
//! Key instruction formats:
//! - Data Processing (Immediate): `MOV`, `ADD`, `SUB` with immediate values.
//! - Data Processing (Register): `ADD`, `SUB` with register operands.
//! - Branch: `B`, `BL`, `RET`.
//!
//! Register encoding:
//! - X0-X30 are general-purpose 64-bit registers.
//! - SP (Stack Pointer) is X31.
//! - W0-W30 are 32-bit versions of X0-X30.
//! - WZR (Zero Register 32-bit) is W31.
//! - XZR (Zero Register 64-bit) is X31.

/// Represents an ARM64 register.
/// 0-30 for X0-X30, 31 for SP/XZR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg {
    X(u8), // 64-bit register (0-30)
    W(u8), // 32-bit register (0-30)
    SP,    // Stack Pointer (X31)
    XZR,   // Zero Register (X31)
}

impl Reg {
    /// Returns the 5-bit encoding for the register.
    /// X0-X30 -> 0-30
    /// SP/XZR -> 31
    pub fn encode(&self) -> u8 {
        match self {
            Reg::X(r) | Reg::W(r) => *r,
            Reg::SP | Reg::XZR => 31,
        }
    }

    /// Creates a Reg from its encoded value.
    pub fn from_encoded(val: u8, is_64bit: bool) -> Self {
        if val == 31 {
            if is_64bit {
                Reg::XZR
            } else {
                Reg::SP
            } // SP for 32-bit context, XZR for 64-bit
        } else if is_64bit {
            Reg::X(val)
        } else {
            Reg::W(val)
        }
    }
}

/// Encodes an ARM64 `MOV` (Move Wide Immediate) instruction.
/// `MOV <Xd|Wd>, #<imm>`
///
/// Format: `0b110100101_0_0000000000000000_00000_00000`
/// `sf` (bit 31): 1 for 64-bit (X), 0 for 32-bit (W)
/// `opc` (bits 30-29): 00 for MOVZ (Move Zero)
/// `hw` (bits 22-21): 00 for 16-bit immediate in bits 0-15
/// `imm16` (bits 5-20): 16-bit immediate value
/// `Rd` (bits 0-4): Destination register
///
/// For `MOV Xd, #imm`, we use `MOVZ` (Move with Zero) with `hw=00`
/// and `imm16` as the immediate.
pub fn encode_mov_imm(rd: Reg, imm: u16) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1, // 64-bit
        Reg::W(_) => 0,                      // 32-bit
    };
    let rd_enc = rd.encode();

    let mut instruction: u32 = if sf == 1 { 0xD2800000 } else { 0x52800000 };

    instruction |= (imm as u32) << 5; // imm16
    instruction |= rd_enc as u32; // Rd

    instruction
}

/// Encodes an ARM64 `RET` (Return from subroutine) instruction.
/// `RET {<Xn>}` (X30 is default if not specified)
///
/// Format: `0b11010110010111110011000000000000`
/// `opc` (bits 24-21): 0000
/// `op2` (bits 20-16): 11111
/// `op3` (bits 15-10): 001100
/// `Rn` (bits 5-9): 11110 (X30)
/// `op4` (bits 0-4): 00000
///
/// For `RET` with default X30: `0xD65F03C0`
pub fn encode_ret() -> u32 {
    0xD65F03C0 // RET X30 (Link Register)
}

/// Encodes an ARM64 `ADD` (Add Immediate) instruction.
/// `ADD <Xd|Wd>, <Xn|Wn>, #<imm>`
///
/// Format: `0b100_1000100_000000000000_00000_00000`
/// `sf` (bit 31): 1 for 64-bit (X), 0 for 32-bit (W)
/// `op` (bit 30): 0 for ADD
/// `S` (bit 29): 0 (no flags set)
/// `sh` (bit 22): 0 for LSL #0 (no shift)
/// `imm12` (bits 10-21): 12-bit immediate value
/// `Rn` (bits 5-9): First source register
/// `Rd` (bits 0-4): Destination register
pub fn encode_add_imm(rd: Reg, rn: Reg, imm: u16) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1, // 64-bit
        Reg::W(_) => 0,                      // 32-bit
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();

    let mut instruction: u32 = if sf == 1 { 0x91000000 } else { 0x11000000 };

    instruction |= (imm as u32) << 10; // imm12
    instruction |= (rn_enc as u32) << 5; // Rn
    instruction |= rd_enc as u32; // Rd

    instruction
}

/// Encodes an ARM64 `ADD` (Add Register) instruction.
/// `ADD <Xd|Wd>, <Xn|Wn>, <Xm|Wm>`
///
/// Format: `0b100_0101100_00000_000000_00000_00000`
/// `sf` (bit 31): 1 for 64-bit (X), 0 for 32-bit (W)
/// `op` (bit 30): 0 for ADD
/// `S` (bit 29): 0 (no flags set)
/// `Rm` (bits 16-20): Second source register
/// `Rn` (bits 5-9): First source register
/// `Rd` (bits 0-4): Destination register
pub fn encode_add_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1, // 64-bit
        Reg::W(_) => 0,                      // 32-bit
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0x8B000000 } else { 0x0B000000 };

    instruction |= (rm_enc as u32) << 16; // Rm
    instruction |= (rn_enc as u32) << 5; // Rn
    instruction |= rd_enc as u32; // Rd

    instruction
}

/// Encodes `STP X29, X30, [SP, #-16]!`.
/// AAPCS64 function prologue.
pub fn encode_stp_fp_lr() -> u32 {
    0xA9BF7BFD
}

/// Encodes `LDP X29, X30, [SP], #16`.
/// AAPCS64 function epilogue.
pub fn encode_ldp_fp_lr() -> u32 {
    0xA8C17BFD
}

/// Encodes `STR Xt, [Xn, #imm]` using unsigned immediate offset (scaled by 8).
pub fn encode_str_imm(rt: Reg, rn: Reg, offset_bytes: u16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_str_imm supports only 64-bit X registers"
    );
    assert_eq!(
        offset_bytes % 8,
        0,
        "STR immediate offset must be 8-byte aligned"
    );
    let imm12 = (offset_bytes / 8) as u32;
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    0xF9000000 | (imm12 << 10) | (rn_enc << 5) | rt_enc
}

/// Encodes `LDR Xt, [Xn, #imm]` using unsigned immediate offset (scaled by 8).
pub fn encode_ldr_imm(rt: Reg, rn: Reg, offset_bytes: u16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_ldr_imm supports only 64-bit X registers"
    );
    assert_eq!(
        offset_bytes % 8,
        0,
        "LDR immediate offset must be 8-byte aligned"
    );
    let imm12 = (offset_bytes / 8) as u32;
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    0xF9400000 | (imm12 << 10) | (rn_enc << 5) | rt_enc
}

/// Encodes `STUR Xt, [Xn, #imm9]` (unscaled, signed 9-bit byte offset).
pub fn encode_stur(rt: Reg, rn: Reg, offset_bytes: i16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_stur supports only 64-bit X registers"
    );
    assert!(
        matches!(rn, Reg::X(_) | Reg::SP),
        "encode_stur base must be Xn or SP"
    );
    assert!(
        (-256..=255).contains(&offset_bytes),
        "STUR offset must fit signed 9-bit"
    );
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    let imm9 = (offset_bytes as i32 & 0x1FF) as u32;
    0xF8000000 | (imm9 << 12) | (rn_enc << 5) | rt_enc
}

/// Encodes `LDUR Xt, [Xn, #imm9]` (unscaled, signed 9-bit byte offset).
pub fn encode_ldur(rt: Reg, rn: Reg, offset_bytes: i16) -> u32 {
    assert!(
        matches!(rt, Reg::X(_)),
        "encode_ldur supports only 64-bit X registers"
    );
    assert!(
        matches!(rn, Reg::X(_) | Reg::SP),
        "encode_ldur base must be Xn or SP"
    );
    assert!(
        (-256..=255).contains(&offset_bytes),
        "LDUR offset must fit signed 9-bit"
    );
    let rt_enc = rt.encode() as u32;
    let rn_enc = rn.encode() as u32;
    let imm9 = (offset_bytes as i32 & 0x1FF) as u32;
    0xF8400000 | (imm9 << 12) | (rn_enc << 5) | rt_enc
}
/// Encodes an ARM64 `SUB` (Subtract Immediate) instruction.
/// `SUB <Xd|Wd>, <Xn|Wn>, #<imm>`
pub fn encode_sub_imm(rd: Reg, rn: Reg, imm: u16) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();

    let mut instruction: u32 = if sf == 1 { 0xD1000000 } else { 0x51000000 };

    instruction |= (imm as u32) << 10;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

/// Encodes an ARM64 `SUB` (Subtract Register) instruction.
/// `SUB <Xd|Wd>, <Xn|Wn>, <Xm|Wm>`
pub fn encode_sub_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0xCB000000 } else { 0x4B000000 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

/// Encodes an ARM64 `MUL` (Multiply Register) instruction.
/// `MUL <Xd|Wd>, <Xn|Wn>, <Xm|Wm>`
pub fn encode_mul_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0x9B007C00 } else { 0x1B007C00 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

/// Encodes an ARM64 `SDIV` (Signed Divide Register) instruction.
/// `SDIV <Xd|Wd>, <Xn|Wn>, <Xm|Wm>`
pub fn encode_sdiv_reg(rd: Reg, rn: Reg, rm: Reg) -> u32 {
    let sf = match rd {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rd_enc = rd.encode();
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    // Verified encodings:
    // - sdiv x0, x1, x2 => 0x9AC20C20
    // - sdiv w0, w1, w2 => 0x1AC20C20
    let mut instruction: u32 = if sf == 1 { 0x9AC00C00 } else { 0x1AC00C00 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= rd_enc as u32;

    instruction
}

/// Encodes an ARM64 `BL` (Branch with Link) instruction.
/// Used for calling functions with relative addressing.
pub fn encode_bl(offset: i32) -> u32 {
    let imm26 = ((offset >> 2) & 0x3FFFFFF) as u32;
    0x94000000 | imm26
}

/// Encodes an ARM64 `BLR` (Branch with Link to Register) instruction.
/// Used for calling functions via absolute address in a register.
/// `BLR Xn`
pub fn encode_blr(rn: Reg) -> u32 {
    let rn_enc = rn.encode();
    0xD63F0000 | ((rn_enc as u32) << 5)
}

/// Encodes a 64-bit immediate value across 4 MOVZ/MOVK instructions.
/// Returns an array of 4 instructions to load the value.
pub fn encode_mov64(reg: Reg, value: u64) -> [u32; 4] {
    let reg_enc = reg.encode();

    let imm0 = (value & 0xFFFF) as u32;
    let imm1 = ((value >> 16) & 0xFFFF) as u32;
    let imm2 = ((value >> 32) & 0xFFFF) as u32;
    let imm3 = ((value >> 48) & 0xFFFF) as u32;

    let instr0 = 0xD2800000 | (0 << 21) | (imm0 << 5) | (reg_enc as u32); // MOVZ hw=0
    let instr1 = 0xF2800000 | (1 << 21) | (imm1 << 5) | (reg_enc as u32); // MOVK hw=1
    let instr2 = 0xF2800000 | (2 << 21) | (imm2 << 5) | (reg_enc as u32); // MOVK hw=2
    let instr3 = 0xF2800000 | (3 << 21) | (imm3 << 5) | (reg_enc as u32); // MOVK hw=3

    [instr0, instr1, instr2, instr3]
}

/// Encodes an ARM64 `CMP` (Compare Immediate) instruction.
/// `CMP <Xn|Wn>, #<imm>`
/// This is equivalent to `SUBS XZR, <Xn|Wn>, #<imm>`.
pub fn encode_cmp_imm(rn: Reg, imm: u16) -> u32 {
    let sf = match rn {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rn_enc = rn.encode();

    let mut instruction: u32 = if sf == 1 { 0xF1000000 } else { 0x71000000 };

    instruction |= (imm as u32) << 10;
    instruction |= (rn_enc as u32) << 5;
    instruction |= 31; // XZR destination

    instruction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_mov_imm() {
        // mov x0, #42 -> 0xD2800540
        assert_eq!(encode_mov_imm(Reg::X(0), 42), 0xD2800540);
        // mov w1, #10 -> 0x52800141
        assert_eq!(encode_mov_imm(Reg::W(1), 10), 0x52800141);
    }

    #[test]
    fn test_encode_ret() {
        // ret -> 0xD65F03C0
        assert_eq!(encode_ret(), 0xD65F03C0);
    }

    #[test]
    fn test_encode_add_imm() {
        // add x0, x0, #1 -> 0x91000400
        assert_eq!(encode_add_imm(Reg::X(0), Reg::X(0), 1), 0x91000400);
        // add x1, x2, #10 -> 0x91002841
        assert_eq!(encode_add_imm(Reg::X(1), Reg::X(2), 10), 0x91002841);
    }

    #[test]
    fn test_encode_add_reg() {
        // add x0, x1, x2 -> 0x8B020020
        assert_eq!(encode_add_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0x8B020020);
        // add w3, w4, w5 -> 0x0B050083
        assert_eq!(encode_add_reg(Reg::W(3), Reg::W(4), Reg::W(5)), 0x0B050083);
    }

    #[test]
    fn test_encode_prologue_epilogue() {
        assert_eq!(encode_stp_fp_lr(), 0xA9BF7BFD);
        assert_eq!(encode_ldp_fp_lr(), 0xA8C17BFD);
    }

    #[test]
    fn test_encode_sub_imm() {
        // sub x0, x0, #1 -> 0xD1000400
        assert_eq!(encode_sub_imm(Reg::X(0), Reg::X(0), 1), 0xD1000400);
    }

    #[test]
    fn test_encode_sub_reg() {
        // sub x0, x1, x2 -> 0xCB020020
        assert_eq!(encode_sub_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0xCB020020);
    }

    #[test]
    fn test_encode_mul_reg() {
        // mul x0, x1, x2 -> 0x9B027C20
        assert_eq!(encode_mul_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0x9B027C20);
    }

    #[test]
    fn test_encode_sdiv_reg() {
        // sdiv x0, x1, x2 -> 0x9AC20C20
        assert_eq!(encode_sdiv_reg(Reg::X(0), Reg::X(1), Reg::X(2)), 0x9AC20C20);
    }

    #[test]
    fn test_encode_bl() {
        // bl label -> offset 0x1000 (example)
        assert_eq!(
            encode_bl(0x1000),
            0x94000000 | ((0x1000 >> 2) & 0x3FFFFFF) as u32
        );
    }

    #[test]
    fn test_encode_blr() {
        // blr x1
        assert_eq!(encode_blr(Reg::X(1)), 0xD63F0000 | (1 << 5));
    }

    #[test]
    fn test_encode_mov64() {
        // mov x0, #0x123456789ABCDEF0
        let result = encode_mov64(Reg::X(0), 0x123456789ABCDEF0);
        assert_eq!(result[0], 0xD29BDE00);
        assert_eq!(result[1], 0xF2B35780);
        assert_eq!(result[2], 0xF2CACF00);
        assert_eq!(result[3], 0xF2E24680);
    }
}
