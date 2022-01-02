#include <stdint.h>

#include "minilib.h"
#include "decode.h"
#include "cpu.h"
#include "loader.h"

extern void riscv_sim_import_uart_out(uint8_t b, uint64_t addr);

#define MEM_SIZE (1 << 20)
static char cpu_memory[MEM_SIZE];
static struct cpu cpu = {
	.mem_size = MEM_SIZE,
	.mem = (uint8_t*) cpu_memory,
	.uart_out = &riscv_sim_import_uart_out
};

int riscv_sim_load_elf(const unsigned char *binary) {
	return load_binary(&cpu, (const char*) binary);
}

uint64_t riscv_sim_get_pc() {
	return cpu.pc;
}

uint64_t riscv_sim_get_reg(int reg) {
	return cpu.regs[reg];
}

struct instruction cins;

int riscv_sim_next() {
	int ret;
	if ((ret = cpu_current_instruction(&cpu, &cins)) != 0)
		return ret;

	if ((ret = cpu_run_instruction(&cpu, &cins)) != 0)
		return ret;

	return 0;
}

#define BUFFER_SIZE (1 << 20)
static char buf[BUFFER_SIZE];
void *riscv_sim_get_buffer(size_t s) {
	return buf;
}

int riscv_sim_current_instruction_to_buf() {
	return instruction_as_string(buf, &cins);
}

