#[cfg(all(target_os = "macos", target_arch = "aarch64"))]
fn main() {
    let jit = forge::jit::trampoline::JitFunction::from_returning_u16(42)
        .expect("failed to build JIT function");
    let result = jit.call_i64();
    println!("JIT returned: {}", result);
}

#[cfg(not(all(target_os = "macos", target_arch = "aarch64")))]
fn main() {
    eprintln!("jit_trampoline is only supported on macOS ARM64 in this build");
}
