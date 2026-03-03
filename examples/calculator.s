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
.t_number:
    .quad 3
    .quad -1
    .quad 6
    .asciz "number"
    .global .t_number
    .align 3
.t_string:
    .quad 3
    .quad -1
    .quad 6
    .asciz "string"
    .global .t_string
    .align 3
.t_list:
    .quad 3
    .quad -1
    .quad 4
    .asciz "list"
    .global .t_list
    .align 3
.t_map:
    .quad 3
    .quad -1
    .quad 3
    .asciz "map"
    .global .t_map
    .align 3
.t_struct:
    .quad 3
    .quad -1
    .quad 6
    .asciz "struct"
    .global .t_struct
    .align 3
.t_null:
    .quad 3
    .quad -1
    .quad 4
    .asciz "null"
    .global .t_null
    .align 3
.t_object:
    .quad 3
    .quad -1
    .quad 6
    .asciz "object"
    .global .t_object
    .align 3
.t_true:
    .quad 3
    .quad -1
    .quad 4
    .asciz "true"
    .global .t_true
    .align 3
.t_false:
    .quad 3
    .quad -1
    .quad 5
    .asciz "false"
    .global .t_false
    .align 3
.str0:
    .quad 3
    .quad -1
    .quad 15
    .asciz "CoRe Calculator"
    .align 3
.str1:
    .quad 3
    .quad -1
    .quad 15
    .asciz "==============="
    .align 3
.str2:
    .quad 3
    .quad -1
    .quad 9
    .asciz "10 + 5 = "
    .align 3
.str3:
    .quad 3
    .quad -1
    .quad 9
    .asciz "10 - 5 = "
    .align 3
.str4:
    .quad 3
    .quad -1
    .quad 9
    .asciz "10 * 5 = "
    .align 3
.str5:
    .quad 3
    .quad -1
    .quad 9
    .asciz "10 / 5 = "
    .align 3
.str6:
    .quad 3
    .quad -1
    .quad 30
    .asciz "Complex: (2 * 3) + (10 / 2) = "

.text
_main:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #224
    str xzr, [sp, #0]
    str xzr, [sp, #8]
    str xzr, [sp, #16]
    str xzr, [sp, #24]
    str xzr, [sp, #32]
    str xzr, [sp, #40]
    str xzr, [sp, #48]
    str xzr, [sp, #56]
    str xzr, [sp, #64]
    str xzr, [sp, #72]
    str xzr, [sp, #80]
    str xzr, [sp, #88]
    str xzr, [sp, #96]
    str xzr, [sp, #104]
    str xzr, [sp, #112]
    str xzr, [sp, #120]
    str xzr, [sp, #128]
    str xzr, [sp, #136]
    str xzr, [sp, #144]
    str xzr, [sp, #152]
    str xzr, [sp, #160]
    str xzr, [sp, #168]
    str xzr, [sp, #176]
    str xzr, [sp, #184]
    str xzr, [sp, #192]
    str xzr, [sp, #200]
    str xzr, [sp, #208]
    str xzr, [sp, #216]
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
    adrp x0, .str1@PAGE
    add x0, x0, .str1@PAGEOFF
    str x0, [sp, #8]
    ldr x0, [sp, #8]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    mov x0, #10
    str x0, [sp, #16]
    ldr x0, [sp, #16]
    str x0, [sp, #24]
    mov x0, #5
    str x0, [sp, #32]
    ldr x0, [sp, #32]
    str x0, [sp, #40]
    ldr x0, [sp, #24]
    ldr x1, [sp, #40]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_add
    str x0, [sp, #48]
    ldr x0, [sp, #48]
    str x0, [sp, #56]
    adrp x0, .str2@PAGE
    add x0, x0, .str2@PAGEOFF
    str x0, [sp, #64]
    ldr x0, [sp, #64]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #56]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #24]
    ldr x1, [sp, #40]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_subtract
    str x0, [sp, #72]
    ldr x0, [sp, #72]
    str x0, [sp, #80]
    adrp x0, .str3@PAGE
    add x0, x0, .str3@PAGEOFF
    str x0, [sp, #88]
    ldr x0, [sp, #88]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #80]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #24]
    ldr x1, [sp, #40]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_multiply
    str x0, [sp, #96]
    ldr x0, [sp, #96]
    str x0, [sp, #104]
    adrp x0, .str4@PAGE
    add x0, x0, .str4@PAGEOFF
    str x0, [sp, #112]
    ldr x0, [sp, #112]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #104]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #24]
    ldr x1, [sp, #40]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_divide
    str x0, [sp, #120]
    ldr x0, [sp, #120]
    str x0, [sp, #128]
    adrp x0, .str5@PAGE
    add x0, x0, .str5@PAGEOFF
    str x0, [sp, #136]
    ldr x0, [sp, #136]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #128]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    mov x0, #2
    str x0, [sp, #144]
    mov x0, #3
    str x0, [sp, #152]
    ldr x0, [sp, #144]
    ldr x1, [sp, #152]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_multiply
    str x0, [sp, #160]
    mov x0, #10
    str x0, [sp, #168]
    mov x0, #2
    str x0, [sp, #176]
    ldr x0, [sp, #168]
    ldr x1, [sp, #176]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_divide
    str x0, [sp, #184]
    ldr x0, [sp, #160]
    ldr x1, [sp, #184]
    mov x19, x0
    mov x0, x1
    bl _inc_rc
    mov x0, x19
    bl _inc_rc
    bl _user_add
    str x0, [sp, #192]
    ldr x0, [sp, #192]
    str x0, [sp, #200]
    adrp x0, .str6@PAGE
    add x0, x0, .str6@PAGEOFF
    str x0, [sp, #208]
    ldr x0, [sp, #208]
    bl _print_val
    adrp x1, .newline@PAGE
    add x1, x1, .newline@PAGEOFF
    mov x2, #1
    mov x0, #1
    mov x16, #4
    svc #0x80
    ldr x0, [sp, #200]
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
    ldr x0, [sp, #24]
    bl _dec_rc
    ldr x0, [sp, #32]
    bl _dec_rc
    ldr x0, [sp, #40]
    bl _dec_rc
    ldr x0, [sp, #48]
    bl _dec_rc
    ldr x0, [sp, #56]
    bl _dec_rc
    ldr x0, [sp, #64]
    bl _dec_rc
    ldr x0, [sp, #72]
    bl _dec_rc
    ldr x0, [sp, #80]
    bl _dec_rc
    ldr x0, [sp, #88]
    bl _dec_rc
    ldr x0, [sp, #96]
    bl _dec_rc
    ldr x0, [sp, #104]
    bl _dec_rc
    ldr x0, [sp, #112]
    bl _dec_rc
    ldr x0, [sp, #120]
    bl _dec_rc
    ldr x0, [sp, #128]
    bl _dec_rc
    ldr x0, [sp, #136]
    bl _dec_rc
    ldr x0, [sp, #144]
    bl _dec_rc
    ldr x0, [sp, #152]
    bl _dec_rc
    ldr x0, [sp, #160]
    bl _dec_rc
    ldr x0, [sp, #168]
    bl _dec_rc
    ldr x0, [sp, #176]
    bl _dec_rc
    ldr x0, [sp, #184]
    bl _dec_rc
    ldr x0, [sp, #192]
    bl _dec_rc
    ldr x0, [sp, #200]
    bl _dec_rc
    ldr x0, [sp, #208]
    bl _dec_rc
    ldr x0, [sp, #216]
    bl _dec_rc
    ldr x0, [sp, #224]
    bl _dec_rc
    ldr x0, [sp, #0]
    add sp, sp, #16
    add sp, sp, #224
    ldp x29, x30, [sp], #16
    ret
_user_add:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    str xzr, [sp, #0]
    str xzr, [sp, #8]
    str xzr, [sp, #16]
    str xzr, [sp, #24]
    str x0, [sp, #0]
    str x1, [sp, #8]
    ldr x0, [sp, #0]
    ldr x1, [sp, #8]
    add x0, x0, x1
    str x0, [sp, #16]
    ldr x0, [sp, #16]
    bl _inc_rc
    b .L1
.L1:
    sub sp, sp, #16
    str x0, [sp, #0]
    ldr x0, [sp, #16]
    bl _dec_rc
    ldr x0, [sp, #24]
    bl _dec_rc
    ldr x0, [sp, #32]
    bl _dec_rc
    ldr x0, [sp, #0]
    add sp, sp, #16
    add sp, sp, #32
    ldp x29, x30, [sp], #16
    ret
_user_multiply:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    str xzr, [sp, #0]
    str xzr, [sp, #8]
    str xzr, [sp, #16]
    str xzr, [sp, #24]
    str x0, [sp, #0]
    str x1, [sp, #8]
    ldr x0, [sp, #0]
    ldr x1, [sp, #8]
    mul x0, x0, x1
    str x0, [sp, #16]
    ldr x0, [sp, #16]
    bl _inc_rc
    b .L2
.L2:
    sub sp, sp, #16
    str x0, [sp, #0]
    ldr x0, [sp, #16]
    bl _dec_rc
    ldr x0, [sp, #24]
    bl _dec_rc
    ldr x0, [sp, #32]
    bl _dec_rc
    ldr x0, [sp, #0]
    add sp, sp, #16
    add sp, sp, #32
    ldp x29, x30, [sp], #16
    ret
_user_subtract:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    str xzr, [sp, #0]
    str xzr, [sp, #8]
    str xzr, [sp, #16]
    str xzr, [sp, #24]
    str x0, [sp, #0]
    str x1, [sp, #8]
    ldr x0, [sp, #0]
    ldr x1, [sp, #8]
    sub x0, x0, x1
    str x0, [sp, #16]
    ldr x0, [sp, #16]
    bl _inc_rc
    b .L3
.L3:
    sub sp, sp, #16
    str x0, [sp, #0]
    ldr x0, [sp, #16]
    bl _dec_rc
    ldr x0, [sp, #24]
    bl _dec_rc
    ldr x0, [sp, #32]
    bl _dec_rc
    ldr x0, [sp, #0]
    add sp, sp, #16
    add sp, sp, #32
    ldp x29, x30, [sp], #16
    ret
_user_divide:
    stp x29, x30, [sp, #-16]!
    mov x29, sp
    sub sp, sp, #32
    str xzr, [sp, #0]
    str xzr, [sp, #8]
    str xzr, [sp, #16]
    str xzr, [sp, #24]
    str x0, [sp, #0]
    str x1, [sp, #8]
    ldr x0, [sp, #0]
    ldr x1, [sp, #8]
    sdiv x0, x0, x1
    str x0, [sp, #16]
    ldr x0, [sp, #16]
    bl _inc_rc
    b .L4
.L4:
    sub sp, sp, #16
    str x0, [sp, #0]
    ldr x0, [sp, #16]
    bl _dec_rc
    ldr x0, [sp, #24]
    bl _dec_rc
    ldr x0, [sp, #32]
    bl _dec_rc
    ldr x0, [sp, #0]
    add sp, sp, #16
    add sp, sp, #32
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
    cbz x0, .L5
    cmp x0, #0x1000
    b.lo .L5
    ldr x1, [x0, #8]
    cmp x1, #0
    b.lt .L5
    add x1, x1, #1
    str x1, [x0, #8]
.L5:
    ret

_dec_rc:
    cbz x0, .L6
    cmp x0, #0x1000
    b.lo .L6
    ldr x1, [x0, #8]
    cmp x1, #0
    b.lt .L6
    sub x1, x1, #1
    str x1, [x0, #8]
    cbnz x1, .L6
    stp x29, x30, [sp, #-16]!
    bl _gc_free
    ldp x29, x30, [sp], #16
.L6:
    ret

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
.L7:
    ldrb w3, [x1, x2]
    cbz w3, .L8
    add x2, x2, #1
    b .L7
.L8:
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
    cbz x22, .L9
    adrp x1, .comma_space@PAGE
    add x1, x1, .comma_space@PAGEOFF
    mov x2, #2
    mov x0, #1
    mov x16, #4
    svc #0x80
.L9:
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
    cbz x22, .L10
    adrp x1, .comma_space@PAGE
    add x1, x1, .comma_space@PAGEOFF
    mov x2, #2
    mov x0, #1
    mov x16, #4
    svc #0x80
.L10:
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

_is_map:
    cbz x0, .is_map_no
    cmp x0, #0x1000
    b.lo .is_map_no
    ldr x1, [x0, #0]
    cmp x1, #2
    cset x0, eq
    ret
.is_map_no:
    mov x0, #0
    ret

_is_list:
    cbz x0, .is_list_no
    cmp x0, #0x1000
    b.lo .is_list_no
    ldr x1, [x0, #0]
    cmp x1, #1
    cset x0, eq
    ret
.is_list_no:
    mov x0, #0
    ret

_is_string:
    cbz x0, .is_string_no
    cmp x0, #0x1000
    b.lo .is_string_no
    ldr x1, [x0, #0]
    cmp x1, #3
    cset x0, eq
    ret
.is_string_no:
    mov x0, #0
    ret

_type:
    cbz x0, .type_null
    cmp x0, #0x1000
    b.lo .type_number
    ldr x1, [x0, #0]
    cmp x1, #1
    b.eq .type_list
    cmp x1, #2
    b.eq .type_map
    cmp x1, #3
    b.eq .type_string
    cmp x1, #4
    b.eq .type_struct
    b .type_object
.type_number:
    adrp x0, .t_number@PAGE
    add x0, x0, .t_number@PAGEOFF
    ret
.type_string:
    adrp x0, .t_string@PAGE
    add x0, x0, .t_string@PAGEOFF
    ret
.type_list:
    adrp x0, .t_list@PAGE
    add x0, x0, .t_list@PAGEOFF
    ret
.type_map:
    adrp x0, .t_map@PAGE
    add x0, x0, .t_map@PAGEOFF
    ret
.type_struct:
    adrp x0, .t_struct@PAGE
    add x0, x0, .t_struct@PAGEOFF
    ret
.type_null:
    adrp x0, .t_null@PAGE
    add x0, x0, .t_null@PAGEOFF
    ret
.type_object:
    adrp x0, .t_object@PAGE
    add x0, x0, .t_object@PAGEOFF
    ret

_bool:
    cbz x0, .bool_false
    cmp x0, #0x1000
    b.lo .bool_num
    ldr x1, [x0, #0]
    cmp x1, #1
    b.eq .bool_lencheck
    cmp x1, #2
    b.eq .bool_lencheck
    cmp x1, #3
    b.eq .bool_lencheck
    mov x0, #1
    ret
.bool_lencheck:
    ldr x2, [x0, #16]
    cmp x2, #0
    cset x0, ne
    ret
.bool_num:
    cmp x0, #0
    cset x0, ne
    ret
.bool_false:
    mov x0, #0
    ret

_num:
    cbz x0, .num_zero
    cmp x0, #0x1000
    b.lo .num_ret
    ldr x1, [x0, #0]
    cmp x1, #3
    b.ne .num_zero
    add x1, x0, #24
    mov x2, #0
    mov x3, #0
    ldrb w4, [x1]
    cmp w4, #45
    b.ne .num_loop
    mov x3, #1
    add x1, x1, #1
.num_loop:
    ldrb w4, [x1]
    cbz w4, .num_done
    cmp w4, #48
    b.lt .num_done
    cmp w4, #57
    b.gt .num_done
    sub w5, w4, #48
    mov x6, #10
    mul x2, x2, x6
    uxtw x5, w5
    add x2, x2, x5
    add x1, x1, #1
    b .num_loop
.num_done:
    cbz x3, .num_pos
    neg x2, x2
.num_pos:
    mov x0, x2
    ret
.num_zero:
    mov x0, #0
    ret
.num_ret:
    ret

_str:
    stp x29, x30, [sp, #-96]!
    mov x29, sp
    stp x19, x20, [sp, #16]
    stp x21, x22, [sp, #32]
    stp x23, x24, [sp, #48]
    stp x25, x26, [sp, #64]
    stp x27, x28, [sp, #80]
    cbz x0, .str_null
    cmp x0, #0x1000
    b.lo .str_from_num
    ldr x1, [x0, #0]
    cmp x1, #3
    b.eq .str_ret
    b .str_object
.str_ret:
    b .str_epilogue
.str_null:
    adrp x0, .t_null@PAGE
    add x0, x0, .t_null@PAGEOFF
    b .str_epilogue
.str_object:
    adrp x0, .t_object@PAGE
    add x0, x0, .t_object@PAGEOFF
    b .str_epilogue
.str_from_num:
    mov x20, x0
    mov x0, #288
    bl _malloc
    mov x19, x0
    mov x0, #3
    str x0, [x19, #0]
    mov x0, #1
    str x0, [x19, #8]
    add x21, x19, #24
    add x22, x21, #255
    mov x23, #0
    mov x24, x20
    mov x25, #0
    cmp x24, #0
    b.ge .itoa_abs
    neg x24, x24
    mov x25, #1
.itoa_abs:
    cbnz x24, .itoa_loop
    mov w0, #48
    strb w0, [x22]
    sub x22, x22, #1
    add x23, x23, #1
    b .itoa_done_digits
.itoa_loop:
    mov x26, #10
    udiv x27, x24, x26
    msub x28, x27, x26, x24
    add x28, x28, #48
    strb w28, [x22]
    sub x22, x22, #1
    add x23, x23, #1
    mov x24, x27
    cbnz x24, .itoa_loop
.itoa_done_digits:
    cbz x25, .itoa_copy
    mov w0, #45
    strb w0, [x22]
    sub x22, x22, #1
    add x23, x23, #1
.itoa_copy:
    add x22, x22, #1
    mov x24, #0
.itoa_copy_loop:
    cmp x24, x23
    b.ge .itoa_copy_done
    ldrb w0, [x22, x24]
    strb w0, [x21, x24]
    add x24, x24, #1
    b .itoa_copy_loop
.itoa_copy_done:
    add x0, x21, x23
    mov w1, #0
    strb w1, [x0]
    str x23, [x19, #16]
    mov x0, x19
    b .str_epilogue
.str_epilogue:
    ldp x27, x28, [sp, #80]
    ldp x25, x26, [sp, #64]
    ldp x23, x24, [sp, #48]
    ldp x21, x22, [sp, #32]
    ldp x19, x20, [sp, #16]
    ldp x29, x30, [sp], #96
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
.L11:
    ldrb w2, [x0]
    ldrb w3, [x1]
    cmp w2, w3
    b.ne .L12
    cbz w2, .L13
    add x0, x0, #1
    add x1, x1, #1
    b .L11
.L12:
    mov x0, #1
    ret
.L13:
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
    // Async spawn not implemented in native codegen yet
    ret

_await:
    // Async await not implemented in native codegen yet
    ret

_sleep:
    // Sleep is a no-op in native/VM mode for now
    ret

_alloc_file:
    // File allocation not implemented in native codegen yet
    mov x0, #0
    ret

_close_file:
    // File close not implemented in native codegen yet
    ret
