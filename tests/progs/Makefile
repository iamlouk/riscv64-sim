
.PHONY: all

all: hello.elf fibs.elf loop.elf

%.elf: %.S
	riscv64-elf-gcc -nostdlib -march=rv64g $^ -o $@

