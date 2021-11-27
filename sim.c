#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>

#include "lib/cpu.h"
#include "lib/loader.h"
#include "lib/decode.h"

_Noreturn void die(const char msg[]) {
	perror(msg);
	exit(EXIT_FAILURE);
}

_Noreturn void fatal(const char *fmt, ...) {
	va_list args;
	va_start(args, fmt);
	vfprintf(stderr, fmt, args);
	va_end(args);
	exit(EXIT_FAILURE);
}

static const char *file_as_string(const char *filename) {
	FILE *f = fopen(filename, "r");
	if (!f)
		die(filename);

	if (fseek(f, 0, SEEK_END) == -1) die(filename);
	size_t size = ftell(f);
	if (fseek(f, 0, SEEK_SET) == -1) die(filename);
	char *buf = malloc(size);
	if (!buf)
		die("malloc");

	if (fread(buf, 1, size, f) != size)
		die(filename);

	if (fclose(f) == -1)
		die(filename);

	return buf;
}

int main(int argc, char *argv[]) {
	if (argc != 2)
		fatal("usage: %s <file.elf>\n", argv[0]);

	const char *binary = file_as_string(argv[1]);
	struct cpu cpu = { 0 };
	cpu.mem_size = (1 << 20);
	cpu.mem = calloc(cpu.mem_size, 1);
	if (!cpu.mem)
		die("calloc");

	if (load_binary(&cpu, binary) != 0)
		fatal("loading binary failed\n");

	for (;;) {
		fprintf(stdout, "PC:%06lx\tt0=%08lx, t1=%08lx, t2=%08lx\n",
			cpu.pc, cpu.regs[5], cpu.regs[6], cpu.regs[7]);

		int64_t prev_pc = cpu.pc;
		run_instruction(&cpu);
		if (cpu.pc == prev_pc)
			break;
	}

	return EXIT_SUCCESS;
}


