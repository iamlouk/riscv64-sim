  .text
  .global _start
  .type   _start, @function

_start:
  addi t0, zero, 0x0
  addi t1, zero, 0x5
  j cond

loop:
  addi t0, t0, 1

cond:
  blt t0, t1, loop

end:
  j end

  .size  _start, .-_start
