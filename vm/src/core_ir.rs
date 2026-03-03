use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Null,
    Number(f64),
    Bool(bool),
    String(Rc<String>),
    List(Rc<RefCell<Vec<Value>>>),
    Map(Rc<RefCell<HashMap<String, Value>>>),
}

impl Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0,
            Value::String(s) => !s.is_empty(),
            Value::List(l) => !l.borrow().is_empty(),
            Value::Map(m) => !m.borrow().is_empty(),
        }
    }
}

#[derive(Clone, Debug)]
enum Operand {
    Local(u16),
    Const(u16),
}

#[derive(Clone, Copy, Debug)]
enum Builtin {
    Len,
    Keys,
    Values,
    Range,
}

#[derive(Clone, Debug)]
enum Callee {
    User(u16),
    Builtin(Builtin),
}

#[derive(Clone, Debug)]
enum Op {
    LoadConst { dst: u16, k: u16 },
    Mov { dst: u16, src: Operand },
    Add { dst: u16, a: Operand, b: Operand },
    Sub { dst: u16, a: Operand, b: Operand },
    Mul { dst: u16, a: Operand, b: Operand },
    Div { dst: u16, a: Operand, b: Operand },
    Eq { dst: u16, a: Operand, b: Operand },
    Ne { dst: u16, a: Operand, b: Operand },
    Lt { dst: u16, a: Operand, b: Operand },
    Gt { dst: u16, a: Operand, b: Operand },
    And { dst: u16, a: Operand, b: Operand },
    Or { dst: u16, a: Operand, b: Operand },
    Not { dst: u16, a: Operand },
    Label { name: String },
    Jmp { label: String },
    Jnz { cond: Operand, label: String },
    Jz { cond: Operand, label: String },
    Call { dst: Option<u16>, callee: Callee, args: Vec<Operand> },
    Ret { value: Option<Operand> },
    Print { value: Operand },
    ListNew { dst: u16, items: Vec<Operand> },
    MapNew { dst: u16 },
    MapSet { map: Operand, key: Operand, value: Operand },
    MapGet { dst: u16, map: Operand, key: Operand },
    IdxGet { dst: u16, list: Operand, index: Operand },
    IdxSet { list: Operand, index: Operand, value: Operand },
}

#[derive(Clone, Debug)]
struct Instr {
    op: Op,
    src_line: usize,
}

#[derive(Clone, Debug)]
struct Function {
    name: String,
    params: Vec<String>,
    locals: HashMap<String, u16>,
    code: Vec<Instr>,
    labels: HashMap<String, usize>,
    local_count: usize,
}

#[derive(Clone, Debug)]
pub struct Program {
    functions: Vec<Function>,
    func_index: HashMap<String, u16>,
    consts: Vec<Value>,
}

#[derive(Debug)]
pub struct Engine<'a, W: Write> {
    program: &'a Program,
    output: &'a mut W,
}

#[derive(Clone, Debug)]
struct Frame {
    func: u16,
    pc: usize,
    locals: Vec<Value>,
    ret_to: Option<(usize, Option<u16>)>,
}

impl<'a, W: Write> Engine<'a, W> {
    pub fn new(program: &'a Program, output: &'a mut W) -> Self {
        Engine { program, output }
    }

    pub fn run(&mut self, entry: &str) -> Result<(), String> {
        let entry_idx = *self
            .program
            .func_index
            .get(entry)
            .ok_or_else(|| format!("Entry function '{}' not found", entry))?;

        let mut frames: Vec<Frame> = Vec::new();
        let entry_func = &self.program.functions[entry_idx as usize];
        frames.push(Frame {
            func: entry_idx,
            pc: 0,
            locals: vec![Value::Null; entry_func.local_count],
            ret_to: None,
        });

        while let Some(frame) = frames.last_mut() {
            let func = &self.program.functions[frame.func as usize];
            if frame.pc >= func.code.len() {
                // Implicit return null
                let ret_value = Value::Null;
                self.return_from(&mut frames, ret_value)?;
                continue;
            }

            let instr = func.code[frame.pc].clone();
            match &instr.op {
                Op::LoadConst { dst, k } => {
                    let v = self.program.consts[*k as usize].clone();
                    set_local(frame, *dst, v);
                    frame.pc += 1;
                }
                Op::Mov { dst, src } => {
                    let v = self.eval_operand(frame, src)?;
                    set_local(frame, *dst, v);
                    frame.pc += 1;
                }
                Op::Add { dst, a, b } => {
                    let va = self.eval_operand(frame, a)?;
                    let vb = self.eval_operand(frame, b)?;
                    set_local(frame, *dst, add_values(va, vb)?);
                    frame.pc += 1;
                }
                Op::Sub { dst, a, b } => {
                    let va = number(self.eval_operand(frame, a)?)?;
                    let vb = number(self.eval_operand(frame, b)?)?;
                    set_local(frame, *dst, Value::Number(va - vb));
                    frame.pc += 1;
                }
                Op::Mul { dst, a, b } => {
                    let va = number(self.eval_operand(frame, a)?)?;
                    let vb = number(self.eval_operand(frame, b)?)?;
                    set_local(frame, *dst, Value::Number(va * vb));
                    frame.pc += 1;
                }
                Op::Div { dst, a, b } => {
                    let va = number(self.eval_operand(frame, a)?)?;
                    let vb = number(self.eval_operand(frame, b)?)?;
                    if vb == 0.0 {
                        return self.err(&frames, &instr, "Division by zero");
                    }
                    set_local(frame, *dst, Value::Number(va / vb));
                    frame.pc += 1;
                }
                Op::Eq { dst, a, b } => {
                    let va = self.eval_operand(frame, a)?;
                    let vb = self.eval_operand(frame, b)?;
                    set_local(frame, *dst, Value::Bool(eq_value(&va, &vb)));
                    frame.pc += 1;
                }
                Op::Ne { dst, a, b } => {
                    let va = self.eval_operand(frame, a)?;
                    let vb = self.eval_operand(frame, b)?;
                    set_local(frame, *dst, Value::Bool(!eq_value(&va, &vb)));
                    frame.pc += 1;
                }
                Op::Lt { dst, a, b } => {
                    let va = number(self.eval_operand(frame, a)?)?;
                    let vb = number(self.eval_operand(frame, b)?)?;
                    set_local(frame, *dst, Value::Bool(va < vb));
                    frame.pc += 1;
                }
                Op::Gt { dst, a, b } => {
                    let va = number(self.eval_operand(frame, a)?)?;
                    let vb = number(self.eval_operand(frame, b)?)?;
                    set_local(frame, *dst, Value::Bool(va > vb));
                    frame.pc += 1;
                }
                Op::And { dst, a, b } => {
                    let va = self.eval_operand(frame, a)?;
                    let vb = self.eval_operand(frame, b)?;
                    set_local(frame, *dst, Value::Bool(va.truthy() && vb.truthy()));
                    frame.pc += 1;
                }
                Op::Or { dst, a, b } => {
                    let va = self.eval_operand(frame, a)?;
                    let vb = self.eval_operand(frame, b)?;
                    set_local(frame, *dst, Value::Bool(va.truthy() || vb.truthy()));
                    frame.pc += 1;
                }
                Op::Not { dst, a } => {
                    let va = self.eval_operand(frame, a)?;
                    set_local(frame, *dst, Value::Bool(!va.truthy()));
                    frame.pc += 1;
                }
                Op::Label { .. } => {
                    frame.pc += 1;
                }
                Op::Jmp { label } => {
                    frame.pc = *func
                        .labels
                        .get(label)
                        .ok_or_else(|| format!("Unknown label '{}'", label))?;
                }
                Op::Jnz { cond, label } => {
                    let v = self.eval_operand(frame, cond)?;
                    if v.truthy() {
                        frame.pc = *func
                            .labels
                            .get(label)
                            .ok_or_else(|| format!("Unknown label '{}'", label))?;
                    } else {
                        frame.pc += 1;
                    }
                }
                Op::Jz { cond, label } => {
                    let v = self.eval_operand(frame, cond)?;
                    if !v.truthy() {
                        frame.pc = *func
                            .labels
                            .get(label)
                            .ok_or_else(|| format!("Unknown label '{}'", label))?;
                    } else {
                        frame.pc += 1;
                    }
                }
                Op::Print { value } => {
                    let v = self.eval_operand(frame, value)?;
                    writeln!(self.output, "{}", display_value(&v, 0)).map_err(|e| e.to_string())?;
                    frame.pc += 1;
                }
                Op::ListNew { dst, items } => {
                    let mut vec = Vec::with_capacity(items.len());
                    for it in items {
                        vec.push(self.eval_operand(frame, it)?);
                    }
                    set_local(frame, *dst, Value::List(Rc::new(RefCell::new(vec))));
                    frame.pc += 1;
                }
                Op::MapNew { dst } => {
                    set_local(frame, *dst, Value::Map(Rc::new(RefCell::new(HashMap::new()))));
                    frame.pc += 1;
                }
                Op::MapSet { map, key, value } => {
                    let m = self.eval_operand(frame, map)?;
                    let k = self.eval_operand(frame, key)?;
                    let v = self.eval_operand(frame, value)?;
                    let Value::Map(m) = m else {
                        return self.err(&frames, &instr, "mset expects a map");
                    };
                    let key_s = key_string(k)?;
                    m.borrow_mut().insert(key_s, v);
                    frame.pc += 1;
                }
                Op::MapGet { dst, map, key } => {
                    let m = self.eval_operand(frame, map)?;
                    let k = self.eval_operand(frame, key)?;
                    let Value::Map(m) = m else {
                        return self.err(&frames, &instr, "mget expects a map");
                    };
                    let key_s = key_string(k)?;
                    let got = m.borrow().get(&key_s).cloned().unwrap_or(Value::Null);
                    set_local(frame, *dst, got);
                    frame.pc += 1;
                }
                Op::IdxGet { dst, list, index } => {
                    let l = self.eval_operand(frame, list)?;
                    let i = number(self.eval_operand(frame, index)?)? as isize;
                    let Value::List(l) = l else {
                        return self.err(&frames, &instr, "idxget expects a list");
                    };
                    let borrow = l.borrow();
                    let idx = if i < 0 { (borrow.len() as isize) + i } else { i } as usize;
                    let got = borrow.get(idx).cloned().unwrap_or(Value::Null);
                    set_local(frame, *dst, got);
                    frame.pc += 1;
                }
                Op::IdxSet { list, index, value } => {
                    let l = self.eval_operand(frame, list)?;
                    let i = number(self.eval_operand(frame, index)?)? as isize;
                    let v = self.eval_operand(frame, value)?;
                    let Value::List(l) = l else {
                        return self.err(&frames, &instr, "idxset expects a list");
                    };
                    let mut borrow = l.borrow_mut();
                    let idx = if i < 0 { (borrow.len() as isize) + i } else { i } as usize;
                    if idx >= borrow.len() {
                        return self.err(&frames, &instr, "idxset index out of bounds");
                    }
                    borrow[idx] = v;
                    frame.pc += 1;
                }
                Op::Call { dst, callee, args } => {
                    // Advance caller pc before the call.
                    let caller_index = frames.len() - 1;
                    frames[caller_index].pc += 1;

                    let arg_vals = args
                        .iter()
                        .map(|a| self.eval_operand(&frames[caller_index], a))
                        .collect::<Result<Vec<_>, _>>()?;

                    match callee {
                        Callee::Builtin(b) => {
                            let result = exec_builtin(*b, &arg_vals)
                                .map_err(|e| format!("Builtin error: {}", e))?;
                            if let Some(d) = *dst {
                                set_local(&mut frames[caller_index], d, result);
                            }
                        }
                        Callee::User(fidx) => {
                            let f = &self.program.functions[*fidx as usize];
                            if arg_vals.len() != f.params.len() {
                                return Err(format!(
                                    "Call arity mismatch for '{}': expected {}, got {}",
                                    f.name,
                                    f.params.len(),
                                    arg_vals.len()
                                ));
                            }
                            let mut locals = vec![Value::Null; f.local_count];
                            for (i, v) in arg_vals.into_iter().enumerate() {
                                locals[i] = v;
                            }
                            frames.push(Frame {
                                func: *fidx,
                                pc: 0,
                                locals,
                                ret_to: Some((caller_index, *dst)),
                            });
                        }
                    }
                }
                Op::Ret { value } => {
                    let v = match value {
                        Some(op) => self.eval_operand(frame, op)?,
                        None => Value::Null,
                    };
                    self.return_from(&mut frames, v)?;
                }
            }
        }

        Ok(())
    }

    fn return_from(&mut self, frames: &mut Vec<Frame>, value: Value) -> Result<(), String> {
        let finished = frames.pop().expect("frame");
        if let Some((caller_index, dst)) = finished.ret_to {
            if let Some(d) = dst {
                set_local(&mut frames[caller_index], d, value);
            }
            Ok(())
        } else {
            // returned from entry
            Ok(())
        }
    }

    fn eval_operand(&self, frame: &Frame, op: &Operand) -> Result<Value, String> {
        Ok(match op {
            Operand::Local(i) => frame.locals[*i as usize].clone(),
            Operand::Const(k) => self.program.consts[*k as usize].clone(),
        })
    }

    fn err<T>(&self, frames: &[Frame], instr: &Instr, msg: &str) -> Result<T, String> {
        let mut stack = Vec::new();
        for f in frames.iter().rev() {
            let func = &self.program.functions[f.func as usize];
            stack.push(format!("{} (pc={}, line={})", func.name, f.pc, instr.src_line));
        }
        Err(format!("{}\nStack:\n  {}", msg, stack.join("\n  ")))
    }
}

fn set_local(frame: &mut Frame, idx: u16, value: Value) {
    frame.locals[idx as usize] = value;
}

fn number(v: Value) -> Result<f64, String> {
    match v {
        Value::Number(n) => Ok(n),
        Value::Bool(b) => Ok(if b { 1.0 } else { 0.0 }),
        _ => Err("Expected number".to_string()),
    }
}

fn add_values(a: Value, b: Value) -> Result<Value, String> {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => Ok(Value::Number(x + y)),
        (Value::String(x), Value::String(y)) => Ok(Value::String(Rc::new(format!("{}{}", x, y)))),
        (Value::String(x), v) => Ok(Value::String(Rc::new(format!("{}{}", x, display_value(&v, 0))))),
        (v, Value::String(y)) => Ok(Value::String(Rc::new(format!("{}{}", display_value(&v, 0), y)))),
        _ => Err("Type error in add".to_string()),
    }
}

fn eq_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        _ => false,
    }
}

fn key_string(v: Value) -> Result<String, String> {
    match v {
        Value::String(s) => Ok((*s).clone()),
        Value::Number(n) => Ok(format!("{}", n)),
        Value::Bool(b) => Ok(if b { "true".to_string() } else { "false".to_string() }),
        Value::Null => Ok("null".to_string()),
        _ => Err("Map key must be string/number/bool/null".to_string()),
    }
}

fn display_value(v: &Value, depth: usize) -> String {
    if depth > 24 {
        return "<max-depth>".to_string();
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Number(n) => {
            if n.fract() == 0.0 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Value::Bool(b) => format!("{}", b),
        Value::String(s) => (**s).clone(),
        Value::List(l) => {
            let items = l
                .borrow()
                .iter()
                .map(|x| display_value(x, depth + 1))
                .collect::<Vec<_>>();
            format!("[{}]", items.join(", "))
        }
        Value::Map(m) => {
            let mut parts = Vec::new();
            for (k, v) in m.borrow().iter() {
                parts.push(format!("{}: {}", k, display_value(v, depth + 1)));
            }
            format!("{{{}}}", parts.join(", "))
        }
    }
}

fn exec_builtin(b: Builtin, args: &[Value]) -> Result<Value, String> {
    match b {
        Builtin::Len => {
            if args.len() != 1 {
                return Err("len expects 1 arg".to_string());
            }
            Ok(Value::Number(match &args[0] {
                Value::String(s) => s.len() as f64,
                Value::List(l) => l.borrow().len() as f64,
                Value::Map(m) => m.borrow().len() as f64,
                _ => 0.0,
            }))
        }
        Builtin::Keys => {
            if args.len() != 1 {
                return Err("keys expects 1 arg".to_string());
            }
            let Value::Map(m) = &args[0] else {
                return Err("keys expects map".to_string());
            };
            let mut keys = m
                .borrow()
                .keys()
                .cloned()
                .map(|k| Value::String(Rc::new(k)))
                .collect::<Vec<_>>();
            keys.sort_by(|a, b| display_value(a, 0).cmp(&display_value(b, 0)));
            Ok(Value::List(Rc::new(RefCell::new(keys))))
        }
        Builtin::Values => {
            if args.len() != 1 {
                return Err("values expects 1 arg".to_string());
            }
            let Value::Map(m) = &args[0] else {
                return Err("values expects map".to_string());
            };
            Ok(Value::List(Rc::new(RefCell::new(
                m.borrow().values().cloned().collect(),
            ))))
        }
        Builtin::Range => {
            if args.len() != 2 {
                return Err("range expects 2 args".to_string());
            }
            let start = number(args[0].clone())? as i64;
            let end = number(args[1].clone())? as i64;
            let mut out = Vec::new();
            let mut i = start;
            while i < end {
                out.push(Value::Number(i as f64));
                i += 1;
            }
            Ok(Value::List(Rc::new(RefCell::new(out))))
        }
    }
}

#[derive(Clone, Debug)]
struct FnSrc {
    name: String,
    params: Vec<String>,
    actions: Vec<(usize, String)>,
}

pub fn parse_and_compile(text: &str) -> Result<Program, String> {
    let funcs = parse_source(text)?;
    compile(funcs)
}

fn parse_source(text: &str) -> Result<Vec<FnSrc>, String> {
    let lines: Vec<(usize, String)> = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect();

    // If we see any "fn name.actions:" / "fn name.params:" we use section mode.
    let section_mode = lines.iter().any(|(_, l)| {
        let t = l.trim_start();
        t.starts_with("fn ")
            && (t.contains(".actions:") || t.contains(".params:") || t.contains(".params :") || t.contains(".actions :"))
    });

    if section_mode {
        parse_section_mode(&lines)
    } else {
        parse_brace_mode(&lines)
    }
}

fn strip_comment(line: &str) -> &str {
    let mut in_str = false;
    let mut prev = '\0';
    for (i, ch) in line.char_indices() {
        if ch == '"' && prev != '\\' {
            in_str = !in_str;
        }
        if !in_str {
            let rest = &line[i..];
            if rest.starts_with("//") || rest.starts_with('#') {
                return &line[..i];
            }
        }
        prev = ch;
    }
    line
}

fn parse_section_mode(lines: &[(usize, String)]) -> Result<Vec<FnSrc>, String> {
    let mut params: HashMap<String, Vec<String>> = HashMap::new();
    let mut actions: HashMap<String, Vec<(usize, String)>> = HashMap::new();

    let mut i = 0usize;
    while i < lines.len() {
        let (ln, raw) = &lines[i];
        let line = strip_comment(raw).trim_end();
        if line.trim().is_empty() {
            i += 1;
            continue;
        }

        let t = line.trim_start();
        if !t.starts_with("fn ") {
            return Err(format!(
                "Unexpected line {} in section IR (expected 'fn ...'): {}",
                ln,
                raw
            ));
        }

        let rest = t.trim_start_matches("fn").trim();
        let Some((head, tail)) = rest.split_once(':') else {
            return Err(format!("Invalid fn section header at line {}: {}", ln, raw));
        };
        let head = head.trim();
        let tail = tail.trim();

        let (fname, section) = if let Some((n, s)) = head.split_once('.') {
            (n.trim().to_string(), s.trim().to_string())
        } else {
            return Err(format!(
                "Section mode requires 'fn name.params:' or 'fn name.actions:' at line {}",
                ln
            ));
        };

        match section.as_str() {
            "params" => {
                let list = tail;
                let p = if list.is_empty() {
                    Vec::new()
                } else {
                    list.split(',')
                        .map(|x| x.trim().to_string())
                        .filter(|x| !x.is_empty())
                        .collect()
                };
                params.insert(fname, p);
                i += 1;
            }
            "actions" => {
                let mut block = Vec::new();
                i += 1;
                while i < lines.len() {
                    let (aln, araw) = &lines[i];
                    if araw.trim().is_empty() {
                        i += 1;
                        continue;
                    }
                    let starts_fn = araw.trim_start().starts_with("fn ");
                    let indented = araw.starts_with(' ') || araw.starts_with('\t');
                    if starts_fn && !indented {
                        break;
                    }
                    if indented {
                        let inner = strip_comment(araw).trim();
                        if !inner.is_empty() {
                            block.push((*aln, inner.to_string()));
                        }
                    } else {
                        return Err(format!(
                            "Expected indented action line at {}, got: {}",
                            aln, araw
                        ));
                    }
                    i += 1;
                }
                actions.insert(fname, block);
            }
            _ => {
                return Err(format!(
                    "Unknown section '{}' at line {} (expected params/actions)",
                    section, ln
                ))
            }
        }
    }

    let mut out = Vec::new();
    for (name, acts) in actions {
        let p = params.remove(&name).unwrap_or_default();
        out.push(FnSrc {
            name,
            params: p,
            actions: acts,
        });
    }

    if out.is_empty() {
        return Err("No functions found in IR".to_string());
    }
    Ok(out)
}

fn parse_brace_mode(lines: &[(usize, String)]) -> Result<Vec<FnSrc>, String> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < lines.len() {
        let (ln, raw) = &lines[i];
        let line = strip_comment(raw).trim();
        if line.is_empty() {
            i += 1;
            continue;
        }
        if !line.starts_with("fn ") {
            return Err(format!(
                "Unexpected line {} in brace IR (expected 'fn ...'): {}",
                ln, raw
            ));
        }

        // fn name: a, b {
        // fn name {
        let header = line.trim_start_matches("fn").trim();
        let (before_brace, has_brace) = if let Some(pos) = header.find('{') {
            (&header[..pos], true)
        } else {
            (header, false)
        };

        let before_brace = before_brace.trim();
        let (name, params) = if let Some((n, p)) = before_brace.split_once(':') {
            let ps = p
                .split(',')
                .map(|x| x.trim().to_string())
                .filter(|x| !x.is_empty())
                .collect::<Vec<_>>();
            (n.trim().to_string(), ps)
        } else {
            (before_brace.to_string(), Vec::new())
        };

        if name.is_empty() {
            return Err(format!("Invalid function name at line {}", ln));
        }

        // Move to body start
        if !has_brace {
            i += 1;
            while i < lines.len() && strip_comment(&lines[i].1).trim().is_empty() {
                i += 1;
            }
            if i >= lines.len() || strip_comment(&lines[i].1).trim() != "{" {
                return Err(format!("Expected '{{' after fn header at line {}", ln));
            }
        }

        // Consume possible '{' line if it was separate
        if !has_brace {
            i += 1;
        } else {
            i += 1;
        }

        let mut body = Vec::new();
        while i < lines.len() {
            let (bln, braw) = &lines[i];
            let bline = strip_comment(braw).trim();
            if bline == "}" {
                break;
            }
            if !bline.is_empty() {
                body.push((*bln, bline.to_string()));
            }
            i += 1;
        }
        if i >= lines.len() {
            return Err(format!("Unclosed function body for '{}' starting at line {}", name, ln));
        }
        // consume '}'
        i += 1;

        out.push(FnSrc {
            name,
            params,
            actions: body,
        });
    }

    if out.is_empty() {
        return Err("No functions found in IR".to_string());
    }
    Ok(out)
}

fn compile(funcs: Vec<FnSrc>) -> Result<Program, String> {
    let mut program = Program {
        functions: Vec::new(),
        func_index: HashMap::new(),
        consts: Vec::new(),
    };

    // Pre-register functions
    for (i, f) in funcs.iter().enumerate() {
        let idx = i as u16;
        if program.func_index.contains_key(&f.name) {
            return Err(format!("Duplicate function '{}'", f.name));
        }
        program.func_index.insert(f.name.clone(), idx);
    }

    // Compile each function
    for f in funcs {
        let mut locals = HashMap::new();
        for (i, p) in f.params.iter().enumerate() {
            locals.insert(p.clone(), i as u16);
        }

        let mut code: Vec<Instr> = Vec::new();

        for (ln, line) in f.actions {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Support "name:" as label sugar
            if line.ends_with(':') && !line.contains(' ') && !line.contains('\t') {
                let label = line.trim_end_matches(':').to_string();
                code.push(Instr {
                    op: Op::Label { name: label },
                    src_line: ln,
                });
                continue;
            }

            let parts = split_tokens(line)?;
            if parts.is_empty() {
                continue;
            }
            let op = parts[0].as_str();
            let instr = match op {
                "say:" | "say" => {
                    if parts.len() != 2 {
                        return Err(format!("say expects 1 arg at line {}", ln));
                    }
                    let value = operand(&mut locals, &mut program.consts, &parts[1])?;
                    Instr {
                        op: Op::Print { value },
                        src_line: ln,
                    }
                }
                "return" => {
                    let value = if parts.len() == 1 {
                        None
                    } else if parts.len() == 2 {
                        Some(operand(&mut locals, &mut program.consts, &parts[1])?)
                    } else {
                        return Err(format!("return expects 0-1 args at line {}", ln));
                    };
                    Instr {
                        op: Op::Ret { value },
                        src_line: ln,
                    }
                }
                "var" => {
                    if parts.len() != 3 {
                        return Err(format!("var expects 'var name: value' at line {}", ln));
                    }
                    let mut name = parts[1].as_str();
                    if let Some(stripped) = name.strip_suffix(':') {
                        name = stripped;
                    } else {
                        return Err(format!("var expects ':' after name at line {}", ln));
                    }
                    let dst = local_index(&mut locals, name)?;
                    if let Ok(lit) = parse_literal(&parts[2]) {
                        let k = const_index(&mut program.consts, lit);
                        Instr {
                            op: Op::LoadConst { dst, k },
                            src_line: ln,
                        }
                    } else {
                        let src = operand(&mut locals, &mut program.consts, &parts[2])?;
                        Instr {
                            op: Op::Mov { dst, src },
                            src_line: ln,
                        }
                    }
                }
                "const" => {
                    if parts.len() < 3 {
                        return Err(format!("const expects 2+ args at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let lit = parse_literal(&parts[2])?;
                    let k = const_index(&mut program.consts, lit);
                    Instr {
                        op: Op::LoadConst { dst, k },
                        src_line: ln,
                    }
                }
                "mov" => {
                    if parts.len() != 3 {
                        return Err(format!("mov expects 2 args at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let src = operand(&mut locals, &mut program.consts, &parts[2])?;
                    Instr {
                        op: Op::Mov { dst, src },
                        src_line: ln,
                    }
                }
                "add" | "sub" | "mul" | "div" | "eq" | "ne" | "lt" | "gt" | "and" | "or" => {
                    if parts.len() != 4 {
                        return Err(format!("{} expects 3 args at line {}", op, ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let a = operand(&mut locals, &mut program.consts, &parts[2])?;
                    let b = operand(&mut locals, &mut program.consts, &parts[3])?;
                    let op = match op {
                        "add" => Op::Add { dst, a, b },
                        "sub" => Op::Sub { dst, a, b },
                        "mul" => Op::Mul { dst, a, b },
                        "div" => Op::Div { dst, a, b },
                        "eq" => Op::Eq { dst, a, b },
                        "ne" => Op::Ne { dst, a, b },
                        "lt" => Op::Lt { dst, a, b },
                        "gt" => Op::Gt { dst, a, b },
                        "and" => Op::And { dst, a, b },
                        "or" => Op::Or { dst, a, b },
                        _ => unreachable!(),
                    };
                    Instr { op, src_line: ln }
                }
                "not" => {
                    if parts.len() != 3 {
                        return Err(format!("not expects 2 args at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let a = operand(&mut locals, &mut program.consts, &parts[2])?;
                    Instr {
                        op: Op::Not { dst, a },
                        src_line: ln,
                    }
                }
                "jmp" => {
                    if parts.len() != 2 {
                        return Err(format!("jmp expects 1 arg at line {}", ln));
                    }
                    Instr {
                        op: Op::Jmp {
                            label: parts[1].clone(),
                        },
                        src_line: ln,
                    }
                }
                "jnz" => {
                    if parts.len() != 3 {
                        return Err(format!("jnz expects 2 args at line {}", ln));
                    }
                    let cond = operand(&mut locals, &mut program.consts, &parts[1])?;
                    Instr {
                        op: Op::Jnz {
                            cond,
                            label: parts[2].clone(),
                        },
                        src_line: ln,
                    }
                }
                "jz" => {
                    if parts.len() != 3 {
                        return Err(format!("jz expects 2 args at line {}", ln));
                    }
                    let cond = operand(&mut locals, &mut program.consts, &parts[1])?;
                    Instr {
                        op: Op::Jz {
                            cond,
                            label: parts[2].clone(),
                        },
                        src_line: ln,
                    }
                }
                "call" => {
                    if parts.len() < 3 {
                        return Err(format!("call expects 2+ args at line {}", ln));
                    }
                    let dst = if parts[1] == "_" {
                        None
                    } else {
                        Some(local_index(&mut locals, &parts[1])?)
                    };
                    let name = parts[2].clone();
                    let callee = match name.as_str() {
                        "len" => Callee::Builtin(Builtin::Len),
                        "keys" => Callee::Builtin(Builtin::Keys),
                        "values" => Callee::Builtin(Builtin::Values),
                        "range" => Callee::Builtin(Builtin::Range),
                        _ => {
                            let idx = *program
                                .func_index
                                .get(&name)
                                .ok_or_else(|| format!("Unknown function '{}' at line {}", name, ln))?;
                            Callee::User(idx)
                        }
                    };

                    let mut args = Vec::new();
                    for a in parts.iter().skip(3) {
                        args.push(operand(&mut locals, &mut program.consts, a)?);
                    }
                    Instr {
                        op: Op::Call { dst, callee, args },
                        src_line: ln,
                    }
                }
                "ret" => {
                    let value = if parts.len() == 1 {
                        None
                    } else if parts.len() == 2 {
                        Some(operand(&mut locals, &mut program.consts, &parts[1])?)
                    } else {
                        return Err(format!("ret expects 0-1 args at line {}", ln));
                    };
                    Instr {
                        op: Op::Ret { value },
                        src_line: ln,
                    }
                }
                "print" => {
                    if parts.len() != 2 {
                        return Err(format!("print expects 1 arg at line {}", ln));
                    }
                    let value = operand(&mut locals, &mut program.consts, &parts[1])?;
                    Instr {
                        op: Op::Print { value },
                        src_line: ln,
                    }
                }
                "list" => {
                    if parts.len() < 2 {
                        return Err(format!("list expects 1+ args at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let mut items = Vec::new();
                    for a in parts.iter().skip(2) {
                        items.push(operand(&mut locals, &mut program.consts, a)?);
                    }
                    Instr {
                        op: Op::ListNew { dst, items },
                        src_line: ln,
                    }
                }
                "map" => {
                    if parts.len() != 2 {
                        return Err(format!("map expects 1 arg at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    Instr {
                        op: Op::MapNew { dst },
                        src_line: ln,
                    }
                }
                "mset" => {
                    if parts.len() != 4 {
                        return Err(format!("mset expects 3 args at line {}", ln));
                    }
                    let map = operand(&mut locals, &mut program.consts, &parts[1])?;
                    let key = operand(&mut locals, &mut program.consts, &parts[2])?;
                    let value = operand(&mut locals, &mut program.consts, &parts[3])?;
                    Instr {
                        op: Op::MapSet { map, key, value },
                        src_line: ln,
                    }
                }
                "mget" => {
                    if parts.len() != 4 {
                        return Err(format!("mget expects 3 args at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let map = operand(&mut locals, &mut program.consts, &parts[2])?;
                    let key = operand(&mut locals, &mut program.consts, &parts[3])?;
                    Instr {
                        op: Op::MapGet { dst, map, key },
                        src_line: ln,
                    }
                }
                "idxget" => {
                    if parts.len() != 4 {
                        return Err(format!("idxget expects 3 args at line {}", ln));
                    }
                    let dst = local_index(&mut locals, &parts[1])?;
                    let list = operand(&mut locals, &mut program.consts, &parts[2])?;
                    let index = operand(&mut locals, &mut program.consts, &parts[3])?;
                    Instr {
                        op: Op::IdxGet { dst, list, index },
                        src_line: ln,
                    }
                }
                "idxset" => {
                    if parts.len() != 4 {
                        return Err(format!("idxset expects 3 args at line {}", ln));
                    }
                    let list = operand(&mut locals, &mut program.consts, &parts[1])?;
                    let index = operand(&mut locals, &mut program.consts, &parts[2])?;
                    let value = operand(&mut locals, &mut program.consts, &parts[3])?;
                    Instr {
                        op: Op::IdxSet { list, index, value },
                        src_line: ln,
                    }
                }
                _ => return Err(format!("Unknown instruction '{}' at line {}", op, ln)),
            };
            code.push(instr);
        }

        let mut labels = HashMap::new();
        for (pc, instr) in code.iter().enumerate() {
            if let Op::Label { name } = &instr.op {
                labels.insert(name.clone(), pc);
            }
        }

        let local_count = locals.len();
        program.functions.push(Function {
            name: f.name,
            params: f.params,
            locals,
            code,
            labels,
            local_count,
        });
    }

    if !program.func_index.contains_key("main") {
        return Err("No 'main' function found".to_string());
    }

    Ok(program)
}

fn local_index(locals: &mut HashMap<String, u16>, name: &str) -> Result<u16, String> {
    if name == "_" {
        return Err("Cannot use '_' as a destination".to_string());
    }
    if let Some(&idx) = locals.get(name) {
        Ok(idx)
    } else {
        let idx = locals.len() as u16;
        locals.insert(name.to_string(), idx);
        Ok(idx)
    }
}

fn operand(locals: &mut HashMap<String, u16>, consts: &mut Vec<Value>, token: &str) -> Result<Operand, String> {
    if let Some(&idx) = locals.get(token) {
        return Ok(Operand::Local(idx));
    }
    if token == "null" {
        let k = const_index(consts, Value::Null);
        return Ok(Operand::Const(k));
    }
    if token == "_" {
        let k = const_index(consts, Value::Null);
        return Ok(Operand::Const(k));
    }
    if let Ok(lit) = parse_literal(token) {
        let k = const_index(consts, lit);
        return Ok(Operand::Const(k));
    }
    // auto-declare as local if referenced (defaults to null)
    let idx = local_index(locals, token)?;
    Ok(Operand::Local(idx))
}

fn const_index(consts: &mut Vec<Value>, v: Value) -> u16 {
    consts.push(v);
    (consts.len() - 1) as u16
}

fn parse_literal(token: &str) -> Result<Value, String> {
    if token == "true" {
        return Ok(Value::Bool(true));
    }
    if token == "false" {
        return Ok(Value::Bool(false));
    }
    if token.starts_with('"') && token.ends_with('"') && token.len() >= 2 {
        let inner = &token[1..token.len() - 1];
        let s = unescape(inner);
        return Ok(Value::String(Rc::new(s)));
    }
    if let Ok(n) = token.parse::<f64>() {
        return Ok(Value::Number(n));
    }
    Err("not a literal".to_string())
}

fn split_tokens(line: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_str = false;
    let mut escape = false;

    for ch in line.chars() {
        if in_str {
            current.push(ch);
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_str = false;
            }
            continue;
        }

        if ch.is_whitespace() {
            if !current.is_empty() {
                out.push(current.clone());
                current.clear();
            }
            continue;
        }

        if ch == '"' {
            if !current.is_empty() {
                // allow quotes only starting a token
                return Err(format!("Unexpected '\"' in token: {}", line));
            }
            in_str = true;
            current.push(ch);
            continue;
        }

        current.push(ch);
    }

    if in_str {
        return Err(format!("Unterminated string in: {}", line));
    }
    if !current.is_empty() {
        out.push(current);
    }
    Ok(out)
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('0') => out.push('\0'),
            Some('"') => out.push('"'),
            Some('\\') => out.push('\\'),
            Some(other) => {
                out.push('\\');
                out.push(other);
            }
            None => out.push('\\'),
        }
    }
    out
}

pub fn sample_program() -> String {
    // A tiny example users can copy/paste.
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    format!(
        r#"# CoRe VM IR (section mode)
fn main.params:
fn main.actions:
  const x 40
  const y 2
  add z x y
  print "Hello from IR @ {now}"
  print z
  ret 0
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_section_mode_and_run() {
        let src = r#"
fn add.params: a, b
fn add.actions:
  add t0 a b
  ret t0

fn main.params:
fn main.actions:
  const x 2
  const y 3
  call z add x y
  print z
  ret 0
"#;
        let program = parse_and_compile(src).unwrap();
        let mut out = Vec::new();
        let mut engine = Engine::new(&program, &mut out);
        engine.run("main").unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "5");
    }

    #[test]
    fn parse_brace_mode_and_loops() {
        let src = r#"
fn main: {
  const i 0
  const sum 0
loop:
  lt c i 5
  jz c done
  add sum sum i
  add i i 1
  jmp loop
done:
  print sum
  ret 0
}
"#;
        let program = parse_and_compile(src).unwrap();
        let mut out = Vec::new();
        let mut engine = Engine::new(&program, &mut out);
        engine.run("main").unwrap();
        assert_eq!(String::from_utf8(out).unwrap().trim(), "10");
    }

    #[test]
    fn map_and_list_ops() {
        let src = r#"
fn main.params:
fn main.actions:
  map m
  mset m "a" 1
  mset m "b" 2
  call ks keys m
  print ks
  list l 10 20 30
  idxget x l 1
  print x
  idxset l 2 99
  print l
  ret 0
"#;
        let program = parse_and_compile(src).unwrap();
        let mut out = Vec::new();
        let mut engine = Engine::new(&program, &mut out);
        engine.run("main").unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("[a, b]") || s.contains("[b, a]"));
        assert!(s.contains("\n20\n"));
        assert!(s.contains("[10, 20, 99]"));
    }
}
