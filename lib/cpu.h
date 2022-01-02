#pragma once

#include <stdint.h>

#include "minilib.h"
#include "decode.h"

#define UART_BASE (0x10000)

struct cpu {
	int64_t  pc;
	uint64_t regs[32];
	size_t   mem_size;
	uint8_t  *mem;

	void (*uart_out)(uint8_t b, uint64_t port);
};

int cpu_current_instruction(struct cpu *cpu, struct instruction *ins);

int cpu_run_instruction(struct cpu *cpu, struct instruction *ins);

int instruction_as_string(char *buf, struct instruction *ins);

