// Loop test - count from 0 to 10
mov x0, #0
mov x1, #10

loop:
add x0, x0, #1
cmp x0, x1
blt loop

// x0 should be 10 now
