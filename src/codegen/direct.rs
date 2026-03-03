use crate::ir::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::{self, Write};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub struct DirectExecutor {
    variables: HashMap<String, Value>,
    functions: HashMap<String, IrFunction>,
}

struct FrameState {
    instructions: Vec<IrInstr>,
    labels: HashMap<String, usize>,
    pc: usize,
    return_dest: Option<String>,
    saved_vars: HashMap<String, Value>,
}

#[derive(Clone)]
enum TaskStatus {
    Running,
    Waiting(u64),
    Sleeping(std::time::Instant),
    Done,
    Error(String),
}

struct TaskState {
    id: u64,
    vars: HashMap<String, Value>,
    frames: Vec<FrameState>,
    status: TaskStatus,
    result: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(f64),
    String(String),
    Bool(bool),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
    Struct(String, Rc<RefCell<HashMap<String, Value>>>),
    Task(u64),
    TaskThunk { func: String, args: Vec<Value> },
}

impl DirectExecutor {
    pub fn new() -> Self {
        DirectExecutor {
            variables: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn execute(&mut self, program: &IrProgram) -> Result<(), String> {
        // Load functions
        for (name, func) in &program.functions {
            self.functions.insert(name.clone(), func.clone());
        }

        // Execute global code (task scheduler; supports async/await + spawn + sleep)
        self.execute_with_scheduler(&program.global_code)?;

        Ok(())
    }

    fn build_labels(instructions: &[IrInstr]) -> HashMap<String, usize> {
        let mut labels = HashMap::new();
        for (i, instr) in instructions.iter().enumerate() {
            if let IrInstr::Label { name } = instr {
                labels.insert(name.clone(), i);
            }
        }
        labels
    }

    fn execute_with_scheduler(&mut self, global_code: &[IrInstr]) -> Result<(), String> {
        use std::collections::VecDeque;

        let mut next_task_id: u64 = 1;
        let mut tasks: HashMap<u64, TaskState> = HashMap::new();
        let mut runnable: VecDeque<u64> = VecDeque::new();

        let main_frame = FrameState {
            instructions: global_code.to_vec(),
            labels: Self::build_labels(global_code),
            pc: 0,
            return_dest: None,
            saved_vars: HashMap::new(),
        };

        tasks.insert(
            0,
            TaskState {
                id: 0,
                vars: HashMap::new(),
                frames: vec![main_frame],
                status: TaskStatus::Running,
                result: None,
            },
        );
        runnable.push_back(0);

        while let Some(task_id) = runnable.pop_front() {
            // Wake sleeping tasks that are ready.
            let now = Instant::now();
            for t in tasks.values_mut() {
                if let TaskStatus::Sleeping(until) = &t.status {
                    if *until <= now {
                        t.status = TaskStatus::Running;
                    }
                }
            }

            let can_step = match tasks.get(&task_id) {
                Some(t) => match &t.status {
                    TaskStatus::Running => true,
                    TaskStatus::Waiting(wait_id) => match tasks.get(wait_id) {
                        Some(w) => matches!(w.status, TaskStatus::Done | TaskStatus::Error(_)),
                        None => true,
                    },
                    _ => false,
                },
                None => false,
            };
            if !can_step {
                continue;
            }

            let mut task = match tasks.remove(&task_id) {
                Some(t) => t,
                None => continue,
            };
            let step_res = self.step_task(&mut task, &mut tasks, &mut next_task_id);
            tasks.insert(task_id, task);
            let outcome = step_res?;

            // If a task completed (or failed), wake any tasks waiting on it.
            if matches!(outcome, TaskStatus::Done | TaskStatus::Error(_)) {
                let mut to_wake = Vec::new();
                for (id, t) in tasks.iter_mut() {
                    if matches!(t.status, TaskStatus::Waiting(w) if w == task_id) {
                        t.status = TaskStatus::Running;
                        to_wake.push(*id);
                    }
                }
                for id in to_wake {
                    if !runnable.contains(&id) {
                        runnable.push_back(id);
                    }
                }
            }

            // If main finished, stop.
            if task_id == 0 {
                if matches!(outcome, TaskStatus::Done) {
                    self.variables = tasks.get(&0).map(|t| t.vars.clone()).unwrap_or_default();
                    return Ok(());
                }
                if let TaskStatus::Error(e) = outcome {
                    return Err(e);
                }
            }

            // Requeue runnable tasks.
            let should_requeue = match tasks.get(&task_id) {
                Some(t) => matches!(t.status, TaskStatus::Running),
                None => false,
            };
            if should_requeue {
                runnable.push_back(task_id);
            }

            // Ensure other running tasks are in the queue.
            for (id, t) in tasks.iter() {
                if *id == task_id {
                    continue;
                }
                if matches!(t.status, TaskStatus::Running) && !runnable.contains(id) {
                    runnable.push_back(*id);
                }
            }

            // If nothing runnable, but some are sleeping, wait a bit.
            if runnable.is_empty() {
                let mut next_wake: Option<Instant> = None;
                for t in tasks.values() {
                    if let TaskStatus::Sleeping(until) = &t.status {
                        next_wake = Some(next_wake.map(|n| n.min(*until)).unwrap_or(*until));
                    }
                }
                if let Some(until) = next_wake {
                    let now = Instant::now();
                    if until > now {
                        let d = until.duration_since(now).min(Duration::from_millis(10));
                        std::thread::sleep(d);
                    }
                    // Wake tasks that are now ready, then refill runnable.
                    let now = Instant::now();
                    for t in tasks.values_mut() {
                        if let TaskStatus::Sleeping(until) = &t.status {
                            if *until <= now {
                                t.status = TaskStatus::Running;
                            }
                        }
                    }
                    for (id, t) in tasks.iter() {
                        if matches!(t.status, TaskStatus::Running) && !runnable.contains(id) {
                            runnable.push_back(*id);
                        }
                    }
                } else {
                    // Deadlock: only waiting tasks remain.
                    let waiting: Vec<(u64, u64)> = tasks
                        .iter()
                        .filter_map(|(id, t)| match t.status {
                            TaskStatus::Waiting(w) => Some((*id, w)),
                            _ => None,
                        })
                        .collect();
                    if !waiting.is_empty() {
                        return Err(format!("Deadlock: tasks waiting ({} total)", waiting.len()));
                    }
                    return Ok(());
                }
            }
        }

        Ok(())
    }

    fn step_task(
        &mut self,
        task: &mut TaskState,
        tasks: &mut HashMap<u64, TaskState>,
        next_task_id: &mut u64,
    ) -> Result<TaskStatus, String> {
        // If waiting, check if awaited task is done.
        if let TaskStatus::Waiting(wait_id) = task.status {
            if let Some(waited) = tasks.get(&wait_id) {
                if matches!(waited.status, TaskStatus::Done) {
                    task.status = TaskStatus::Running;
                } else if let TaskStatus::Error(e) = &waited.status {
                    task.status =
                        TaskStatus::Error(format!("awaited task {} failed: {}", wait_id, e));
                } else {
                    return Ok(task.status.clone());
                }
            } else {
                task.status = TaskStatus::Error(format!("awaited task {} not found", wait_id));
            }
        }

        // Sleeping tasks are handled by scheduler wakeup.
        if !matches!(task.status, TaskStatus::Running) {
            return Ok(task.status.clone());
        }

        // No frame => done
        if task.frames.is_empty() {
            task.status = TaskStatus::Done;
            return Ok(TaskStatus::Done);
        }

        let frame_idx = task.frames.len() - 1;
        let pc = task.frames[frame_idx].pc;
        if pc >= task.frames[frame_idx].instructions.len() {
            // Implicit return None
            self.finish_frame(task, None)?;
            return Ok(task.status.clone());
        }

        let instr = task.frames[frame_idx].instructions[pc].clone();

        match instr {
            IrInstr::Label { .. } => {
                task.frames[frame_idx].pc += 1;
            }
            IrInstr::Jump { label } => {
                if let Some(target) = task.frames[frame_idx].labels.get(&label).copied() {
                    task.frames[frame_idx].pc = target;
                } else {
                    return Err(format!("Unknown label: {}", label));
                }
            }
            IrInstr::JumpIf { cond, label } => {
                let val = task
                    .vars
                    .get(&cond)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", cond))?;
                let should_jump = match val {
                    Value::Bool(b) => b,
                    Value::Number(n) => n != 0.0,
                    Value::String(s) => !s.is_empty(),
                    Value::List(l) => !l.borrow().is_empty(),
                    Value::Map(m) => !m.borrow().is_empty(),
                    Value::Struct(_, _) => true,
                    Value::Task(_) | Value::TaskThunk { .. } => true,
                };
                if should_jump {
                    if let Some(target) = task.frames[frame_idx].labels.get(&label).copied() {
                        task.frames[frame_idx].pc = target;
                    } else {
                        return Err(format!("Unknown label: {}", label));
                    }
                } else {
                    task.frames[frame_idx].pc += 1;
                }
            }
            IrInstr::Return { value } => {
                let ret = match value {
                    Some(v) => Some(
                        task.vars
                            .get(&v)
                            .cloned()
                            .ok_or_else(|| format!("Undefined variable: {}", v))?,
                    ),
                    None => None,
                };
                self.finish_frame(task, ret)?;
                return Ok(task.status.clone());
            }
            IrInstr::Await {
                dest,
                task: task_var,
            } => {
                let val = task
                    .vars
                    .get(&task_var)
                    .cloned()
                    .ok_or_else(|| format!("Undefined variable: {}", task_var))?;

                match val {
                    Value::Task(id) => {
                        if let Some(waited) = tasks.get(&id) {
                            if matches!(waited.status, TaskStatus::Done) {
                                let res = waited.result.clone().unwrap_or(Value::Bool(true));
                                task.vars.insert(dest, res);
                                task.frames[frame_idx].pc += 1;
                            } else if let TaskStatus::Error(e) = &waited.status {
                                task.status =
                                    TaskStatus::Error(format!("awaited task {} failed: {}", id, e));
                                return Ok(task.status.clone());
                            } else {
                                task.status = TaskStatus::Waiting(id);
                                return Ok(task.status.clone());
                            }
                        } else {
                            return Err(format!("awaited task {} not found", id));
                        }
                    }
                    Value::TaskThunk { func, args } => {
                        let id =
                            self.spawn_task_from_thunk(task, tasks, next_task_id, &func, &args)?;
                        task.vars.insert(task_var, Value::Task(id));
                        task.status = TaskStatus::Waiting(id);
                        return Ok(task.status.clone());
                    }
                    other => {
                        task.vars.insert(dest, other);
                        task.frames[frame_idx].pc += 1;
                    }
                }
            }
            IrInstr::Call { dest, func, args } => {
                // Evaluate arguments now.
                let mut arg_vals = Vec::new();
                for a in &args {
                    let v = task
                        .vars
                        .get(a)
                        .cloned()
                        .ok_or_else(|| format!("Undefined variable: {}", a))?;
                    arg_vals.push(v);
                }

                // Builtins that can affect scheduling.
                match func.as_str() {
                    "spawn" => {
                        if arg_vals.len() != 1 {
                            return Err("spawn() expects 1 argument".to_string());
                        }
                        let thunk = arg_vals.into_iter().next().unwrap();
                        let id = match thunk {
                            Value::Task(id) => id,
                            Value::TaskThunk { func, args } => {
                                self.spawn_task_from_thunk(task, tasks, next_task_id, &func, &args)?
                            }
                            _ => {
                                return Err("spawn() expects an async function call (task thunk)"
                                    .to_string())
                            }
                        };
                        if let Some(d) = dest {
                            task.vars.insert(d, Value::Task(id));
                        }
                        task.frames[frame_idx].pc += 1;
                        return Ok(task.status.clone());
                    }
                    "sleep" => {
                        if arg_vals.len() != 1 {
                            return Err("sleep() expects 1 argument (ms)".to_string());
                        }
                        let ms = match &arg_vals[0] {
                            Value::Number(n) => (*n).max(0.0) as u64,
                            Value::Bool(b) => {
                                if *b {
                                    1
                                } else {
                                    0
                                }
                            }
                            _ => return Err("sleep() expects a number".to_string()),
                        };
                        if let Some(d) = dest {
                            task.vars.insert(d, Value::Number(0.0));
                        }
                        task.frames[frame_idx].pc += 1;
                        task.status =
                            TaskStatus::Sleeping(Instant::now() + Duration::from_millis(ms));
                        return Ok(task.status.clone());
                    }
                    _ => {}
                }

                // Builtin function calls (pure)
                if [
                    "len",
                    "keys",
                    "values",
                    "range",
                    "open",
                    "close",
                    "type",
                    "is_map",
                    "is_list",
                    "is_string",
                    "str",
                    "num",
                    "bool",
                ]
                .contains(&func.as_str())
                {
                    let res = self.eval_builtin(func.as_str(), &arg_vals)?;
                    if let Some(d) = dest {
                        task.vars.insert(d, res);
                    }
                    task.frames[frame_idx].pc += 1;
                    return Ok(task.status.clone());
                }

                // User functions
                let function = self
                    .functions
                    .get(&func)
                    .ok_or_else(|| format!("Unknown function: {}", func))?
                    .clone();

                // Async function call returns a thunk (until spawned/awaited).
                if function.is_async {
                    let thunk = Value::TaskThunk {
                        func: func.clone(),
                        args: arg_vals,
                    };
                    if let Some(d) = dest {
                        task.vars.insert(d, thunk);
                    }
                    task.frames[frame_idx].pc += 1;
                    return Ok(task.status.clone());
                }

                // Synchronous call: push frame with dynamic-scope snapshot.
                if function.params.len() != args.len() {
                    return Err(format!(
                        "Function '{}' expects {} args, got {}",
                        func,
                        function.params.len(),
                        args.len()
                    ));
                }

                let saved_vars = task.vars.clone();
                for (param, val) in function.params.iter().zip(arg_vals.into_iter()) {
                    task.vars.insert(param.clone(), val);
                }

                task.frames[frame_idx].pc += 1; // advance caller before entering callee
                let callee_labels = Self::build_labels(&function.instructions);
                task.frames.push(FrameState {
                    instructions: function.instructions.clone(),
                    labels: callee_labels,
                    pc: 0,
                    return_dest: dest,
                    saved_vars,
                });
            }
            other => {
                // Regular instruction; reuse existing interpreter logic by swapping variables.
                let mut temp = HashMap::new();
                std::mem::swap(&mut temp, &mut task.vars);
                std::mem::swap(&mut temp, &mut self.variables);
                let res = self.execute_instruction(&other);
                std::mem::swap(&mut temp, &mut self.variables);
                std::mem::swap(&mut temp, &mut task.vars);
                res?;
                task.frames[frame_idx].pc += 1;
            }
        }

        Ok(task.status.clone())
    }

    fn spawn_task_from_thunk(
        &mut self,
        parent: &TaskState,
        tasks: &mut HashMap<u64, TaskState>,
        next_task_id: &mut u64,
        func: &str,
        args: &[Value],
    ) -> Result<u64, String> {
        let function = self
            .functions
            .get(func)
            .ok_or_else(|| format!("Unknown function: {}", func))?
            .clone();

        if function.params.len() != args.len() {
            return Err(format!(
                "Async function '{}' expects {} args, got {}",
                func,
                function.params.len(),
                args.len()
            ));
        }

        let id = *next_task_id;
        *next_task_id += 1;

        let mut vars = parent.vars.clone(); // capture dynamic scope snapshot
        for (param, val) in function.params.iter().zip(args.iter().cloned()) {
            vars.insert(param.clone(), val);
        }

        let labels = Self::build_labels(&function.instructions);
        let frame = FrameState {
            instructions: function.instructions.clone(),
            labels,
            pc: 0,
            return_dest: None,
            saved_vars: HashMap::new(),
        };

        tasks.insert(
            id,
            TaskState {
                id,
                vars,
                frames: vec![frame],
                status: TaskStatus::Running,
                result: None,
            },
        );

        Ok(id)
    }

    fn finish_frame(&mut self, task: &mut TaskState, value: Option<Value>) -> Result<(), String> {
        let frame = task.frames.pop().expect("frame exists");
        if let Some(parent) = task.frames.last_mut() {
            // Restore caller vars
            task.vars = frame.saved_vars;
            if let Some(dest) = frame.return_dest {
                if let Some(v) = value {
                    task.vars.insert(dest, v);
                }
            }
            // Continue execution in caller (pc already advanced before call)
            let _ = parent;
        } else {
            // Task completed
            task.result = value;
            task.status = TaskStatus::Done;
        }
        Ok(())
    }

    fn eval_builtin(&self, func: &str, arg_vals: &[Value]) -> Result<Value, String> {
        match func {
            "len" if arg_vals.len() == 1 => match &arg_vals[0] {
                Value::String(s) => Ok(Value::Number(s.len() as f64)),
                Value::List(l) => Ok(Value::Number(l.borrow().len() as f64)),
                Value::Map(m) => Ok(Value::Number(m.borrow().len() as f64)),
                _ => Err("len() expects string, list or map".to_string()),
            },
            "keys" if arg_vals.len() == 1 => {
                if let Value::Map(m) = &arg_vals[0] {
                    Ok(Value::List(Rc::new(RefCell::new(
                        m.borrow()
                            .keys()
                            .map(|k| Value::String(k.clone()))
                            .collect(),
                    ))))
                } else {
                    Err("keys() expects map".to_string())
                }
            }
            "values" if arg_vals.len() == 1 => {
                if let Value::Map(m) = &arg_vals[0] {
                    Ok(Value::List(Rc::new(RefCell::new(
                        m.borrow().values().cloned().collect(),
                    ))))
                } else {
                    Err("values() expects map".to_string())
                }
            }
            "range" if arg_vals.len() == 2 => {
                if let (Value::Number(start), Value::Number(end)) = (&arg_vals[0], &arg_vals[1]) {
                    let mut items = Vec::new();
                    let mut curr = *start;
                    while curr < *end {
                        items.push(Value::Number(curr));
                        curr += 1.0;
                    }
                    Ok(Value::List(Rc::new(RefCell::new(items))))
                } else {
                    Err("range() expects two numbers".to_string())
                }
            }
            "type" if arg_vals.len() == 1 => {
                let t = match &arg_vals[0] {
                    Value::Number(_) => "number",
                    Value::String(_) => "string",
                    Value::Bool(_) => "bool",
                    Value::List(_) => "list",
                    Value::Map(_) => "map",
                    Value::Struct(_, _) => "struct",
                    Value::Task(_) => "task",
                    Value::TaskThunk { .. } => "task",
                };
                Ok(Value::String(t.to_string()))
            }
            "is_map" if arg_vals.len() == 1 => {
                Ok(Value::Number(if matches!(&arg_vals[0], Value::Map(_)) {
                    1.0
                } else {
                    0.0
                }))
            }
            "is_list" if arg_vals.len() == 1 => {
                Ok(Value::Number(if matches!(&arg_vals[0], Value::List(_)) {
                    1.0
                } else {
                    0.0
                }))
            }
            "is_string" if arg_vals.len() == 1 => {
                Ok(Value::Number(if matches!(&arg_vals[0], Value::String(_)) {
                    1.0
                } else {
                    0.0
                }))
            }
            "str" if arg_vals.len() == 1 => Ok(Value::String(self.value_to_string(&arg_vals[0]))),
            "num" if arg_vals.len() == 1 => match &arg_vals[0] {
                Value::Number(n) => Ok(Value::Number(*n)),
                Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
                Value::String(s) => {
                    let parsed = s
                        .trim()
                        .parse::<f64>()
                        .map_err(|_| format!("num() could not parse '{}'", s))?;
                    Ok(Value::Number(parsed))
                }
                _ => Err("num() expects number, bool, or string".to_string()),
            },
            "bool" if arg_vals.len() == 1 => {
                let b = match &arg_vals[0] {
                    Value::Bool(b) => *b,
                    Value::Number(n) => *n != 0.0,
                    Value::String(s) => !s.is_empty(),
                    Value::List(l) => !l.borrow().is_empty(),
                    Value::Map(m) => !m.borrow().is_empty(),
                    Value::Struct(_, _) => true,
                    Value::Task(_) | Value::TaskThunk { .. } => true,
                };
                Ok(Value::Bool(b))
            }
            "open" if arg_vals.len() == 1 => Ok(Value::String("FILE_HANDLE".to_string())),
            "close" if arg_vals.len() == 1 => Ok(Value::Bool(true)),
            _ => Err(format!("Invalid native function call: {}", func)),
        }
    }

    fn execute_instructions(&mut self, instructions: &[IrInstr]) -> Result<Option<Value>, String> {
        // First pass: scanning labels
        let mut labels = HashMap::new();
        for (i, instr) in instructions.iter().enumerate() {
            if let IrInstr::Label { name } = instr {
                labels.insert(name.clone(), i);
            }
        }

        let mut pc = 0;
        while pc < instructions.len() {
            let instr = &instructions[pc];

            match instr {
                IrInstr::Label { .. } => {
                    pc += 1;
                }
                IrInstr::Jump { label } => {
                    if let Some(target) = labels.get(label) {
                        pc = *target;
                    } else {
                        return Err(format!("Unknown label: {}", label));
                    }
                }
                IrInstr::JumpIf { cond, label } => {
                    let val = self.get_var(cond)?;
                    let should_jump = match val {
                        Value::Bool(b) => b,
                        Value::Number(n) => n != 0.0,
                        _ => false,
                    };

                    if should_jump {
                        if let Some(target) = labels.get(label) {
                            pc = *target;
                        } else {
                            return Err(format!("Unknown label: {}", label));
                        }
                    } else {
                        pc += 1;
                    }
                }
                IrInstr::Return { value } => {
                    if let Some(var) = value {
                        let val = self.get_var(var)?;
                        return Ok(Some(val));
                    }
                    return Ok(None);
                }
                IrInstr::Call { dest, func, args }
                    if [
                        "len",
                        "keys",
                        "values",
                        "range",
                        "open",
                        "close",
                        "type",
                        "is_map",
                        "is_list",
                        "is_string",
                        "str",
                        "num",
                        "bool",
                    ]
                    .contains(&func.as_str()) =>
                {
                    // Native functions
                    let arg_vals: Vec<Value> = args
                        .iter()
                        .map(|a| self.get_var(a))
                        .collect::<Result<Vec<_>, _>>()?;
                    let result = self.eval_builtin(func.as_str(), &arg_vals)?;

                    if let Some(d) = dest {
                        self.variables.insert(d.clone(), result);
                    }
                    pc += 1;
                }
                IrInstr::Call { dest, func, args } => {
                    let function = self
                        .functions
                        .get(func)
                        .ok_or_else(|| format!("Unknown function: {}", func))?
                        .clone();

                    let saved_vars = self.variables.clone();

                    for (param, arg) in function.params.iter().zip(args.iter()) {
                        let val = self.get_var(arg)?;
                        self.variables.insert(param.clone(), val);
                    }

                    let result = self.execute_instructions(&function.instructions)?;

                    self.variables = saved_vars;

                    if let (Some(d), Some(val)) = (dest, result) {
                        self.variables.insert(d.clone(), val);
                    }
                    pc += 1;
                }
                IrInstr::Spawn { task } => {
                    // Async spawn stub
                    println!("(Async spawn not implemented in interpreter)");
                    self.variables.insert(task.clone(), Value::Bool(true));
                    pc += 1;
                }
                IrInstr::Await { dest, task: _ } => {
                    // Async await stub
                    println!("(Async await not implemented in interpreter)");
                    self.variables.insert(dest.clone(), Value::Bool(true));
                    pc += 1;
                }
                IrInstr::AllocFile { dest, path } => {
                    // File alloc stub
                    println!("(File alloc not implemented in interpreter)");
                    self.variables
                        .insert(dest.clone(), Value::String(format!("FILE:{}", path)));
                    pc += 1;
                }
                IrInstr::CloseFile { handle: _ } => {
                    // File close stub
                    println!("(File close not implemented in interpreter)");
                    pc += 1;
                }
                IrInstr::LinkFile { path } => {
                    // File linking stub
                    println!("(File linking not implemented in interpreter: {})", path);
                    pc += 1;
                }
                IrInstr::Hardwire { path } => {
                    // Hardwire update stub
                    println!("(Hardwire update not implemented in interpreter: {})", path);
                    pc += 1;
                }
                IrInstr::PreScan { target } => {
                    // Pre-scan stub
                    println!("(Pre-scan not implemented in interpreter: {})", target);
                    pc += 1;
                }
                _ => {
                    // Regular instruction
                    self.execute_instruction(instr)?;
                    pc += 1;
                }
            }
        }
        Ok(None)
    }

    fn execute_instruction(&mut self, instr: &IrInstr) -> Result<(), String> {
        match instr {
            IrInstr::LoadConst { dest, value } => {
                let val = match value {
                    IrValue::Number(n) => Value::Number(*n),
                    IrValue::String(s) => Value::String(s.clone()),
                    IrValue::Bool(b) => Value::Bool(*b),
                };
                self.variables.insert(dest.clone(), val);
            }
            IrInstr::Add { dest, left, right } => {
                let left_val = self.get_var(left)?;
                let right_val = self.get_var(right)?;

                let result = match (left_val, right_val) {
                    (Value::Number(l), Value::Number(r)) => Value::Number(l + r),
                    (Value::String(l), Value::String(r)) => Value::String(format!("{}{}", l, r)),
                    _ => return Err("Type error in addition".to_string()),
                };

                self.variables.insert(dest.clone(), result);
            }
            IrInstr::Sub { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                self.variables
                    .insert(dest.clone(), Value::Number(left_val - right_val));
            }
            IrInstr::Mul { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                self.variables
                    .insert(dest.clone(), Value::Number(left_val * right_val));
            }
            IrInstr::Div { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;

                if right_val == 0.0 {
                    return Err("Division by zero".to_string());
                }

                self.variables
                    .insert(dest.clone(), Value::Number(left_val / right_val));
            }
            IrInstr::Eq { dest, left, right } => {
                let left_val = self.get_var(left)?;
                let right_val = self.get_var(right)?;
                self.variables
                    .insert(dest.clone(), Value::Bool(left_val == right_val));
            }
            IrInstr::Ne { dest, left, right } => {
                let left_val = self.get_var(left)?;
                let right_val = self.get_var(right)?;
                self.variables
                    .insert(dest.clone(), Value::Bool(left_val != right_val));
            }
            IrInstr::Lt { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                self.variables
                    .insert(dest.clone(), Value::Bool(left_val < right_val));
            }
            IrInstr::Gt { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;

                self.variables
                    .insert(dest.clone(), Value::Bool(left_val > right_val));
            }
            IrInstr::LogicAnd { dest, left, right } => {
                let left_val = self.get_bool(left)?;
                let right_val = self.get_bool(right)?;
                self.variables
                    .insert(dest.clone(), Value::Bool(left_val && right_val));
            }
            IrInstr::LogicOr { dest, left, right } => {
                let left_val = self.get_bool(left)?;
                let right_val = self.get_bool(right)?;
                self.variables
                    .insert(dest.clone(), Value::Bool(left_val || right_val));
            }
            IrInstr::LogicNot { dest, src } => {
                let val = self.get_bool(src)?;
                self.variables.insert(dest.clone(), Value::Bool(!val));
            }
            IrInstr::Print { src } => {
                let val = self.get_var(src)?;
                self.print_value(&val);
                println!();
            }
            IrInstr::PrintNum { src } => {
                let val = self.get_var(src)?;
                self.print_value(&val);
                println!();
            }
            IrInstr::Input { dest, prompt } => {
                let prompt_str = if let Ok(val) = self.get_var(prompt) {
                    match val {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => prompt.to_string(),
                    }
                } else {
                    prompt.to_string()
                };

                print!("{}", prompt_str);
                io::stdout().flush().unwrap();

                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|e| e.to_string())?;

                let input = input.trim().to_string();
                self.variables.insert(dest.clone(), Value::String(input));
            }
            IrInstr::FAdd { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                self.variables
                    .insert(dest.clone(), Value::Number(left_val + right_val));
            }
            IrInstr::FSub { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                self.variables
                    .insert(dest.clone(), Value::Number(left_val - right_val));
            }
            IrInstr::FMul { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                self.variables
                    .insert(dest.clone(), Value::Number(left_val * right_val));
            }
            IrInstr::FDiv { dest, left, right } => {
                let left_val = self.get_number(left)?;
                let right_val = self.get_number(right)?;
                if right_val == 0.0 {
                    return Err("Division by zero".to_string());
                }
                self.variables
                    .insert(dest.clone(), Value::Number(left_val / right_val));
            }
            IrInstr::AllocStruct { dest, name } => {
                self.variables.insert(
                    dest.clone(),
                    Value::Struct(name.clone(), Rc::new(RefCell::new(HashMap::new()))),
                );
            }
            IrInstr::SetMember { obj, member, value } => {
                let obj_val = self.get_var(obj)?;
                let val = self.get_var(value)?;
                if let Value::Struct(_, fields_ref) = obj_val {
                    fields_ref.borrow_mut().insert(member.clone(), val);
                } else {
                    return Err("SetMember on non-struct type".to_string());
                }
            }
            IrInstr::GetMember { dest, obj, member } => {
                let obj_val = self.get_var(obj)?;
                if let Value::Struct(_, fields_ref) = obj_val {
                    let fields = fields_ref.borrow();
                    if let Some(val) = fields.get(member) {
                        self.variables.insert(dest.clone(), val.clone());
                    } else {
                        return Err(format!("Member '{}' not found in struct", member));
                    }
                } else {
                    return Err("GetMember on non-struct type".to_string());
                }
            }
            IrInstr::Move { dest, src } => {
                let val = self.get_var(src)?;
                self.variables.insert(dest.clone(), val);
            }
            IrInstr::BitAnd { dest, left, right } => {
                let l = self.get_number(left)? as i64;
                let r = self.get_number(right)? as i64;
                self.variables
                    .insert(dest.clone(), Value::Number((l & r) as f64));
            }
            IrInstr::BitOr { dest, left, right } => {
                let l = self.get_number(left)? as i64;
                let r = self.get_number(right)? as i64;
                self.variables
                    .insert(dest.clone(), Value::Number((l | r) as f64));
            }
            IrInstr::BitXor { dest, left, right } => {
                let l = self.get_number(left)? as i64;
                let r = self.get_number(right)? as i64;
                self.variables
                    .insert(dest.clone(), Value::Number((l ^ r) as f64));
            }
            IrInstr::BitNot { dest, src } => {
                let s = self.get_number(src)? as i64;
                self.variables
                    .insert(dest.clone(), Value::Number((!s) as f64));
            }
            IrInstr::Shl { dest, left, right } => {
                let l = self.get_number(left)? as i64;
                let r = self.get_number(right)? as i64;
                self.variables
                    .insert(dest.clone(), Value::Number((l << r) as f64));
            }
            IrInstr::Shr { dest, left, right } => {
                let l = self.get_number(left)? as i64;
                let r = self.get_number(right)? as i64;
                self.variables
                    .insert(dest.clone(), Value::Number((l >> r) as f64));
            }

            IrInstr::AllocList { dest, items } => {
                let mut list_val = Vec::new();
                for item_name in items {
                    let val = self.get_var(item_name)?;
                    list_val.push(val);
                }
                self.variables
                    .insert(dest.clone(), Value::List(Rc::new(RefCell::new(list_val))));
            }
            IrInstr::AllocMap { dest } => {
                self.variables.insert(
                    dest.clone(),
                    Value::Map(Rc::new(RefCell::new(HashMap::new()))),
                );
            }
            IrInstr::SetMap { map, key, value } => {
                let map_val = self.get_var(map)?;
                let key_val = self.get_var(key)?;
                let value_val = self.get_var(value)?;

                let key_str = match key_val {
                    Value::String(s) => s,
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => return Err("Map key must be string, number or bool".to_string()),
                };

                if let Value::Map(map_ref) = map_val {
                    map_ref.borrow_mut().insert(key_str, value_val);
                } else {
                    return Err(format!("Variable '{}' is not a map", map));
                }
            }
            // Temporarily mapped GetIndex to handle both
            IrInstr::GetIndex { dest, src, index } => {
                let src_val = self.get_var(src)?;
                let idx_val_raw = self.get_var(index)?;

                if let Value::List(items_ref) = src_val {
                    let items = items_ref.borrow();
                    let idx = match idx_val_raw {
                        Value::Number(n) => n as usize,
                        _ => return Err("List index must be a number".to_string()),
                    };
                    if idx < items.len() {
                        self.variables.insert(dest.clone(), items[idx].clone());
                    } else {
                        return Err(format!(
                            "Index out of bounds: {} (len {})",
                            idx,
                            items.len()
                        ));
                    }
                } else if let Value::Map(map_ref) = src_val {
                    let key = match idx_val_raw {
                        Value::String(s) => s,
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => b.to_string(),
                        _ => return Err("Map key index must be string, number or bool".to_string()),
                    };

                    let map = map_ref.borrow();
                    if let Some(val) = map.get(&key) {
                        self.variables.insert(dest.clone(), val.clone());
                    } else {
                        return Err(format!("Key '{}' not found in map", key));
                    }
                } else {
                    return Err(format!("Indexing non-list/non-map type"));
                }
            }
            IrInstr::SetIndex { src, index, value } => {
                let target_val = self.get_var(src)?;
                let idx_val_raw = self.get_var(index)?;
                let val_to_set = self.get_var(value)?;

                match target_val {
                    Value::List(items_ref) => {
                        if let Value::Number(n) = idx_val_raw {
                            let idx = n as usize;
                            let mut items = items_ref.borrow_mut();
                            if idx < items.len() {
                                items[idx] = val_to_set;
                            } else {
                                return Err(format!(
                                    "Index out of bounds: {} (len {})",
                                    idx,
                                    items.len()
                                ));
                            }
                        } else {
                            return Err("List index must be a number".to_string());
                        }
                    }
                    Value::Map(map_ref) => {
                        let key = match idx_val_raw {
                            Value::String(s) => s,
                            Value::Number(n) => n.to_string(),
                            Value::Bool(b) => b.to_string(),
                            _ => return Err("Map key must be string, number or bool".to_string()),
                        };
                        map_ref.borrow_mut().insert(key, val_to_set);
                    }
                    _ => return Err("SetIndex on non-list/non-map type".to_string()),
                }
            }
            _ => {
                // Placeholder for other instructions
            }
        }

        Ok(())
    }

    fn print_value(&self, val: &Value) {
        match val {
            Value::Number(n) => print!("{}", n),
            Value::String(s) => print!("{}", s),
            Value::Bool(b) => print!("{}", b),
            Value::List(items_ref) => {
                let items = items_ref.borrow();
                print!("[");
                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    self.print_value(item);
                }
                print!("]");
            }
            Value::Map(map_ref) => {
                let map = map_ref.borrow();
                print!("{{");
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("\"{}\": ", k);
                    self.print_value(v);
                }
                print!("}}");
            }
            Value::Struct(name, fields_ref) => {
                let fields = fields_ref.borrow();
                print!("{} {{", name);
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        print!(", ");
                    }
                    print!("{}: ", k);
                    self.print_value(v);
                }
                print!("}}");
            }
            Value::Task(id) => {
                print!("<task {}>", id);
            }
            Value::TaskThunk { func, args } => {
                let arg_s = args
                    .iter()
                    .map(|v| self.value_to_string(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                print!("<thunk {}({})>", func, arg_s);
            }
        }
    }

    fn value_to_string(&self, val: &Value) -> String {
        match val {
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::List(items_ref) => {
                let items = items_ref.borrow();
                let parts = items
                    .iter()
                    .map(|v| self.value_to_string(v))
                    .collect::<Vec<_>>();
                format!("[{}]", parts.join(", "))
            }
            Value::Map(map_ref) => {
                let map = map_ref.borrow();
                let mut keys = map.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                let parts = keys
                    .into_iter()
                    .map(|k| {
                        let v = map.get(&k).expect("key exists");
                        format!("\"{}\": {}", k, self.value_to_string(v))
                    })
                    .collect::<Vec<_>>();
                format!("{{{}}}", parts.join(", "))
            }
            Value::Struct(name, fields_ref) => {
                let fields = fields_ref.borrow();
                let mut keys = fields.keys().cloned().collect::<Vec<_>>();
                keys.sort();
                let parts = keys
                    .into_iter()
                    .map(|k| {
                        let v = fields.get(&k).expect("key exists");
                        format!("{}: {}", k, self.value_to_string(v))
                    })
                    .collect::<Vec<_>>();
                format!("{} {{{}}}", name, parts.join(", "))
            }
            Value::Task(id) => format!("<task {}>", id),
            Value::TaskThunk { func, args } => {
                let arg_s = args
                    .iter()
                    .map(|v| self.value_to_string(v))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("<thunk {}({})>", func, arg_s)
            }
        }
    }

    fn get_var(&self, name: &str) -> Result<Value, String> {
        self.variables
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Undefined variable: {}", name))
    }

    fn get_number(&self, name: &str) -> Result<f64, String> {
        match self.get_var(name)? {
            Value::Number(n) => Ok(n),
            _ => Err(format!(
                "Expected number, got different type for '{}'",
                name
            )),
        }
    }

    fn get_bool(&self, name: &str) -> Result<bool, String> {
        match self.get_var(name)? {
            Value::Bool(b) => Ok(b),
            _ => Err(format!(
                "Expected boolean, got different type for '{}'",
                name
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;

    fn exec_source(source: &str) -> DirectExecutor {
        let tokens: Vec<(crate::lexer::Token, std::ops::Range<usize>)> = Lexer::new(source)
            .map(|(t, span)| (t.expect("tokenize"), span))
            .collect();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("parse");
        let mut ir_builder = crate::ir::IrBuilder::new();
        let ir_program = ir_builder.build(&program, None).expect("ir build");

        let mut ex = DirectExecutor::new();
        ex.execute(&ir_program).expect("execute");
        ex
    }

    #[test]
    fn await_on_async_call_runs_and_returns_value() {
        let ex = exec_source(
            r#"
async fn add_async: a, b {
    return a + b
}

var r: await add_async: 2, 3
"#,
        );

        assert_eq!(ex.variables.get("r"), Some(&Value::Number(5.0)));
    }

    #[test]
    fn spawn_then_await_runs_task_and_returns_value() {
        let ex = exec_source(
            r#"
async fn add_async: a, b {
    return a + b
}

var t: spawn: add_async: 10, 20
var r: await t
"#,
        );

        assert_eq!(ex.variables.get("r"), Some(&Value::Number(30.0)));
    }
}
