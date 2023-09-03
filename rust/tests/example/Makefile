
# Trims line ends, replaces whitespaces sequences by single space:
# awk '{$1=$1;print}'

# I know this chain of "cut" and "sed" could be more efficient:
objdump.txt: hello-world.libc.elf
	riscv64-linux-gnu-objdump -d hello-world.libc.elf | grep -P ":\\t" | cut -f1 -d"#" | cut -f1 -d"<" | sed -e 's/\s*$//' > objdump.txt


%.elf: %.c
	riscv64-elf-gcc -O1 -static -o $@ $<

%.o: %.c
	riscv64-elf-gcc -O1 -c -o $@ $<

