#pragma once

#include <stdint.h>

#include "minilib.h"

enum instruction_t {
	RISCV_INVALID,
	RISCV_UNKOWN,

	// RV32I Base Instruction Set
	RISCV_LUI,
	RISCV_AUIPC,
	RISCV_JAL,
	RISCV_JALR,
	RISCV_BEQ,
	RISCV_BNE,
	RISCV_BLT,
	RISCV_BGE,
	RISCV_BLTU,
	RISCV_BGEU,
	RISCV_LB,
	RISCV_LH,
	RISCV_LW,
	RISCV_LBU,
	RISCV_LHU,
	RISCV_SB,
	RISCV_SH,
	RISCV_SW,
	RISCV_ADDI,
	RISCV_SLTI,
	RISCV_SLTIU,
	RISCV_XORI,
	RISCV_ORI,
	RISCV_ANDI,
	RISCV_SLLI,
	RISCV_SRLI,
	RISCV_SRAI,
	RISCV_ADD,
	RISCV_SUB,
	RISCV_SLL,
	RISCV_SLT,
	RISCV_SLTU,
	RISCV_XOR,
	RISCV_SRL,
	RISCV_SRA,
	RISCV_OR,
	RISCV_AND,
	RISCV_FENCE,
	RISCV_FENCE_I,
	RISCV_ECALL,
	RISCV_EBREAK,
	RISCV_CSRRS,
	RISCV_CSRRC,
	RISCV_CSRRWI,
	RISCV_CSRSI,
	RISCV_CSRCI,

	// RV64I Base Instruction Set
	RISCV_LWU,
	RISCV_LD,
	RISCV_SD,
	// RISCV_SLLI,
	// RISCV_SRLI,
	RISCV_ADDIW,
	RISCV_SLLIW,
	RISCV_SRAIW,
	RISCV_ADDW,
	RISCV_SUBW,
	RISCV_SLLW,
	RISCV_SRLW,
	RISCV_SRAW
};

#define RISCV_FLAG_JUMP       (1 << 0)
#define RISCV_FLAG_COMPRESSED (1 << 1)
#define RISCV_FLAG_R_TYPE     (1 << 2)
#define RISCV_FLAG_I_TYPE     (1 << 3)
#define RISCV_FLAG_S_TYPE     (1 << 4)
#define RISCV_FLAG_B_TYPE     (1 << 5)
#define RISCV_FLAG_U_TYPE     (1 << 6)
#define RISCV_FLAG_J_TYPE     (1 << 7)

struct instruction {
	enum instruction_t id;
	int64_t address;
	uint8_t size;
	uint8_t flags;
	uint16_t op_count;
	union {
		int64_t reg;
		int64_t imm;
	} operands[3];
};

size_t riscv_decode_single(
	struct instruction *ins,
	const char *data,
	int64_t off);

