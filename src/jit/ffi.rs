//! Phase 7: Runtime Calls (FFI)
//!
//! Provides support for JIT-compiled code to call external Rust functions.
//! This enables I/O, memory allocation, and other runtime services.

use crate::jit::encoder::{encode_blr, encode_mov64, Reg};
use crate::jit::runtime;

pub type RuntimeFn = extern "C" fn(i64) -> i64;

pub struct FfiHandle {
    pub ptr: *const (),
    pub name: String,
}

impl FfiHandle {
    pub fn from_ptr(name: &str, ptr: *const ()) -> Self {
        Self {
            ptr,
            name: name.to_string(),
        }
    }

    pub fn addr(&self) -> u64 {
        self.ptr as u64
    }
}

pub struct FfiEmitter;

impl FfiEmitter {
    pub fn emit_call(func_addr: u64) -> Vec<u8> {
        let instrs = encode_mov64(Reg::X(9), func_addr);
        let blr = encode_blr(Reg::X(9));
        let mut code = Vec::new();
        for instr in &instrs {
            code.extend_from_slice(&instr.to_le_bytes());
        }
        code.extend_from_slice(&blr.to_le_bytes());
        code
    }
}

pub struct RuntimeFunctions;

// Simple print function for JIT - prints an integer
extern "C" fn jit_print_int(value: i64) {
    println!("{}", value);
}

impl RuntimeFunctions {
    // Simple integer print for JIT
    pub fn print_int() -> FfiHandle {
        FfiHandle::from_ptr("jit_print_int", jit_print_int as *const ())
    }

    pub fn print() -> FfiHandle {
        FfiHandle::from_ptr("rt_print", runtime::rt_print as *const ())
    }

    pub fn alloc_string() -> FfiHandle {
        FfiHandle::from_ptr("rt_alloc_string", runtime::rt_alloc_string as *const ())
    }

    pub fn alloc_float() -> FfiHandle {
        FfiHandle::from_ptr("rt_alloc_float", runtime::rt_alloc_float as *const ())
    }

    pub fn to_str() -> FfiHandle {
        FfiHandle::from_ptr("rt_to_str", runtime::rt_to_str as *const ())
    }

    pub fn to_num() -> FfiHandle {
        FfiHandle::from_ptr("rt_to_num", runtime::rt_to_num as *const ())
    }

    // GC
    pub fn retain() -> FfiHandle {
        FfiHandle::from_ptr("rt_retain", runtime::rt_retain as *const ())
    }
    pub fn release() -> FfiHandle {
        FfiHandle::from_ptr("rt_release", runtime::rt_release as *const ())
    }

    // Float Ops
    pub fn float_add() -> FfiHandle {
        FfiHandle::from_ptr("rt_float_add", runtime::rt_float_add as *const ())
    }
    pub fn float_sub() -> FfiHandle {
        FfiHandle::from_ptr("rt_float_sub", runtime::rt_float_sub as *const ())
    }
    pub fn float_mul() -> FfiHandle {
        FfiHandle::from_ptr("rt_float_mul", runtime::rt_float_mul as *const ())
    }
    pub fn float_div() -> FfiHandle {
        FfiHandle::from_ptr("rt_float_div", runtime::rt_float_div as *const ())
    }

    // List
    pub fn alloc_list() -> FfiHandle {
        FfiHandle::from_ptr("rt_alloc_list", runtime::rt_alloc_list as *const ())
    }
    pub fn list_push() -> FfiHandle {
        FfiHandle::from_ptr("rt_list_push", runtime::rt_list_push as *const ())
    }
    pub fn list_get() -> FfiHandle {
        FfiHandle::from_ptr("rt_list_get", runtime::rt_list_get as *const ())
    }
    pub fn list_set() -> FfiHandle {
        FfiHandle::from_ptr("rt_list_set", runtime::rt_list_set as *const ())
    }
    pub fn list_len() -> FfiHandle {
        FfiHandle::from_ptr("rt_list_len", runtime::rt_list_len as *const ())
    }

    // Map
    pub fn alloc_map() -> FfiHandle {
        FfiHandle::from_ptr("rt_alloc_map", runtime::rt_alloc_map as *const ())
    }
    pub fn map_set() -> FfiHandle {
        FfiHandle::from_ptr("rt_map_set", runtime::rt_map_set as *const ())
    }
    pub fn map_get() -> FfiHandle {
        FfiHandle::from_ptr("rt_map_get", runtime::rt_map_get as *const ())
    }
    pub fn map_keys() -> FfiHandle {
        FfiHandle::from_ptr("rt_map_keys", runtime::rt_map_keys as *const ())
    }

    // File I/O
    pub fn file_open() -> FfiHandle {
        FfiHandle::from_ptr("rt_file_open", runtime::rt_file_open as *const ())
    }
    pub fn file_read() -> FfiHandle {
        FfiHandle::from_ptr("rt_file_read", runtime::rt_file_read as *const ())
    }
    pub fn file_close() -> FfiHandle {
        FfiHandle::from_ptr("rt_file_close", runtime::rt_file_close as *const ())
    }

    // Exceptions
    pub fn push_try() -> FfiHandle {
        FfiHandle::from_ptr("rt_push_try", runtime::rt_push_try as *const ())
    }
    pub fn pop_try() -> FfiHandle {
        FfiHandle::from_ptr("rt_pop_try", runtime::rt_pop_try as *const ())
    }
    pub fn throw() -> FfiHandle {
        FfiHandle::from_ptr("rt_throw", runtime::rt_throw as *const ())
    }
    pub fn get_last_error() -> FfiHandle {
        FfiHandle::from_ptr("rt_get_last_error", runtime::rt_get_last_error as *const ())
    }
}
