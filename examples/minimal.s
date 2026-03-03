.global _main
.data
.msg: .asciz "Minimal VM Test\n"
.text
_main:
    adrp x0, .msg@PAGE
    add x0, x0, .msg@PAGEOFF
    mov x1, x0
    mov x2, #16
    mov x0, #1
    mov x16, #4
    svc #0x80
    mov x0, #0
    mov x16, #1
    svc #0x80
