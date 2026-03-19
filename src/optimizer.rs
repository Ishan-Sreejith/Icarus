use crate::ir::{IrInstr, IrProgram, IrValue};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone, Copy)]
pub struct OptimizationStats {
    pub const_folds: usize,
    pub removed_dead: usize,
    pub simplified_branches: usize,
}

pub fn optimize_program(program: &mut IrProgram) -> OptimizationStats {
    let mut stats = OptimizationStats::default();
    program.global_code = optimize_block(&program.global_code, &mut stats);
    for func in program.functions.values_mut() {
        func.instructions = optimize_block(&func.instructions, &mut stats);
    }
    stats
}

fn optimize_block(instrs: &[IrInstr], stats: &mut OptimizationStats) -> Vec<IrInstr> {
    let mut consts: HashMap<String, IrValue> = HashMap::new();
    let mut folded: Vec<IrInstr> = Vec::with_capacity(instrs.len());

    for instr in instrs {
        match instr {
            IrInstr::Label { .. } => {
                consts.clear();
                folded.push(instr.clone());
            }
            IrInstr::Jump { .. } | IrInstr::Return { .. } => {
                consts.clear();
                folded.push(instr.clone());
            }
            IrInstr::LoadConst { dest, value } => {
                consts.insert(dest.clone(), value.clone());
                folded.push(instr.clone());
            }
            IrInstr::Move { dest, src } => {
                if dest == src {
                    stats.removed_dead += 1;
                    continue;
                }
                if let Some(value) = consts.get(src).cloned() {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Add { dest, left, right } => {
                if let Some(value) = fold_add(left, right, &consts) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Sub { dest, left, right } => {
                if let Some(value) = fold_num_bin(left, right, &consts, |a, b| a - b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Mul { dest, left, right } => {
                if let Some(value) = fold_num_bin(left, right, &consts, |a, b| a * b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Div { dest, left, right } => {
                if let Some(value) = fold_num_bin_nonzero(left, right, &consts, |a, b| a / b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::FAdd { dest, left, right } => {
                if let Some(value) = fold_num_bin(left, right, &consts, |a, b| a + b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::FSub { dest, left, right } => {
                if let Some(value) = fold_num_bin(left, right, &consts, |a, b| a - b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::FMul { dest, left, right } => {
                if let Some(value) = fold_num_bin(left, right, &consts, |a, b| a * b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::FDiv { dest, left, right } => {
                if let Some(value) = fold_num_bin_nonzero(left, right, &consts, |a, b| a / b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Eq { dest, left, right } => {
                if let Some(value) = fold_eq(left, right, &consts, true) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Ne { dest, left, right } => {
                if let Some(value) = fold_eq(left, right, &consts, false) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Lt { dest, left, right } => {
                if let Some(value) = fold_cmp(left, right, &consts, |a, b| a < b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Gt { dest, left, right } => {
                if let Some(value) = fold_cmp(left, right, &consts, |a, b| a > b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::LogicAnd { dest, left, right } => {
                if let Some(value) = fold_bool_bin(left, right, &consts, |a, b| a && b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::LogicOr { dest, left, right } => {
                if let Some(value) = fold_bool_bin(left, right, &consts, |a, b| a || b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::LogicNot { dest, src } => {
                if let Some(value) = fold_bool_not(src, &consts) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::BitAnd { dest, left, right } => {
                if let Some(value) = fold_int_bin(left, right, &consts, |a, b| a & b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::BitOr { dest, left, right } => {
                if let Some(value) = fold_int_bin(left, right, &consts, |a, b| a | b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::BitXor { dest, left, right } => {
                if let Some(value) = fold_int_bin(left, right, &consts, |a, b| a ^ b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::BitNot { dest, src } => {
                if let Some(value) = fold_int_not(src, &consts) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Shl { dest, left, right } => {
                if let Some(value) = fold_int_bin(left, right, &consts, |a, b| a << b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::Shr { dest, left, right } => {
                if let Some(value) = fold_int_bin(left, right, &consts, |a, b| a >> b) {
                    consts.insert(dest.clone(), value.clone());
                    stats.const_folds += 1;
                    folded.push(IrInstr::LoadConst {
                        dest: dest.clone(),
                        value,
                    });
                } else {
                    consts.remove(dest);
                    folded.push(instr.clone());
                }
            }
            IrInstr::JumpIf { cond, label } => {
                if let Some(value) = fold_truthy(cond, &consts) {
                    stats.simplified_branches += 1;
                    if value {
                        folded.push(IrInstr::Jump { label: label.clone() });
                    } else {
                        // Drop the branch entirely (fallthrough)
                    }
                } else {
                    folded.push(instr.clone());
                }
                consts.clear();
            }
            IrInstr::Call { dest, .. } => {
                if let Some(d) = dest {
                    consts.remove(d);
                }
                folded.push(instr.clone());
            }
            IrInstr::Input { dest, .. }
            | IrInstr::AllocStruct { dest, .. }
            | IrInstr::AllocList { dest, .. }
            | IrInstr::AllocMap { dest }
            | IrInstr::GetIndex { dest, .. }
            | IrInstr::GetMember { dest, .. }
            | IrInstr::GetMap { dest, .. }
            | IrInstr::Await { dest, .. } => {
                consts.remove(dest);
                folded.push(instr.clone());
            }
            _ => {
                folded.push(instr.clone());
            }
        }
    }

    let has_control_flow = folded.iter().any(|instr| {
        matches!(
            instr,
            IrInstr::Jump { .. } | IrInstr::JumpIf { .. } | IrInstr::Label { .. }
        )
    });

    if has_control_flow {
        // DCE without CFG analysis is unsafe around jumps/labels (loop back-edges).
        return folded;
    }

    let mut live: HashSet<String> = HashSet::new();
    let mut out: Vec<IrInstr> = Vec::with_capacity(folded.len());
    for instr in folded.iter().rev() {
        let def = instr_def(instr);
        let uses = instr_uses(instr);
        let has_side_effect = instr_side_effect(instr);

        let keep = match def {
            Some(d) => live.contains(d) || has_side_effect,
            None => true,
        };

        if keep {
            out.push(instr.clone());
        } else {
            stats.removed_dead += 1;
        }

        if let Some(d) = def {
            live.remove(d);
        }
        for u in uses {
            live.insert(u.to_string());
        }
    }
    out.reverse();
    out
}

fn fold_add(left: &str, right: &str, consts: &HashMap<String, IrValue>) -> Option<IrValue> {
    let l = consts.get(left)?;
    let r = consts.get(right)?;
    match (l, r) {
        (IrValue::Number(a), IrValue::Number(b)) => Some(IrValue::Number(a + b)),
        (IrValue::String(a), IrValue::String(b)) => Some(IrValue::String(format!("{}{}", a, b))),
        _ => None,
    }
}

fn fold_num_bin(
    left: &str,
    right: &str,
    consts: &HashMap<String, IrValue>,
    op: impl FnOnce(f64, f64) -> f64,
) -> Option<IrValue> {
    match (consts.get(left)?, consts.get(right)?) {
        (IrValue::Number(a), IrValue::Number(b)) => Some(IrValue::Number(op(*a, *b))),
        _ => None,
    }
}

fn fold_num_bin_nonzero(
    left: &str,
    right: &str,
    consts: &HashMap<String, IrValue>,
    op: impl FnOnce(f64, f64) -> f64,
) -> Option<IrValue> {
    match (consts.get(left)?, consts.get(right)?) {
        (IrValue::Number(a), IrValue::Number(b)) if *b != 0.0 => {
            Some(IrValue::Number(op(*a, *b)))
        }
        _ => None,
    }
}

fn fold_cmp(
    left: &str,
    right: &str,
    consts: &HashMap<String, IrValue>,
    op: impl FnOnce(f64, f64) -> bool,
) -> Option<IrValue> {
    match (consts.get(left)?, consts.get(right)?) {
        (IrValue::Number(a), IrValue::Number(b)) => Some(IrValue::Bool(op(*a, *b))),
        _ => None,
    }
}

fn fold_eq(
    left: &str,
    right: &str,
    consts: &HashMap<String, IrValue>,
    eq: bool,
) -> Option<IrValue> {
    let l = consts.get(left)?;
    let r = consts.get(right)?;
    let res = match (l, r) {
        (IrValue::Number(a), IrValue::Number(b)) => a == b,
        (IrValue::String(a), IrValue::String(b)) => a == b,
        (IrValue::Bool(a), IrValue::Bool(b)) => a == b,
        _ => false,
    };
    Some(IrValue::Bool(if eq { res } else { !res }))
}

fn fold_bool_bin(
    left: &str,
    right: &str,
    consts: &HashMap<String, IrValue>,
    op: impl FnOnce(bool, bool) -> bool,
) -> Option<IrValue> {
    match (consts.get(left)?, consts.get(right)?) {
        (IrValue::Bool(a), IrValue::Bool(b)) => Some(IrValue::Bool(op(*a, *b))),
        _ => None,
    }
}

fn fold_bool_not(src: &str, consts: &HashMap<String, IrValue>) -> Option<IrValue> {
    match consts.get(src)? {
        IrValue::Bool(b) => Some(IrValue::Bool(!b)),
        _ => None,
    }
}

fn fold_int_bin(
    left: &str,
    right: &str,
    consts: &HashMap<String, IrValue>,
    op: impl FnOnce(i64, i64) -> i64,
) -> Option<IrValue> {
    match (consts.get(left)?, consts.get(right)?) {
        (IrValue::Number(a), IrValue::Number(b)) => {
            Some(IrValue::Number(op(*a as i64, *b as i64) as f64))
        }
        _ => None,
    }
}

fn fold_int_not(src: &str, consts: &HashMap<String, IrValue>) -> Option<IrValue> {
    match consts.get(src)? {
        IrValue::Number(a) => Some(IrValue::Number((!(*a as i64)) as f64)),
        _ => None,
    }
}

fn fold_truthy(cond: &str, consts: &HashMap<String, IrValue>) -> Option<bool> {
    match consts.get(cond)? {
        IrValue::Bool(b) => Some(*b),
        IrValue::Number(n) => Some(*n != 0.0),
        _ => None,
    }
}

fn instr_def(instr: &IrInstr) -> Option<&str> {
    match instr {
        IrInstr::Add { dest, .. }
        | IrInstr::Sub { dest, .. }
        | IrInstr::Mul { dest, .. }
        | IrInstr::Div { dest, .. }
        | IrInstr::FAdd { dest, .. }
        | IrInstr::FSub { dest, .. }
        | IrInstr::FMul { dest, .. }
        | IrInstr::FDiv { dest, .. }
        | IrInstr::Eq { dest, .. }
        | IrInstr::Ne { dest, .. }
        | IrInstr::Lt { dest, .. }
        | IrInstr::Gt { dest, .. }
        | IrInstr::LogicAnd { dest, .. }
        | IrInstr::LogicOr { dest, .. }
        | IrInstr::LogicNot { dest, .. }
        | IrInstr::BitAnd { dest, .. }
        | IrInstr::BitOr { dest, .. }
        | IrInstr::BitXor { dest, .. }
        | IrInstr::BitNot { dest, .. }
        | IrInstr::Shl { dest, .. }
        | IrInstr::Shr { dest, .. }
        | IrInstr::AllocStruct { dest, .. }
        | IrInstr::LoadConst { dest, .. }
        | IrInstr::Move { dest, .. }
        | IrInstr::AllocList { dest, .. }
        | IrInstr::GetIndex { dest, .. }
        | IrInstr::AllocMap { dest }
        | IrInstr::GetMap { dest, .. }
        | IrInstr::GetMember { dest, .. }
        | IrInstr::Input { dest, .. }
        | IrInstr::Await { dest, .. } => Some(dest),
        IrInstr::Call { dest: Some(dest), .. } => Some(dest),
        _ => None,
    }
}

fn instr_uses<'a>(instr: &'a IrInstr) -> Vec<&'a str> {
    match instr {
        IrInstr::Add { left, right, .. }
        | IrInstr::Sub { left, right, .. }
        | IrInstr::Mul { left, right, .. }
        | IrInstr::Div { left, right, .. }
        | IrInstr::FAdd { left, right, .. }
        | IrInstr::FSub { left, right, .. }
        | IrInstr::FMul { left, right, .. }
        | IrInstr::FDiv { left, right, .. }
        | IrInstr::Eq { left, right, .. }
        | IrInstr::Ne { left, right, .. }
        | IrInstr::Lt { left, right, .. }
        | IrInstr::Gt { left, right, .. }
        | IrInstr::LogicAnd { left, right, .. }
        | IrInstr::LogicOr { left, right, .. }
        | IrInstr::BitAnd { left, right, .. }
        | IrInstr::BitOr { left, right, .. }
        | IrInstr::BitXor { left, right, .. }
        | IrInstr::Shl { left, right, .. }
        | IrInstr::Shr { left, right, .. } => vec![left, right],
        IrInstr::LogicNot { src, .. } | IrInstr::BitNot { src, .. } => vec![src],
        IrInstr::Move { src, .. } => vec![src],
        IrInstr::AllocList { items, .. } => items.iter().map(|s| s.as_str()).collect(),
        IrInstr::GetIndex { src, index, .. } => vec![src, index],
        IrInstr::SetIndex { src, index, value } => vec![src, index, value],
        IrInstr::SetMap { map, key, value } => vec![map, key, value],
        IrInstr::GetMap { map, key, .. } => vec![map, key],
        IrInstr::SetMember { obj, value, .. } => vec![obj, value],
        IrInstr::GetMember { obj, .. } => vec![obj],
        IrInstr::Print { src } | IrInstr::PrintNum { src } => vec![src],
        IrInstr::Input { prompt, .. } => vec![prompt],
        IrInstr::Call { args, .. } => args.iter().map(|s| s.as_str()).collect(),
        IrInstr::Return { value: Some(v) } => vec![v],
        IrInstr::JumpIf { cond, .. } => vec![cond],
        IrInstr::CloseFile { handle } => vec![handle],
        IrInstr::Spawn { task } => vec![task],
        IrInstr::Await { task, .. } => vec![task],
        IrInstr::PreScan { target } => vec![target],
        _ => Vec::new(),
    }
}

fn instr_side_effect(instr: &IrInstr) -> bool {
    match instr {
        IrInstr::Print { .. }
        | IrInstr::PrintNum { .. }
        | IrInstr::Input { .. }
        | IrInstr::SetIndex { .. }
        | IrInstr::SetMap { .. }
        | IrInstr::SetMember { .. }
        | IrInstr::Call { .. }
        | IrInstr::Return { .. }
        | IrInstr::Jump { .. }
        | IrInstr::JumpIf { .. }
        | IrInstr::Label { .. }
        | IrInstr::AllocFile { .. }
        | IrInstr::CloseFile { .. }
        | IrInstr::Spawn { .. }
        | IrInstr::Await { .. }
        | IrInstr::LinkFile { .. }
        | IrInstr::Hardwire { .. }
        | IrInstr::PreScan { .. } => true,
        _ => false,
    }
}
