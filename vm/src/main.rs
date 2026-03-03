mod vm;
mod core_ir;

use colored::Colorize;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::fs;
use std::io::{self, Write};
use vm::VM;

fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.len() == 2 && args[1] == "--sample-ir" {
        print!("{}", core_ir::sample_program());
        return;
    }

    if args.len() == 5 && args[1] == "--dump" {
        let file = &args[2];
        let label = &args[3];
        let len: usize = args[4].parse().unwrap_or(64);

        let content = fs::read_to_string(file).unwrap_or_else(|e| {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        });
        let mut vm = VM::new();
        if let Err(e) = vm.load_program(&content) {
            eprintln!("Error loading program: {}", e);
            std::process::exit(1);
        }

        let addr = match vm.labels.get(label) {
            Some(a) => *a,
            None => {
                eprintln!("Label not found: {}", label);
                std::process::exit(1);
            }
        };
        let end = (addr + len).min(vm.memory.len());
        println!("{} @ 0x{:x} ({} bytes):", label, addr, end - addr);
        for (i, b) in vm.memory[addr..end].iter().enumerate() {
            if i % 16 == 0 {
                print!("\n{:08x}: ", addr + i);
            }
            print!("{:02x} ", b);
        }
        println!();
        return;
    }
    
    if args.len() > 1 {
        // Mode: Run file directly (assembly or CoRe IR)
        if args.len() == 3 && (args[1] == "--ir" || args[1] == "--coreir") {
            run_core_ir_file(&args[2]);
            return;
        }

        let path = &args[1];
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Error reading file: {}", e);
                std::process::exit(1);
            }
        };

        let looks_like_ir = detect_core_ir(&content) || path.ends_with(".cir") || path.ends_with(".ir");
        if looks_like_ir {
            run_core_ir_text(&content);
            return;
        }

        let mut vm = VM::new();
        if let Err(e) = vm.load_program(&content) {
            eprintln!("Error loading program: {}", e);
            std::process::exit(1);
        }
        vm.step_mode = false;
        if let Err(e) = vm.run() {
            eprintln!("Runtime Error: {}", e);
            std::process::exit(1);
        }
        return;
    }

    println!("{}", "Arm64 Virtual Runtime v1.0".bright_cyan().bold());
    println!("{}", "Type 'help' for commands\n".bright_black());
    
    let mut vm = VM::new();
    let mut rl = DefaultEditor::new().expect("Failed to create readline");
    
    loop {
        let readline = rl.readline(&format!("{} ", "arm64>".bright_green()));
        
        match readline {
            Ok(line) => {
                let line = line.trim();
                
                if line.is_empty() {
                    continue;
                }
                
                rl.add_history_entry(line).ok();
                
                match handle_command(&mut vm, line) {
                    Ok(should_continue) => {
                        if !should_continue {
                            break;
                        }
                    }
                    Err(e) => {
                        println!("{} {}", "Error:".bright_red(), e);
                    }
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("Interrupted");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("EOF");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }
}

fn detect_core_ir(text: &str) -> bool {
    for line in text.lines() {
        let t = line.trim_start();
        if t.is_empty() || t.starts_with('#') || t.starts_with("//") {
            continue;
        }
        return t.starts_with("fn ");
    }
    false
}

fn run_core_ir_file(path: &str) {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            std::process::exit(1);
        }
    };
    run_core_ir_text(&content);
}

fn run_core_ir_text(text: &str) {
    let program = match core_ir::parse_and_compile(text) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("IR Parse/Compile Error: {}", e);
            std::process::exit(1);
        }
    };

    let mut out = io::stdout().lock();
    let mut engine = core_ir::Engine::new(&program, &mut out);
    if let Err(e) = engine.run("main") {
        eprintln!("IR Runtime Error: {}", e);
        std::process::exit(1);
    }
    out.flush().ok();
}

fn handle_command(vm: &mut VM, cmd: &str) -> Result<bool, String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    
    if parts.is_empty() {
        return Ok(true);
    }
    
    match parts[0] {
        "help" | "h" => {
            print_help();
        }
        "load" | "l" => {
            if parts.len() < 2 {
                return Err("Usage: load <file>".to_string());
            }
            let content = fs::read_to_string(parts[1])
                .map_err(|e| format!("Failed to read file: {}", e))?;
            vm.load_program(&content)
                .map_err(|e| format!("Failed to load program: {}", e))?;
            println!("{} Loaded {} instructions", "✓".bright_green(), vm.program.len());
        }
        "run" | "r" => {
            vm.step_mode = false;
            match vm.run() {
                Ok(_) => println!("{} Execution completed", "✓".bright_green()),
                Err(e) => return Err(e),
            }
        }
        "step" | "s" => {
            vm.step_mode = true;
            match vm.step() {
                Ok(_) => {
                    if vm.pc as usize >= vm.program.len() {
                        println!("{} Program finished", "✓".bright_green());
                    } else {
                        println!("{} PC: {} | {}", 
                            "→".bright_yellow(), 
                            vm.pc, 
                            vm.program.get(vm.pc as usize).unwrap_or(&"".to_string()));
                    }
                }
                Err(e) => return Err(e),
            }
        }
        "regs" | "reg" => {
            vm.print_registers();
        }
        "reset" => {
            *vm = VM::new();
            println!("{} VM reset", "✓".bright_green());
        }
        "prog" | "p" => {
            println!("\n{}", "=== Program ===".bright_cyan());
            for (i, instr) in vm.program.iter().enumerate() {
                if i == vm.pc as usize {
                    println!("{} {:3}: {}", "→".bright_yellow(), i, instr.bright_white());
                } else {
                    println!("  {:3}: {}", i, instr.bright_black());
                }
            }
            println!("{}\n", "===============".bright_cyan());
        }
        "exec" | "e" => {
            // Execute a single instruction directly
            let instr = parts[1..].join(" ");
            vm.program.push(instr.clone());
            vm.pc = vm.program.len() as i64 - 1;
            match vm.step() {
                Ok(_) => println!("{} Executed: {}", "✓".bright_green(), instr),
                Err(e) => {
                    vm.program.pop();
                    return Err(e);
                }
            }
        }
        "quit" | "q" | "exit" => {
            println!("Goodbye!");
            return Ok(false);
        }
        _ => {
            // Try to execute as assembly instruction
            vm.program.push(cmd.to_string());
            vm.pc = vm.program.len() as i64 - 1;
            match vm.step() {
                Ok(_) => println!("{} Executed", "✓".bright_green()),
                Err(e) => {
                    vm.program.pop();
                    return Err(format!("Unknown command or invalid instruction: {}", e));
                }
            }
        }
    }
    
    Ok(true)
}

fn print_help() {
    println!("\n{}", "Available Commands:".bright_cyan().bold());
    println!("  {}  - Show this help", "help, h".bright_yellow());
    println!("  {}  - Load assembly from file", "load <file>, l <file>".bright_yellow());
    println!("  {}  - Run program to completion", "run, r".bright_yellow());
    println!("  {}  - Execute one instruction", "step, s".bright_yellow());
    println!("  {}  - Show registers", "regs, reg".bright_yellow());
    println!("  {}  - Show program listing", "prog, p".bright_yellow());
    println!("  {}  - Execute instruction directly", "exec <instr>, e <instr>".bright_yellow());
    println!("  {}  - Reset VM", "reset".bright_yellow());
    println!("  {}  - Exit VM", "quit, q, exit".bright_yellow());
    println!();
    println!("{}", "CoRe IR Mode:".bright_cyan().bold());
    println!("  {}  - Run CoRe IR file", "arm64vm --ir file.cir".bright_yellow());
    println!("  {}  - Print a sample IR file", "arm64vm --sample-ir".bright_yellow());
    
    println!("\n{}", "Supported Instructions:".bright_cyan().bold());
    println!("  {} - Arithmetic", "add, sub, mul, sdiv".bright_yellow());
    println!("  {} - Data movement", "mov".bright_yellow());
    println!("  {} - Branches", "b, bl, ret, beq, bne, blt, bgt".bright_yellow());
    println!("  {} - Compare", "cmp".bright_yellow());
    println!("  {} - System call", "svc".bright_yellow());
    
    println!("\n{}", "Examples:".bright_cyan().bold());
    println!("  {}", "mov x0, #42".bright_black());
    println!("  {}", "add x1, x0, #10".bright_black());
    println!("  {}", "mul x2, x0, x1".bright_black());
    println!();
}
