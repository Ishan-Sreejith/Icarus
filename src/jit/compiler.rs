
use crate::ir::{IrFunction, IrInstr, IrValue};
use crate::jit::branching::{BranchKind, LabelManager};
use crate::jit::context::JitContext;
use crate::jit::encoder::*;
use crate::jit::hotpath::HotpathTracker;
use crate::jit::memory::JitMemory;
use crate::jit::memory_table::{AllocationType, MemoryTable};
use crate::jit::phase11::JitProfile;
use crate::jit::regalloc::{ArithmeticEncoder, Location, RegisterMap};
use crate::jit::runtime;
use crate::jit::symbol_table::{SymbolLocation, SymbolTable, ValueType};
use std::cmp::Reverse;
use std::collections::{HashMap, HashSet, VecDeque};

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

pub struct JitCompiler<'a> {
    regmap: RegisterMap,
    labels: LabelManager,
    context: &'a mut JitContext,
    locals: HashSet<String>,
    current_function: Option<String>,

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
            current_function: None,
            profile: JitProfile::new(100, 1000),
            symbol_table: SymbolTable::new(),
            memory_table: MemoryTable::new(),
            hotpath_tracker: HotpathTracker::with_defaults(),
        }
    }

    pub fn compile_function(&mut self, func: &IrFunction) -> Result<(), String> {
        self.current_function = Some(func.name.clone());
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
            let sym_loc = Self::location_to_symbol(loc);
            let _ = self
                .symbol_table
                .declare_variable(param.clone(), ValueType::Unknown, sym_loc);
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
        self.current_function = None;

        Ok(())
    }

    pub fn compile(&mut self, instrs: &[IrInstr], preserve_arg_regs: usize) -> Result<Vec<u8>, String> {
        let mut emit = ArithmeticEncoder::new();
        let mut has_explicit_return = false;
        self.labels.define_label("__fn_entry", emit.len());

        let rt_print = runtime::rt_print as *const () as u64;
        let rt_release = runtime::rt_release as *const () as u64;
        let rt_retain = runtime::rt_retain as *const () as u64;
        let rt_alloc_string = runtime::rt_alloc_string as *const () as u64;
        let rt_alloc_list = runtime::rt_alloc_list as *const () as u64;
        let rt_list_push = runtime::rt_list_push as *const () as u64;
        let rt_alloc_map = runtime::rt_alloc_map as *const () as u64;
        let rt_map_set = runtime::rt_map_set as *const () as u64;
        let rt_map_get = runtime::rt_map_get as *const () as u64;
        let rt_map_keys = runtime::rt_map_keys as *const () as u64;
        let rt_map_values = runtime::rt_map_values as *const () as u64;
        let rt_index_get = runtime::rt_index_get as *const () as u64;
        let rt_index_set = runtime::rt_index_set as *const () as u64;
        let rt_list_pop = runtime::rt_list_pop as *const () as u64;
        let rt_list_len = runtime::rt_list_len as *const () as u64;
        let rt_to_str = runtime::rt_to_str as *const () as u64;
        let rt_to_num = runtime::rt_to_num as *const () as u64;
        let rt_range = runtime::rt_range as *const () as u64;
        let rt_abs = runtime::rt_abs as *const () as u64;
        let rt_min = runtime::rt_min as *const () as u64;
        let rt_max = runtime::rt_max as *const () as u64;
        let rt_sqrt = runtime::rt_sqrt as *const () as u64;
        let rt_pow = runtime::rt_pow as *const () as u64;
        let rt_contains = runtime::rt_contains as *const () as u64;
        let rt_is_map = runtime::rt_is_map as *const () as u64;
        let rt_is_list = runtime::rt_is_list as *const () as u64;
        let rt_is_string = runtime::rt_is_string as *const () as u64;
        let rt_float_add = runtime::rt_float_add as *const () as u64;
        let rt_float_sub = runtime::rt_float_sub as *const () as u64;
        let rt_float_mul = runtime::rt_float_mul as *const () as u64;
        let rt_float_div = runtime::rt_float_div as *const () as u64;
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
        let rt_input = runtime::rt_input as *const () as u64;

        self.preallocate_hot_vars(instrs, 4);

        for instr in instrs {
            let maybe_dest: Option<&str> = match instr {
                IrInstr::LoadConst { dest, .. }
                | IrInstr::Add { dest, .. }
                | IrInstr::Sub { dest, .. }
                | IrInstr::Mul { dest, .. }
                | IrInstr::Div { dest, .. }
                | IrInstr::Move { dest, .. }
                | IrInstr::FAdd { dest, .. }
                | IrInstr::FSub { dest, .. }
                | IrInstr::FMul { dest, .. }
                | IrInstr::FDiv { dest, .. }
                | IrInstr::AllocMap { dest }
                | IrInstr::AllocList { dest, .. }
                | IrInstr::AllocStruct { dest, .. }
                | IrInstr::GetIndex { dest, .. }
                | IrInstr::GetMember { dest, .. }
                | IrInstr::GetMap { dest, .. }
                | IrInstr::Await { dest, .. }
                | IrInstr::Lt { dest, .. }
                | IrInstr::Gt { dest, .. }
                | IrInstr::Eq { dest, .. }
                | IrInstr::Ne { dest, .. }
                | IrInstr::LogicNot { dest, .. }
                | IrInstr::LogicAnd { dest, .. }
                | IrInstr::LogicOr { dest, .. }
                | IrInstr::Input { dest, .. } => Some(dest.as_str()),
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

        let mut assigned: HashSet<String> = HashSet::new();
        let mut assigned_order: VecDeque<String> = VecDeque::new();

        for instr in instrs {
            self.track_metadata(instr);
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
                IrInstr::Input { dest, prompt } => {
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
                    let prompt_loc = self
                        .regmap
                        .get(prompt)
                        .ok_or_else(|| format!("Undefined var: {}", prompt))?;

                    emit_call1(&mut emit, rt_input, prompt_loc);

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
                IrInstr::AllocStruct { dest, name: _ } => {
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
                IrInstr::SetIndex { src, index, value } => {
                    let src_loc = self
                        .regmap
                        .get(src)
                        .ok_or_else(|| format!("Undefined var: {}", src))?;
                    let idx_loc = self
                        .regmap
                        .get(index)
                        .ok_or_else(|| format!("Undefined var: {}", index))?;
                    let val_loc = self
                        .regmap
                        .get(value)
                        .ok_or_else(|| format!("Undefined var: {}", value))?;
                    emit_call3(&mut emit, rt_index_set, src_loc, idx_loc, val_loc);
                }
                IrInstr::SetMember { obj, member, value } => {
                    let obj_loc = self
                        .regmap
                        .get(obj)
                        .ok_or_else(|| format!("Undefined var: {}", obj))?;
                    let val_loc = self
                        .regmap
                        .get(value)
                        .ok_or_else(|| format!("Undefined var: {}", value))?;
                    let (ptr, len) = self.context.intern_bytes(member.as_bytes().to_vec());
                    emit_save_call_regs(&mut emit);
                    emit_mov64_to_reg(&mut emit, 0, ptr as u64);
                    emit_mov64_to_reg(&mut emit, 1, len as u64);
                    emit.emit_call(rt_alloc_string);
                    emit.emit_u32_le(encode_add_imm(Reg::X(10), Reg::X(0), 0)); // key
                    emit_restore_call_regs(&mut emit);

                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(9, obj_loc);
                    emit.load_to_reg(11, val_loc);
                    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
                    emit.emit_u32_le(encode_add_imm(Reg::X(1), Reg::X(10), 0));
                    emit.emit_u32_le(encode_add_imm(Reg::X(2), Reg::X(11), 0));
                    emit.emit_call(rt_map_set);
                    emit_restore_call_regs(&mut emit);
                }
                IrInstr::GetMember { dest, obj, member } => {
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
                    let obj_loc = self
                        .regmap
                        .get(obj)
                        .ok_or_else(|| format!("Undefined var: {}", obj))?;
                    let (ptr, len) = self.context.intern_bytes(member.as_bytes().to_vec());
                    emit_save_call_regs(&mut emit);
                    emit_mov64_to_reg(&mut emit, 0, ptr as u64);
                    emit_mov64_to_reg(&mut emit, 1, len as u64);
                    emit.emit_call(rt_alloc_string);
                    emit.emit_u32_le(encode_add_imm(Reg::X(10), Reg::X(0), 0)); // key
                    emit_restore_call_regs(&mut emit);

                    emit_save_call_regs(&mut emit);
                    emit.load_to_reg(9, obj_loc);
                    emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
                    emit.emit_u32_le(encode_add_imm(Reg::X(1), Reg::X(10), 0));
                    emit.emit_call(rt_map_get);
                    emit.emit_u32_le(encode_add_imm(Reg::X(9), Reg::X(0), 0));
                    emit_restore_call_regs(&mut emit);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::GetMap { dest, map, key } => {
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
                    let map_loc = self
                        .regmap
                        .get(map)
                        .ok_or_else(|| format!("Undefined var: {}", map))?;
                    let key_loc = self
                        .regmap
                        .get(key)
                        .ok_or_else(|| format!("Undefined var: {}", key))?;
                    emit_call2(&mut emit, rt_map_get, map_loc, key_loc);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::FAdd { dest, left, right } => {
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
                    emit_call2(&mut emit, rt_float_add, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::FSub { dest, left, right } => {
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
                    emit_call2(&mut emit, rt_float_sub, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::FMul { dest, left, right } => {
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
                    emit_call2(&mut emit, rt_float_mul, l, r);
                    emit.store_from_reg(9, dst_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
                    }
                }
                IrInstr::FDiv { dest, left, right } => {
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
                    emit_call2(&mut emit, rt_float_div, l, r);
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
                    if func == "print" || func == "println" {
                        if args.len() != 1 {
                            return Err(format!("{} expects 1 argument", func));
                        }
                        let src_loc = self
                            .regmap
                            .get(&args[0])
                            .ok_or_else(|| format!("Undefined var: {}", args[0]))?;
                        emit_save_call_regs(&mut emit);
                        emit.load_to_reg(9, src_loc);
                        emit.emit_u32_le(encode_add_imm(Reg::X(0), Reg::X(9), 0));
                        emit.emit_call(rt_print);
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
                            emit_mov64_to_reg(&mut emit, 9, encode_int(0));
                            emit.store_from_reg(9, dst_loc);
                            if assigned.insert(dest.to_string()) {
                                assigned_order.push_back(dest.to_string());
                            }
                        }
                        continue;
                    }

                    let builtin_call = match (func.as_str(), args.len()) {
                        ("len", 1) => Some((rt_list_len, args[0].as_str())),
                        ("str", 1) => Some((rt_to_str, args[0].as_str())),
                        ("num", 1) => Some((rt_to_num, args[0].as_str())),
                        ("keys", 1) => Some((rt_map_keys, args[0].as_str())),
                        ("values", 1) => Some((rt_map_values, args[0].as_str())),
                        ("pop", 1) => Some((rt_list_pop, args[0].as_str())),
                        ("abs", 1) => Some((rt_abs, args[0].as_str())),
                        ("sqrt", 1) => Some((rt_sqrt, args[0].as_str())),
                        ("is_map", 1) => Some((rt_is_map, args[0].as_str())),
                        ("is_list", 1) => Some((rt_is_list, args[0].as_str())),
                        ("is_string", 1) => Some((rt_is_string, args[0].as_str())),
                        ("range", 2) => None,
                        ("push", 2) => None,
                        _ => None,
                    };

                    if let Some((callee, a0)) = builtin_call {
                        let a0_loc = self
                            .regmap
                            .get(a0)
                            .ok_or_else(|| format!("Undefined var: {}", a0))?;
                        emit_call1(&mut emit, callee, a0_loc);
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
                        continue;
                    }

                    if func == "range" && args.len() == 2 {
                        let a0 = self
                            .regmap
                            .get(&args[0])
                            .ok_or_else(|| format!("Undefined var: {}", args[0]))?;
                        let a1 = self
                            .regmap
                            .get(&args[1])
                            .ok_or_else(|| format!("Undefined var: {}", args[1]))?;
                        emit_call2(&mut emit, rt_range, a0, a1);
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
                        continue;
                    }

                    if (func == "min" || func == "max" || func == "pow" || func == "contains")
                        && args.len() == 2
                    {
                        let a0 = self
                            .regmap
                            .get(&args[0])
                            .ok_or_else(|| format!("Undefined var: {}", args[0]))?;
                        let a1 = self
                            .regmap
                            .get(&args[1])
                            .ok_or_else(|| format!("Undefined var: {}", args[1]))?;
                        let callee = match func.as_str() {
                            "min" => rt_min,
                            "max" => rt_max,
                            "pow" => rt_pow,
                            "contains" => rt_contains,
                            _ => unreachable!(),
                        };
                        emit_call2(&mut emit, callee, a0, a1);
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
                        continue;
                    }

                    if func == "push" && args.len() == 2 {
                        let list_loc = self
                            .regmap
                            .get(&args[0])
                            .ok_or_else(|| format!("Undefined var: {}", args[0]))?;
                        let item_loc = self
                            .regmap
                            .get(&args[1])
                            .ok_or_else(|| format!("Undefined var: {}", args[1]))?;
                        emit_call2(&mut emit, rt_list_push, list_loc, item_loc);
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
                            emit_call1(&mut emit, rt_list_len, list_loc);
                            emit.store_from_reg(9, dst_loc);
                            if assigned.insert(dest.to_string()) {
                                assigned_order.push_back(dest.to_string());
                            }
                        }
                        continue;
                    }

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
                    if self.current_function.as_deref() == Some(func.as_str()) {
                        let off = emit.len();
                        emit.emit_u32_le(0);
                        self.labels
                            .record_branch(off, "__fn_entry", BranchKind::BL);
                    } else {
                        let addr = self
                            .context
                            .get_function_addr(func)
                            .ok_or_else(|| format!("Unknown function: {}", func))?;
                        emit.emit_call(addr);
                    }
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
                IrInstr::Spawn { task } => {
                    let task_loc = self
                        .regmap
                        .get(task)
                        .ok_or_else(|| format!("Undefined var: {}", task))?;
                    emit.emit_mov(task_loc, task_loc);
                }
                IrInstr::Await { dest, task } => {
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
                    let task_loc = self
                        .regmap
                        .get(task)
                        .ok_or_else(|| format!("Undefined var: {}", task))?;
                    emit.emit_mov(dst_loc, task_loc);
                    if assigned.insert(dest.to_string()) {
                        assigned_order.push_back(dest.to_string());
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
                | IrInstr::FAdd { dest, .. }
                | IrInstr::FSub { dest, .. }
                | IrInstr::FMul { dest, .. }
                | IrInstr::FDiv { dest, .. }
                | IrInstr::Move { dest, .. }
                | IrInstr::Lt { dest, .. }
                | IrInstr::Gt { dest, .. }
                | IrInstr::Eq { dest, .. }
                | IrInstr::Ne { dest, .. }
                | IrInstr::LogicAnd { dest, .. }
                | IrInstr::LogicOr { dest, .. }
                | IrInstr::LogicNot { dest, .. }
                | IrInstr::AllocMap { dest }
                | IrInstr::AllocList { dest, .. }
                | IrInstr::GetIndex { dest, .. }
                | IrInstr::GetMap { dest, .. }
                | IrInstr::GetMember { dest, .. }
                | IrInstr::Await { dest, .. } => Some(dest.as_str()),
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

    fn preallocate_hot_vars(&mut self, instrs: &[IrInstr], max: usize) {
        let mut counts: HashMap<String, usize> = HashMap::new();
        for instr in instrs {
            if let Some(def) = instr_def_var(instr) {
                *counts.entry(def.to_string()).or_insert(0) += 1;
            }
            for var in instr_read_vars(instr) {
                *counts.entry(var.to_string()).or_insert(0) += 1;
            }
        }

        let mut vars: Vec<(String, usize)> = counts.into_iter().collect();
        vars.sort_by_key(|(_, count)| Reverse(*count));
        for (var, _) in vars.into_iter().take(max) {
            let _ = self.regmap.alloc(&var);
        }
    }

    fn track_metadata(&mut self, instr: &IrInstr) {
        self.memory_table.increment_instruction();
        let time = self.memory_table.get_instruction_counter();

        if let Some(def) = instr_def_var(instr) {
            self.hotpath_tracker.record_var_write(def, time);
            let value_type = instr_def_type(instr);
            let sym_loc = self.regmap.get(def).and_then(Self::location_to_symbol);
            if self.symbol_table.lookup(def).is_some() {
                if let Some(loc) = sym_loc {
                    let _ = self.symbol_table.update_location(def, loc);
                }
            } else {
                let _ = self
                    .symbol_table
                    .declare_variable(def.to_string(), value_type, sym_loc);
            }

            if matches!(
                instr,
                IrInstr::AllocList { .. }
                    | IrInstr::AllocMap { .. }
                    | IrInstr::AllocStruct { .. }
                    | IrInstr::LoadConst {
                        value: IrValue::String(_),
                        ..
                    }
            ) {
                let (alloc_type, size) = match instr {
                    IrInstr::AllocList { items, .. } => (AllocationType::List, items.len()),
                    IrInstr::AllocMap { .. } => (AllocationType::Map, 0),
                    IrInstr::AllocStruct { .. } => (AllocationType::Object, 0),
                    IrInstr::LoadConst {
                        value: IrValue::String(s),
                        ..
                    } => (AllocationType::String, s.len()),
                    _ => (AllocationType::Object, 0),
                };
                let id = self.memory_table.allocate(alloc_type, size);
                self.memory_table.increment_ref(id);
            }
        }

        for var in instr_read_vars(instr) {
            self.hotpath_tracker.record_var_read(var, time);
            self.symbol_table.increment_reference(var);
        }

        if let IrInstr::Call { func, .. } = instr {
            self.symbol_table.increment_call_count(func);
        }
    }

    fn location_to_symbol(loc: Location) -> Option<SymbolLocation> {
        Some(match loc {
            Location::Register(r) => SymbolLocation::Register(r),
            Location::Stack(off) => SymbolLocation::Stack(off),
        })
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

fn instr_def_var(instr: &IrInstr) -> Option<&str> {
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

fn instr_read_vars<'a>(instr: &'a IrInstr) -> Vec<&'a str> {
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

fn instr_def_type(instr: &IrInstr) -> ValueType {
    match instr {
        IrInstr::LoadConst { value, .. } => match value {
            IrValue::Number(_) => ValueType::Int,
            IrValue::String(_) => ValueType::String,
            IrValue::Bool(_) => ValueType::Bool,
        },
        IrInstr::Add { .. }
        | IrInstr::Sub { .. }
        | IrInstr::Mul { .. }
        | IrInstr::Div { .. }
        | IrInstr::BitAnd { .. }
        | IrInstr::BitOr { .. }
        | IrInstr::BitXor { .. }
        | IrInstr::BitNot { .. }
        | IrInstr::Shl { .. }
        | IrInstr::Shr { .. } => ValueType::Int,
        IrInstr::FAdd { .. }
        | IrInstr::FSub { .. }
        | IrInstr::FMul { .. }
        | IrInstr::FDiv { .. } => ValueType::Float,
        IrInstr::Eq { .. }
        | IrInstr::Ne { .. }
        | IrInstr::Lt { .. }
        | IrInstr::Gt { .. }
        | IrInstr::LogicAnd { .. }
        | IrInstr::LogicOr { .. }
        | IrInstr::LogicNot { .. } => ValueType::Bool,
        IrInstr::AllocList { .. } => ValueType::List,
        IrInstr::AllocMap { .. } => ValueType::Map,
        IrInstr::AllocStruct { .. } => ValueType::Object,
        IrInstr::Input { .. } => ValueType::String,
        _ => ValueType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jit::context::JitContext;

    fn decode_int(v: u64) -> i64 {
        (v as i64) >> 1
    }

    fn run(instrs: Vec<IrInstr>) -> u64 {
        let mut ctx = JitContext::new();
        let mut jit = JitCompiler::new(&mut ctx);
        jit.execute_global(&instrs).expect("jit execute")
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn jit_e2e_builtin_pow_and_sqrt() {
        let result = run(vec![
            IrInstr::LoadConst {
                dest: "a".to_string(),
                value: IrValue::Number(2.0),
            },
            IrInstr::LoadConst {
                dest: "b".to_string(),
                value: IrValue::Number(8.0),
            },
            IrInstr::Call {
                dest: Some("p".to_string()),
                func: "pow".to_string(),
                args: vec!["a".to_string(), "b".to_string()],
            },
            IrInstr::Call {
                dest: Some("s".to_string()),
                func: "sqrt".to_string(),
                args: vec!["p".to_string()],
            },
            IrInstr::Call {
                dest: Some("out".to_string()),
                func: "num".to_string(),
                args: vec!["s".to_string()],
            },
            IrInstr::Return {
                value: Some("out".to_string()),
            },
        ]);

        assert_eq!(decode_int(result), 16);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn jit_e2e_builtin_range_len_pop() {
        let result = run(vec![
            IrInstr::LoadConst {
                dest: "s".to_string(),
                value: IrValue::Number(1.0),
            },
            IrInstr::LoadConst {
                dest: "e".to_string(),
                value: IrValue::Number(5.0),
            },
            IrInstr::Call {
                dest: Some("r".to_string()),
                func: "range".to_string(),
                args: vec!["s".to_string(), "e".to_string()],
            },
            IrInstr::Call {
                dest: Some("len".to_string()),
                func: "len".to_string(),
                args: vec!["r".to_string()],
            },
            IrInstr::Call {
                dest: Some("last".to_string()),
                func: "pop".to_string(),
                args: vec!["r".to_string()],
            },
            IrInstr::Add {
                dest: "sum".to_string(),
                left: "len".to_string(),
                right: "last".to_string(),
            },
            IrInstr::Return {
                value: Some("sum".to_string()),
            },
        ]);

        assert_eq!(decode_int(result), 8);
    }

    #[test]
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    fn jit_e2e_builtin_contains() {
        let result = run(vec![
            IrInstr::LoadConst {
                dest: "hay".to_string(),
                value: IrValue::String("forge compiler".to_string()),
            },
            IrInstr::LoadConst {
                dest: "needle".to_string(),
                value: IrValue::String("comp".to_string()),
            },
            IrInstr::Call {
                dest: Some("ok".to_string()),
                func: "contains".to_string(),
                args: vec!["hay".to_string(), "needle".to_string()],
            },
            IrInstr::Return {
                value: Some("ok".to_string()),
            },
        ]);

        assert_eq!(decode_int(result), 1);
    }
}
