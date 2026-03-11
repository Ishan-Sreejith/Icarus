#![allow(dead_code)]

use crate::jit::encoder::Reg;
use std::collections::HashMap;

pub fn encode_b(offset: i32) -> u32 {
    let imm26 = (offset & 0x3FFFFFF) as u32;
    0x14000000 | imm26
}

pub fn encode_b_ne(offset: i32) -> u32 {
    let imm19 = ((offset >> 2) & 0x7FFFF) as u32;
    0x54000001 | (imm19 << 5)
}

pub fn encode_b_eq(offset: i32) -> u32 {
    let imm19 = ((offset >> 2) & 0x7FFFF) as u32;
    0x54000000 | (imm19 << 5)
}

pub fn encode_b_lt(offset: i32) -> u32 {
    let imm19 = ((offset >> 2) & 0x7FFFF) as u32;
    0x5400000B | (imm19 << 5)
}

pub fn encode_b_gt(offset: i32) -> u32 {
    let imm19 = ((offset >> 2) & 0x7FFFF) as u32;
    0x5400000C | (imm19 << 5)
}

pub fn encode_cmp_reg(rn: Reg, rm: Reg) -> u32 {
    let sf = match rn {
        Reg::X(_) | Reg::SP | Reg::XZR => 1,
        Reg::W(_) => 0,
    };
    let rn_enc = rn.encode();
    let rm_enc = rm.encode();

    let mut instruction: u32 = if sf == 1 { 0xEB000000 } else { 0x6B000000 };

    instruction |= (rm_enc as u32) << 16;
    instruction |= (rn_enc as u32) << 5;
    instruction |= 31; // ZR destination

    instruction
}

#[derive(Debug, Clone, Copy)]
pub enum BranchKind {
    B,
    BNe,
    BEq,
    BLt,
    BGt,
    BL,
}

pub struct LabelManager {
    labels: HashMap<String, usize>,
    pending: Vec<(usize, String, BranchKind)>,
}

impl LabelManager {
    pub fn new() -> Self {
        Self {
            labels: HashMap::new(),
            pending: Vec::new(),
        }
    }

    pub fn define_label(&mut self, name: &str, offset: usize) {
        self.labels.insert(name.to_string(), offset);
    }

    pub fn record_branch(&mut self, offset: usize, label: &str, kind: BranchKind) {
        self.pending.push((offset, label.to_string(), kind));
    }

    pub fn patch_branches(&self, code: &mut [u8]) -> Result<(), String> {
        for (branch_offset, label, kind) in &self.pending {
            let target = self
                .labels
                .get(label)
                .ok_or_else(|| format!("Undefined label: {}", label))?;

            let relative_bytes = *target as i32 - *branch_offset as i32;
            if std::env::var("CORE_JIT_DUMP").is_ok() {
                eprintln!(
                    "CORE_JIT_DUMP: patch {:?} at {:#x} -> {} {:#x} (rel {:+#x})",
                    kind, branch_offset, label, target, relative_bytes
                );
            }

            let branch_instr = match kind {
                BranchKind::B => encode_b(relative_bytes / 4),
                BranchKind::BNe => encode_b_ne(relative_bytes),
                BranchKind::BEq => encode_b_eq(relative_bytes),
                BranchKind::BLt => encode_b_lt(relative_bytes),
                BranchKind::BGt => encode_b_gt(relative_bytes),
                BranchKind::BL => crate::jit::encoder::encode_bl(relative_bytes),
            };
            let bytes = branch_instr.to_le_bytes();
            code[*branch_offset..*branch_offset + 4].copy_from_slice(&bytes);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_b() {
        let instr = encode_b(10);
        assert_eq!(instr & 0x3FFFFFF, 10);
    }

    #[test]
    fn test_encode_cmp_reg() {
        let instr = encode_cmp_reg(Reg::X(0), Reg::X(1));
        assert!(instr != 0);
    }

    #[test]
    fn test_label_manager() {
        let mut mgr = LabelManager::new();
        mgr.define_label("loop", 0);
        mgr.define_label("end", 16);
        assert_eq!(mgr.labels.get("loop"), Some(&0));
        assert_eq!(mgr.labels.get("end"), Some(&16));
    }
}
