use crate::ir::*;
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::Write;

pub struct Arm64CodeGen {
    output: String,
    label_counter: usize,
    stack_offset: i32,
    var_offsets: HashMap<String, i32>,
    string_labels: HashMap<String, String>,
    epilogue_label: String,
    user_functions: HashSet<String>,
}

impl Arm64CodeGen {
    pub fn new() -> Self {
        Arm64CodeGen {
            output: String::new(),
            label_counter: 0,
            stack_offset: 0,
            var_offsets: HashMap::new(),
            string_labels: HashMap::new(),
            epilogue_label: String::new(),
            user_functions: HashSet::new(),
        }
    }

    fn emit(&mut self, line: &str) {
        self.output.push_str(line);
        self.output.push('\n');
    }

    fn fresh_label(&mut self) -> String {
        let label = format!(".L{}", self.label_counter);
        self.label_counter += 1;
        label
    }

    fn sanitize(&self, name: &str) -> String {
        name.replace("-", "_")
    }

    pub fn generate(&mut self, program: &IrProgram) -> Result<String, String> {
        self.user_functions = program.functions.keys().cloned().collect();

        // Generate assembly header
        self.emit(".global _main");
        self.emit(".align 4");
        self.emit("");

        // Generate data section for strings
        self.emit(".data");
        self.generate_data_section(program);
        self.emit("");

        // Generate code section
        self.emit(".text");

        // Generate main function from global code
        if !program.global_code.is_empty() {
            self.emit("_main:");
            self.epilogue_label = self.fresh_label();
            self.precalculate_stack_offsets(&program.global_code, &[]);
            self.generate_function_prologue(&[]);

            for instr in &program.global_code {
                self.generate_instruction(instr)?;
            }

            // Exit result
            self.emit("    mov x0, #0");
            self.generate_function_epilogue();
        } else {
            // If no global code, create a dummy main that returns 0
            self.emit("_main:");
            self.emit("    mov x0, #0");
            self.emit("    ret");
        }

        // Generate user-defined functions
        for (name, func) in &program.functions {
            self.emit(&format!("_user_{}:", self.sanitize(name)));
            self.epilogue_label = self.fresh_label();
            self.precalculate_stack_offsets(&func.instructions, &func.params);
            self.generate_function_prologue(&func.params);

            for instr in &func.instructions {
                self.generate_instruction(instr)?;
            }

            self.generate_function_epilogue();
        }

        // Recursive print dispatcher: _print_val(value)
        self.emit("");
        self.emit("_print_val:");
        self.emit("    stp x29, x30, [sp, #-32]!");
        self.emit("    str x19, [sp, #16]");
        self.emit("    mov x29, sp");
        self.emit("    mov x19, x0"); // save value

        self.emit("    cmp x19, #0x1000");
        self.emit("    b.hi .print_val_ptr");

        // It's a number
        self.emit("    mov x0, x19");
        self.emit("    bl _print_num_no_nl");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".print_val_ptr:");
        self.emit("    ldr x0, [x19, #0]"); // tag
        self.emit("    cmp x0, #1"); // List
        self.emit("    b.eq .print_val_list");
        self.emit("    cmp x0, #2"); // Map
        self.emit("    b.eq .print_val_map");
        self.emit("    cmp x0, #3"); // String
        self.emit("    b.eq .print_val_str");

        // Unknown or raw
        self.emit("    mov x0, x19");
        self.emit("    bl _print_str_no_nl");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".print_val_str:");
        self.emit("    add x0, x19, #24"); // Offset to CharData
        self.emit("    bl _print_str_no_nl");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".print_val_list:");
        self.emit("    mov x0, x19");
        self.emit("    bl _print_list");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".print_val_map:");
        self.emit("    mov x0, x19");
        self.emit("    bl _print_map");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        // Reference Counting Helpers
        self.emit("");
        self.emit("_inc_rc:");
        let inc_done = self.fresh_label();
        self.emit(&format!("    cbz x0, {}", inc_done));
        self.emit("    cmp x0, #0x1000");
        self.emit(&format!("    b.lo {}", inc_done));
        self.emit("    ldr x1, [x0, #8]"); // RefCount
        self.emit("    cmp x1, #0");
        self.emit(&format!("    b.lt {}", inc_done)); // Skip if RC < 0 (static object)
        self.emit("    add x1, x1, #1");
        self.emit("    str x1, [x0, #8]");
        self.emit(&format!("{}:", inc_done));
        self.emit("    ret");

        self.emit("");
        self.emit("_dec_rc:");
        let dec_done = self.fresh_label();
        self.emit(&format!("    cbz x0, {}", dec_done));
        self.emit("    cmp x0, #0x1000");
        self.emit(&format!("    b.lo {}", dec_done));
        self.emit("    ldr x1, [x0, #8]"); // RefCount
        self.emit("    cmp x1, #0");
        self.emit(&format!("    b.lt {}", dec_done)); // Skip if static
        self.emit("    sub x1, x1, #1");
        self.emit("    str x1, [x0, #8]");
        self.emit(&format!("    cbnz x1, {}", dec_done));

        // RC reached 0, free object
        self.emit("    stp x29, x30, [sp, #-16]!");
        self.emit("    bl _gc_free");
        self.emit("    ldp x29, x30, [sp], #16");
        self.emit(&format!("{}:", dec_done));
        self.emit("    ret");

        self.emit("");
        self.emit("_gc_free:");
        self.emit("    stp x29, x30, [sp, #-32]!");
        self.emit("    str x19, [sp, #16]");
        self.emit("    mov x29, sp");
        self.emit("    mov x19, x0");
        self.emit("    ldr x0, [x19, #0]"); // Tag
        self.emit("    cmp x0, #1"); // List
        self.emit("    b.eq .gc_free_list");
        self.emit("    cmp x0, #2"); // Map
        self.emit("    b.eq .gc_free_map");
        self.emit("    cmp x0, #4"); // Struct
        self.emit("    b.eq .gc_free_struct");

        // Default: just free header (for Strings/etc)
        self.emit("    mov x0, x19");
        self.emit("    bl _free");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".gc_free_list:");
        self.emit("    mov x0, x19");
        self.emit("    bl _free_list");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".gc_free_map:");
        self.emit("    mov x0, x19");
        self.emit("    bl _free_map");
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit(".gc_free_struct:");
        self.emit("    mov x0, x19");
        self.emit("    bl _free_map"); // Structs reuse map storage
        self.emit("    ldr x19, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #32");
        self.emit("    ret");

        self.emit("");
        self.emit("_free_list:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    stp x20, x21, [sp, #16]");
        self.emit("    stp x22, x19, [sp, #32]"); // Save x19 because _gc_free uses it too
        self.emit("    mov x29, sp");

        self.emit("    ldr x20, [x19, #16]"); // length
        self.emit("    ldr x21, [x19, #24]"); // dataPtr
        self.emit("    mov x22, #0");
        self.emit(".free_list_loop:");
        self.emit("    cmp x22, x20");
        self.emit("    b.ge .free_list_done");
        self.emit("    ldr x0, [x21, x22, lsl #3]");
        self.emit("    bl _dec_rc");
        self.emit("    add x22, x22, #1");
        self.emit("    b .free_list_loop");
        self.emit(".free_list_done:");
        self.emit("    mov x0, x21");
        self.emit("    bl _free");
        self.emit("    mov x0, x19");
        self.emit("    bl _free");

        self.emit("    ldp x22, x19, [sp, #32]");
        self.emit("    ldp x20, x21, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        self.emit("");
        self.emit("_free_map:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    stp x20, x21, [sp, #16]");
        self.emit("    stp x22, x19, [sp, #32]");
        self.emit("    mov x29, sp");

        self.emit("    ldr x20, [x19, #16]"); // count
        self.emit("    ldr x21, [x19, #24]"); // entriesPtr
        self.emit("    mov x22, #0");
        self.emit(".free_map_loop:");
        self.emit("    cmp x22, x20");
        self.emit("    b.ge .free_map_done");

        self.emit("    lsl x23, x22, #4");
        self.emit("    add x23, x21, x23");
        self.emit("    ldr x0, [x23, #0]"); // key
        self.emit("    bl _dec_rc");

        // Reload entry pointer because _dec_rc clobbered x23
        self.emit("    lsl x23, x22, #4");
        self.emit("    add x23, x21, x23");
        self.emit("    ldr x0, [x23, #8]"); // value
        self.emit("    bl _dec_rc");

        self.emit("    add x22, x22, #1");
        self.emit("    b .free_map_loop");

        self.emit(".free_map_done:");
        self.emit("    mov x0, x21");
        self.emit("    bl _free");
        self.emit("    mov x0, x19");
        self.emit("    bl _free");

        self.emit("    ldp x22, x19, [sp, #32]");
        self.emit("    ldp x20, x21, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        // Helper: _print_num_no_nl(num)
        self.emit("");
        self.emit("_print_num_no_nl:");
        self.emit("    stp x29, x30, [sp, #-16]!");
        self.emit("    mov x0, x0"); // number is already in x0? No, usually in x1 for printf but we use a custom sig
        self.emit("    mov x1, x0");
        self.emit("    sub sp, sp, #16");
        self.emit("    str x1, [sp, #0]");
        self.emit("    adrp x0, .fmt_int_raw@PAGE");
        self.emit("    add x0, x0, .fmt_int_raw@PAGEOFF");
        self.emit("    bl _printf");
        self.emit("    add sp, sp, #16");
        self.emit("    mov x0, #0");
        self.emit("    bl _fflush");
        self.emit("    ldp x29, x30, [sp], #16");
        self.emit("    ret");

        // Helper: _print_str_no_nl(str)
        self.emit("");
        self.emit("_print_str_no_nl:");
        self.emit("    stp x29, x30, [sp, #-16]!");
        self.emit("    mov x1, x0"); // string pointer
                                     // strlen
        self.emit("    mov x2, #0");
        let strlen_loop = self.fresh_label();
        let strlen_done = self.fresh_label();
        self.emit(&format!("{}:", strlen_loop));
        self.emit("    ldrb w3, [x1, x2]");
        self.emit(&format!("    cbz w3, {}", strlen_done));
        self.emit("    add x2, x2, #1");
        self.emit(&format!("    b {}", strlen_loop));
        self.emit(&format!("{}:", strlen_done));
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");
        self.emit("    ldp x29, x30, [sp], #16");
        self.emit("    ret");

        // Helper: _print_list(list)
        self.emit("");
        self.emit("_print_list:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    mov x29, sp");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    mov x19, x0"); // list header

        // Print "["
        self.emit("    adrp x1, .list_start@PAGE");
        self.emit("    add x1, x1, .list_start@PAGEOFF");
        self.emit("    mov x2, #1");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");

        self.emit("    ldr x20, [x19, #16]"); // length
        self.emit("    ldr x21, [x19, #24]"); // data
        self.emit("    mov x22, #0"); // index

        self.emit(".print_list_loop:");
        self.emit("    cmp x22, x20");
        self.emit("    b.ge .print_list_done");

        // Print comma if index > 0
        let list_no_comma = self.fresh_label();
        self.emit(&format!("    cbz x22, {}", list_no_comma));
        self.emit("    adrp x1, .comma_space@PAGE");
        self.emit("    add x1, x1, .comma_space@PAGEOFF");
        self.emit("    mov x2, #2");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");
        self.emit(&format!("{}:", list_no_comma));

        // Save state before recursion
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    ldr x0, [x21, x22, lsl #3]");
        self.emit("    bl _print_val");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");

        self.emit("    add x22, x22, #1");
        self.emit("    b .print_list_loop");

        self.emit(".print_list_done:");
        // Print "]"
        self.emit("    adrp x1, .list_end@PAGE");
        self.emit("    add x1, x1, .list_end@PAGEOFF");
        self.emit("    mov x2, #1");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");

        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        // Helper: _print_map(map)
        self.emit("");
        self.emit("_print_map:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    mov x29, sp");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    mov x19, x0"); // map header

        // Print "{"
        self.emit("    adrp x1, .map_start@PAGE");
        self.emit("    add x1, x1, .map_start@PAGEOFF");
        self.emit("    mov x2, #1");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");

        self.emit("    ldr x20, [x19, #16]"); // count
        self.emit("    ldr x21, [x19, #24]"); // entries
        self.emit("    mov x22, #0"); // index

        self.emit(".print_map_loop:");
        self.emit("    cmp x22, x20");
        self.emit("    b.ge .print_map_done");

        // Print comma if index > 0
        let map_no_comma = self.fresh_label();
        self.emit(&format!("    cbz x22, {}", map_no_comma));
        self.emit("    adrp x1, .comma_space@PAGE");
        self.emit("    add x1, x1, .comma_space@PAGEOFF");
        self.emit("    mov x2, #2");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");
        self.emit(&format!("{}:", map_no_comma));

        // Print key
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");

        self.emit("    lsl x25, x22, #4");
        self.emit("    ldr x0, [x21, x25]");
        self.emit("    bl _print_val"); // Could be number or string

        // Print ": "
        self.emit("    adrp x1, .colon_space@PAGE");
        self.emit("    add x1, x1, .colon_space@PAGEOFF");
        self.emit("    mov x2, #2");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");

        // Print value
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    lsl x25, x22, #4");
        // Pair is [key][value].
        self.emit("    add x25, x21, x25");
        self.emit("    ldr x0, [x25, #8]");

        self.emit("    stp x21, x22, [sp, #32]"); // restack
        self.emit("    bl _print_val");

        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");

        self.emit("    add x22, x22, #1");
        self.emit("    b .print_map_loop");

        self.emit(".print_map_done:");
        // Print "}"
        self.emit("    adrp x1, .map_end@PAGE");
        self.emit("    add x1, x1, .map_end@PAGEOFF");
        self.emit("    mov x2, #1");
        self.emit("    mov x0, #1");
        self.emit("    mov x16, #4");
        self.emit("    svc #0x80");

        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        // Existing print_num
        self.emit("");
        self.emit("_print_num:");
        self.emit("    stp x29, x30, [sp, #-16]!");
        self.emit("    mov x29, sp");
        self.emit("    sub sp, sp, #16");
        self.emit("    str x1, [sp, #0]");
        self.emit("    adrp x0, .fmt_int@PAGE");
        self.emit("    add x0, x0, .fmt_int@PAGEOFF");
        self.emit("    bl _printf");
        self.emit("    add sp, sp, #16");
        self.emit("    mov x0, #0");
        self.emit("    bl _fflush");
        self.emit("    ldp x29, x30, [sp], #16");
        self.emit("    ret");

        // Stub implementations for builtins (proper logic)
        self.emit("");
        self.emit("_len:");
        self.emit("    stp x29, x30, [sp, #-16]!");
        self.emit("    cbz x0, .len_null");
        self.emit("    ldr x0, [x0, #16]");
        self.emit("    ldp x29, x30, [sp], #16");
        self.emit("    ret");
        self.emit(".len_null:");
        self.emit("    mov x0, #0");
        self.emit("    ldp x29, x30, [sp], #16");
        self.emit("    ret");

        // Type checks
        self.emit("");
        self.emit("_is_map:");
        self.emit("    cbz x0, .is_map_no");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .is_map_no");
        self.emit("    ldr x1, [x0, #0]");
        self.emit("    cmp x1, #2");
        self.emit("    cset x0, eq");
        self.emit("    ret");
        self.emit(".is_map_no:");
        self.emit("    mov x0, #0");
        self.emit("    ret");

        self.emit("");
        self.emit("_is_list:");
        self.emit("    cbz x0, .is_list_no");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .is_list_no");
        self.emit("    ldr x1, [x0, #0]");
        self.emit("    cmp x1, #1");
        self.emit("    cset x0, eq");
        self.emit("    ret");
        self.emit(".is_list_no:");
        self.emit("    mov x0, #0");
        self.emit("    ret");

        self.emit("");
        self.emit("_is_string:");
        self.emit("    cbz x0, .is_string_no");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .is_string_no");
        self.emit("    ldr x1, [x0, #0]");
        self.emit("    cmp x1, #3");
        self.emit("    cset x0, eq");
        self.emit("    ret");
        self.emit(".is_string_no:");
        self.emit("    mov x0, #0");
        self.emit("    ret");

        // type(x) -> "number" | "string" | "list" | "map" | "struct" | "null" | "object"
        self.emit("");
        self.emit("_type:");
        self.emit("    cbz x0, .type_null");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .type_number");
        self.emit("    ldr x1, [x0, #0]");
        self.emit("    cmp x1, #1");
        self.emit("    b.eq .type_list");
        self.emit("    cmp x1, #2");
        self.emit("    b.eq .type_map");
        self.emit("    cmp x1, #3");
        self.emit("    b.eq .type_string");
        self.emit("    cmp x1, #4");
        self.emit("    b.eq .type_struct");
        self.emit("    b .type_object");

        self.emit(".type_number:");
        self.emit("    adrp x0, .t_number@PAGE");
        self.emit("    add x0, x0, .t_number@PAGEOFF");
        self.emit("    ret");
        self.emit(".type_string:");
        self.emit("    adrp x0, .t_string@PAGE");
        self.emit("    add x0, x0, .t_string@PAGEOFF");
        self.emit("    ret");
        self.emit(".type_list:");
        self.emit("    adrp x0, .t_list@PAGE");
        self.emit("    add x0, x0, .t_list@PAGEOFF");
        self.emit("    ret");
        self.emit(".type_map:");
        self.emit("    adrp x0, .t_map@PAGE");
        self.emit("    add x0, x0, .t_map@PAGEOFF");
        self.emit("    ret");
        self.emit(".type_struct:");
        self.emit("    adrp x0, .t_struct@PAGE");
        self.emit("    add x0, x0, .t_struct@PAGEOFF");
        self.emit("    ret");
        self.emit(".type_null:");
        self.emit("    adrp x0, .t_null@PAGE");
        self.emit("    add x0, x0, .t_null@PAGEOFF");
        self.emit("    ret");
        self.emit(".type_object:");
        self.emit("    adrp x0, .t_object@PAGE");
        self.emit("    add x0, x0, .t_object@PAGEOFF");
        self.emit("    ret");

        // bool(x) -> 0/1
        self.emit("");
        self.emit("_bool:");
        self.emit("    cbz x0, .bool_false");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .bool_num");
        self.emit("    ldr x1, [x0, #0]"); // tag
        self.emit("    cmp x1, #1"); // list
        self.emit("    b.eq .bool_lencheck");
        self.emit("    cmp x1, #2"); // map
        self.emit("    b.eq .bool_lencheck");
        self.emit("    cmp x1, #3"); // string
        self.emit("    b.eq .bool_lencheck");
        self.emit("    mov x0, #1");
        self.emit("    ret");
        self.emit(".bool_lencheck:");
        self.emit("    ldr x2, [x0, #16]");
        self.emit("    cmp x2, #0");
        self.emit("    cset x0, ne");
        self.emit("    ret");
        self.emit(".bool_num:");
        self.emit("    cmp x0, #0");
        self.emit("    cset x0, ne");
        self.emit("    ret");
        self.emit(".bool_false:");
        self.emit("    mov x0, #0");
        self.emit("    ret");

        // num(x) -> integer (best-effort)
        self.emit("");
        self.emit("_num:");
        self.emit("    cbz x0, .num_zero");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .num_ret");
        // Expect string object (tag 3). If not, return 0.
        self.emit("    ldr x1, [x0, #0]");
        self.emit("    cmp x1, #3");
        self.emit("    b.ne .num_zero");
        self.emit("    add x1, x0, #24"); // char ptr
        self.emit("    mov x2, #0"); // result
        self.emit("    mov x3, #0"); // sign flag
        self.emit("    ldrb w4, [x1]");
        self.emit("    cmp w4, #45"); // '-'
        self.emit("    b.ne .num_loop");
        self.emit("    mov x3, #1");
        self.emit("    add x1, x1, #1");
        self.emit(".num_loop:");
        self.emit("    ldrb w4, [x1]");
        self.emit("    cbz w4, .num_done");
        self.emit("    cmp w4, #48"); // '0'
        self.emit("    b.lt .num_done");
        self.emit("    cmp w4, #57"); // '9'
        self.emit("    b.gt .num_done");
        self.emit("    sub w5, w4, #48");
        self.emit("    mov x6, #10");
        self.emit("    mul x2, x2, x6");
        self.emit("    uxtw x5, w5");
        self.emit("    add x2, x2, x5");
        self.emit("    add x1, x1, #1");
        self.emit("    b .num_loop");
        self.emit(".num_done:");
        self.emit("    cbz x3, .num_pos");
        self.emit("    neg x2, x2");
        self.emit(".num_pos:");
        self.emit("    mov x0, x2");
        self.emit("    ret");
        self.emit(".num_zero:");
        self.emit("    mov x0, #0");
        self.emit("    ret");
        self.emit(".num_ret:");
        self.emit("    ret");

        // str(x) -> String object (tag 3). Best-effort.
        self.emit("");
        self.emit("_str:");
        self.emit("    stp x29, x30, [sp, #-96]!");
        self.emit("    mov x29, sp");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    stp x23, x24, [sp, #48]");
        self.emit("    stp x25, x26, [sp, #64]");
        self.emit("    stp x27, x28, [sp, #80]");
        self.emit("    cbz x0, .str_null");
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .str_from_num");
        self.emit("    ldr x1, [x0, #0]");
        self.emit("    cmp x1, #3");
        self.emit("    b.eq .str_ret");
        self.emit("    b .str_object");
        self.emit(".str_ret:");
        self.emit("    b .str_epilogue");
        self.emit(".str_null:");
        self.emit("    adrp x0, .t_null@PAGE");
        self.emit("    add x0, x0, .t_null@PAGEOFF");
        self.emit("    b .str_epilogue");
        self.emit(".str_object:");
        self.emit("    adrp x0, .t_object@PAGE");
        self.emit("    add x0, x0, .t_object@PAGEOFF");
        self.emit("    b .str_epilogue");
        self.emit(".str_from_num:");
        // Allocate String Object: [Tag=3][RC=1][Len][CharData(256)]
        self.emit("    mov x20, x0"); // number
        self.emit("    mov x0, #288");
        self.emit("    bl _malloc");
        self.emit("    mov x19, x0"); // str obj
        self.emit("    mov x0, #3");
        self.emit("    str x0, [x19, #0]");
        self.emit("    mov x0, #1");
        self.emit("    str x0, [x19, #8]");
        self.emit("    add x21, x19, #24"); // dest buffer start
        self.emit("    add x22, x21, #255"); // write ptr at end
        self.emit("    mov x23, #0"); // len
        self.emit("    mov x24, x20"); // work value
        self.emit("    mov x25, #0"); // neg flag
        self.emit("    cmp x24, #0");
        self.emit("    b.ge .itoa_abs");
        self.emit("    neg x24, x24");
        self.emit("    mov x25, #1");
        self.emit(".itoa_abs:");
        self.emit("    cbnz x24, .itoa_loop");
        self.emit("    mov w0, #48"); // '0'
        self.emit("    strb w0, [x22]");
        self.emit("    sub x22, x22, #1");
        self.emit("    add x23, x23, #1");
        self.emit("    b .itoa_done_digits");
        self.emit(".itoa_loop:");
        self.emit("    mov x26, #10");
        self.emit("    udiv x27, x24, x26");
        self.emit("    msub x28, x27, x26, x24"); // rem = x24 - q*10
        self.emit("    add x28, x28, #48");
        self.emit("    strb w28, [x22]");
        self.emit("    sub x22, x22, #1");
        self.emit("    add x23, x23, #1");
        self.emit("    mov x24, x27");
        self.emit("    cbnz x24, .itoa_loop");
        self.emit(".itoa_done_digits:");
        self.emit("    cbz x25, .itoa_copy");
        self.emit("    mov w0, #45"); // '-'
        self.emit("    strb w0, [x22]");
        self.emit("    sub x22, x22, #1");
        self.emit("    add x23, x23, #1");
        self.emit(".itoa_copy:");
        self.emit("    add x22, x22, #1"); // start ptr
        self.emit("    mov x24, #0"); // i
        self.emit(".itoa_copy_loop:");
        self.emit("    cmp x24, x23");
        self.emit("    b.ge .itoa_copy_done");
        self.emit("    ldrb w0, [x22, x24]");
        self.emit("    strb w0, [x21, x24]");
        self.emit("    add x24, x24, #1");
        self.emit("    b .itoa_copy_loop");
        self.emit(".itoa_copy_done:");
        self.emit("    add x0, x21, x23");
        self.emit("    mov w1, #0");
        self.emit("    strb w1, [x0]");
        self.emit("    str x23, [x19, #16]"); // len
        self.emit("    mov x0, x19");
        self.emit("    b .str_epilogue");

        self.emit(".str_epilogue:");
        self.emit("    ldp x27, x28, [sp, #80]");
        self.emit("    ldp x25, x26, [sp, #64]");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #96");
        self.emit("    ret");

        self.emit("");
        self.emit("_set_struct_member:");
        // x0=obj, x1=name_str, x2=value
        self.emit("    b _set_map_generic");

        self.emit("");
        self.emit("_get_struct_member:");
        // x0=obj, x1=name_str
        self.emit("    b _get_map_generic");

        // Generic Map/Struct helper
        self.emit("");
        self.emit("_set_map_generic:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    mov x19, x0"); // obj
        self.emit("    mov x20, x1"); // key
        self.emit("    mov x21, x2"); // val

        self.emit("    ldr x22, [x19, #16]"); // count
        self.emit("    ldr x23, [x19, #24]"); // entries

        // Increment RC
        self.emit("    mov x0, x20");
        self.emit("    bl _inc_rc");
        self.emit("    mov x0, x21");
        self.emit("    bl _inc_rc");

        self.emit("    add x24, x22, #1");
        self.emit("    lsl x0, x24, #4");
        self.emit("    mov x25, x0");
        self.emit("    mov x0, x23");
        self.emit("    mov x1, x25");
        self.emit("    bl _realloc");
        self.emit("    str x0, [x19, #24]");
        self.emit("    mov x23, x0");

        self.emit("    lsl x26, x22, #4");
        self.emit("    add x27, x23, x26");
        self.emit("    str x20, [x27, #0]");
        self.emit("    str x21, [x27, #8]");
        self.emit("    str x24, [x19, #16]");

        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        self.emit("");
        self.emit("_get_map_generic:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    mov x19, x0"); // obj
        self.emit("    mov x20, x1"); // key (string label)

        self.emit("    ldr x21, [x19, #16]"); // count
        self.emit("    ldr x22, [x19, #24]"); // entries
        self.emit("    mov x23, #0"); // index

        self.emit(".get_map_loop:");
        self.emit("    cmp x23, x21");
        self.emit("    b.ge .get_map_not_found");

        self.emit("    lsl x24, x23, #4");
        self.emit("    add x24, x22, x24");
        self.emit("    ldr x0, [x24, #0]"); // current key
        self.emit("    mov x1, x20"); // target key
        self.emit("    bl _strcmp"); // We'll need a real strcmp
        self.emit("    cbz x0, .get_map_found");

        self.emit("    add x23, x23, #1");
        self.emit("    b .get_map_loop");

        self.emit(".get_map_found:");
        self.emit("    lsl x24, x23, #4");
        self.emit("    add x24, x22, x24");
        self.emit("    ldr x0, [x24, #8]"); // value
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        self.emit(".get_map_not_found:");
        self.emit("    mov x0, #0");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        self.emit("");
        self.emit("_strcmp:");
        // Simple strcmp: loop until mismatch or null
        let strcmp_loop = self.fresh_label();
        let strcmp_mismatch = self.fresh_label();
        let strcmp_equal = self.fresh_label();
        self.emit(&format!("{}:", strcmp_loop));
        self.emit("    ldrb w2, [x0]");
        self.emit("    ldrb w3, [x1]");
        self.emit("    cmp w2, w3");
        self.emit(&format!("    b.ne {}", strcmp_mismatch));
        self.emit(&format!("    cbz w2, {}", strcmp_equal));
        self.emit("    add x0, x0, #1");
        self.emit("    add x1, x1, #1");
        self.emit(&format!("    b {}", strcmp_loop));
        self.emit(&format!("{}:", strcmp_mismatch));
        self.emit("    mov x0, #1");
        self.emit("    ret");
        self.emit(&format!("{}:", strcmp_equal));
        self.emit("    mov x0, #0");
        self.emit("    ret");

        self.emit("");
        self.emit("_keys:");
        self.emit("    stp x29, x30, [sp, #-64]!");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    stp x23, x24, [sp, #48]");
        self.emit("    mov x29, sp");
        self.emit("    cbz x0, .keys_null");

        // Map header: [Tag=2][RC][Count][EntriesPtr]
        self.emit("    ldr x19, [x0, #16]"); // count
        self.emit("    ldr x20, [x0, #24]"); // entries

        // Allocate list: [Tag=1][RC][Length][DataPtr]
        self.emit("    mov x21, x19"); // length
        self.emit("    mov x0, #32");
        self.emit("    bl _malloc");
        self.emit("    mov x22, x0");
        self.emit("    mov x25, #1"); // Tag List
        self.emit("    str x25, [x22, #0]");
        self.emit("    mov x25, #1"); // RefCount
        self.emit("    str x25, [x22, #8]");
        self.emit("    str x21, [x22, #16]");

        self.emit("    lsl x0, x21, #3");
        self.emit("    bl _malloc");
        self.emit("    str x0, [x22, #24]");
        self.emit("    mov x23, x0");

        self.emit("    mov x24, #0");
        self.emit(".keys_loop:");
        self.emit("    cmp x24, x21");
        self.emit("    b.ge .keys_done");
        self.emit("    lsl x25, x24, #4");
        self.emit("    ldr x0, [x20, x25]");
        self.emit("    bl _inc_rc"); // Increment RC for key
        self.emit("    lsl x25, x24, #3");
        self.emit("    str x0, [x23, x25]");
        self.emit("    add x24, x24, #1");
        self.emit("    b .keys_loop");

        self.emit(".keys_done:");
        self.emit("    mov x0, x22");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");
        self.emit(".keys_null:");
        self.emit("    mov x0, #0");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");

        self.emit("");
        self.emit("_range:");
        self.emit("    stp x29, x30, [sp, #-48]!");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    mov x29, sp");

        self.emit("    sub x21, x1, x0");
        self.emit("    cmp x21, #0");
        self.emit("    b.lt .range_empty");
        self.emit("    mov x19, x0");
        self.emit("    mov x20, x1");

        self.emit("    mov x0, #32");
        self.emit("    bl _malloc");
        self.emit("    mov x22, x0");
        self.emit("    mov x25, #1"); // Tag List
        self.emit("    str x25, [x22, #0]");
        self.emit("    mov x25, #1"); // RefCount
        self.emit("    str x25, [x22, #8]");
        self.emit("    str x21, [x22, #16]");

        self.emit("    lsl x0, x21, #3");
        self.emit("    bl _malloc");
        self.emit("    str x0, [x22, #24]");
        self.emit("    mov x23, x0");

        self.emit("    mov x24, #0");
        self.emit(".range_loop:");
        self.emit("    cmp x24, x21");
        self.emit("    b.ge .range_done");
        self.emit("    add x25, x19, x24");
        self.emit("    lsl x26, x24, #3");
        self.emit("    str x25, [x23, x26]");
        self.emit("    add x24, x24, #1");
        self.emit("    b .range_loop");

        self.emit(".range_done:");
        self.emit("    mov x0, x22");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");
        self.emit(".range_empty:");
        self.emit("    mov x0, #0");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #48");
        self.emit("    ret");

        self.emit("");
        self.emit("_values:");
        self.emit("    stp x29, x30, [sp, #-64]!");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    stp x23, x24, [sp, #48]");
        self.emit("    mov x29, sp");
        self.emit("    cbz x0, .values_null");

        self.emit("    ldr x19, [x0, #16]"); // count
        self.emit("    ldr x20, [x0, #24]"); // entries

        self.emit("    mov x21, x19"); // length
        self.emit("    mov x0, #32");
        self.emit("    bl _malloc");
        self.emit("    mov x22, x0");
        self.emit("    mov x25, #1"); // Tag List
        self.emit("    str x25, [x22, #0]");
        self.emit("    mov x25, #1"); // RefCount
        self.emit("    str x25, [x22, #8]");
        self.emit("    str x21, [x22, #16]");

        self.emit("    lsl x0, x21, #3");
        self.emit("    bl _malloc");
        self.emit("    str x0, [x22, #24]");
        self.emit("    mov x23, x0");

        self.emit("    mov x24, #0");
        self.emit(".values_loop:");
        self.emit("    cmp x24, x21");
        self.emit("    b.ge .values_done");
        self.emit("    lsl x25, x24, #4");
        self.emit("    add x25, x20, x25");
        self.emit("    ldr x0, [x25, #8]"); // value is at offset 8
        self.emit("    bl _inc_rc");
        self.emit("    lsl x25, x24, #3");
        self.emit("    str x0, [x23, x25]");
        self.emit("    add x24, x24, #1");
        self.emit("    b .values_loop");

        self.emit(".values_done:");
        self.emit("    mov x0, x22");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");
        self.emit(".values_null:");
        self.emit("    mov x0, #0");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");

        // Helper: _get_index(target, index)
        self.emit("");
        self.emit("_get_index:");
        self.emit("    stp x29, x30, [sp, #-64]!");
        self.emit("    stp x19, x20, [sp, #16]");
        self.emit("    stp x21, x22, [sp, #32]");
        self.emit("    stp x23, x24, [sp, #48]");
        self.emit("    mov x29, sp");
        self.emit("    cbz x0, .get_null");

        self.emit("    mov x19, x0");
        self.emit("    mov x20, x1");

        self.emit("    ldr x0, [x19, #0]"); // Tag
        self.emit("    cmp x0, #1"); // List
        self.emit("    b.eq .get_list");
        self.emit("    cmp x0, #2"); // Map
        self.emit("    b.eq .get_map");
        self.emit("    b .get_null");

        self.emit(".get_list:");
        self.emit("    ldr x2, [x19, #16]"); // len
        self.emit("    cmp x20, x2");
        self.emit("    b.ge .get_null");
        self.emit("    ldr x2, [x19, #24]"); // data
        self.emit("    ldr x0, [x2, x20, lsl #3]");
        self.emit("    bl _inc_rc");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");

        self.emit(".get_map:");
        self.emit("    ldr x21, [x19, #16]"); // count
        self.emit("    ldr x22, [x19, #24]"); // entries
        self.emit("    mov x23, #0");
        self.emit(".map_get_loop:");
        self.emit("    cmp x23, x21");
        self.emit("    b.ge .get_null");

        self.emit("    lsl x24, x23, #4");
        self.emit("    add x24, x22, x24");
        self.emit("    ldr x0, [x24, #0]"); // key pointer
        self.emit("    mov x1, x20"); // search key

        // Fast address check
        self.emit("    cmp x0, x1");
        self.emit("    b.eq .map_get_found");

        // Fallback: strcmp check if both are pointers > 0x1000
        self.emit("    cmp x0, #0x1000");
        self.emit("    b.lo .map_get_next");
        self.emit("    cmp x1, #0x1000");
        self.emit("    b.lo .map_get_next");

        // Tag check: strings have tag 3
        self.emit("    ldr x2, [x0, #0]");
        self.emit("    cmp x2, #3");
        self.emit("    b.ne .map_get_next");
        self.emit("    ldr x2, [x1, #0]");
        self.emit("    cmp x2, #3");
        self.emit("    b.ne .map_get_next");

        // Call strcmp on char data (offset 24)
        self.emit("    add x0, x0, #24");
        self.emit("    add x1, x1, #24");
        self.emit("    bl _strcmp");
        self.emit("    cbz x0, .map_get_found");

        self.emit(".map_get_next:");
        self.emit("    add x23, x23, #1");
        self.emit("    b .map_get_loop");

        self.emit(".map_get_found:");
        self.emit("    lsl x24, x23, #4");
        self.emit("    add x24, x22, x24");
        self.emit("    ldr x0, [x24, #8]"); // value
        self.emit("    bl _inc_rc");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");

        self.emit(".get_null:");
        self.emit("    mov x0, #0");
        self.emit("    ldp x23, x24, [sp, #48]");
        self.emit("    ldp x21, x22, [sp, #32]");
        self.emit("    ldp x19, x20, [sp, #16]");
        self.emit("    ldp x29, x30, [sp], #64");
        self.emit("    ret");

        // Async/Await stubs
        self.emit("");
        self.emit("_spawn:");
        self.emit("    // Async spawn not implemented in native codegen yet");
        self.emit("    ret");

        self.emit("");
        self.emit("_await:");
        self.emit("    // Async await not implemented in native codegen yet");
        self.emit("    ret");

        self.emit("");
        self.emit("_sleep:");
        self.emit("    // Sleep is a no-op in native/VM mode for now");
        self.emit("    ret");

        // File I/O stubs
        self.emit("");
        self.emit("_alloc_file:");
        self.emit("    // File allocation not implemented in native codegen yet");
        self.emit("    mov x0, #0");
        self.emit("    ret");

        self.emit("");
        self.emit("_close_file:");
        self.emit("    // File close not implemented in native codegen yet");
        self.emit("    ret");

        Ok(self.output.clone())
    }

    fn generate_data_section(&mut self, program: &IrProgram) {
        // Define newline symbol
        self.emit("    .align 3");
        self.emit(".newline:");
        self.emit("    .asciz \"\\n\"");
        self.emit("    .global .newline"); // Export newline symbol

        // Define format string for printf
        self.emit("    .align 3");
        self.emit(".fmt_int:");
        self.emit("    .asciz \"%ld\\n\"");
        self.emit("    .global .fmt_int");

        self.emit("    .align 3");
        self.emit(".fmt_int_raw:");
        self.emit("    .asciz \"%ld\"");
        self.emit("    .global .fmt_int_raw");

        self.emit("    .align 3");
        self.emit(".list_start:");
        self.emit("    .asciz \"[\"");
        self.emit("    .global .list_start");

        self.emit("    .align 3");
        self.emit(".list_end:");
        self.emit("    .asciz \"]\"");
        self.emit("    .global .list_end");

        self.emit("    .align 3");
        self.emit(".map_start:");
        self.emit("    .asciz \"{\"");
        self.emit("    .global .map_start");

        self.emit("    .align 3");
        self.emit(".map_end:");
        self.emit("    .asciz \"}\"");
        self.emit("    .global .map_end");

        self.emit("    .align 3");
        self.emit(".comma_space:");
        self.emit("    .asciz \", \"");
        self.emit("    .global .comma_space");

        self.emit("    .align 3");
        self.emit(".colon_space:");
        self.emit("    .asciz \": \"");
        self.emit("    .global .colon_space");

        // Type/Conversion helper strings (String objects)
        for (label, text) in [
            (".t_number", "number"),
            (".t_string", "string"),
            (".t_list", "list"),
            (".t_map", "map"),
            (".t_struct", "struct"),
            (".t_null", "null"),
            (".t_object", "object"),
            (".t_true", "true"),
            (".t_false", "false"),
        ] {
            self.emit("    .align 3");
            self.emit(&format!("{}:", label));
            self.emit("    .quad 3"); // Tag String
            self.emit("    .quad -1"); // RC Static
            self.emit(&format!("    .quad {}", text.len())); // Length
            self.emit(&format!("    .asciz \"{}\"", text));
            self.emit(&format!("    .global {}", label));
        }

        let mut string_id = 0;
        let mut string_literals = Vec::new();

        // Helper to collect strings from instructions
        fn collect_strings(instrs: &[IrInstr], strings: &mut Vec<String>) {
            for instr in instrs {
                match instr {
                    IrInstr::LoadConst {
                        value: IrValue::String(s),
                        ..
                    } => {
                        if !strings.contains(s) {
                            strings.push(s.clone());
                        }
                    }
                    IrInstr::Input { prompt, .. } => {
                        if !strings.contains(prompt) {
                            strings.push(prompt.clone());
                        }
                    }
                    _ => {}
                }
            }
        }

        collect_strings(&program.global_code, &mut string_literals);
        for func in program.functions.values() {
            collect_strings(&func.instructions, &mut string_literals);
        }

        // Emit labels and store in mapping
        for s in string_literals {
            let label = format!(".str{}", string_id);
            self.emit("    .align 3"); // Ensure 8-byte alignment for the header
            self.emit(&format!("{}:", label));
            self.emit("    .quad 3"); // Tag String
            self.emit("    .quad -1"); // RC Static
            self.emit(&format!("    .quad {}", s.len())); // Length
            self.emit(&format!("    .asciz \"{}\"", s));
            self.string_labels.insert(s.clone(), label);
            string_id += 1;
        }
    }

    fn precalculate_stack_offsets(&mut self, instructions: &[IrInstr], params: &[String]) {
        self.var_offsets.clear();
        // Use a Vec to preserve first-seen insertion order, giving deterministic offsets.
        let mut ordered_vars: Vec<String> = Vec::new();

        // Parameters come first
        for param in params {
            if !ordered_vars.contains(param) {
                ordered_vars.push(param.clone());
            }
        }

        // Walk instructions in order; record each dest the first time it appears
        for instr in instructions {
            let dest_opt: Option<&String> = match instr {
                IrInstr::LoadConst { dest, .. }
                | IrInstr::Move { dest, .. }
                | IrInstr::Add { dest, .. }
                | IrInstr::Sub { dest, .. }
                | IrInstr::Mul { dest, .. }
                | IrInstr::Div { dest, .. }
                | IrInstr::AllocList { dest, .. }
                | IrInstr::AllocMap { dest }
                | IrInstr::GetIndex { dest, .. }
                | IrInstr::Input { dest, .. }
                | IrInstr::Call {
                    dest: Some(dest), ..
                }
                | IrInstr::Eq { dest, .. }
                | IrInstr::Ne { dest, .. }
                | IrInstr::Lt { dest, .. }
                | IrInstr::Gt { dest, .. }
                | IrInstr::LogicAnd { dest, .. }
                | IrInstr::LogicOr { dest, .. }
                | IrInstr::LogicNot { dest, .. }
                | IrInstr::FAdd { dest, .. }
                | IrInstr::FSub { dest, .. }
                | IrInstr::FMul { dest, .. }
                | IrInstr::FDiv { dest, .. }
                | IrInstr::BitAnd { dest, .. }
                | IrInstr::BitOr { dest, .. }
                | IrInstr::BitXor { dest, .. }
                | IrInstr::BitNot { dest, .. }
                | IrInstr::Shl { dest, .. }
                | IrInstr::Shr { dest, .. }
                | IrInstr::AllocStruct { dest, .. }
                | IrInstr::GetMember { dest, .. }
                | IrInstr::Spawn { task: dest }
                | IrInstr::Await { dest, .. }
                | IrInstr::AllocFile { dest, .. } => Some(dest),
                _ => None,
            };
            if let Some(dest) = dest_opt {
                if !ordered_vars.contains(dest) {
                    ordered_vars.push(dest.clone());
                }
            }
        }

        // Assign offsets in stable first-seen order
        let mut offset = 0i32;
        for var in &ordered_vars {
            self.var_offsets.insert(var.clone(), offset);
            offset += 8;
        }

        // Align to 16 bytes
        self.stack_offset = (offset + 15) & !15;
    }

    fn generate_function_prologue(&mut self, params: &[String]) {
        self.emit("    stp x29, x30, [sp, #-16]!");
        self.emit("    mov x29, sp");

        if self.stack_offset > 0 {
            self.emit(&format!("    sub sp, sp, #{}", self.stack_offset));

            // Zero out stack frame for RC safety
            let num_slots = self.stack_offset / 8;
            for i in 0..num_slots {
                self.emit(&format!("    str xzr, [sp, #{}]", i * 8));
            }
        }

        // Store arguments on stack
        for (i, param) in params.iter().enumerate() {
            if i < 8 {
                let reg = format!("x{}", i);
                let offset = self.var_offsets.get(param).unwrap();
                self.emit(&format!("    str {}, [sp, #{}]", reg, offset));
            }
        }
    }

    fn generate_function_epilogue(&mut self) {
        self.emit(&format!("{}:", self.epilogue_label));

        // Save return value
        self.emit("    sub sp, sp, #16");
        self.emit("    str x0, [sp, #0]");

        // Dec RC for all local variables
        let mut offsets: Vec<i32> = self.var_offsets.values().cloned().collect();
        offsets.sort();
        for offset in offsets {
            // Need to adjust offset because we just pushed return value
            self.emit(&format!("    ldr x0, [sp, #{}]", offset + 16));
            self.emit("    bl _dec_rc");
        }

        // Restore return value
        self.emit("    ldr x0, [sp, #0]");
        self.emit("    add sp, sp, #16");

        if self.stack_offset > 0 {
            self.emit(&format!("    add sp, sp, #{}", self.stack_offset));
        }

        self.emit("    ldp x29, x30, [sp], #16");
        self.emit("    ret");
    }

    fn generate_instruction(&mut self, instr: &IrInstr) -> Result<(), String> {
        match instr {
            IrInstr::Label { name } => {
                let sanitized = self.sanitize(name);
                self.emit(&format!("{}:", sanitized));
            }
            IrInstr::Jump { label } => {
                self.emit(&format!("    b {}", self.sanitize(label)));
            }
            IrInstr::JumpIf { cond, label } => {
                self.load_var("x0", cond);
                self.emit("    cmp x0, #0");
                self.emit(&format!("    b.ne {}", self.sanitize(label)));
            }
            IrInstr::LoadConst { dest, value } => {
                match value {
                    IrValue::Number(n) => {
                        // Load immediate number into register
                        let int_val = *n as i64;
                        self.emit(&format!("    mov x0, #{}", int_val));
                        self.store_var(dest, "x0");
                    }
                    IrValue::String(s) => {
                        // Load string address
                        let label = self
                            .string_labels
                            .get(s)
                            .map(|l| l.clone())
                            .unwrap_or_else(|| ".str0".to_string());
                        self.emit(&format!("    adrp x0, {}@PAGE", label));
                        self.emit(&format!("    add x0, x0, {}@PAGEOFF", label));
                        self.store_var(dest, "x0");
                    }
                    IrValue::Bool(b) => {
                        let val = if *b { 1 } else { 0 };
                        self.emit(&format!("    mov x0, #{}", val));
                        self.store_var(dest, "x0");
                    }
                }
            }
            IrInstr::Add { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    add x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::Sub { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    sub x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::Mul { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    mul x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::Div { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    sdiv x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::Print { src } => {
                self.load_var("x0", src);
                self.emit("    bl _print_val");

                // Print newline
                self.emit("    adrp x1, .newline@PAGE");
                self.emit("    add x1, x1, .newline@PAGEOFF");
                self.emit("    mov x2, #1");
                self.emit("    mov x0, #1");
                self.emit("    mov x16, #4");
                self.emit("    svc #0x80");
            }
            IrInstr::PrintNum { src } => {
                self.load_var("x1", src);
                self.emit("    bl _print_num");
            }
            IrInstr::Call { dest, func, args } => {
                if args.len() > 8 {
                    return Err(format!(
                        "ARM64 backend supports up to 8 call arguments (got {})",
                        args.len()
                    ));
                }

                // Load arguments into x0-x7
                for (i, arg) in args.iter().enumerate() {
                    let reg = format!("x{}", i);
                    self.load_var(&reg, arg);
                }

                // Borrow arguments for the duration of the call (callee epilogue will dec its params)
                if !args.is_empty() {
                    self.emit("    mov x19, x0"); // save arg0
                    for i in 1..args.len() {
                        self.emit(&format!("    mov x0, x{}", i));
                        self.emit("    bl _inc_rc");
                    }
                    self.emit("    mov x0, x19");
                    self.emit("    bl _inc_rc");
                }

                let target = match func.as_str() {
                    "len" | "keys" | "values" | "range" | "spawn" | "sleep" | "type" | "is_map"
                    | "is_list" | "is_string" | "str" | "num" | "bool" => {
                        format!("_{}", self.sanitize(func))
                    }
                    _ => format!("_user_{}", self.sanitize(func)),
                };

                self.emit(&format!("    bl {}", target));

                match dest {
                    Some(d) => self.store_var(d, "x0"),
                    None => {
                        // Drop unused return value
                        self.emit("    bl _dec_rc");
                    }
                }
            }
            IrInstr::Return { value } => {
                if let Some(v) = value {
                    self.load_var("x0", v);
                    // Ensure returned pointer survives local RC decrements in epilogue
                    self.emit("    bl _inc_rc");
                } else {
                    self.emit("    mov x0, #0");
                }
                self.emit(&format!("    b {}", self.epilogue_label));
            }
            IrInstr::Input { dest, prompt } => {
                // Print prompt using _print_val
                let label_opt = self.string_labels.get(prompt).cloned();
                if let Some(label) = label_opt {
                    self.emit(&format!("    adrp x0, {}@PAGE", label));
                    self.emit(&format!("    add x0, x0, {}@PAGEOFF", label));
                } else {
                    // It's likely an identifier/variable, load it from stack
                    self.load_var("x0", prompt);
                }
                self.emit("    bl _print_val");

                // Allocate String Object: [Tag=3][RC=1][Len=0][CharData(256)]
                self.emit("    mov x0, #288"); // 32 bytes header + 256 bytes data
                self.emit("    bl _malloc");
                self.emit("    mov x19, x0");

                self.emit("    mov x0, #3"); // Tag String
                self.emit("    str x0, [x19, #0]");
                self.emit("    mov x0, #1"); // RefCount = 1
                self.emit("    str x0, [x19, #8]");

                // Read from stdin into x19 + 24 (CharData offset)
                self.emit("    add x1, x19, #24");
                self.emit("    mov x2, #255");
                self.emit("    mov x0, #0"); // stdin
                self.emit("    mov x16, #3"); // read syscall
                self.emit("    svc #0x80");

                // x0 = bytes read. Null-terminate and remove newline.
                self.emit("    mov x2, x0");
                self.emit("    add x1, x19, #24");

                let input_null_term = self.fresh_label();
                let input_no_nl = self.fresh_label();
                let input_store_len = self.fresh_label();

                self.emit(&format!("    cbz x2, {}", input_null_term));

                // Remove newline if present
                self.emit("    sub x2, x2, #1"); // index of last char
                self.emit("    add x4, x1, x2"); // addr of last char
                self.emit("    ldrb w3, [x4]"); // load last char
                self.emit("    cmp w3, #10"); // is it \n?
                self.emit(&format!("    b.ne {}", input_no_nl));
                self.emit("    mov w3, #0"); // replace with \0
                self.emit("    strb w3, [x4]");
                self.emit(&format!("    b {}", input_store_len));
                self.emit(&format!("{}:", input_no_nl));
                self.emit("    add x2, x2, #1"); // restore length if no \n removed
                self.emit("    add x4, x1, x2");
                self.emit("    mov w3, #0");
                self.emit("    strb w3, [x4]"); // null-terminate after last char
                self.emit(&format!("{}:", input_store_len));

                // Store final length in x19[16]
                self.emit("    str x2, [x19, #16]");

                self.emit(&format!("{}:", input_null_term));

                // Dec RC for old value in dest
                self.load_var("x0", dest);
                self.emit("    bl _dec_rc");

                self.store_var(dest, "x19");

                // RC of the new object (x19) is 1 from malloc.
                // Storing it in dest "transfers" this ownership.
                // If we incremented it, we'd need another dec later.
                // The issue is most likely in epilogue or function calls.
            }
            IrInstr::Eq { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    cmp x0, x1");
                self.emit("    cset x0, eq");
                self.store_var(dest, "x0");
            }
            IrInstr::Ne { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    cmp x0, x1");
                self.emit("    cset x0, ne");
                self.store_var(dest, "x0");
            }
            IrInstr::Lt { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    cmp x0, x1");
                self.emit("    cset x0, lt");
                self.store_var(dest, "x0");
            }
            IrInstr::Gt { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    cmp x0, x1");
                self.emit("    cset x0, gt");
                self.store_var(dest, "x0");
            }
            IrInstr::LogicAnd { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    cmp x0, #0");
                self.emit("    cset w2, ne");
                self.emit("    cmp x1, #0");
                self.emit("    cset w3, ne");
                self.emit("    and w0, w2, w3");
                self.store_var(dest, "x0");
            }
            IrInstr::LogicOr { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    cmp x0, #0");
                self.emit("    cset w2, ne");
                self.emit("    cmp x1, #0");
                self.emit("    cset w3, ne");
                self.emit("    orr w0, w2, w3");
                self.store_var(dest, "x0");
            }
            IrInstr::LogicNot { dest, src } => {
                self.load_var("x0", src);
                self.emit("    cmp x0, #0");
                self.emit("    cset x0, eq");
                self.store_var(dest, "x0");
            }
            IrInstr::FAdd { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    fmov d0, x0");
                self.emit("    fmov d1, x1");
                self.emit("    fadd d0, d0, d1");
                self.emit("    fmov x0, d0");
                self.store_var(dest, "x0");
            }
            IrInstr::FSub { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    fmov d0, x0");
                self.emit("    fmov d1, x1");
                self.emit("    fsub d0, d0, d1");
                self.emit("    fmov x0, d0");
                self.store_var(dest, "x0");
            }
            IrInstr::FMul { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    fmov d0, x0");
                self.emit("    fmov d1, x1");
                self.emit("    fmul d0, d0, d1");
                self.emit("    fmov x0, d0");
                self.store_var(dest, "x0");
            }
            IrInstr::FDiv { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    fmov d0, x0");
                self.emit("    fmov d1, x1");
                self.emit("    fdiv d0, d0, d1");
                self.emit("    fmov x0, d0");
                self.store_var(dest, "x0");
            }
            IrInstr::BitAnd { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    and x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::BitOr { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    orr x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::BitXor { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    eor x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::BitNot { dest, src } => {
                self.load_var("x0", src);
                self.emit("    mvn x0, x0");
                self.store_var(dest, "x0");
            }
            IrInstr::Shl { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    lsl x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::Shr { dest, left, right } => {
                self.load_var("x0", left);
                self.load_var("x1", right);
                self.emit("    lsr x0, x0, x1");
                self.store_var(dest, "x0");
            }
            IrInstr::AllocStruct { dest, name: _ } => {
                // Struct: [Tag=4][RefCount][NamePtr][FieldsMapPtr]
                self.emit("    mov x0, #32");
                self.emit("    bl _malloc");
                self.emit("    mov x19, x0");
                self.emit("    mov x0, #4"); // Tag Struct
                self.emit("    str x0, [x19, #0]");
                self.emit("    mov x0, #1"); // RC
                self.emit("    str x0, [x19, #8]");
                // Metadata for name could go here, for now use name as label if needed
                self.emit("    str xzr, [x19, #16]"); // Count
                self.emit("    str xzr, [x19, #24]"); // Entries
                self.store_var(dest, "x19");
            }
            IrInstr::SetMember { obj, member, value } => {
                self.load_var("x19", obj);
                self.load_var("x21", value);
                // For simplicity, structs are dynamic maps in this POC
                // and we reuse SetMap logic.
                // We'll need a string label for the member name
                let label = self.fresh_label();
                self.emit(".data");
                self.emit(&format!("{}: .asciz \"{}\"", label, member));
                self.emit(".text");
                self.emit(&format!("    adrp x20, {}@PAGE", label));
                self.emit(&format!("    add x20, x20, {}@PAGEOFF", label));
                // Call internal helper _set_struct_member(obj, member_name_str, value)
                self.emit("    mov x0, x19");
                self.emit("    mov x1, x20");
                self.emit("    mov x2, x21");
                self.emit("    bl _set_struct_member");
            }
            IrInstr::GetMember { dest, obj, member } => {
                self.load_var("x19", obj);
                let label = self.fresh_label();
                self.emit(".data");
                self.emit(&format!("{}: .asciz \"{}\"", label, member));
                self.emit(".text");
                self.emit(&format!("    adrp x20, {}@PAGE", label));
                self.emit(&format!("    add x20, x20, {}@PAGEOFF", label));
                self.emit("    mov x0, x19");
                self.emit("    mov x1, x20");
                self.emit("    bl _get_struct_member");
                self.store_var(dest, "x0");
            }
            IrInstr::AllocList { dest, items } => {
                // List: [Tag=1][RefCount][Length][DataPtr]
                self.emit("    mov x0, #32");
                self.emit("    bl _malloc");
                self.emit("    mov x19, x0");

                self.emit("    mov x0, #1"); // Tag List
                self.emit("    str x0, [x19, #0]");
                self.emit("    mov x0, #1"); // RefCount = 1
                self.emit("    str x0, [x19, #8]");

                let len = items.len() as i64;
                self.emit(&format!("    mov x0, #{}", len));
                self.emit("    str x0, [x19, #16]");

                self.emit(&format!("    mov x0, #{}", len * 8));
                self.emit("    bl _malloc");
                self.emit("    str x0, [x19, #24]");
                self.emit("    mov x20, x0");

                for (i, item) in items.iter().enumerate() {
                    self.load_var("x0", item);
                    self.emit("    bl _inc_rc");
                    self.emit(&format!("    str x0, [x20, #{}]", i * 8));
                }

                self.store_var(dest, "x19");
            }
            IrInstr::AllocMap { dest } => {
                // Map: [Tag=2][RefCount][Count][EntriesPtr]
                self.emit("    mov x0, #32");
                self.emit("    bl _malloc");
                self.emit("    mov x19, x0");
                self.emit("    mov x0, #2"); // Tag Map
                self.emit("    str x0, [x19, #0]");
                self.emit("    mov x0, #1"); // RefCount = 1
                self.emit("    str x0, [x19, #8]");
                self.emit("    str xzr, [x19, #16]");
                self.emit("    str xzr, [x19, #24]");
                self.store_var(dest, "x19");
            }
            IrInstr::GetMap { dest, map, key } => {
                self.load_var("x0", map);
                self.load_var("x1", key);
                self.emit("    bl _get_map_generic");
                // Map retains ownership; caller receives a new reference
                self.emit("    bl _inc_rc");
                self.store_var(dest, "x0");
            }
            IrInstr::SetMap { map, key, value } => {
                self.load_var("x19", map);
                self.load_var("x20", key);
                self.load_var("x21", value);

                // Header: [Tag][RC][Count][EntriesPtr]
                self.emit("    ldr x22, [x19, #16]"); // count
                self.emit("    ldr x23, [x19, #24]"); // entries

                // For now, always append (but we should check for duplicates)
                // In a real Map, we'd dec old key/value if overwriting.
                // Sinceเรา just append, we just inc new key/value.
                self.emit("    mov x0, x20");
                self.emit("    bl _inc_rc");
                self.emit("    mov x0, x21");
                self.emit("    bl _inc_rc");

                self.emit("    add x24, x22, #1");
                self.emit("    lsl x0, x24, #4");
                self.emit("    mov x25, x0");
                self.emit("    mov x0, x23");
                self.emit("    mov x1, x25");
                self.emit("    bl _realloc");
                self.emit("    str x0, [x19, #24]");
                self.emit("    mov x23, x0");

                self.emit("    lsl x26, x22, #4");
                self.emit("    add x27, x23, x26");
                self.emit("    str x20, [x27, #0]");
                self.emit("    str x21, [x27, #8]");

                self.emit("    str x24, [x19, #16]");
            }
            IrInstr::GetIndex { dest, src, index } => {
                self.load_var("x0", src);
                self.load_var("x1", index);
                self.emit("    bl _get_index");
                self.store_var(dest, "x0");
            }
            IrInstr::SetIndex { src, index, value } => {
                let end_label = self.fresh_label();
                let set_list_label = self.fresh_label();
                let set_map_label = self.fresh_label();
                let err_label = self.fresh_label();

                self.load_var("x19", src);
                self.load_var("x20", index);
                self.load_var("x21", value);

                self.emit(&format!("    cbz x19, {}", err_label));
                self.emit("    ldr x0, [x19, #0]"); // Tag
                self.emit("    cmp x0, #1"); // List
                self.emit(&format!("    b.eq {}", set_list_label));
                self.emit("    cmp x0, #2"); // Map
                self.emit(&format!("    b.eq {}", set_map_label));
                self.emit(&format!("    b {}", err_label));

                self.emit(&format!("{}:", set_list_label));
                self.emit("    ldr x22, [x19, #16]"); // len
                self.emit("    cmp x20, x22");
                self.emit(&format!("    b.ge {}", err_label));

                self.emit("    ldr x23, [x19, #24]"); // data
                                                      // Dec old value
                self.emit("    ldr x0, [x23, x20, lsl #3]");
                self.emit("    bl _dec_rc");
                // Inc new value
                self.emit("    mov x0, x21");
                self.emit("    bl _inc_rc");
                // Store
                self.emit("    str x21, [x23, x20, lsl #3]");
                self.emit(&format!("    b {}", end_label));

                self.emit(&format!("{}:", set_map_label));
                // SetMap logic (simplified append for now)
                self.emit("    ldr x22, [x19, #16]"); // count
                self.emit("    ldr x23, [x19, #24]"); // entriesPtr

                // Inc key and value
                self.emit("    mov x0, x20");
                self.emit("    bl _inc_rc");
                self.emit("    mov x0, x21");
                self.emit("    bl _inc_rc");

                self.emit("    add x24, x22, #1");
                self.emit("    lsl x0, x24, #4");
                self.emit("    mov x25, x0");
                self.emit("    mov x0, x23");
                self.emit("    mov x1, x25");
                self.emit("    bl _realloc");
                self.emit("    str x0, [x19, #24]");
                self.emit("    mov x23, x0");

                self.emit("    lsl x26, x22, #4");
                self.emit("    add x27, x23, x26");
                self.emit("    str x20, [x27, #0]");
                self.emit("    str x21, [x27, #8]");
                self.emit("    str x24, [x19, #16]");

                self.emit(&format!("{}:", err_label));
                self.emit("    // Handle error (quietly for now)");
                self.emit(&format!("{}:", end_label));
            }
            IrInstr::Spawn { task } => {
                self.emit("    bl _spawn");
                self.store_var(task, "x0");
            }
            IrInstr::Await { dest, task } => {
                self.load_var("x0", task);
                self.emit("    bl _await");
                self.store_var(dest, "x0");
            }
            IrInstr::AllocFile { dest, path } => {
                // Load string address for path
                let label = self
                    .string_labels
                    .get(path)
                    .map(|l| l.clone())
                    .unwrap_or_else(|| ".str0".to_string());
                self.emit(&format!("    adrp x0, {}@PAGE", label));
                self.emit(&format!("    add x0, x0, {}@PAGEOFF", label));
                self.emit("    bl _alloc_file");
                self.store_var(dest, "x0");
            }
            IrInstr::CloseFile { handle } => {
                self.load_var("x0", handle);
                self.emit("    bl _close_file");
            }
            IrInstr::Move { dest, src } => {
                self.load_var("x0", src);
                self.store_var(dest, "x0");
            }
            IrInstr::LinkFile { .. } | IrInstr::Hardwire { .. } | IrInstr::PreScan { .. } => {
                // No-op in native codegen
            }
        }

        Ok(())
    }

    fn load_var(&mut self, reg: &str, var: &str) {
        if let Some(offset) = self.var_offsets.get(var) {
            self.emit(&format!("    ldr {}, [sp, #{}]", reg, offset));
        } else {
            self.emit(&format!(
                "    // Warning: variable {} not found in stack frame",
                var
            ));
            self.emit(&format!("    mov {}, #0", reg));
        }
    }

    fn store_var(&mut self, var: &str, reg: &str) {
        let offset = if let Some(off) = self.var_offsets.get(var) {
            *off
        } else {
            let off = self.var_offsets.len() as i32 * 8;
            self.var_offsets.insert(var.to_string(), off);
            off
        };
        self.emit(&format!("    str {}, [sp, #{}]", reg, offset));
    }

    pub fn write_to_file(&self, path: &str) -> std::io::Result<()> {
        let mut file = File::create(path)?;
        file.write_all(self.output.as_bytes())?;
        Ok(())
    }
}
