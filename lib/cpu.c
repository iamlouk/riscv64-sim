#include <stdint.h>
#include <stdbool.h>

#include "decode.h"
#include "cpu.h"

static void eval_jal(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[ins->operands[0].reg] = cpu->pc + ins->size;
	cpu->pc += ins->operands[1].imm;
}

static void eval_addi(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[ins->operands[0].reg] = cpu->regs[ins->operands[1].reg] + ins->operands[2].imm;
}

static void eval_blt(struct cpu *cpu, struct instruction *ins) {
	int64_t a = cpu->regs[ins->operands[0].reg],
			b = cpu->regs[ins->operands[1].reg];
	if (a < b)
		cpu->pc += ins->operands[2].imm;
	else
		cpu->pc += ins->size;
}

struct instruction_table_entry {
	const char *name;
	void (*eval)(struct cpu *cpu, struct instruction *ins);
};

static const struct instruction_table_entry instructions[] = {
	[RISCV_JAL]  = { .name = "jal",  .eval = eval_jal  },
	[RISCV_BLT]  = { .name = "blt",  .eval = eval_blt  },
	[RISCV_ADDI] = { .name = "addi", .eval = eval_addi },
};

int cpu_current_instruction(struct cpu *cpu, struct instruction *ins) {
	riscv_decode_single(ins, cpu->mem, cpu->pc);
	return 0;
}

int cpu_run_instruction(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[0] = 0x0;
	if (ins->id >= (sizeof(instructions) / sizeof(instructions[0]))
			|| instructions[ins->id].eval == NULL)
		return -1;

	instructions[ins->id].eval(cpu, ins);
	if ((ins->flags & RISCV_FLAG_JUMP) == 0x0)
		cpu->pc += ins->size;

	cpu->regs[0] = 0x0;
	return 0;
}

int instruction_as_string(char *buf, struct instruction *ins) {
	if (ins->id >= (sizeof(instructions) / sizeof(instructions[0]))
			|| instructions[ins->id].name == NULL) {
		strcpy(buf, "unkown");
		return 6;
	}

	const char *name = instructions[ins->id].name;
	if (ins->flags & RISCV_FLAG_R_TYPE)
		return sprintf(buf, "%s %lu, %lu, %lu", name, ins->operands[0].reg,
				ins->operands[1].reg, ins->operands[2].reg);
	if (ins->flags & RISCV_FLAG_I_TYPE)
		return sprintf(buf, "%s %lu, %lu, %lx", name, ins->operands[0].reg,
				ins->operands[1].reg, ins->operands[2].imm);
	if (ins->flags & RISCV_FLAG_S_TYPE)
		return sprintf(buf, "%s %lu, %lu, %lx", name, ins->operands[0].reg,
				ins->operands[1].reg, ins->operands[2].imm);
	if (ins->flags & RISCV_FLAG_B_TYPE)
		return sprintf(buf, "%s %lu, %lu, %lx", name, ins->operands[0].reg,
				ins->operands[1].reg, ins->operands[2].imm);
	if (ins->flags & RISCV_FLAG_U_TYPE)
		return sprintf(buf, "%s %lu, %lx", name,
				ins->operands[0].reg, ins->operands[1].imm);
	if (ins->flags & RISCV_FLAG_J_TYPE)
		return sprintf(buf, "%s %lu, %lx", name,
				ins->operands[0].reg, ins->operands[1].imm);

	strcpy(buf, "unkown");
	return 6;
}

