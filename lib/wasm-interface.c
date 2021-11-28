#include <stdint.h>

#include "minilib.h"
#include "decode.h"
#include "cpu.h"
#include "loader.h"


#define MEM_SIZE (1 << 20)

static char memory[MEM_SIZE];
static struct cpu cpu = {
	.mem_size = MEM_SIZE,
	.mem = memory
};

int riscv_sim_load_elf(const char *binary) {
	return load_binary(&cpu, binary);
}

uint64_t riscv_sim_get_pc() {
	return cpu.pc;
}

uint64_t riscv_sim_get_reg(int reg) {
	return cpu.regs[reg];
}

int riscv_sim_next() {
	return run_instruction(&cpu);
}

