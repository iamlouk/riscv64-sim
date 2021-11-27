#pragma once

#include <stdlib.h>
#include <stdint.h>

#include "decode.h"

struct cpu {
	int64_t pc;
	uint64_t regs[32];
	size_t mem_size;
	char *mem;
};

void run_instruction(struct cpu *cpu);

