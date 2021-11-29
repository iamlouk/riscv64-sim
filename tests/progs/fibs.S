	.text
	.global _start
	.type _start, @function
_start:
	li a0, 0x1
	jal ra, fibs
	mv x16, a0

	li a0, 0x2
	jal ra, fibs
	mv x17, a0

	li a0, 0x3
	jal ra, fibs
	mv x18, a0

	li a0, 0x4
	jal ra, fibs
	mv x19, a0

	li a0, 0x5
	jal ra, fibs
	mv x20, a0

	li a0, 0x6
	jal ra, fibs
	mv x21, a0

	li a0, 0x7
	jal ra, fibs
	mv x22, a0

end:
	jal zero, end

	.global fibs
	.type fibs, @function
fibs:
	# argument n in a0
	li t0, 0x0 # counter
	li t1, 0x0 # t1 = a
	li t2, 0x1 # t2 = b
	j cond
loop:
	mv a1, t1
	add t1, t1, t2
	mv t2, a1
	addi t0, t0, 1
cond:
	blt t0, a0, loop
	mv a0, t1
	ret

