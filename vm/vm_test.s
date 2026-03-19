.global _main
.align 4

.data
    .align 3
.newline:
    .asciz "\n"
    .global .newline
    .align 3
.fmt_int:
    .asciz "%ld\n"
    .global .fmt_int
    .align 3
.fmt_int_raw:
    .asciz "%ld"
    .global .fmt_int_raw
    .align 3
.list_start:
    .asciz "["
    .global .list_start
    .align 3
.list_end:
    .asciz "]"
    .global .list_end
    .align 3
.map_start:
    .asciz "{"
    .global .map_start
    .align 3
.map_end:
    .asciz "}"
    .global .map_end
    .align 3
.comma_space:
    .asciz ", "
    .global .comma_space
    .align 3
.colon_space:
    .asciz ": "
    .global .colon_space
    .align 3
.str0:
    .quad 3
    .quad -1
    .quad 11
    .asciz "VM Success!"

.text
_main:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #16
    str xzr, [sp, #0]
    str xzr, [sp, #8]
    adrp x0, .str0@PAGE
    add x0, x0, .str0@PAGEOFF
    str x0, [sp, #0]
    ldr x0, [sp, #0]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    mov x0, #0
.L0:
    sub sp, sp, #16
    str x0, [sp, #0]
    ldr x0, [sp, #16]
    bl _dec_rc
    ldr x0, [sp, #0]
    add sp, sp, #16
    add sp, sp, #16
    ldp x29, x30, [sp], #16
    ret

_print_val:
    stp x29, x30, [sp, #-32]!
    str x19, [sp, #16]
    mov x29, sp
    mov x19, x0
    cmp x19, #0x1000
    b.hi .print_val_ptr
    mov x0, x19
    bl _print_num_no_nl
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.print_val_ptr:
    ldr x0, [x19, #0]
    cmp x0, #1
    b.eq .print_val_list
    cmp x0, #2
    b.eq .print_val_map
    cmp x0, #3
    b.eq .print_val_str
    mov x0, x19
    bl _print_str_no_nl
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.print_val_str:
    add x0, x19, #24
    bl _print_str_no_nl
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.print_val_list:
    mov x0, x19
    bl _print_list
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.print_val_map:
    mov x0, x19
    bl _print_map
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret

_inc_rc:
    cbz x0, 1f
    cmp x0, #0x1000
    b.lo 1f
    ldr x1, [x0, #8]
    cmp x1, #0
    b.lt 1f
    add x1, x1, #1
    str x1, [x0, #8]
1:  ret

_dec_rc:
    cbz x0, 1f
    cmp x0, #0x1000
    b.lo 1f
    ldr x1, [x0, #8]
    cmp x1, #0
    b.lt 1f
    sub x1, x1, #1
    str x1, [x0, #8]
    cbnz x1, 1f
    stp x29, x30, [sp, #-16]!
    bl _gc_free
    ldp x29, x30, [sp], #16
1:  ret

_gc_free:
    stp x29, x30, [sp, #-32]!
    str x19, [sp, #16]
    mov x29, sp
    mov x19, x0
    ldr x0, [x19, #0]
    cmp x0, #1
    b.eq .gc_free_list
    cmp x0, #2
    b.eq .gc_free_map
    cmp x0, #4
    b.eq .gc_free_struct
    mov x0, x19
    bl _free
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.gc_free_list:
    mov x0, x19
    bl _free_list
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.gc_free_map:
    mov x0, x19
    bl _free_map
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret
.gc_free_struct:
    mov x0, x19
    bl _free_map
    ldr x19, [sp, #16]
    ldp x29, x30, [sp], #32
    ret

_free_list:
    stp x29, x30, [sp, #-48]!
    stp x20, x21, [sp, #16]
    stp x22, x19, [sp, #32]
    mov x29, sp
    ldr x20, [x19, #16]
    ldr x21, [x19, #24]
    mov x22, #0
.free_list_loop:
    cmp x22, x20
    b.ge .free_list_done
    ldr x0, [x21, x22, lsl #3]
    bl _dec_rc
    add x22, x22, #1
    b .free_list_loop
.free_list_done:
    mov x0, x21
    bl _free
    mov x0, x19
    bl _free
    ldp x22, x19, [sp, #32]
    ldp x20, x21, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_free_map:
    stp x29, x30, [sp, #-48]!
    stp x20, x21, [sp, #16]
    stp x22, x19, [sp, #32]
    mov x29, sp
    ldr x20, [x19, #16]
    ldr x21, [x19, #24]
    mov x22, #0
.free_map_loop:
    cmp x22, x20
    b.ge .free_map_done
    lsl x23, x22, #4
    add x23, x21, x23
    ldr x0, [x23, #0]
    bl _dec_rc
    lsl x23, x22, #4
    add x23, x21, x23
    ldr x0, [x23, #8]
    bl _dec_rc
    add x22, x22, #1
    b .free_map_loop
.free_map_done:
    mov x0, x21
    bl _free
    mov x0, x19
    bl _free
    ldp x22, x19, [sp, #32]
    ldp x20, x21, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_print_num_no_nl:
    stp x29, x30, [sp, #-16]!
    mov x0, x0
    mov x1, x0
    sub sp, sp, #16
    str x1, [sp, #0]
    adrp x0, .fmt_int_raw@PAGE
    add x0, x0, .fmt_int_raw@PAGEOFF
    bl _printf
    add sp, sp, #16
    mov x0, #0
    bl _fflush
    ldp x29, x30, [sp], #16
    ret

_print_str_no_nl:
    stp x29, x30, [sp, #-16]!
    mov x1, x0
    mov x2, #0
1:
    ldrb w3, [x1, x2]
    cbz w3, 2f
    add x2, x2, #1
    b 1b
2:
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldp x29, x30, [sp], #16
    ret

_print_list:
    stp x29, x30, [sp, #-48]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    mov x19, x0
    adrp x1, .list_start@PAGE
    add x1, x1, .list_start@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x20, [x19, #16]
    ldr x21, [x19, #24]
    mov x22, #0
.print_list_loop:
    cmp x22, x20
    b.ge .print_list_done
    cbz x22, 1f
    adrp x1, .comma_space@PAGE
    add x1, x1, .comma_space@PAGEOFF
    mov x2, #2
    mov x0, #1
    mov x16, #4
    svc #0x80
1:
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    ldr x0, [x21, x22, lsl #3]
    bl _print_val
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    add x22, x22, #1
    b .print_list_loop
.print_list_done:
    adrp x1, .list_end@PAGE
    add x1, x1, .list_end@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_print_map:
    stp x29, x30, [sp, #-48]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    mov x19, x0
    adrp x1, .map_start@PAGE
    add x1, x1, .map_start@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x20, [x19, #16]
    ldr x21, [x19, #24]
    mov x22, #0
.print_map_loop:
    cmp x22, x20
    b.ge .print_map_done
    cbz x22, 1f
    adrp x1, .comma_space@PAGE
    add x1, x1, .comma_space@PAGEOFF
    mov x2, #2
    mov x0, #1
    mov x16, #4
    svc #0x80
1:
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    lsl x25, x22, #4
    ldr x0, [x21, x25]
    bl _print_val
    adrp x1, .colon_space@PAGE
    add x1, x1, .colon_space@PAGEOFF
    mov x2, #2
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldp x21, x22, [sp, #32]
    lsl x25, x22, #4
    add x25, x21, x25
    ldr x0, [x25, #8]
    stp x21, x22, [sp, #32]
    bl _print_val
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    add x22, x22, #1
    b .print_map_loop
.print_map_done:
    adrp x1, .map_end@PAGE
    add x1, x1, .map_end@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_print_num:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #16
    str x1, [sp, #0]
    adrp x0, .fmt_int@PAGE
    add x0, x0, .fmt_int@PAGEOFF
    bl _printf
    add sp, sp, #16
    mov x0, #0
    bl _fflush
    ldp x29, x30, [sp], #16
    ret

_len:
    stp x29, x30, [sp, #-16]!
    cbz x0, .len_null
    ldr x0, [x0, #16]
    ldp x29, x30, [sp], #16
    ret
.len_null:
    mov x0, #0
    ldp x29, x30, [sp], #16
    ret

_set_struct_member:
    b _set_map_generic

_get_struct_member:
    b _get_map_generic

_set_map_generic:
    stp x29, x30, [sp, #-48]!
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    mov x19, x0
    mov x20, x1
    mov x21, x2
    ldr x22, [x19, #16]
    ldr x23, [x19, #24]
    mov x0, x20
    bl _inc_rc
    mov x0, x21
    bl _inc_rc
    add x24, x22, #1
    lsl x0, x24, #4
    mov x25, x0
    mov x0, x23
    mov x1, x25
    bl _realloc
    str x0, [x19, #24]
    mov x23, x0
    lsl x26, x22, #4
    add x27, x23, x26
    str x20, [x27, #0]
    str x21, [x27, #8]
    str x24, [x19, #16]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_get_map_generic:
    stp x29, x30, [sp, #-48]!
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    mov x19, x0
    mov x20, x1
    ldr x21, [x19, #16]
    ldr x22, [x19, #24]
    mov x23, #0
.get_map_loop:
    cmp x23, x21
    b.ge .get_map_not_found
    lsl x24, x23, #4
    add x24, x22, x24
    ldr x0, [x24, #0]
    mov x1, x20
    bl _strcmp
    cbz x0, .get_map_found
    add x23, x23, #1
    b .get_map_loop
.get_map_found:
    lsl x24, x23, #4
    add x24, x22, x24
    ldr x0, [x24, #8]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret
.get_map_not_found:
    mov x0, #0
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_strcmp:
1:
    ldrb w2, [x0]
    ldrb w3, [x1]
    cmp w2, w3
    b.ne 2f
    cbz w2, 3f
    add x0, x0, #1
    add x1, x1, #1
    b 1b
2:
    mov x0, #1
    ret
3:
    mov x0, #0
    ret

_keys:
    stp x29, x30, [sp, #-64]!
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    stp x23, x24, [sp, #48]
    mov x29, sp
    cbz x0, .keys_null
    ldr x19, [x0, #16]
    ldr x20, [x0, #24]
    mov x21, x19
    mov x0, #32
    bl _malloc
    mov x22, x0
    mov x25, #1
    str x25, [x22, #0]
    mov x25, #1
    str x25, [x22, #8]
    str x21, [x22, #16]
    lsl x0, x21, #3
    bl _malloc
    str x0, [x22, #24]
    mov x23, x0
    mov x24, #0
.keys_loop:
    cmp x24, x21
    b.ge .keys_done
    lsl x25, x24, #4
    ldr x0, [x20, x25]
    bl _inc_rc
    lsl x25, x24, #3
    str x0, [x23, x25]
    add x24, x24, #1
    b .keys_loop
.keys_done:
    mov x0, x22
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret
.keys_null:
    mov x0, #0
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret

_range:
    stp x29, x30, [sp, #-48]!
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    mov x29, sp
    sub x21, x1, x0
    cmp x21, #0
    b.lt .range_empty
    mov x19, x0
    mov x20, x1
    mov x0, #32
    bl _malloc
    mov x22, x0
    mov x25, #1
    str x25, [x22, #0]
    mov x25, #1
    str x25, [x22, #8]
    str x21, [x22, #16]
    lsl x0, x21, #3
    bl _malloc
    str x0, [x22, #24]
    mov x23, x0
    mov x24, #0
.range_loop:
    cmp x24, x21
    b.ge .range_done
    add x25, x19, x24
    lsl x26, x24, #3
    str x25, [x23, x26]
    add x24, x24, #1
    b .range_loop
.range_done:
    mov x0, x22
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret
.range_empty:
    mov x0, #0
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #48
    ret

_values:
    stp x29, x30, [sp, #-64]!
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    stp x23, x24, [sp, #48]
    mov x29, sp
    cbz x0, .values_null
    ldr x19, [x0, #16]
    ldr x20, [x0, #24]
    mov x21, x19
    mov x0, #32
    bl _malloc
    mov x22, x0
    mov x25, #1
    str x25, [x22, #0]
    mov x25, #1
    str x25, [x22, #8]
    str x21, [x22, #16]
    lsl x0, x21, #3
    bl _malloc
    str x0, [x22, #24]
    mov x23, x0
    mov x24, #0
.values_loop:
    cmp x24, x21
    b.ge .values_done
    lsl x25, x24, #4
    add x25, x20, x25
    ldr x0, [x25, #8]
    bl _inc_rc
    lsl x25, x24, #3
    str x0, [x23, x25]
    add x24, x24, #1
    b .values_loop
.values_done:
    mov x0, x22
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret
.values_null:
    mov x0, #0
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret

_get_index:
    stp x29, x30, [sp, #-64]!
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    stp x23, x24, [sp, #48]
    mov x29, sp
    cbz x0, .get_null
    mov x19, x0
    mov x20, x1
    ldr x0, [x19, #0]
    cmp x0, #1
    b.eq .get_list
    cmp x0, #2
    b.eq .get_map
    b .get_null
.get_list:
    ldr x2, [x19, #16]
    cmp x20, x2
    b.ge .get_null
    ldr x2, [x19, #24]
    ldr x0, [x2, x20, lsl #3]
    bl _inc_rc
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret
.get_map:
    ldr x21, [x19, #16]
    ldr x22, [x19, #24]
    mov x23, #0
.map_get_loop:
    cmp x23, x21
    b.ge .get_null
    lsl x24, x23, #4
    add x24, x22, x24
    ldr x0, [x24, #0]
    mov x1, x20
    cmp x0, x1
    b.eq .map_get_found
    cmp x0, #0x1000
    b.lo .map_get_next
    cmp x1, #0x1000
    b.lo .map_get_next
    ldr x2, [x0, #0]
    cmp x2, #3
    b.ne .map_get_next
    ldr x2, [x1, #0]
    cmp x2, #3
    b.ne .map_get_next
    add x0, x0, #24
    add x1, x1, #24
    bl _strcmp
    cbz x0, .map_get_found
.map_get_next:
    add x23, x23, #1
    b .map_get_loop
.map_get_found:
    lsl x24, x23, #4
    add x24, x22, x24
    ldr x0, [x24, #8]
    bl _inc_rc
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret
.get_null:
    mov x0, #0
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #64
    ret

_spawn:
    ret

_await:
    ret

_alloc_file:
    mov x0, #0
    ret

_close_file:
    ret
