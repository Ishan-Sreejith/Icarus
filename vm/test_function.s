// Function call test
mov x0, #5
mov x1, #3
bl multiply
// Result in x0

b end

multiply:
mul x0, x0, x1
ret

end:
nop
