	.text
	.global _start
	.type   _start, @function
_start:
	nop
	li t0, 0x10000 # UART_BASE
	li t1, 0x68 # 'h'
	sb t1, (t0)
	li t1, 0x61 # 'a'
	sb t1, (t0)
	li t1, 0x6c # 'l'
	sb t1, (t0)
	li t1, 0x6c # 'l'
	sb t1, (t0)
	li t1, 0x6f # 'o'
	sb t1, (t0)
	li t1, 0xa # '\n'
	sb t1, (t0)
end:
	j end

	.size  _start, .-_start
