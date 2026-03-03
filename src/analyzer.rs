use crate::ir::*;
use std::collections::{HashMap, HashSet};

pub struct Analyzer {
    errors: Vec<String>,
    warnings: Vec<String>,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    pub fn analyze(&mut self, program: &IrProgram) -> Result<(), Vec<String>> {
        self.check_dead_code(program);
        self.check_type_consistency(program);
        self.check_resource_leaks(program);

        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(self.errors.clone())
        }
    }

    pub fn get_warnings(&self) -> &[String] {
        &self.warnings
    }

    fn check_dead_code(&mut self, program: &IrProgram) {
        for (name, func) in &program.functions {
            let mut reachable = HashSet::new();
            let mut worklist = vec![0];

            while let Some(idx) = worklist.pop() {
                if idx >= func.instructions.len() || reachable.contains(&idx) {
                    continue;
                }

                reachable.insert(idx);

                match &func.instructions[idx] {
                    IrInstr::Jump { .. } => {
                        // Find target label
                    }
                    IrInstr::JumpIf { .. } => {
                        worklist.push(idx + 1);
                    }
                    IrInstr::Return { .. } => {
                        // End of path
                    }
                    _ => {
                        worklist.push(idx + 1);
                    }
                }
            }

            for (idx, _instr) in func.instructions.iter().enumerate() {
                if !reachable.contains(&idx) {
                    self.warnings.push(format!(
                        "Dead code in function '{}' at instruction {}",
                        name, idx
                    ));
                }
            }
        }
    }

    fn check_type_consistency(&mut self, program: &IrProgram) {
        for (name, func) in &program.functions {
            let mut types: HashMap<String, ValueType> = HashMap::new();

            // Initialize parameters as numbers (for now, assume numeric parameters)
            for param in &func.params {
                types.insert(param.clone(), ValueType::Number);
            }

            for instr in &func.instructions {
                match instr {
                    IrInstr::LoadConst { dest, value } => {
                        let ty = match value {
                            IrValue::Number(_) => ValueType::Number,
                            IrValue::String(_) => ValueType::String,
                            IrValue::Bool(_) => ValueType::Bool,
                        };
                        types.insert(dest.clone(), ty);
                    }
                    IrInstr::Add { dest, left, right }
                    | IrInstr::Sub { dest, left, right }
                    | IrInstr::Mul { dest, left, right }
                    | IrInstr::Div { dest, left, right } => {
                        let left_ty = types.get(left).unwrap_or(&ValueType::Number);
                        let right_ty = types.get(right).unwrap_or(&ValueType::Number);

                        if let (ValueType::Number, ValueType::Number) = (left_ty, right_ty) {
                            types.insert(dest.clone(), ValueType::Number);
                        } else if let (ValueType::String, ValueType::String) = (left_ty, right_ty) {
                            // String concatenation for Add
                            if matches!(instr, IrInstr::Add { .. }) {
                                types.insert(dest.clone(), ValueType::String);
                            } else {
                                self.errors.push(format!(
                                    "Type error in function '{}': Cannot perform arithmetic on strings",
                                    name
                                ));
                            }
                        } else {
                            // Don't error on unknown types (might be parameters)
                            types.insert(dest.clone(), ValueType::Number);
                        }
                    }
                    IrInstr::Move { dest, src } => {
                        if let Some(ty) = types.get(src) {
                            types.insert(dest.clone(), ty.clone());
                        } else {
                            types.insert(dest.clone(), ValueType::Number);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn check_resource_leaks(&mut self, program: &IrProgram) {
        for (name, func) in &program.functions {
            let mut open_files = HashSet::new();

            for instr in &func.instructions {
                match instr {
                    IrInstr::AllocFile { dest, .. } => {
                        open_files.insert(dest.clone());
                    }
                    IrInstr::CloseFile { handle } => {
                        open_files.remove(handle);
                    }
                    IrInstr::Return { .. } => {
                        if !open_files.is_empty() {
                            self.warnings.push(format!(
                                "Potential resource leak in function '{}': {} file(s) not closed",
                                name,
                                open_files.len()
                            ));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum ValueType {
    Number,
    String,
    Bool,
    Unknown,
}

#[allow(dead_code)]
pub fn validate_stack_alignment(instructions: &[IrInstr]) -> Result<(), String> {
    // ARM64 requires 16-byte stack alignment
    let stack_offset = 0;

    for instr in instructions {
        match instr {
            IrInstr::Call { .. } => {
                if stack_offset % 16 != 0 {
                    return Err(format!(
                        "Stack misalignment before call: offset {} is not 16-byte aligned",
                        stack_offset
                    ));
                }
            }
            _ => {}
        }
    }

    Ok(())
}
