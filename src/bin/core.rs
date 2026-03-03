use std::env;
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();

    let mut cmd = find_forge();
    cmd.args(args);

    match cmd.status() {
        Ok(status) => ExitCode::from(status.code().unwrap_or(1) as u8),
        Err(err) => {
            eprintln!("core: failed to execute forge: {}", err);
            ExitCode::from(1)
        }
    }
}

fn find_forge() -> Command {
    if let Ok(current) = env::current_exe() {
        if let Some(dir) = current.parent() {
            let candidate = dir.join("forge");
            if candidate.exists() {
                return Command::new(candidate);
            }
        }
    }
    Command::new("forge")
}
