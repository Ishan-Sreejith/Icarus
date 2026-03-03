//! JIT Compiler Module
//!
//! Low-level ARM64 JIT that lowers IR to machine code.
//!
//! This version uses the same `EncodedValue` model as `jit::runtime`:
//! - tagged ints (`(n<<1)|1`)
//! - `Rc<RefCell<GcData>>` pointers for heap values
//!
//! Most operations are implemented by calling `rt_*` helpers to keep the
//! machine-code surface area small and correct.

use crate::ir::{IrFunction, IrInstr, IrValue};
use crate::jit::branching::{BranchKind, LabelManager};
use crate::jit::context::JitContext;
use crate::jit::encoder::*;
use crate::jit::hotpath::HotpathTracker;
use crate::jit::memory::JitMemory;
use crate::jit::memory_table::MemoryTable;
use crate::jit::phase11::JitProfile;
use crate::jit::regalloc::{ArithmeticEncoder, Location, RegisterMap};
use crate::jit::runtime;
use crate::jit::symbol_table::SymbolTable;
use std::collections::{HashSet, VecDeque};

fn encode_int(val: i64) -> u64 {
    ((val as u64) << 1) | 1
}

const CALL_SAVE_BYTES: u16 = 8 * 8;

fn emit_save_call_regs(emit: &mut ArithmeticEncoder) {
    emit.emit_u32_le(encode_sub_imm(Reg::SP, Reg::SP, CALL_SAVE_BYTES));
    for r in 0..8u16 {
        emit.emit_u32_le(encode_str_imm(Reg::X(r as u8), Reg::SP, r * 8));
    }
}

fn emit_restore_call_regs(emit: &mut ArithmeticEncoder) {
    for r in 0..8u16 {
        emit.emit_u32_le(encode_ldr_imm(Reg::X(r as u8), Reg::SP, r * 8));
    }
    emit.emit_u32_le(encode_add_imm(Reg::SP, Reg::SP, CALL_SAVE_BYTES));
}

fn emit_mov64_to_reg(emit: &mut ArithmeticEncoder, reg: u8, value: u64) {
    for instr in encode_mov64(Reg::X(reg), value) {
        emit.emit_u32_le(instr);
    }
}

fn emit_call2(emit: &mut ArithmeticEncoder, func_addr: u64, arg0: Location, arg1: Location) {
    emit_save_call_regs(emit);
    emit.load_to_reg(9, arg0);
    emit.load_to_reg(10, arg1);
    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
    emit.emit_u32_le(encode_add_imm(Reg::X(1), Reg::X(10), 0));
    emit.emit_call(func_addr);
    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
    emit_restore_call_regs(emit);
}

fn emit_call1(emit: &mut ArithmeticEncoder, func_addr: u64, arg0: Location) {
    emit_save_call_regs(emit);
    emit.load_to_reg(9, arg0);
    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
    emit.emit_call(func_addr);
    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
    emit_restore_call_regs(emit);
}

fn emit_call3(
    emit: &mut ArithmeticEncoder,
    func_addr: u64,
    arg0: Location,
    arg1: Location,
    arg2: Location,
) {
    emit_save_call_regs(emit);
    emit.load_to_reg(9, arg0);
    emit.load_to_reg(10, arg1);
    emit.load_to_reg(11, arg2);
    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
    emit.emit_u32_le(encode_add_imm(Reg::X(1), Reg::X(10), 0));
    emit.emit_u32_le(encode_add_imm(Reg::X(2), Reg::X(11), 0));
    emit.emit_call(func_addr);
    emit_restore_call_regs(emit);
}

/// A JIT compiler that generates machine code from IR.
pub struct JitCompiler<'a> {
    regmap: RegisterMap,
    labels: LabelManager,
    context: &'a mut JitContext,
    locals: HashSet<String>,

    // Existing components (not fully wired yet, but kept for future work).
    profile: JitProfile,
    symbol_table: SymbolTable,
    memory_table: MemoryTable,
    hotpath_tracker: HotpathTracker,
}

impl<'a> JitCompiler<'a> {
    pub fn new(context: &'a mut JitContext) -> Self {
        Self {
            regmap: RegisterMap::new(),
            labels: LabelManager::new(),
            context,
            locals: HashSet::new(),
            profile: JitProfile::new(100, 1000),
            symbol_table: SymbolTable::new(),
            memory_table: MemoryTable::new(),
            hotpath_tracker: HotpathTracker::with_defaults(),
        }
    }

    pub fn compile_function(&mut self, func: &IrFunction) -> Result<(), String> {
        self.hotpath_tracker.record_function_call(&func.name);
        if self.profile.tick_call() {
            self.profile.promote();
        }

        self.regmap.clear();
        self.labels = LabelManager::new();
        self.locals.clear();

        self.symbol_table.enter_scope();
        self.memory_table.push_frame(func.name.len() as u64, None);

        for (i, param) in func.params.iter().enumerate() {
            if i >= 8 {
                return Err("More than 8 parameters not supported yet".to_string());
            }
            let loc = self.regmap.alloc(param)?;
            if let Location::Register(r) = loc {
                if r != i as u8 {
                    return Err(format!(
                        "Parameter {} allocated to wrong register {}",
                        param, r
                    ));
                }
            }
            self.locals.insert(param.clone());
        }

        let code = self.compile(&func.instructions, func.params.len())?;

        let mut mem = JitMemory::new(code.len()).map_err(|e| e.to_string())?;
        mem.write_code(0, &code).map_err(|e| e.to_string())?;
        mem.make_executable().map_err(|e| e.to_string())?;

        let addr = mem.as_ptr() as u64;
        self.context.register_function(&func.name, addr);
        self.context.add_code_block(mem);

        self.symbol_table.exit_scope();
        if let Some(allocations) = self.memory_table.pop_frame() {
            for alloc_id in allocations {
                self.memory_table.decrement_ref(alloc_id);
            }
        }

        Ok(())
    }

    pub fn compile(&mut self, instrs: &[IrInstr], preserve_arg_regs: usize) -> Result<Vec<u8>, String> {
        let mut emit = ArithmeticEncoder::new();
        let mut has_explicit_return = false;

        let rt_print = runtime::rt_print as *const () as u64;
        let rt_release = runtime::rt_release as *const () as u64;
        let rt_retain = runtime::rt_retain as *const () as u64;
        let rt_alloc_string = runtime::rt_alloc_string as *const () as u64;
        let rt_alloc_list = runtime::rt_alloc_list as *const () as u64;
        let rt_list_push = runtime::rt_list_push as *const () as u64;
        let rt_alloc_map = runtime::rt_alloc_map as *const () as u64;
        let rt_map_set = runtime::rt_map_set as *const () as u64;
        let rt_index_get = runtime::rt_index_get as *const () as u64;
        let rt_add = runtime::rt_add as *const () as u64;
        let rt_sub = runtime::rt_sub as *const () as u64;
        let rt_mul = runtime::rt_mul as *const () as u64;
        let rt_div = runtime::rt_div as *const () as u64;
        let rt_eq = runtime::rt_eq as *const () as u64;
        let rt_ne = runtime::rt_ne as *const () as u64;
        let rt_lt = runtime::rt_lt as *const () as u64;
        let rt_gt = runtime::rt_gt as *const () as u64;
        let rt_and = runtime::rt_and as *const () as u64;
        let rt_or = runtime::rt_or as *const () as u64;
        let rt_not = runtime::rt_not as *const () as u64;
        let rt_is_truthy = runtime::rt_is_truthy as *const () as u64;

        // Pre-allocate all destinations for stable stack frame sizing.
        for instr in instrs {
            let maybe_dest: Option<&str> = match instr {
                IrInstr::LoadConst { dest, .. }
                | IrInstr::Add { dest, .. }
                | IrInstr::Sub { dest, .. }
                | IrInstr::Mul { dest, .. }
                | IrInstr::Div { dest, .. }
                | IrInstr::Move { dest, .. }
                | IrInstr::AllocMap { dest }
                | IrInstr::AllocList { dest, .. }
                | IrInstr::AllocStruct { dest, .. }
                | IrInstr::GetIndex { dest, .. }
                | IrInstr::GetMember { dest, .. }
                | IrInstr::GetMap { dest, .. }
                | IrInstr::Lt { dest, .. }
                | IrInstr::Gt { dest, .. }
                | IrInstr::Eq { dest, .. }
                | IrInstr::Ne { dest, .. }
                | IrInstr::LogicNot { dest, .. }
                | IrInstr::LogicAnd { dest, .. }
                | IrInstr::LogicOr { dest, .. } => Some(dest.as_str()),
                IrInstr::Call {
                    dest: Some(dest), ..
                } => Some(dest.as_str()),
                _ => None,
            };
            if let Some(dest) = maybe_dest {
                if self.regmap.get(dest).is_none() {
                    let _ = self.regmap.alloc(dest);
                }
            }
        }

        let frame_bytes = self.regmap.stack_frame_bytes();
        emit.emit_u32_le(encode_stp_fp_lr());
        emit.emit_u32_le(encode_add_imm(Reg::X(29), Reg::SP, 0));
        if frame_bytes != 0 {
            emit.emit_u32_le(encode_sub_imm(Reg::SP, Reg::SP, frame_bytes));
        }

        // Initialize locals/temps to 0 so teardown `rt_release` is safe even for values that are
        // only assigned on some control-flow paths (e.g. if/else).
        //
        // IMPORTANT: when compiling a function, x0.. are used for incoming parameters. Don't
        // clobber those registers here; only clear stack slots and non-arg registers.
        emit_mov64_to_reg(&mut emit, 9, 0);
        for loc in self.regmap.var_map.values().copied() {
            match loc {
                Location::Stack(_) => emit.store_from_reg(9, loc),
                Location::Register(r) => {
                    if (r as usize) >= preserve_arg_regs {
                        emit.store_from_reg(9, loc);
                    }
                }
            }
        }

        // Track which variables have been initialized so we can release on overwrite/teardown.
        let mut assigned: HashSet<String> = HashSet::new();
        let mut assigned_order: VecDeque<String> = VecDeque::new();

        for instr in instrs {
            match instr {
                IrInstr::LoadConst { dest, value } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    match value {
                        IrValue::Number(n) => {
                            emit_mov64_to_reg(&mut emit, 9, encode_int(*n as i64));
                            emit.store_from_reg(9, dst_loc);
                        }
                        IrValue::Bool(b) => {
                            emit_mov64_to_reg(&mut emit, 9, encode_int(if *b { 1 } else { 0 }));
                            emit.store_from_reg(9, dst_loc);
                        }
                        IrValue::String(s) => {
                            let (ptr, len) = self.context.intern_bytes(s.as_bytes().to_vec());
                            emit_save_call_regs(&mut emit);
                            emit_mov64_to_reg(&mut emit, 0, ptr as u64);
                            emit_mov64_to_reg(&mut emit, 1, len as u64);
                            emit.emit_call(rt_alloc_string);
                            emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
                            emit_restore_call_regs(&mut emit);
                            emit.store_from_reg(9, dst_loc);
                        }
                    }
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Move { dest, src } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let src_loc = self
                        .regmap
                        .get(src)
                        .ok_or_else(|| format!("Undefined var: {}", src))?;

                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(0, src_loc);
                    emit.emit_call(rt_retain);
                    emit_restore_call_regs(&mut emit);

                    emit.emit_mov(dst_loc, src_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Add { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_add, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Sub { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_sub, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Mul { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_mul, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Div { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_div, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Eq { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_eq, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Ne { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_ne, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::LogicNot { dest, src } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let s = self
                        .regmap
                        .get(src)
                        .ok_or_else(|| format!("Undefined var: {}", src))?;
                    emit_call1(&mut emit, rt_not, s);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::LogicAnd { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_and, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::LogicOr { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_or, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Lt { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_lt, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Gt { dest, left, right } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let l = self
                        .regmap
                        .get(left)
                        .ok_or_else(|| format!("Undefined var: {}", left))?;
                    let r = self
                        .regmap
                        .get(right)
                        .ok_or_else(|| format!("Undefined var: {}", right))?;
                    emit_call2(&mut emit, rt_gt, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::AllocList { dest, items } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;

                    emit_save_call_regs(&mut emit);
                    emit_mov64_to_reg(&mut emit, 0, items.len() as u64);
                    emit.emit_call(rt_alloc_list);
                    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
                    emit_restore_call_regs(&mut emit);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }

                    let list_loc = self
                        .regmap
                        .get(dest)
                        .ok_or_else(|| format!("Undefined var: {}", dest))?;
                    for item in items {
                        let item_loc = self
                            .regmap
                            .get(item)
                            .ok_or_else(|| format!("Undefined var: {}", item))?;
                        emit_call2(&mut emit, rt_list_push, list_loc, item_loc);
                    }
                }
                IrInstr::AllocMap { dest } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    emit_save_call_regs(&mut emit);
                    emit.emit_call(rt_alloc_map);
                    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
                    emit_restore_call_regs(&mut emit);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::SetMap { map, key, value } => {
                    let map_loc = self
                        .regmap
                        .get(map)
                        .ok_or_else(|| format!("Undefined var: {}", map))?;
                    let key_loc = self
                        .regmap
                        .get(key)
                        .ok_or_else(|| format!("Undefined var: {}", key))?;
                    let val_loc = self
                        .regmap
                        .get(value)
                        .ok_or_else(|| format!("Undefined var: {}", value))?;
                    emit_call3(&mut emit, rt_map_set, map_loc, key_loc, val_loc);
                }
                IrInstr::GetIndex { dest, src, index } => {
                    if assigned.contains(dest) {
                        let loc = self
                            .regmap
                            .get(dest)
                            .ok_or_else(|| format!("Undefined var: {}", dest))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(0, loc);
                        emit.emit_call(rt_release);
                        emit_restore_call_regs(&mut emit);
                    }
                    let dst_loc = self.regmap.alloc(dest)?;
                    let src_loc = self
                        .regmap
                        .get(src)
                        .ok_or_else(|| format!("Undefined var: {}", src))?;
                    let idx_loc = self
                        .regmap
                        .get(index)
                        .ok_or_else(|| format!("Undefined var: {}", index))?;
                    emit_call2(&mut emit, rt_index_get, src_loc, idx_loc);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::Print { src } => {
                    let src_loc = self
                        .regmap
                        .get(src)
                        .ok_or_else(|| format!("Undefined var: {}", src))?;
                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(9, src_loc);
                    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
                    emit.emit_call(rt_print);
                    emit_restore_call_regs(&mut emit);
                }
                IrInstr::PrintNum { src } => {
                    let src_loc = self
                        .regmap
                        .get(src)
                        .ok_or_else(|| format!("Undefined var: {}", src))?;
                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(9, src_loc);
                    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
                    emit.emit_call(rt_print);
                    emit_restore_call_regs(&mut emit);
                }
                IrInstr::Call { dest, func, args } => {
                    emit_save_call_regs(&mut emit);
                    if args.len() > 8 {
                        return Err("More than 8 call args not supported yet".to_string());
                    }
                    for (i, arg) in args.iter().enumerate() {
                        let loc = self
                            .regmap
                            .get(arg)
                            .ok_or_else(|| format!("Undefined var: {}", arg))?;
                        emit.load_to_reg(i as u8, loc);
                    }
                    let addr = self
                        .context
                        .get_function_addr(func)
                        .ok_or_else(|| format!("Unknown function: {}", func))?;
                    emit.emit_call(addr);
                    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
                    emit_restore_call_regs(&mut emit);
                    if let Some(dest) = dest {
                        if assigned.contains(dest) {
                            let loc = self
                                .regmap
                                .get(dest)
                                .ok_or_else(|| format!("Undefined var: {}", dest))?;
                            emit_save_call_regs(&mut emit);
                            emit.load_to_reg(0, loc);
                            emit.emit_call(rt_release);
                            emit_restore_call_regs(&mut emit);
                        }
                        let dst_loc = self.regmap.alloc(dest)?;
                        emit.store_from_reg(9, dst_loc);
                        if assigned.insert(dest.to_string()) {
                            assigned_order.push_back(dest.to_string());
                        }
                    }
                }
                IrInstr::Label { name } => self.labels.define_label(name, emit.len()),
                IrInstr::Jump { label } => {
                    let off = emit.len();
                    emit.emit_u32_le(0);
                    self.labels.record_branch(off, label, BranchKind::B);
                }
                IrInstr::JumpIf { cond, label } => {
                    let cond_loc = self
                        .regmap
                        .get(cond)
                        .ok_or_else(|| format!("Undefined var: {}", cond))?;
                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(9, cond_loc);
                    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
                    emit.emit_call(rt_is_truthy);
                    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
                    emit_restore_call_regs(&mut emit);
                    // rt_is_truthy returns an EncodedValue int (0/1), i.e. 1 for false and 3 for true.
                    // We stash the call result into x9 before restoring x0-x7, so compare x9 against
                    // encode_int(0) == 1.
                    emit.emit_u32_le(encode_cmp_imm(Reg::X(9), 1)); // encode_int(0)
                    let off = emit.len();
                    emit.emit_u32_le(0);
                    self.labels.record_branch(off, label, BranchKind::BNe);
                }
                IrInstr::Return { value } => {
                    has_explicit_return = true;
                    let ret_var = value.as_deref();
                    if let Some(v) = value {
                        let loc = self
                            .regmap
                            .get(v)
                            .ok_or_else(|| format!("Undefined var: {}", v))?;
                        emit.load_to_reg(0, loc);
                    } else {
                        emit_mov64_to_reg(&mut emit, 0, encode_int(0));
                    }

                    for v in assigned_order.iter() {
                        if ret_var == Some(v.as_str()) {
                            continue;
                        }
                        if let Some(loc) = self.regmap.get(v) {
                            emit_save_call_regs(&mut emit);
                            emit.load_to_reg(0, loc);
                            emit.emit_call(rt_release);
                            emit_restore_call_regs(&mut emit);
                        }
                    }

                    if frame_bytes != 0 {
                        emit.emit_u32_le(encode_add_imm(Reg::SP, Reg::SP, frame_bytes));
                    }
                    emit.emit_u32_le(encode_ldp_fp_lr());
                    emit.emit_u32_le(encode_ret());
                }
                _ => {}
            }
        }

        if !has_explicit_return {
            let last_var = instrs.iter().rev().find_map(|instr| match instr {
                IrInstr::LoadConst { dest, .. }
                | IrInstr::Add { dest, .. }
                | IrInstr::Sub { dest, .. }
                | IrInstr::Mul { dest, .. }
                | IrInstr::Div { dest, .. }
                | IrInstr::Move { dest, .. }
                | IrInstr::Lt { dest, .. }
                | IrInstr::Gt { dest, .. }
                | IrInstr::Eq { dest, .. }
                | IrInstr::AllocMap { dest }
                | IrInstr::AllocList { dest, .. }
                | IrInstr::GetIndex { dest, .. } => Some(dest.as_str()),
                IrInstr::Call {
                    dest: Some(dest), ..
                } => Some(dest.as_str()),
                _ => None,
            });

            if let Some(v) = last_var {
                if let Some(loc) = self.regmap.get(v) {
                    emit.load_to_reg(0, loc);
                } else {
                    emit_mov64_to_reg(&mut emit, 0, encode_int(0));
                }
            } else {
                emit_mov64_to_reg(&mut emit, 0, encode_int(0));
            }

            for v in assigned_order.iter() {
                if last_var == Some(v.as_str()) {
                    continue;
                }
                if let Some(loc) = self.regmap.get(v) {
                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(0, loc);
                    emit.emit_call(rt_release);
                    emit_restore_call_regs(&mut emit);
                }
            }

            if frame_bytes != 0 {
                emit.emit_u32_le(encode_add_imm(Reg::SP, Reg::SP, frame_bytes));
            }
            emit.emit_u32_le(encode_ldp_fp_lr());
            emit.emit_u32_le(encode_ret());
        }

        let mut code = emit.into_bytes();
        self.labels.patch_branches(&mut code)?;
        Ok(code)
    }

    pub fn execute_global(&mut self, instrs: &[IrInstr]) -> Result<u64, String> {
        let code = self.compile(instrs, 0)?;

        let mut mem = JitMemory::new(code.len()).map_err(|e| e.to_string())?;
        if std::env::var("CORE_JIT_DUMP").is_ok() {
            if let Err(e) = std::fs::write("/tmp/core_jit_dump.bin", &code) {
                eprintln!("CORE_JIT_DUMP: failed to write /tmp/core_jit_dump.bin: {}", e);
            } else {
                eprintln!("CORE_JIT_DUMP: wrote {} bytes to /tmp/core_jit_dump.bin", code.len());
            }
        }
        mem.write_code(0, &code).map_err(|e| e.to_string())?;
        mem.make_executable().map_err(|e| e.to_string())?;

        let func: extern "C" fn() -> u64 = unsafe { std::mem::transmute(mem.as_ptr()) };
        let result = func();
        self.context.add_code_block(mem);

        Ok(result)
    }
}
