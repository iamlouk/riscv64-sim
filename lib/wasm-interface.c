#include <stdint.h>

#include "minilib.h"
#include "decode.h"
#include "cpu.h"
#include "loader.h"


#define MEM_SIZE (1 << 20)
static char cpu_memory[MEM_SIZE];
static struct cpu cpu = {
	.mem_size = MEM_SIZE,
	.mem = cpu_memory
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

int riscv_sim_next() {
	return run_instruction(&cpu);
}

#define BUFFER_SIZE (1 << 20)
static char priv_buffer[BUFFER_SIZE];
void *riscv_sim_get_buffer(size_t s) {
	return (void*)priv_buffer;
}

int get_byte(const unsigned char *buf, size_t n) {
	return buf[n];
}

