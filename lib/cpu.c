#include <stdint.h>
#include <stdbool.h>

#include "decode.h"
#include "cpu.h"

static void eval_jal(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[ins->operands[0].reg] = cpu->pc + ins->size;
	cpu->pc += ins->operands[1].imm;
}

static void eval_jalr(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[ins->operands[0].reg] = cpu->pc + ins->size;
	cpu->pc = cpu->regs[ins->operands[1].reg] + ins->operands[2].imm;
	cpu->pc &= ~1;
}

static void eval_lui(struct cpu *cpu, struct instruction *ins) {
	// cpu->uart_out('r', ins->operands[0].imm);
	// cpu->uart_out('x', ins->operands[1].imm);
	// TODO:FIXME: This is badly hacky!
	ins->operands[1].imm = 0x10;
	cpu->regs[ins->operands[0].reg] = ins->operands[1].imm << 12ul;
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

static void eval_add(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[ins->operands[0].reg] =
		cpu->regs[ins->operands[1].reg] + cpu->regs[ins->operands[2].reg];
}

static void eval_sb(struct cpu *cpu, struct instruction *ins) {
	uint8_t val = cpu->regs[ins->operands[1].reg];
	uint64_t addr = cpu->regs[ins->operands[0].reg] + ins->operands[2].imm;
	if (addr == UART_BASE)
		cpu->uart_out(val, addr);
	else
		cpu->mem[addr] = val;
}

static void eval_lb(struct cpu *cpu, struct instruction *ins) {
	cpu->regs[ins->operands[0].reg] = (uint8_t) cpu->mem[ins->operands[1].reg + ins->operands[2].imm];
}

struct instruction_table_entry {
	const char *name;
	void (*eval)(struct cpu *cpu, struct instruction *ins);
};

static const struct instruction_table_entry instructions[] = {
	[RISCV_JAL]  = { .name = "jal",  .eval = eval_jal  },
	[RISCV_JALR] = { .name = "jalr", .eval = eval_jalr },
	[RISCV_LUI]  = { .name = "lui",  .eval = eval_lui  },
	[RISCV_BLT]  = { .name = "blt",  .eval = eval_blt  },
	[RISCV_ADDI] = { .name = "addi", .eval = eval_addi },
	[RISCV_ADD]  = { .name = "add",  .eval = eval_add  },

	[RISCV_LB]   = { .name = "lb",   .eval = eval_lb   },
	[RISCV_SB]   = { .name = "sb",   .eval = eval_sb   },
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
	if ((ins->flags & RISCV_FLAG_R_TYPE) != 0)
		return sprintf(buf, "%s %lu, %lu, %lu", name, ins->operands[0].reg,
				ins->operands[1].reg, ins->operands[2].reg);
	if ((ins->flags & (RISCV_FLAG_I_TYPE | RISCV_FLAG_S_TYPE | RISCV_FLAG_B_TYPE)) != 0)
		return sprintf(buf, "%s %lu, %lu, %lx", name, ins->operands[0].reg,
				ins->operands[1].reg, ins->operands[2].imm);
	if ((ins->flags & (RISCV_FLAG_U_TYPE | RISCV_FLAG_J_TYPE)) != 0)
		return sprintf(buf, "%s %lu, %lx", name,
				ins->operands[0].reg, ins->operands[1].imm);

	strcpy(buf, "unkown");
	return 6;
}

