# Create objdump files that can be compared with the disassembler/objdump
# alternative that is part of this emulator.
%.dump: %.elf
	riscv64-linux-gnu-objdump -d $< \
	  | grep -P ":\\t" \
	  | cut -f1 -d"#" \
	  | cut -f1 -d"<" \
	  | sed -e 's/\s*$//' > $@

ifeq (, $(shell which riscv64-elf-gcc))
  RISCV64CC = riscv64-unknown-elf-gcc
else
  RISCV64CC = riscv64-elf-gcc
endif

%.elf: %.c
	riscv64-elf-gcc -Wall -Werror -mabi=lp64 -march=rv64imac -g -O1 -static -o $@ $<

.PHONY: all clean

all: hello-world.elf bubblesort.elf nqueens.elf

clean:
	rm -rf ./*.elf ./*.dump
