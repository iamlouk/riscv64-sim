#pragma once

#include <stdint.h>

#include "minilib.h"
#include "decode.h"

struct cpu {
	int64_t pc;
	uint64_t regs[32];
	size_t mem_size;
	char *mem;
};

int run_instruction(struct cpu *cpu);

