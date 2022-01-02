#include <stdlib.h>
#include <stdio.h>
#include <stdint.h>
#include <stdarg.h>

#include "decode.h"

static void fatal(const char fmt[], ...) {
	va_list ap;
	va_start(ap, fmt);
	vfprintf(stderr, fmt, ap);
	va_end(ap);
	exit(EXIT_FAILURE);
}

static const uint8_t data[] = {
	0x37, 0xcc, 0x03, 0x00,
	0x6f, 0x00, 0x80, 0x03,
	0xe3, 0xc6, 0x62, 0xfc,
	0x63, 0x05, 0x03, 0x00,
	0x6f, 0x00, 0xe0, 0x10,
	0x63, 0x4d, 0x07, 0x00,
	0x6f, 0xf0, 0xbf, 0xf7,
};

static const struct instruction expected[] = {
	{
		.id = RISCV_LUI,
		.address = 0,
		.op_count = 2,
		.operands[0].reg = 24,
		.operands[1].imm = 0x3c
	},
	{
		.id = RISCV_JAL,
		.address = 4,
		.flags = RISCV_FLAG_JUMP,
		.op_count = 2,
		.operands[0].reg = 0,
		.operands[1].imm = 0x38
	},
	{
		.id = RISCV_BLT,
		.address = 8,
		.flags = RISCV_FLAG_JUMP,
		.op_count = 3,
		.operands[0].reg = 5,
		.operands[1].reg = 6,
		.operands[2].imm = -0x34
	},
	{
		.id = RISCV_BEQ,
		.address = 12,
		.flags = RISCV_FLAG_JUMP,
		.op_count = 3,
		.operands[0].reg = 6,
		.operands[1].reg = 0,
		.operands[2].imm = 0xa
	},
	{
		.id = RISCV_JAL,
		.address = 16,
		.flags = RISCV_FLAG_JUMP,
		.op_count = 2,
		.operands[0].reg = 0,
		.operands[1].imm = 0x10e
	},
	{
		.id = RISCV_BLT,
		.address = 20,
		.flags = RISCV_FLAG_JUMP,
		.op_count = 3,
		.operands[0].reg = 14,
		.operands[1].reg = 0,
		.operands[2].imm = 0x1a
	},
	{
		.id = RISCV_JAL,
		.address = 24,
		.flags = RISCV_FLAG_JUMP,
		.op_count = 2,
		.operands[0].reg = 0,
		.operands[1].imm = -0x86
	}
};

int main() {
	struct instruction ins;
	int64_t off = 0;
	size_t n = sizeof(expected) / sizeof(expected[0]);
	for (size_t i = 0; i < n; i++) {
		const struct instruction *exp = &expected[i];
		off += riscv_decode_single(&ins, data, off);

		if (ins.id != exp->id)
			fatal("test #%04lu: instruction id mismatch\n", i);

		if (ins.address != exp->address || ins.flags != exp->flags)
			fatal("test #%04lu: address or flags mismatch\n", i);

		if (ins.op_count != exp->op_count)
			fatal("test #%04lu: wrong operand count\n", i);

		for (int op = 0; op < ins.op_count; op++) {
			if (ins.operands[op].imm != exp->operands[op].imm)
				fatal("test #%04lu: operand #%d does not match\n", i, op);
		}
	}

	printf("Success!\n");
	return EXIT_SUCCESS;
}

