#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>
#include <stdbool.h>
#include <assert.h>

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

void run_instruction(struct cpu *cpu) {
	struct instruction ins;
	riscv_decode_single(&ins, cpu->mem, cpu->pc);
	if (ins.id >= (sizeof(instructions) / sizeof(instructions[0])) || instructions[ins.id].eval == NULL) {
		assert(false);
		return;
	}

	cpu->regs[0] = 0x0;
	instructions[ins.id].eval(cpu, &ins);
	if ((ins.flags & RISCV_FLAG_JUMP) == 0x0)
		cpu->pc += ins.size;
}

