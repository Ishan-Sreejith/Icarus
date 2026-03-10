use crate::ast::*;
use crate::lexer;
use crate::parser;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrInstr {
    Add {
        dest: String,
        left: String,
        right: String,
    },
    Sub {
        dest: String,
        left: String,
        right: String,
    },
    Mul {
        dest: String,
        left: String,
        right: String,
    },
    Div {
        dest: String,
        left: String,
        right: String,
    },

    Eq {
        dest: String,
        left: String,
        right: String,
    },
    Ne {
        dest: String,
        left: String,
        right: String,
    },
    Lt {
        dest: String,
        left: String,
        right: String,
    },

    Gt {
        dest: String,
        left: String,
        right: String,
    },

    LogicAnd {
        dest: String,
        left: String,
        right: String,
    },
    LogicOr {
        dest: String,
        left: String,
        right: String,
    },
    LogicNot {
        dest: String,
        src: String,
    },

    FAdd {
        dest: String,
        left: String,
        right: String,
    },
    FSub {
        dest: String,
        left: String,
        right: String,
    },
    FMul {
        dest: String,
        left: String,
        right: String,
    },
    FDiv {
        dest: String,
        left: String,
        right: String,
    },

    BitAnd {
        dest: String,
        left: String,
        right: String,
    },
    BitOr {
        dest: String,
        left: String,
        right: String,
    },
    BitXor {
        dest: String,
        left: String,
        right: String,
    },
    BitNot {
        dest: String,
        src: String,
    },
    Shl {
        dest: String,
        left: String,
        right: String,
    },
    Shr {
        dest: String,
        left: String,
        right: String,
    },

    AllocStruct {
        dest: String,
        name: String,
    },
    SetMember {
        obj: String,
        member: String,
        value: String,
    },
    GetMember {
        dest: String,
        obj: String,
        member: String,
    },

    LoadConst {
        dest: String,
        value: IrValue,
    },
    Move {
        dest: String,
        src: String,
    },

    AllocList {
        dest: String,
        items: Vec<String>,
    },
    GetIndex {
        dest: String,
        src: String,
        index: String,
    },
    SetIndex {
        src: String,
        index: String,
        value: String,
    },

    AllocMap {
        dest: String,
    },
    SetMap {
        map: String,
        key: String,
        value: String,
    },
    GetMap {
        dest: String,
        map: String,
        key: String,
    },

    Print {
        src: String,
    },
    PrintNum {
        src: String,
    },
    Input {
        dest: String,
        prompt: String,
    },

    Call {
        dest: Option<String>,
        func: String,
        args: Vec<String>,
    },
    Return {
        value: Option<String>,
    },
    Label {
        name: String,
    },
    Jump {
        label: String,
    },
    JumpIf {
        cond: String,
        label: String,
    },

    AllocFile {
        dest: String,
        path: String,
    },
    CloseFile {
        handle: String,
    },

    Spawn {
        task: String,
    },
    Await {
        dest: String,
        task: String,
    },

    LinkFile {
        path: String,
    },

    Hardwire {
        path: String,
    },

    PreScan {
        target: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IrValue {
    Number(f64),
    String(String),
    Bool(bool),
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<String>,
    pub instructions: Vec<IrInstr>,
    pub is_async: bool,
    pub locals: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct IrProgram {
    pub functions: HashMap<String, IrFunction>,
    pub global_code: Vec<IrInstr>,
}

pub struct IrBuilder {
    temp_counter: usize,
    label_counter: usize,
    var_types: HashMap<String, ValueType>,
    structs: HashSet<String>,
    import_cache: HashMap<PathBuf, Program>,
    processed_modules: HashSet<PathBuf>,
    import_stack: Vec<PathBuf>,
    try_stack: Vec<(String, String)>, // (catch_label, err_var)
    trait_methods: HashMap<String, Vec<String>>,
    known_functions: HashMap<String, usize>,
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
enum ValueType {
    Number,
    String,
    Bool,
    Struct(String),
    List,
    Map,
    Unknown,
}

impl IrBuilder {
    pub fn new() -> Self {
        IrBuilder {
            temp_counter: 0,
            label_counter: 0,
            var_types: HashMap::new(),
            structs: HashSet::new(),
            import_cache: HashMap::new(),
            processed_modules: HashSet::new(),
            import_stack: Vec::new(),
            try_stack: Vec::new(),
            trait_methods: HashMap::new(),
            known_functions: HashMap::new(),
        }
    }

    fn fresh_temp(&mut self) -> String {
        let temp = format!("t{}", self.temp_counter);
        self.temp_counter += 1;
        temp
    }

    fn fresh_label(&mut self) -> String {
        let label = format!("L{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    pub fn build(
        &mut self,
        program: &Program,
        entry_file: Option<&Path>,
    ) -> Result<IrProgram, String> {
        self.import_cache.clear();
        self.processed_modules.clear();
        self.import_stack.clear();
        self.try_stack.clear();
        self.trait_methods.clear();
        self.known_functions.clear();

        let mut ir_program = IrProgram {
            functions: HashMap::new(),
            global_code: Vec::new(),
        };

        let default_entry = Path::new("<memory>");
        let entry = entry_file.unwrap_or(default_entry);
        self.process_program(program, entry, &mut ir_program)?;

        Ok(ir_program)
    }

    fn process_program(
        &mut self,
        program: &Program,
        current_file: &Path,
        ir_program: &mut IrProgram,
    ) -> Result<(), String> {
        for item in &program.items {
            match item {
                TopLevel::Function(func) => {
                    if ir_program.functions.contains_key(&func.name) {
                        return Err(format!(
                            "Duplicate function '{}' (imported via {})",
                            func.name,
                            current_file.display()
                        ));
                    }
                    self.known_functions
                        .insert(func.name.clone(), func.params.len());
                    let ir_func = self.build_function(func)?;
                    ir_program.functions.insert(func.name.clone(), ir_func);
                }
                TopLevel::Statement(stmt) => {
                    self.build_statement(stmt, &mut ir_program.global_code)?;
                }
                TopLevel::Struct(s) => {
                    self.structs.insert(s.name.clone());
                }
                TopLevel::Trait(t) => {
                    self.trait_methods.insert(t.name.clone(), t.methods.clone());
                }
                TopLevel::Impl(imp) => {
                    if let Some(required) = self.trait_methods.get(&imp.trait_name).cloned() {
                        let provided: HashSet<String> =
                            imp.methods.iter().map(|m| m.name.clone()).collect();
                        for r in required {
                            if !provided.contains(&r) {
                                return Err(format!(
                                    "Impl of trait '{}' for '{}' is missing method '{}'",
                                    imp.trait_name, imp.type_name, r
                                ));
                            }
                        }
                    }

                    for method in &imp.methods {
                        let new_name = format!("{}_{}", imp.type_name, method.name);
                        if ir_program.functions.contains_key(&new_name) {
                            return Err(format!(
                                "Duplicate function '{}' (generated from impl of {} for {})",
                                new_name, imp.trait_name, imp.type_name
                            ));
                        }

                        let mut renamed = method.clone();
                        renamed.name = new_name.clone();
                        self.known_functions
                            .insert(renamed.name.clone(), renamed.params.len());
                        let ir_func = self.build_function(&renamed)?;
                        ir_program.functions.insert(new_name, ir_func);
                    }
                }
                TopLevel::Import(path) => {
                    self.process_import(path, current_file, ir_program)?;
                }
                TopLevel::Use(path) => {
                    ir_program.global_code.push(IrInstr::LinkFile {
                        path: self
                            .resolve_import_path(path, current_file)?
                            .to_string_lossy()
                            .to_string(),
                    });
                }
                TopLevel::Hardwire(path) => {
                    ir_program.global_code.push(IrInstr::Hardwire {
                        path: self
                            .resolve_import_path(path, current_file)?
                            .to_string_lossy()
                            .to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    fn process_import(
        &mut self,
        import_str: &str,
        current_file: &Path,
        ir_program: &mut IrProgram,
    ) -> Result<(), String> {
        let resolved = self.resolve_import_path(import_str, current_file)?;

        let canonical = fs::canonicalize(&resolved).map_err(|e| {
            format!(
                "Failed to resolve import '{}' from '{}': {}",
                import_str,
                current_file.display(),
                e
            )
        })?;

        if self.import_stack.contains(&canonical) {
            let mut chain = self
                .import_stack
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>();
            chain.push(canonical.display().to_string());
            return Err(format!(
                "Cyclic import detected:\n{}",
                chain.join("\n  -> ")
            ));
        }
        if self.processed_modules.contains(&canonical) {
            return Ok(());
        }

        self.import_stack.push(canonical.clone());

        let imported_program = if let Some(p) = self.import_cache.get(&canonical).cloned() {
            p
        } else {
            let p = parse_core_file(&canonical)
                .map_err(|e| format!("Failed to import '{}': {}", canonical.display(), e))?;
            self.import_cache.insert(canonical.clone(), p.clone());
            p
        };

        self.process_program(&imported_program, &canonical, ir_program)?;
        self.processed_modules.insert(canonical.clone());

        self.import_stack.pop();
        Ok(())
    }

    fn resolve_import_path(
        &self,
        import_str: &str,
        current_file: &Path,
    ) -> Result<PathBuf, String> {
        let raw = PathBuf::from(import_str);
        let base_dir = current_file.parent().unwrap_or_else(|| Path::new("."));

        if raw.extension().is_some() {
            if raw.is_absolute() {
                return Ok(raw);
            }
            return Ok(base_dir.join(raw));
        }

        let mut fr = raw.clone();
        fr.set_extension("fr");
        let mut mtro = raw;
        mtro.set_extension("mtro");

        if fr.is_absolute() {
            if fr.exists() {
                return Ok(fr);
            }
            if mtro.exists() {
                return Ok(mtro);
            }
            return Ok(fr);
        }

        let fr_rel = base_dir.join(&fr);
        if fr_rel.exists() {
            return Ok(fr_rel);
        }
        let mtro_rel = base_dir.join(&mtro);
        if mtro_rel.exists() {
            return Ok(mtro_rel);
        }

        Ok(fr_rel)
    }

    fn build_function(&mut self, func: &FnDef) -> Result<IrFunction, String> {
        let mut instructions = Vec::new();

        for param in &func.params {
            self.var_types.insert(param.clone(), ValueType::Unknown);
        }

        for stmt in &func.body {
            self.build_statement(stmt, &mut instructions)?;
        }

        Ok(IrFunction {
            name: func.name.clone(),
            params: func.params.clone(),
            instructions,
            is_async: func.is_async,
            locals: Vec::new(),
        })
    }

    fn build_statement(&mut self, stmt: &Stmt, instrs: &mut Vec<IrInstr>) -> Result<(), String> {
        match stmt {
            Stmt::VarDecl(name, expr) => {
                let temp = self.build_expr(expr, instrs)?;
                let inferred = match expr {
                    Expr::Number(_) | Expr::Float(_) => ValueType::Number,
                    Expr::String(_) | Expr::Ask(_) => ValueType::String,
                    Expr::Bool(_) => ValueType::Bool,
                    Expr::List(_) => ValueType::List,
                    Expr::Map(_) => ValueType::Map,
                    Expr::Identifier(id) if self.structs.contains(id) => {
                        ValueType::Struct(id.clone())
                    }
                    _ => ValueType::Unknown,
                };
                self.var_types.insert(name.clone(), inferred);
                instrs.push(IrInstr::Move {
                    dest: name.clone(),
                    src: temp,
                });
            }
            Stmt::Assign(target, value) => {
                let val_temp = self.build_expr(value, instrs)?;
                match target {
                    Expr::Identifier(target_name) => {
                        let inferred = match value {
                            Expr::Number(_) | Expr::Float(_) => ValueType::Number,
                            Expr::String(_) | Expr::Ask(_) => ValueType::String,
                            Expr::Bool(_) => ValueType::Bool,
                            Expr::List(_) => ValueType::List,
                            Expr::Map(_) => ValueType::Map,
                            Expr::Identifier(id) if self.structs.contains(id) => {
                                ValueType::Struct(id.clone())
                            }
                            _ => self
                                .var_types
                                .get(target_name)
                                .cloned()
                                .unwrap_or(ValueType::Unknown),
                        };
                        self.var_types.insert(target_name.clone(), inferred);
                        instrs.push(IrInstr::Move {
                            dest: target_name.clone(),
                            src: val_temp,
                        });
                    }
                    Expr::Index(obj, idx) => {
                        let obj_temp = self.build_expr(obj, instrs)?;
                        let idx_temp = self.build_expr(idx, instrs)?;
                        instrs.push(IrInstr::SetIndex {
                            src: obj_temp,
                            index: idx_temp,
                            value: val_temp,
                        });
                    }
                    Expr::Member(obj, member) => {
                        let obj_temp = self.build_expr(obj, instrs)?;
                        instrs.push(IrInstr::SetMember {
                            obj: obj_temp,
                            member: member.clone(),
                            value: val_temp,
                        });
                    }
                    _ => return Err("Invalid assignment target".to_string()),
                }
            }
            Stmt::Say(expr) => {
                let temp = self.build_expr(expr, instrs)?;
                instrs.push(IrInstr::Print { src: temp });
            }
            Stmt::Return(expr) => {
                let temp = self.build_expr(expr, instrs)?;
                instrs.push(IrInstr::Return { value: Some(temp) });
            }
            Stmt::Import(_) => {
                return Err("import is only supported at the top level (module scope)".to_string());
            }
            Stmt::TryCatch(try_body, _err_var, catch_body) => {
                let thrown_slot = "__core_thrown".to_string();
                let catch_label = self.fresh_label();
                let end_label = self.fresh_label();

                self.try_stack.push((catch_label.clone(), _err_var.clone()));

                for stmt in try_body {
                    self.build_statement(stmt, instrs)?;
                }

                self.try_stack.pop();
                instrs.push(IrInstr::Jump {
                    label: end_label.clone(),
                });

                instrs.push(IrInstr::Label { name: catch_label });
                instrs.push(IrInstr::Move {
                    dest: _err_var.clone(),
                    src: thrown_slot,
                });
                for stmt in catch_body {
                    self.build_statement(stmt, instrs)?;
                }
                instrs.push(IrInstr::Label { name: end_label });
            }
            Stmt::Throw(expr) => {
                let thrown_slot = "__core_thrown".to_string();
                let val = self.build_expr(expr, instrs)?;
                let (catch_label, _err_var) = self
                    .try_stack
                    .last()
                    .cloned()
                    .ok_or_else(|| "throw used outside of a try/catch block".to_string())?;

                instrs.push(IrInstr::Move {
                    dest: thrown_slot,
                    src: val,
                });
                instrs.push(IrInstr::Jump { label: catch_label });
            }
            Stmt::Struct(_) => {
            }
            Stmt::Expr(expr) => {
                self.build_expr(expr, instrs)?;
            }
            Stmt::Block(stmts) => {
                for stmt in stmts {
                    self.build_statement(stmt, instrs)?;
                }
            }
            Stmt::If(cond, then_block, else_block) => {
                let cond_temp = self.build_expr(cond, instrs)?;
                let not_cond = self.fresh_temp();
                instrs.push(IrInstr::LogicNot {
                    dest: not_cond.clone(),
                    src: cond_temp,
                });
                let else_label = self.fresh_label();
                let end_label = self.fresh_label();

                instrs.push(IrInstr::JumpIf {
                    cond: not_cond,
                    label: else_label.clone(),
                });

                for stmt in then_block {
                    self.build_statement(stmt, instrs)?;
                }
                instrs.push(IrInstr::Jump {
                    label: end_label.clone(),
                });

                instrs.push(IrInstr::Label { name: else_label });
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.build_statement(stmt, instrs)?;
                    }
                }

                instrs.push(IrInstr::Label { name: end_label });
            }
            Stmt::While(cond, body) => {
                let start_label = self.fresh_label();
                let end_label = self.fresh_label();

                instrs.push(IrInstr::Label {
                    name: start_label.clone(),
                });

                let cond_temp = self.build_expr(cond, instrs)?;
                let not_cond = self.fresh_temp();
                instrs.push(IrInstr::LogicNot {
                    dest: not_cond.clone(),
                    src: cond_temp,
                });
                instrs.push(IrInstr::JumpIf {
                    cond: not_cond,
                    label: end_label.clone(),
                });

                for stmt in body {
                    self.build_statement(stmt, instrs)?;
                }

                instrs.push(IrInstr::Jump { label: start_label });
                instrs.push(IrInstr::Label { name: end_label });
            }
            Stmt::For(var_name, iterable, body) => {
                match iterable {
                    Expr::Range(start_expr, end_expr) => {
                        let start_temp = self.build_expr(start_expr, instrs)?;
                        let end_temp = self.build_expr(end_expr, instrs)?;

                        instrs.push(IrInstr::Move {
                            dest: var_name.clone(),
                            src: start_temp,
                        });

                        let start_label = self.fresh_label();
                        let end_label = self.fresh_label();

                        instrs.push(IrInstr::Label {
                            name: start_label.clone(),
                        });

                        let cond_temp = self.fresh_temp();
                        instrs.push(IrInstr::Lt {
                            dest: cond_temp.clone(),
                            left: var_name.clone(),
                            right: end_temp,
                        });

                        let continue_label = self.fresh_label();
                        instrs.push(IrInstr::JumpIf {
                            cond: cond_temp,
                            label: continue_label.clone(),
                        });
                        instrs.push(IrInstr::Jump {
                            label: end_label.clone(),
                        });

                        instrs.push(IrInstr::Label {
                            name: continue_label,
                        });

                        for stmt in body {
                            self.build_statement(stmt, instrs)?;
                        }

                        let one_temp = self.fresh_temp();
                        instrs.push(IrInstr::LoadConst {
                            dest: one_temp.clone(),
                            value: IrValue::Number(1.0),
                        });

                        let next_val = self.fresh_temp();
                        instrs.push(IrInstr::Add {
                            dest: next_val.clone(),
                            left: var_name.clone(),
                            right: one_temp,
                        });
                        instrs.push(IrInstr::Move {
                            dest: var_name.clone(),
                            src: next_val,
                        });

                        instrs.push(IrInstr::Jump { label: start_label });
                        instrs.push(IrInstr::Label { name: end_label });
                    }
                    _ => {
                        let iterable_temp = self.build_expr(iterable, instrs)?;

                        let iter_list = self.fresh_temp();
                        let is_map = self.fresh_temp();
                        instrs.push(IrInstr::Call {
                            dest: Some(is_map.clone()),
                            func: "is_map".to_string(),
                            args: vec![iterable_temp.clone()],
                        });

                        let map_label = self.fresh_label();
                        let init_label = self.fresh_label();
                        instrs.push(IrInstr::JumpIf {
                            cond: is_map,
                            label: map_label.clone(),
                        });

                        instrs.push(IrInstr::Move {
                            dest: iter_list.clone(),
                            src: iterable_temp.clone(),
                        });
                        instrs.push(IrInstr::Jump {
                            label: init_label.clone(),
                        });

                        instrs.push(IrInstr::Label { name: map_label });
                        let keys_temp = self.fresh_temp();
                        instrs.push(IrInstr::Call {
                            dest: Some(keys_temp.clone()),
                            func: "keys".to_string(),
                            args: vec![iterable_temp],
                        });
                        instrs.push(IrInstr::Move {
                            dest: iter_list.clone(),
                            src: keys_temp,
                        });

                        instrs.push(IrInstr::Label { name: init_label });

                        let idx_var = self.fresh_temp();
                        let zero_temp = self.fresh_temp();
                        instrs.push(IrInstr::LoadConst {
                            dest: zero_temp.clone(),
                            value: IrValue::Number(0.0),
                        });
                        instrs.push(IrInstr::Move {
                            dest: idx_var.clone(),
                            src: zero_temp,
                        });

                        let start_label = self.fresh_label();
                        let end_label = self.fresh_label();
                        instrs.push(IrInstr::Label {
                            name: start_label.clone(),
                        });

                        let len_temp = self.fresh_temp();
                        instrs.push(IrInstr::Call {
                            dest: Some(len_temp.clone()),
                            func: "len".to_string(),
                            args: vec![iter_list.clone()],
                        });

                        let cond_temp = self.fresh_temp();
                        instrs.push(IrInstr::Lt {
                            dest: cond_temp.clone(),
                            left: idx_var.clone(),
                            right: len_temp,
                        });

                        let body_label = self.fresh_label();
                        instrs.push(IrInstr::JumpIf {
                            cond: cond_temp,
                            label: body_label.clone(),
                        });
                        instrs.push(IrInstr::Jump {
                            label: end_label.clone(),
                        });

                        instrs.push(IrInstr::Label { name: body_label });

                        let item_temp = self.fresh_temp();
                        instrs.push(IrInstr::GetIndex {
                            dest: item_temp.clone(),
                            src: iter_list,
                            index: idx_var.clone(),
                        });
                        instrs.push(IrInstr::Move {
                            dest: var_name.clone(),
                            src: item_temp,
                        });

                        for stmt in body {
                            self.build_statement(stmt, instrs)?;
                        }

                        let one_temp = self.fresh_temp();
                        instrs.push(IrInstr::LoadConst {
                            dest: one_temp.clone(),
                            value: IrValue::Number(1.0),
                        });
                        let next_idx = self.fresh_temp();
                        instrs.push(IrInstr::Add {
                            dest: next_idx.clone(),
                            left: idx_var.clone(),
                            right: one_temp,
                        });
                        instrs.push(IrInstr::Move {
                            dest: idx_var,
                            src: next_idx,
                        });

                        instrs.push(IrInstr::Jump { label: start_label });
                        instrs.push(IrInstr::Label { name: end_label });
                    }
                }
            }
        }
        Ok(())
    }

    fn build_expr(&mut self, expr: &Expr, instrs: &mut Vec<IrInstr>) -> Result<String, String> {
        match expr {
            Expr::Float(n) => {
                let temp = self.fresh_temp();
                instrs.push(IrInstr::LoadConst {
                    dest: temp.clone(),
                    value: IrValue::Number(*n), // We use IrValue::Number for both
                });
                Ok(temp)
            }
            Expr::Number(n) => {
                let temp = self.fresh_temp();
                instrs.push(IrInstr::LoadConst {
                    dest: temp.clone(),
                    value: IrValue::Number(*n),
                });
                Ok(temp)
            }
            Expr::String(s) => {
                let temp = self.fresh_temp();
                instrs.push(IrInstr::LoadConst {
                    dest: temp.clone(),
                    value: IrValue::String(s.clone()),
                });
                Ok(temp)
            }
            Expr::Identifier(name) => {
                if self.structs.contains(name) {
                    let dest = self.fresh_temp();
                    instrs.push(IrInstr::AllocStruct {
                        dest: dest.clone(),
                        name: name.clone(),
                    });
                    self.var_types
                        .insert(dest.clone(), ValueType::Struct(name.clone()));
                    Ok(dest)
                } else if !self.var_types.contains_key(name)
                    && self.known_functions.get(name).copied() == Some(0)
                {
                    let dest = self.fresh_temp();
                    instrs.push(IrInstr::Call {
                        dest: Some(dest.clone()),
                        func: name.clone(),
                        args: Vec::new(),
                    });
                    Ok(dest)
                } else {
                    Ok(name.clone())
                }
            }
            Expr::Add(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Add {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::Sub(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Sub {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::Mul(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Mul {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::Div(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Div {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::Le(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;

                let gt = self.fresh_temp();
                self.var_types.insert(gt.clone(), ValueType::Number);
                instrs.push(IrInstr::Gt {
                    dest: gt.clone(),
                    left: left_temp,
                    right: right_temp,
                });

                let dest = self.fresh_temp();
                self.var_types.insert(dest.clone(), ValueType::Number);
                instrs.push(IrInstr::LogicNot {
                    dest: dest.clone(),
                    src: gt,
                });
                Ok(dest)
            }
            Expr::Ge(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;

                let lt = self.fresh_temp();
                self.var_types.insert(lt.clone(), ValueType::Number);
                instrs.push(IrInstr::Lt {
                    dest: lt.clone(),
                    left: left_temp,
                    right: right_temp,
                });

                let dest = self.fresh_temp();
                self.var_types.insert(dest.clone(), ValueType::Number);
                instrs.push(IrInstr::LogicNot {
                    dest: dest.clone(),
                    src: lt,
                });
                Ok(dest)
            }
            Expr::Eq(left, right)
            | Expr::Ne(left, right)
            | Expr::Lt(left, right)
            | Expr::Gt(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                self.var_types.insert(dest.clone(), ValueType::Number); // Bools are numbers in ARM64
                let instr = match expr {
                    Expr::Eq(_, _) => IrInstr::Eq {
                        dest: dest.clone(),
                        left: left_temp,
                        right: right_temp,
                    },
                    Expr::Ne(_, _) => IrInstr::Ne {
                        dest: dest.clone(),
                        left: left_temp,
                        right: right_temp,
                    },
                    Expr::Lt(_, _) => IrInstr::Lt {
                        dest: dest.clone(),
                        left: left_temp,
                        right: right_temp,
                    },
                    Expr::Gt(_, _) => IrInstr::Gt {
                        dest: dest.clone(),
                        left: left_temp,
                        right: right_temp,
                    },
                    _ => return Err("Unsupported comparison expression".to_string()),
                };
                instrs.push(instr);
                Ok(dest)
            }
            Expr::And(left, right) | Expr::Or(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                self.var_types.insert(dest.clone(), ValueType::Number);
                let instr = match expr {
                    Expr::And(_, _) => IrInstr::LogicAnd {
                        dest: dest.clone(),
                        left: left_temp,
                        right: right_temp,
                    },
                    Expr::Or(_, _) => IrInstr::LogicOr {
                        dest: dest.clone(),
                        left: left_temp,
                        right: right_temp,
                    },
                    _ => return Err("Unsupported logical expression".to_string()),
                };
                instrs.push(instr);
                Ok(dest)
            }
            Expr::Not(sub_expr) => {
                let temp = self.build_expr(sub_expr, instrs)?;
                let dest = self.fresh_temp();
                self.var_types.insert(dest.clone(), ValueType::Number);
                instrs.push(IrInstr::LogicNot {
                    dest: dest.clone(),
                    src: temp,
                });
                Ok(dest)
            }
            Expr::BitAnd(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::BitAnd {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::BitOr(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::BitOr {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::BitXor(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::BitXor {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::BitNot(src) => {
                let src_temp = self.build_expr(src, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::BitNot {
                    dest: dest.clone(),
                    src: src_temp,
                });
                Ok(dest)
            }
            Expr::Shl(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Shl {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::Shr(left, right) => {
                let left_temp = self.build_expr(left, instrs)?;
                let right_temp = self.build_expr(right, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Shr {
                    dest: dest.clone(),
                    left: left_temp,
                    right: right_temp,
                });
                Ok(dest)
            }
            Expr::Bool(b) => {
                let temp = self.fresh_temp();
                instrs.push(IrInstr::LoadConst {
                    dest: temp.clone(),
                    value: IrValue::Bool(*b),
                });
                Ok(temp)
            }
            Expr::Neg(sub_expr) => {
                let temp = self.build_expr(sub_expr, instrs)?;
                let dest = self.fresh_temp();
                let zero = self.fresh_temp();
                instrs.push(IrInstr::LoadConst {
                    dest: zero.clone(),
                    value: IrValue::Number(0.0),
                });
                instrs.push(IrInstr::Sub {
                    dest: dest.clone(),
                    left: zero,
                    right: temp,
                });
                Ok(dest)
            }
            Expr::Await(sub_expr) => {
                let task_temp = self.build_expr(sub_expr, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Await {
                    dest: dest.clone(),
                    task: task_temp,
                });
                Ok(dest)
            }
            Expr::Ask(prompt_expr) => {
                let prompt_temp = self.build_expr(prompt_expr, instrs)?;
                let dest = self.fresh_temp();
                self.var_types.insert(dest.clone(), ValueType::String);
                instrs.push(IrInstr::Input {
                    dest: dest.clone(),
                    prompt: prompt_temp,
                });
                Ok(dest)
            }

            Expr::Range(start, end) => {
                let start_temp = self.build_expr(start, instrs)?;
                let end_temp = self.build_expr(end, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Call {
                    dest: Some(dest.clone()),
                    func: "range".to_string(),
                    args: vec![start_temp, end_temp],
                });
                Ok(dest)
            }

            Expr::Call(func, args) => {
                if let Some(builtin_name) = func
                    .strip_prefix("__ff_builtin_")
                    .and_then(|name| name.strip_suffix("__"))
                {
                    if builtin_name == "ask" {
                        if args.len() != 1 {
                            return Err("ask() override expects exactly 1 argument".to_string());
                        }
                        let prompt_temp = self.build_expr(&args[0], instrs)?;
                        let dest = self.fresh_temp();
                        self.var_types.insert(dest.clone(), ValueType::String);
                        instrs.push(IrInstr::Input {
                            dest: dest.clone(),
                            prompt: prompt_temp,
                        });
                        return Ok(dest);
                    }

                    let mut arg_temps = Vec::new();
                    for arg in args {
                        arg_temps.push(self.build_expr(arg, instrs)?);
                    }
                    let dest = self.fresh_temp();
                    instrs.push(IrInstr::Call {
                        dest: Some(dest.clone()),
                        func: builtin_name.to_string(),
                        args: arg_temps,
                    });
                    return Ok(dest);
                }

                let mut arg_temps = Vec::new();
                for arg in args {
                    arg_temps.push(self.build_expr(arg, instrs)?);
                }
                let dest = self.fresh_temp();
                instrs.push(IrInstr::Call {
                    dest: Some(dest.clone()),
                    func: func.clone(),
                    args: arg_temps,
                });
                Ok(dest)
            }

            Expr::MethodCall(receiver, method, args) => {
                let receiver_val = self.build_expr(receiver, instrs)?;

                let resolved_name = match receiver.as_ref() {
                    Expr::Identifier(var) => match self.var_types.get(var) {
                        Some(ValueType::Struct(ty)) => format!("{}_{}", ty, method),
                        _ => method.clone(),
                    },
                    _ => method.clone(),
                };

                let mut arg_temps = Vec::with_capacity(args.len() + 1);
                arg_temps.push(receiver_val);
                for arg in args {
                    arg_temps.push(self.build_expr(arg, instrs)?);
                }

                let dest = self.fresh_temp();
                instrs.push(IrInstr::Call {
                    dest: Some(dest.clone()),
                    func: resolved_name,
                    args: arg_temps,
                });
                Ok(dest)
            }

            Expr::Map(entries) => {
                let map_temp = self.fresh_temp();
                instrs.push(IrInstr::AllocMap {
                    dest: map_temp.clone(),
                });
                self.var_types.insert(map_temp.clone(), ValueType::Map);

                for (key, value) in entries {
                    let key_temp = self.build_expr(key, instrs)?;
                    let value_temp = self.build_expr(value, instrs)?;

                    instrs.push(IrInstr::SetMap {
                        map: map_temp.clone(),
                        key: key_temp,
                        value: value_temp,
                    });
                }

                Ok(map_temp)
            }
            Expr::Index(target, index) => {
                let target_temp = self.build_expr(target, instrs)?;
                let index_temp = self.build_expr(index, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::GetIndex {
                    dest: dest.clone(),
                    src: target_temp,
                    index: index_temp,
                });
                Ok(dest)
            }
            Expr::Member(target, member) => {
                let target_temp = self.build_expr(target, instrs)?;
                let dest = self.fresh_temp();
                instrs.push(IrInstr::GetMember {
                    dest: dest.clone(),
                    obj: target_temp,
                    member: member.clone(),
                });
                Ok(dest)
            }
            Expr::List(items) => {
                let mut item_temps = Vec::new();
                for item in items {
                    item_temps.push(self.build_expr(item, instrs)?);
                }
                let dest = self.fresh_temp();
                instrs.push(IrInstr::AllocList {
                    dest: dest.clone(),
                    items: item_temps,
                });
                self.var_types.insert(dest.clone(), ValueType::List);
                Ok(dest)
            }
        }
    }
}

fn parse_core_file(path: &Path) -> Result<Program, String> {
    let source = crate::ff::preprocess_file(path)?;

    let tokens: Result<Vec<_>, _> = lexer::Lexer::new(&source)
        .map(|(token, span)| match token {
            Ok(t) => Ok((t, span)),
            Err(e) => Err(e),
        })
        .collect();

    let tokens = tokens.map_err(|e| format!("Lexer error in '{}': {}", path.display(), e))?;
    let mut parser = parser::Parser::new(tokens);
    parser
        .parse()
        .map_err(|e| format!("Parser error in '{}': {}", path.display(), e))
}

#[cfg(test)]
mod import_tests {
    use super::*;
    use crate::lexer::Token;
    use logos::Logos;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn parse_source(source: &str) -> Result<Program, String> {
        let mut tokens = Vec::new();
        let mut lex = Token::lexer(source);
        while let Some(token) = lex.next() {
            let span = lex.span();
            match token {
                Ok(t) => tokens.push((t, span)),
                Err(_) => return Err(format!("Lexer error at {:?}", span)),
            }
        }
        let mut parser = crate::parser::Parser::new(tokens);
        parser.parse()
    }

    fn temp_dir(prefix: &str) -> std::path::PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("core_{}_{}", prefix, now))
    }

    #[test]
    fn test_import_resolves_relative_to_entry_file() {
        let dir = temp_dir("import_rel");
        fs::create_dir_all(&dir).unwrap();

        let lib_path = dir.join("lib.fr");
        fs::write(&lib_path, "fn add: a, b { return a + b }").unwrap();

        let main_src = r#"import "lib.fr"
var x: add: 1, 2
say: x
"#;
        let program = parse_source(main_src).unwrap();

        let mut builder = IrBuilder::new();
        let ir = builder.build(&program, Some(&dir.join("main.fr"))).unwrap();
        assert!(ir.functions.contains_key("add"));
    }

    #[test]
    fn test_import_cycle_is_detected() {
        let dir = temp_dir("import_cycle");
        fs::create_dir_all(&dir).unwrap();

        let a_path = dir.join("a.fr");
        let b_path = dir.join("b.fr");
        fs::write(&a_path, r#"import "b.fr" fn fa: { return 0 }"#).unwrap();
        fs::write(&b_path, r#"import "a.fr" fn fb: { return 0 }"#).unwrap();

        let program = parse_core_file(&a_path).unwrap();
        let mut builder = IrBuilder::new();
        let err = builder.build(&program, Some(&a_path)).unwrap_err();
        assert!(err.contains("Cyclic import"));
    }

    #[test]
    fn test_import_without_extension_falls_back_to_mtro() {
        let dir = temp_dir("import_mtro");
        fs::create_dir_all(&dir).unwrap();

        let plugin_path = dir.join("plugin.mtro");
        fs::write(&plugin_path, "fn plug: { return 123 }").unwrap();

        let main_src = r#"import "plugin"
var x: plug:
say: x
"#;
        let program = parse_source(main_src).unwrap();

        let mut builder = IrBuilder::new();
        let ir = builder.build(&program, Some(&dir.join("main.fr"))).unwrap();
        assert!(ir.functions.contains_key("plug"));
    }
}
