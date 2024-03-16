# Create objdump files that can be compared with the disassembler/objdump
# alternative that is part of this emulator.
%.dump: %.elf
	riscv64-linux-gnu-objdump -d $< \
	  | grep -P ":\\t" \
	  | cut -f1 -d"#" \
	  | cut -f1 -d"<" \
	  | sed -e 's/\s*$//' > $@

%.elf: %.c
	riscv64-elf-gcc -Wall -Werror -mabi=lp64 -march=rv64imac -g -O1 -static -o $@ $<

.PHONY: all clean

all: hello-world.elf bubblesort.elf nqueens.elf debug.elf

clean:
	rm -rf ./*.elf ./*.dump
