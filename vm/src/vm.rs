use std::collections::HashMap;
use std::io::{self, Read, Write};

#[derive(Default, Debug)]
struct Flags {
    n: bool, // Negative
    z: bool, // Zero
    c: bool, // Carry
    v: bool, // Overflow
}

#[derive(Debug, Clone, Copy)]
enum Operand {
    Reg(usize),
    Imm(i64),
    // For memory operands like [sp, #16]
    Mem { base: usize, offset: i64 },
    // For adrp/add label combos
    Label(usize),
}

#[derive(Debug, Clone, Copy)]
enum OpCode {
    ADD, SUB, MUL, SDIV, UDIV, MSUB, NEG, UXTW,
    FADD, FSUB, FMUL, FDIV, FMOV,
    AND, ORR, EOR, MVN, LSL, LSR,
    MOV, LDR, LDRB, STR, STRB, STP, LDP,
    B, BL, RET,
    CMP, CSET,
    B_COND(Condition),
    CBZ, CBNZ,
    ADRP,
    SVC,
    NOP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Condition {
    EQ, NE, LT, GT, LE, GE,
    HI, LS, HS, LO, MI, PL, VS, VC,
}

#[derive(Debug, Clone)]
struct Instruction {
    opcode: OpCode,
    // Using Vec for flexibility; most instructions use 2-3 operands.
    operands: Vec<Operand>,
}

/// ARM64 Virtual Machine
pub struct VM {
    /// General purpose registers x0-x30 and SP (x31)
    pub registers: [i64; 32],
    /// Floating point registers d0-d31
    pub fp_registers: [f64; 32],
    /// Stack pointer (sp)
    pub sp: i64,
    /// Program counter
    pub pc: i64,
    /// Memory (2MB: 1MB for stack, 1MB for heap/data)
    pub memory: Vec<u8>,
    /// Data segment offset in memory
    pub data_segment_offset: usize,
    /// Labels for jumps (code and data)
    pub labels: HashMap<String, usize>,
    /// Program instructions (now in a bytecode format)
    pub program: Vec<Instruction>,
    /// For debugging: map PC to original source line
    pub debug_info: HashMap<usize, String>,
    /// Execution state
    pub running: bool,
    /// Step mode
    pub step_mode: bool,
    /// Heap allocator pointer
    pub heap_ptr: usize,
    /// Tracked heap allocations (ptr -> size)
    allocations: HashMap<usize, usize>,
    /// Condition flags
    flags: Flags,
}

impl VM {
    pub fn new() -> Self {
        VM {
            registers: [0; 32],
            fp_registers: [0.0; 32],
            sp: (1024 * 1024) as i64, // Stack starts at 1MB and grows down
            pc: 0,
            memory: vec![0; 2 * 1024 * 1024],
            data_segment_offset: 1024 * 1024, // Data segment starts at 1MB
            labels: HashMap::new(),
            program: Vec::new(),
            debug_info: HashMap::new(),
            running: false,
            step_mode: false,
            heap_ptr: 1024 * 1024, // Heap starts at 1MB
            allocations: HashMap::new(),
            flags: Flags::default(),
        }
    }

    pub fn load_program(&mut self, asm: &str) -> Result<(), String> {
        // Reset state
        self.reset_state();

        let mut text_section_lines = Vec::new();
        let mut data_section_lines = Vec::new();
        let mut current_section = ".text";

        for line in asm.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("//") || line.starts_with("#") || line.starts_with(".global") {
                continue;
            }
            if line == ".data" { current_section = ".data"; continue; }
            if line == ".text" { current_section = ".text"; continue; }

            if current_section == ".data" {
                data_section_lines.push(line);
            } else {
                text_section_lines.push(line);
            }
        }

        // Process data section first to populate data labels
        self.process_data_section(&data_section_lines)?;

        // Process text section to populate code labels and then parse instructions
        self.process_text_section(&text_section_lines)?;

        self.pc = self.labels.get("_main").cloned().unwrap_or(0) as i64;
        // Treat returning from _main as program termination.
        self.registers[30] = self.program.len() as i64;
        self.running = true;
        Ok(())
    }

    fn reset_state(&mut self) {
        self.registers = [0; 32];
        self.fp_registers = [0.0; 32];
        self.sp = (1024 * 1024) as i64;
        self.pc = 0;
        self.running = false;
        self.step_mode = false;
        self.heap_ptr = 1024 * 1024;
        self.allocations.clear();
        self.memory.fill(0);
        self.program.clear();
        self.labels.clear();
        self.debug_info.clear();
    }

    fn process_data_section(&mut self, lines: &[&str]) -> Result<(), String> {
        let mut data_ptr = self.heap_ptr;
        for line in lines {
            if let Some(rest) = line.strip_prefix(".align") {
                let n = rest.trim().parse::<usize>().unwrap_or(0);
                let align = 1usize.checked_shl(n as u32).unwrap_or(1).max(1);
                data_ptr = (data_ptr + align - 1) & !(align - 1);
                continue;
            }

            if let Some(first) = line.split_whitespace().next() {
                if first.ends_with(':') {
                    let label = first.trim_end_matches(':').trim().to_string();
                    self.labels.insert(label, data_ptr);
                    let rest = line[first.len()..].trim();
                    if !rest.is_empty() {
                        write_data_directive(self, rest, &mut data_ptr)?;
                    }
                    continue;
                }
            }

            if line.starts_with(".asciz") || line.starts_with(".quad") {
                write_data_directive(self, line, &mut data_ptr)?;
                continue;
            }
        }
        self.heap_ptr = (data_ptr + 15) & !15;
        Ok(())
    }

    fn process_text_section(&mut self, lines: &[&str]) -> Result<(), String> {
        // First pass: find all code labels
        let mut line_num = 0;
        for line in lines {
            if line.starts_with('.') && !line.ends_with(':') { continue; }
            if line.ends_with(':') {
                let label = line.trim_end_matches(':').to_string();
                self.labels.insert(label, line_num);
            } else {
                line_num += 1;
            }
        }

        // Second pass: parse instructions
        for line in lines {
            if line.ends_with(':') || (line.starts_with('.') && !line.ends_with(':')) {
                continue;
            }
            let pc = self.program.len();
            let instr = self.parse_instruction(line)?;
            self.program.push(instr);
            self.debug_info.insert(pc, line.to_string());
        }
        Ok(())
    }
    
    pub fn run(&mut self) -> Result<(), String> {
        while self.running && (self.pc as usize) < self.program.len() {
            self.step()?;
            if self.step_mode { break; }
        }
        Ok(())
    }

    pub fn step(&mut self) -> Result<(), String> {
        let pc = self.pc as usize;
        if pc >= self.program.len() {
            self.running = false;
            return Ok(());
        }
        let instr = self.program[pc].clone();
        self.execute_instruction(&instr)?;
        Ok(())
    }
    
    fn execute_instruction(&mut self, instr: &Instruction) -> Result<(), String> {
        use OpCode::*;
        let ops = &instr.operands;

        match instr.opcode {
            ADD => self.set_reg_op(ops[0], self.get_op(ops[1])?.wrapping_add(self.get_op(ops[2])?)),
            SUB => self.set_reg_op(ops[0], self.get_op(ops[1])?.wrapping_sub(self.get_op(ops[2])?)),
            MUL => self.set_reg_op(ops[0], self.get_op(ops[1])?.wrapping_mul(self.get_op(ops[2])?)),
            SDIV => {
                let v2 = self.get_op(ops[2])?;
                if v2 == 0 { return Err("Division by zero".to_string()); }
                self.set_reg_op(ops[0], self.get_op(ops[1])? / v2);
            }
            UDIV => {
                let v2 = self.get_op(ops[2])? as u64;
                if v2 == 0 { self.set_reg_op(ops[0], 0); }
                else { self.set_reg_op(ops[0], ((self.get_op(ops[1])? as u64) / v2) as i64); }
            }
            MSUB => self.set_reg_op(ops[0], self.get_op(ops[3])?.wrapping_sub(self.get_op(ops[1])?.wrapping_mul(self.get_op(ops[2])?))),
            NEG => self.set_reg_op(ops[0], 0i64.wrapping_sub(self.get_op(ops[1])?)),
            UXTW => self.set_reg_op(ops[0], (self.get_op(ops[1])? as u64 & 0xFFFF_FFFF) as i64),
            FADD => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? + self.get_fp_op(ops[2])?),
            FSUB => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? - self.get_fp_op(ops[2])?),
            FMUL => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? * self.get_fp_op(ops[2])?),
            FDIV => self.set_fp_reg_op(ops[0], self.get_fp_op(ops[1])? / self.get_fp_op(ops[2])?),
            FMOV => {
                if let Operand::Reg(d) = ops[0] { // fmov d, x
                    self.set_fp_reg(d, f64::from_bits(self.get_op(ops[1])? as u64));
                } else { // fmov x, d
                    self.set_reg_op(ops[0], self.get_fp_op(ops[1])?.to_bits() as i64);
                }
            }
            AND => self.set_reg_op(ops[0], self.get_op(ops[1])? & self.get_op(ops[2])?),
            ORR => self.set_reg_op(ops[0], self.get_op(ops[1])? | self.get_op(ops[2])?),
            EOR => self.set_reg_op(ops[0], self.get_op(ops[1])? ^ self.get_op(ops[2])?),
            MVN => self.set_reg_op(ops[0], !self.get_op(ops[1])?),
            LSL => self.set_reg_op(ops[0], self.get_op(ops[1])? << self.get_op(ops[2])?),
            LSR => self.set_reg_op(ops[0], ((self.get_op(ops[1])? as u64) >> self.get_op(ops[2])?) as i64),
            MOV => self.set_reg_op(ops[0], self.get_op(ops[1])?),
            LDR => {
                let addr = self.get_addr(ops[1])?;
                let val = i64::from_le_bytes(self.memory[addr..addr+8].try_into().unwrap());
                self.set_reg_op(ops[0], val);
            }
            LDRB => {
                let addr = self.get_addr(ops[1])?;
                self.set_reg_op(ops[0], self.memory[addr] as i64);
            }
            STR => {
                let val = self.get_op(ops[0])?;
                let addr = self.get_addr(ops[1])?;
                self.memory[addr..addr+8].copy_from_slice(&val.to_le_bytes());
            }
            STRB => {
                let val = (self.get_op(ops[0])? & 0xFF) as u8;
                let addr = self.get_addr(ops[1])?;
                self.memory[addr] = val;
            }
            STP => {
                let val1 = self.get_op(ops[0])?;
                let val2 = self.get_op(ops[1])?;
                let addr = self.get_addr(ops[2])?;
                self.memory[addr..addr+8].copy_from_slice(&val1.to_le_bytes());
                self.memory[addr+8..addr+16].copy_from_slice(&val2.to_le_bytes());
                if let Operand::Mem { base, offset } = ops[2] {
                    if let Some(arg_str) = self.debug_info.get(&(self.pc as usize)) {
                        if arg_str.contains('!') {
                            self.set_reg(base, self.get_reg(base) + offset);
                        }
                    }
                }
            }
            LDP => {
                let addr = self.get_addr(ops[2])?;
                let val1 = i64::from_le_bytes(self.memory[addr..addr+8].try_into().unwrap());
                let val2 = i64::from_le_bytes(self.memory[addr+8..addr+16].try_into().unwrap());
                self.set_reg_op(ops[0], val1);
                self.set_reg_op(ops[1], val2);
                if let Operand::Mem { base, offset } = ops[2] {
                     if let Some(arg_str) = self.debug_info.get(&(self.pc as usize)) {
                        if arg_str.contains("],") { // Post-index
                            self.set_reg(base, self.get_reg(base) + offset);
                        }
                    }
                }
            }
            B => { self.pc = self.get_op(ops[0])? - 1; }
            BL => {
                self.registers[30] = self.pc + 1;
                if let Operand::Label(target) = ops[0] {
                    self.pc = target as i64 - 1;
                } else {
                    let label_name = self.debug_info.get(&(self.pc as usize)).unwrap().split_whitespace().nth(1).unwrap();
                    self.exec_bl_syscall(label_name)?;
                }
            }
            RET => { self.pc = self.get_reg(30) - 1; }
            CMP => {
                let val1 = self.get_op(ops[0])?;
                let val2 = self.get_op(ops[1])?;
                let (result, overflow) = val1.overflowing_sub(val2);
                self.flags.z = result == 0;
                self.flags.n = result < 0;
                self.flags.c = (val1 as u64) >= (val2 as u64);
                self.flags.v = overflow;
            }
            CSET => {
                let cond = if let OpCode::B_COND(c) = instr.opcode { c } else { unreachable!() };
                let val = self.check_condition(cond)?;
                self.set_reg_op(ops[0], if val { 1 } else { 0 });
            }
            B_COND(cond) => {
                if self.check_condition(cond)? { self.pc = self.get_op(ops[0])? - 1; }
            }
            CBZ => {
                if self.get_op(ops[0])? == 0 { self.pc = self.get_op(ops[1])? - 1; }
            }
            CBNZ => {
                if self.get_op(ops[0])? != 0 { self.pc = self.get_op(ops[1])? - 1; }
            }
            ADRP => {
                // This is now a pseudo-op, the real work is done by ADD
                // In a real JIT, this would calculate page addresses. Here, we let ADD handle the label.
            }
            SVC => self.exec_svc()?,
            NOP => {},
        }
        self.pc += 1;
        Ok(())
    }

    fn get_op(&self, op: Operand) -> Result<i64, String> {
        match op {
            Operand::Reg(idx) => Ok(self.get_reg(idx)),
            Operand::Imm(val) => Ok(val),
            Operand::Label(addr) => Ok(addr as i64),
            _ => Err("Invalid operand type for integer operation".to_string()),
        }
    }

    fn get_fp_op(&self, op: Operand) -> Result<f64, String> {
        match op {
            Operand::Reg(idx) => Ok(self.get_fp_reg(idx)),
            _ => Err("Invalid operand type for float operation".to_string()),
        }
    }

    fn set_reg_op(&mut self, op: Operand, val: i64) {
        if let Operand::Reg(idx) = op { self.set_reg(idx, val); }
    }
    fn set_fp_reg_op(&mut self, op: Operand, val: f64) {
        if let Operand::Reg(idx) = op { self.set_fp_reg(idx, val); }
    }

    fn get_addr(&self, op: Operand) -> Result<usize, String> {
        if let Operand::Mem { base, offset } = op {
            let addr = self.get_reg(base) + offset;
            if addr < 0 { return Err(format!("Memory address underflow: {}", addr)); }
            Ok(addr as usize)
        } else {
            Err("Operand is not a memory address".to_string())
        }
    }
    
    fn parse_reg(&self, s: &str) -> Result<usize, String> {
        let s = s.trim();
        if s == "xzr" || s == "wzr" {
            Ok(usize::MAX)
        } else if s == "sp" {
            Ok(31)
        } else if s.starts_with('x') || s.starts_with('w') || s.starts_with('d') || s.starts_with('s') {
            s[1..].parse::<usize>().map_err(|_| format!("Invalid register format: {}", s))
        } else {
            Err(format!("Invalid register: {}", s))
        }
    }

    fn parse_imm(&self, s: &str) -> Result<i64, String> {
        let s = s.trim().trim_start_matches('#');
        if let Some(hex) = s.strip_prefix("0x") {
            i64::from_str_radix(hex, 16).map_err(|_| format!("Invalid immediate: {}", s))
        } else if let Some(hex) = s.strip_prefix("-0x") {
            i64::from_str_radix(hex, 16).map(|v| -v).map_err(|_| format!("Invalid immediate: {}", s))
        } else {
            s.parse::<i64>().map_err(|_| format!("Invalid immediate: {}", s))
        }
    }

    fn get_reg(&self, idx: usize) -> i64 {
        if idx == usize::MAX { 0 } else if idx == 31 { self.sp } else { self.registers[idx] }
    }
    fn set_reg(&mut self, idx: usize, val: i64) {
        if idx == usize::MAX { return; }
        if idx == 31 { self.sp = val; } else { self.registers[idx] = val; }
    }
    fn get_fp_reg(&self, idx: usize) -> f64 { self.fp_registers[idx] }
    fn set_fp_reg(&mut self, idx: usize, val: f64) { self.fp_registers[idx] = val; }

    fn parse_mem_operand(&self, operand: &str) -> Result<Operand, String> {
        let operand = operand.trim().trim_start_matches('[').trim_end_matches(']');
        let parts: Vec<&str> = operand.split(',').map(|s| s.trim()).collect();
        let base_reg_idx = self.parse_reg(parts[0])?;
        let mut offset = 0i64;
        if parts.len() > 1 {
            if parts[1].starts_with('#') {
                offset = self.parse_imm(parts[1])?;
            } else {
                // [base, index, lsl #3] not supported by this simplified parser, but could be added.
                return Err("Indexed memory operands not supported in this VM version".to_string());
            }
        }
        Ok(Operand::Mem { base: base_reg_idx, offset })
    }

    fn check_condition(&self, cond: Condition) -> Result<bool, String> {
        Ok(match cond {
            Condition::EQ => self.flags.z,
            Condition::NE => !self.flags.z,
            Condition::LT => self.flags.n != self.flags.v,
            Condition::GT => !self.flags.z && (self.flags.n == self.flags.v),
            Condition::LE => self.flags.z || (self.flags.n != self.flags.v),
            Condition::GE => self.flags.n == self.flags.v,
            Condition::HI => self.flags.c && !self.flags.z,
            Condition::LS => !self.flags.c || self.flags.z,
            Condition::HS => self.flags.c,
            Condition::LO => !self.flags.c,
            Condition::MI => self.flags.n,
            Condition::PL => !self.flags.n,
            Condition::VS => self.flags.v,
            Condition::VC => !self.flags.v,
        })
    }
    
    fn exec_bl_syscall(&mut self, label: &str) -> Result<(), String> {
        match label {
            "_malloc" => {
                let size = self.registers[0].max(0) as usize;
                let ptr = self.vm_malloc(size)?;
                self.registers[0] = ptr as i64;
            }
            "_free" => {
                let ptr = self.registers[0] as usize;
                self.vm_free(ptr);
                self.registers[0] = 0;
            }
            "_realloc" => {
                let old_ptr = self.registers[0] as usize;
                let new_size = self.registers[1].max(0) as usize;
                let new_ptr = self.vm_realloc(old_ptr, new_size)?;
                self.registers[0] = new_ptr as i64;
            }
            "_fflush" => { self.registers[0] = 0; }
            "_printf" => self.vm_printf()?,
            _ => return Err(format!("Unknown syscall label: {}", label)),
        }
        Ok(())
    }

    fn exec_svc(&mut self) -> Result<(), String> {
        match self.registers[16] {
            1 => { self.running = false; }
            3 => { // read
                let fd = self.registers[0];
                let buf_ptr = self.registers[1] as usize;
                let len = self.registers[2] as usize;
                if fd == 0 {
                    let mut buffer = vec![0; len];
                    let bytes_read = io::stdin().read(&mut buffer).map_err(|e| e.to_string())?;
                    if buf_ptr + bytes_read <= self.memory.len() {
                        self.memory[buf_ptr..buf_ptr + bytes_read].copy_from_slice(&buffer[..bytes_read]);
                        self.registers[0] = bytes_read as i64;
                    } else {
                        return Err(format!("Memory access out of bounds for SVC read at 0x{:x}", buf_ptr));
                    }
                }
            }
            4 => { // write
                let fd = self.registers[0];
                let buf_ptr = self.registers[1] as usize;
                let len = self.registers[2] as usize;
                if fd == 1 {
                    let end = (buf_ptr + len).min(self.memory.len());
                    print!("{}", String::from_utf8_lossy(&self.memory[buf_ptr..end]));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn vm_malloc(&mut self, size: usize) -> Result<usize, String> {
        let aligned = (size + 15) & !15;
        if aligned == 0 { return Ok(0); }
        let ptr = (self.heap_ptr + 15) & !15;
        let end = ptr.saturating_add(aligned);
        if end > self.memory.len() { return Err(format!("Out of VM memory allocating {} bytes", aligned)); }
        self.heap_ptr = end;
        self.allocations.insert(ptr, aligned);
        Ok(ptr)
    }

    fn vm_free(&mut self, ptr: usize) {
        self.allocations.remove(&ptr);
        // A real implementation would add this to a free list.
    }

    fn vm_realloc(&mut self, old_ptr: usize, new_size: usize) -> Result<usize, String> {
        if old_ptr == 0 { return self.vm_malloc(new_size); }
        let old_size = self.allocations.get(&old_ptr).copied().unwrap_or(0);
        let new_ptr = self.vm_malloc(new_size)?;
        let copy_len = old_size.min(new_size);
        if copy_len > 0 {
            if old_ptr + copy_len > self.memory.len() || new_ptr + copy_len > self.memory.len() {
                return Err("Realloc copy out of bounds".to_string());
            }
            let src = self.memory[old_ptr..old_ptr + copy_len].to_vec();
            self.memory[new_ptr..new_ptr + copy_len].copy_from_slice(&src);
        }
        self.vm_free(old_ptr);
        Ok(new_ptr)
    }

    fn read_c_string(&self, addr: usize) -> Result<String, String> {
        if addr >= self.memory.len() { return Err(format!("CString pointer out of bounds: 0x{:x}", addr)); }
        let mut bytes = Vec::new();
        let mut i = addr;
        while i < self.memory.len() && self.memory[i] != 0 {
            bytes.push(self.memory[i]);
            i += 1;
        }
        Ok(String::from_utf8_lossy(&bytes).to_string())
    }

    fn vm_printf(&mut self) -> Result<(), String> {
        let fmt_ptr = self.registers[0] as usize;
        let fmt = self.read_c_string(fmt_ptr)?;
        let sp = self.sp as usize;
        if sp + 8 > self.memory.len() { return Err("Stack out of bounds in _printf".to_string()); }
        let arg = i64::from_le_bytes(self.memory[sp..sp + 8].try_into().unwrap());

        if fmt.contains("%ld") {
            let output = fmt.replace("%ld", &arg.to_string());
            print!("{}", output);
            io::stdout().flush().ok();
            self.registers[0] = 0;
            Ok(())
        } else {
            Err(format!("Unsupported _printf format: {}", fmt))
        }
    }

    pub fn print_registers(&self) {
        println!("\n=== Registers ===");
        for i in 0..31 {
            if i % 4 == 0 { println!(); }
            print!("x{:02}: {:<16}  ", i, self.registers[i]);
        }
        println!("\npc:  {:<16}  sp: {:<16}", self.pc, self.sp);
        println!("Flags: N:{} Z:{} C:{} V:{}", self.flags.n as u8, self.flags.z as u8, self.flags.c as u8, self.flags.v as u8);
        println!("================\n");
    }

    fn parse_instruction(&self, line: &str) -> Result<Instruction, String> {
        let parts: Vec<&str> = split_asm_args(line);
        if parts.is_empty() { return Ok(Instruction { opcode: OpCode::NOP, operands: vec![] }); }
        let mnemonic = parts[0];
        let args = &parts[1..];

        let mut operands = Vec::new();
        for arg in args {
            if arg.starts_with('[') {
                operands.push(self.parse_mem_operand(arg)?);
            } else if arg.starts_with('#') {
                operands.push(Operand::Imm(self.parse_imm(arg)?));
            } else if self.labels.contains_key(*arg) {
                operands.push(Operand::Label(self.labels[*arg]));
            } else if arg.starts_with('x') || arg.starts_with('w') || arg.starts_with('d') || arg.starts_with('s') || *arg == "sp" {
                operands.push(Operand::Reg(self.parse_reg(arg)?));
            } else if let Ok(imm) = self.parse_imm(arg) {
                // For things like `svc #0x80` where the '#' is optional in some assemblers
                operands.push(Operand::Imm(imm));
            } else {
                // Assume it's a label for a syscall
                operands.push(Operand::Reg(usize::MAX)); // Placeholder
            }
        }

        let opcode = match mnemonic {
            "add" => {
                // Handle `add x0, x0, .L0@PAGEOFF` case from ADRP
                if args[2].contains("@PAGEOFF") {
                    let dest = self.parse_reg(args[0])?;
                    let base = self.parse_reg(args[1])?;
                    let label_part = args[2].split('@').next().unwrap();
                    let label_addr = *self.labels.get(label_part).ok_or_else(|| format!("Label not found: {}", label_part))?;
                    operands = vec![Operand::Reg(dest), Operand::Reg(base), Operand::Imm(label_addr as i64)];
                }
                OpCode::ADD
            }
            "sub" => OpCode::SUB, "mul" => OpCode::MUL, "sdiv" => OpCode::SDIV, "udiv" => OpCode::UDIV,
            "msub" => OpCode::MSUB, "neg" => OpCode::NEG, "uxtw" => OpCode::UXTW,
            "fadd" => OpCode::FADD, "fsub" => OpCode::FSUB, "fmul" => OpCode::FMUL, "fdiv" => OpCode::FDIV,
            "fmov" => OpCode::FMOV,
            "and" => OpCode::AND, "orr" => OpCode::ORR, "eor" => OpCode::EOR, "mvn" => OpCode::MVN,
            "lsl" => OpCode::LSL, "lsr" => OpCode::LSR,
            "mov" => OpCode::MOV, "ldr" => OpCode::LDR, "ldrb" => OpCode::LDRB, "str" => OpCode::STR,
            "strb" => OpCode::STRB, "stp" => OpCode::STP, "ldp" => OpCode::LDP,
            "b" => OpCode::B, "bl" => OpCode::BL, "ret" => OpCode::RET,
            "cmp" => OpCode::CMP,
            "cset" => OpCode::CSET,
            "b.eq" => OpCode::B_COND(Condition::EQ), "b.ne" => OpCode::B_COND(Condition::NE),
            "b.lt" => OpCode::B_COND(Condition::LT), "b.gt" => OpCode::B_COND(Condition::GT),
            "b.le" => OpCode::B_COND(Condition::LE), "b.ge" => OpCode::B_COND(Condition::GE),
            "b.hi" => OpCode::B_COND(Condition::HI), "b.ls" => OpCode::B_COND(Condition::LS),
            "b.hs" | "b.cs" => OpCode::B_COND(Condition::HS),
            "b.lo" | "b.cc" => OpCode::B_COND(Condition::LO),
            "b.mi" => OpCode::B_COND(Condition::MI), "b.pl" => OpCode::B_COND(Condition::PL),
            "b.vs" => OpCode::B_COND(Condition::VS), "b.vc" => OpCode::B_COND(Condition::VC),
            "cbz" => OpCode::CBZ, "cbnz" => OpCode::CBNZ,
            "adrp" => OpCode::ADRP, // This is now a pseudo-op
            "svc" => OpCode::SVC,
            "nop" => OpCode::NOP,
            _ => return Err(format!("Unknown instruction: {}", mnemonic)),
        };

        Ok(Instruction { opcode, operands })
    }
}

fn split_asm_args(s: &str) -> Vec<String> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut bracket_depth: i32 = 0;

    for ch in s.chars() {
        match ch {
            '[' => { bracket_depth += 1; current.push(ch); }
            ']' => { bracket_depth = (bracket_depth - 1).max(0); current.push(ch); }
            ',' if bracket_depth == 0 => {
                let trimmed = current.trim();
                if !trimmed.is_empty() { args.push(trimmed.to_string()); }
                current.clear();
            }
            _ => current.push(ch),
        }
    }

    let trimmed = current.trim();
    if !trimmed.is_empty() { args.push(trimmed.to_string()); }
    args
}

fn write_data_directive(vm: &mut VM, line: &str, data_ptr: &mut usize) -> Result<(), String> {
    if line.starts_with(".asciz") {
        let raw = line.split('"').nth(1).unwrap_or("");
        let s = unescape_asciz(raw);
        let bytes = s.as_bytes();
        if *data_ptr + bytes.len() + 1 > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr..*data_ptr + bytes.len()].copy_from_slice(bytes);
        vm.memory[*data_ptr + bytes.len()] = 0;
        *data_ptr += bytes.len() + 1;
        Ok(())
    } else if line.starts_with(".quad") {
        let val_str = line.split_whitespace().nth(1).unwrap_or("0");
        let val = vm.parse_imm(val_str)?;
        if *data_ptr + 8 > vm.memory.len() { return Err("Data segment write out of bounds".to_string()); }
        vm.memory[*data_ptr..*data_ptr + 8].copy_from_slice(&val.to_le_bytes());
        *data_ptr += 8;
        Ok(())
    } else {
        Ok(())
    }
}

fn unescape_asciz(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' { out.push(ch); continue; }
        match chars.next() {
            Some('n') => out.push('\n'), Some('t') => out.push('\t'), Some('r') => out.push('\r'),
            Some('0') => out.push('\0'), Some('\"') => out.push('\"'), Some('\\') => out.push('\\'),
            Some(other) => { out.push('\\'); out.push(other); }
            None => out.push('\\'),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_asm_args_preserves_bracket_commas() {
        let args = split_asm_args("x29, x30, [sp, #-16]!");
        assert_eq!(args, vec!["x29", "x30", "[sp, #-16]!"]);
        let args = split_asm_args("x0, [x21, x22, lsl #3]");
        assert_eq!(args, vec!["x0", "[x21, x22, lsl #3]"]);
    }

    #[test]
    fn test_load_program_data_and_bytecode() {
        let asm = r#"
.data
greet: .asciz "hi"
.text
_main:
    mov x0, #42
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        let addr = *vm.labels.get("greet").unwrap();
        assert_eq!(&vm.memory[addr..addr + 3], b"hi\0");
        assert_eq!(vm.program.len(), 2);
        assert!(matches!(vm.program[0].opcode, OpCode::MOV));
        assert!(matches!(vm.program[0].operands[1], Operand::Imm(42)));
        assert!(matches!(vm.program[1].opcode, OpCode::RET));
    }

    #[test]
    fn test_run_simple_bytecode() {
        let asm = r#"
.text
_main:
    mov x0, #10
    mov x1, #32
    add x2, x0, x1
    ret
"#;
        let mut vm = VM::new();
        vm.load_program(asm).unwrap();
        vm.run().unwrap();
        assert_eq!(vm.get_reg(2), 42);
    }
}
