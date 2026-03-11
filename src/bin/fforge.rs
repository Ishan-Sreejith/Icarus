use std::env;
use std::process::ExitCode;
use std::process::Command;

use forge::ff;
use forge::ir::IrBuilder;
use forge::jit::compiler::JitCompiler;
use forge::jit::context::JitContext;
use forge::jit::runtime::EncodedValue;
use forge::lexer::Lexer;
use forge::parser::Parser;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    let mut debug = false;
    let mut filename: Option<String> = None;
    let mut out_path: Option<String> = None;

    for arg in args.iter().skip(1) {
        match arg.as_str() {
            "-h" | "--help" => {
                eprintln!("Usage: fforge [-d|--debug] <file.fr>");
                eprintln!("       fforge --out <syntax.ff>");
                eprintln!("  Executes the file via the ARM64 JIT (aarch64 only).");
                return ExitCode::from(0);
            }
            "-d" | "--debug" => debug = true,
            "--out" => {
                out_path = Some(String::new());
            }
            _ if arg.starts_with('-') => {
                eprintln!("Unknown option: {}", arg);
                eprintln!("Usage: fforge [-d|--debug] <file.fr>");
                eprintln!("       fforge --out <syntax.ff>");
                return ExitCode::from(1);
            }
            _ => {
                if let Some(path) = out_path.as_mut() {
                    if path.is_empty() {
                        *path = arg.clone();
                        continue;
                    }
                }
                if filename.is_some() {
                    eprintln!("Only one input file is supported.");
                    eprintln!("Usage: fforge [-d|--debug] <file.fr>");
                    eprintln!("       fforge --out <syntax.ff>");
                    return ExitCode::from(1);
                }
                filename = Some(arg.clone());
            }
        }
    }

    if let Some(out_path) = out_path {
        if out_path.is_empty() {
            eprintln!("Usage: fforge --out <syntax.ff>");
            return ExitCode::from(1);
        }
        return match ff::write_default_syntax_ff(std::path::Path::new(&out_path)) {
            Ok(()) => {
                println!("Wrote {}", out_path);
                ExitCode::from(0)
            }
            Err(e) => {
                eprintln!("{}", e);
                ExitCode::from(1)
            }
        };
    }

    let Some(filename) = filename else {
        eprintln!("Usage: fforge [-d|--debug] <file.fr>");
        eprintln!("       fforge --out <syntax.ff>");
        return ExitCode::from(1);
    };
    debug_log(debug, "fforge starting...");

    #[cfg(not(target_arch = "aarch64"))]
    {
        eprintln!("fforge: JIT is only supported on aarch64; falling back to `forge --rust`.");
        let status = Command::new(find_forge())
            .arg("--rust")
            .arg(&filename)
            .status();
        return match status {
            Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
            Err(e) => {
                eprintln!("fforge: failed to execute forge fallback: {}", e);
                ExitCode::from(1)
            }
        };
    }

    debug_log(debug, &format!("Reading file: {}", filename));
    let source = match ff::preprocess_file(std::path::Path::new(&filename)) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading file: {}", e);
            return ExitCode::from(1);
        }
    };
    debug_log(debug, &format!("File read, {} bytes", source.len()));

    debug_log(debug, "Starting lexer...");
    let lexer_results: Vec<_> = Lexer::new(&source).collect();
    debug_log(
        debug,
        &format!("Lexer produced {} results", lexer_results.len()),
    );
    let mut tokens = Vec::new();

    for (result, span) in lexer_results {
        match result {
            Ok(t) => tokens.push((t, span)),
            Err(e) => {
                eprintln!("Lexer Error: {}", e);
                return ExitCode::from(1);
            }
        }
    }
    debug_log(debug, &format!("Lexer complete, {} tokens", tokens.len()));

    debug_log(debug, "Starting parser...");
    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parser Error: {}", e);
            return ExitCode::from(1);
        }
    };
    debug_log(debug, "Parser complete");

    debug_log(debug, "Starting IR generation...");
    let mut ir_builder = IrBuilder::new();
    let ir = match ir_builder.build(&ast, None) {
        Ok(i) => i,
        Err(e) => {
            eprintln!("IR Error: {}", e);
            return ExitCode::from(1);
        }
    };
    debug_log(debug, "IR generation complete");
    if debug && std::env::var("CORE_JIT_DUMP").is_ok() {
        eprintln!("[DEBUG] Global IR:");
        for (i, instr) in ir.global_code.iter().enumerate() {
            eprintln!("  {:03}: {:?}", i, instr);
        }
    }

    if debug {
        println!("→ JIT Compiling & Executing {}...", filename);
    }
    debug_log(debug, "Creating JIT context and compiler...");

    let mut context = JitContext::new();
    let mut jit = JitCompiler::new(&mut context);

    debug_log(debug, &format!("Compiling {} functions...", ir.functions.len()));

    let mut func_names: Vec<_> = ir.functions.keys().cloned().collect();
    func_names.sort_by(|a, b| b.cmp(a)); // Reverse order

    for name in func_names {
        if let Some(func) = ir.functions.get(&name) {
            debug_log(debug, &format!("Compiling function: {}", name));
            if let Err(e) = jit.compile_function(func) {
                eprintln!("✗ JIT Compilation Error (Function {}): {}", name, e);
                return run_forge_fallback(&filename);
            }
            debug_log(debug, &format!("Function {} compiled OK", name));
        }
    }

    debug_log(
        debug,
        &format!(
            "Executing global code with {} instructions...",
            ir.global_code.len()
        ),
    );
    match jit.execute_global(&ir.global_code) {
        Ok(res) => {
            debug_log(debug, &format!("Execution complete, result: {}", res));
            if debug {
                let rendered = render_encoded(res, debug);
                println!("✓ Result: {}", rendered);
            }
            ExitCode::from(0)
        }
        Err(e) => {
            eprintln!("✗ JIT Runtime Error: {}", e);
            run_forge_fallback(&filename)
        }
    }
}

fn render_encoded(val: EncodedValue, debug: bool) -> String {
    if val == 0 {
        return "null".to_string();
    }
    if (val & 1) == 1 {
        return ((val as i64) >> 1).to_string();
    }
    if debug {
        return format!("<object @ 0x{:x}>", val);
    }
    "<object>".to_string()
}

fn debug_log(enabled: bool, msg: &str) {
    if enabled {
        eprintln!("[DEBUG] {}", msg);
    }
}

#[cfg(not(target_arch = "aarch64"))]
fn find_forge() -> String {
    if let Ok(current) = env::current_exe() {
        if let Some(dir) = current.parent() {
            let candidate = dir.join("forge");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    "forge".to_string()
}

#[cfg(target_arch = "aarch64")]
fn find_forge() -> String {
    if let Ok(current) = env::current_exe() {
        if let Some(dir) = current.parent() {
            let candidate = dir.join("forge");
            if candidate.exists() {
                return candidate.to_string_lossy().to_string();
            }
        }
    }
    "forge".to_string()
}

fn run_forge_fallback(filename: &str) -> ExitCode {
    eprintln!("fforge: falling back to `forge --rust` for {}", filename);
    let mut candidate = None;
    let local_debug = std::path::Path::new("target/debug/forge");
    let local_release = std::path::Path::new("target/release/forge");
    if local_debug.exists() {
        candidate = Some(local_debug.to_string_lossy().to_string());
    } else if local_release.exists() {
        candidate = Some(local_release.to_string_lossy().to_string());
    }

    let status = if let Some(bin) = candidate {
        Command::new(bin).arg("--rust").arg(filename).status()
    } else {
        Command::new(find_forge()).arg("--rust").arg(filename).status()
    };
    match status {
        Ok(s) => ExitCode::from(s.code().unwrap_or(1) as u8),
        Err(e) => {
            eprintln!("fforge: failed to execute forge fallback: {}", e);
            ExitCode::from(1)
        }
    }
}
