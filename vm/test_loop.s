mov x0, #0
mov x1, #10

loop:
add x0, x0, #1
cmp x0, x1
blt loop

