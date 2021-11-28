#include <stdint.h>
#include <stdbool.h>

#include "minilib.h"
#include "decode.h"

static inline int64_t sign_extend(int64_t x, unsigned int sign_bit) {
	int64_t m = 1lu << (sign_bit - 1lu);
	x = x & ((1lu << sign_bit) - 1lu);
	return (x ^ m) - m;
}

static inline int get_rd    (uint32_t raw) { return (raw >>  7) & 0b0011111; }
static inline int get_rs1   (uint32_t raw) { return (raw >> 15) & 0b0011111; }
static inline int get_rs2   (uint32_t raw) { return (raw >> 20) & 0b0011111; }
static inline int get_funct3(uint32_t raw) { return (raw >> 12) & 0b0000111; }

struct decode_table_entry {
	enum instruction_t funct3_table[(1 << 3)];
	void (*decode_type)(struct instruction *inst, int32_t raw);
	void (*special_case)(
			struct instruction *inst, int32_t raw,
			struct decode_table_entry *table_entry);
};

static void decode_i_type(struct instruction *inst, int32_t raw) {
	inst->flags |= RISCV_FLAG_I_TYPE;
	inst->op_count = 3;
	inst->operands[0].reg = get_rd(raw);
	inst->operands[1].reg = get_rs1(raw);
	inst->operands[2].imm = sign_extend((raw & 0xfff00000) >> 20, 12);
}

static void decode_s_type(struct instruction *inst, int32_t raw) {
	inst->flags |= RISCV_FLAG_S_TYPE;
	inst->op_count = 3;
	inst->operands[0].reg = get_rs1(raw);
	inst->operands[1].reg = get_rs2(raw);
	inst->operands[2].imm = sign_extend(
			((raw & 0xfe000000) >> (25 - 5)) |
			((raw & 0x00000f80) >> (7  - 0)), 12);
}

static void decode_b_type(struct instruction *inst, int32_t raw) {
	inst->flags |= RISCV_FLAG_JUMP | RISCV_FLAG_B_TYPE;
	inst->op_count = 3;
	inst->operands[0].reg = get_rs1(raw);
	inst->operands[1].reg = get_rs2(raw);
	inst->operands[2].imm = sign_extend(
			((raw & 0x80000000) >> (31 - 12)) |
			((raw & 0x7e000000) >> (25 -  5)) |
			((raw & 0x00000f00) >> ( 8 -  1)) |
			((raw & 0x00000080) << 4), 12);
}

static void decode_u_type(struct instruction *inst, int32_t raw) {
	inst->flags |= RISCV_FLAG_U_TYPE;
	inst->op_count = 2;
	inst->operands[0].reg = get_rd(raw);
	inst->operands[1].imm = sign_extend((raw & 0xfffff000), 32);
}

static void decode_j_type(struct instruction *inst, int32_t raw) {
	inst->id = RISCV_JAL;
	inst->flags |= RISCV_FLAG_JUMP | RISCV_FLAG_J_TYPE;
	inst->op_count = 2;
	inst->operands[0].reg = get_rd(raw);
	inst->operands[1].imm = sign_extend(
		((raw & 0x80000000) >> (31 - 20)) |
		((raw & 0x7fe00000) >> (21 -  1)) |
		((raw & 0x00100000) >> (20 - 11)) |
		((raw & 0x000ff000) >> (12 - 12)), 20);
}

static void decode_lui(
		struct instruction *inst, int32_t raw,
		struct decode_table_entry *table_entry) {
	inst->id = RISCV_LUI;
	inst->operands[1].imm >>= 12;
}

static void decode_auipc(
		struct instruction *inst, int32_t raw,
		struct decode_table_entry *table_entry) {
	inst->id = RISCV_AUIPC;
}

static void decode_jalr(
		struct instruction *inst, int32_t raw,
		struct decode_table_entry *table_entry) {
	inst->id = get_funct3(raw) == 0x0 ? RISCV_JALR : RISCV_UNKOWN;
	inst->flags |= RISCV_FLAG_JUMP;
}

static struct decode_table_entry decode_table[] = {
	[0b01101] = {
		.decode_type = decode_u_type,
		.special_case = decode_lui
	},
	[0b00101] = {
		.decode_type = decode_u_type,
		.special_case = decode_auipc
	},
	[0b11011] = {
		.decode_type = decode_j_type
	},
	[0b11001] = {
		.decode_type = decode_i_type,
		.special_case = decode_jalr
	},
	[0b11000] = {
		.funct3_table = {
			[0b000] = RISCV_BEQ,
			[0b001] = RISCV_BNE,
			[0b100] = RISCV_BLT,
			[0b101] = RISCV_BGE,
			[0b110] = RISCV_BLTU,
			[0b111] = RISCV_BGEU,
		},
		.decode_type = decode_b_type
	},
	[0b00000] = {
		.funct3_table = {
			[0b000] = RISCV_LB,
			[0b001] = RISCV_LH,
			[0b010] = RISCV_LW,
			[0b100] = RISCV_LBU,
			[0b101] = RISCV_LHU,
		},
		.decode_type = decode_i_type
	},
	[0b01000] = {
		.funct3_table = {
			[0b000] = RISCV_SB,
			[0b001] = RISCV_SH,
			[0b010] = RISCV_SW
		},
		.decode_type = decode_s_type
	},
	[0b00100] = {
		.funct3_table = {
			[0b000] = RISCV_ADDI,
			[0b010] = RISCV_SLTI,
			[0b011] = RISCV_SLTIU,
			[0b100] = RISCV_XORI,
			[0b110] = RISCV_ORI,
			[0b111] = RISCV_ANDI
			// TODO: SLLI, SRLI, SRAI
		},
		.decode_type = decode_i_type
	}
};

size_t riscv_decode_single(
		struct instruction *ins,
		const char *data,
		int64_t off) {
	memset(ins, 0, sizeof(struct instruction));
	ins->address = off;
	if ((data[off] & 0x3) != 0x3) {
		ins->flags |= RISCV_FLAG_COMPRESSED;
		ins->id = RISCV_UNKOWN;
		ins->size = 2;
		return 2;
	}

	ins->size = 4;
	uint32_t raw = (data[off] & 0xff)
		| ((data[off + 1] & 0xff) <<  8)
		| ((data[off + 2] & 0xff) << 16)
		| ((data[off + 3] & 0xff) << 24);

	size_t opcode = (raw & 0b1111100) >> 2;
	if (opcode >= (sizeof(decode_table) / sizeof(decode_table[0])) || decode_table[opcode].decode_type == NULL) {
		ins->id = RISCV_UNKOWN;
		return 4;
	}

	struct decode_table_entry *dte = &decode_table[opcode];
	ins->id = dte->funct3_table[get_funct3(raw)];
	dte->decode_type(ins, raw);
	if (dte->special_case != NULL)
		dte->special_case(ins, raw, dte);

	return 4;
}

