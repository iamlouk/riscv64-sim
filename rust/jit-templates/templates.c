#include <stddef.h>
#include <stdint.h>

/*
 * A file of small pieces of individual instructions that can be pieced
 * together to build a JIT.
 *
 */




void jit_piece_load_u64(uint64_t regs[32]) {
	uint64_t c = 0x1122334455667788;
	regs[0] = c;
}

void jit_piece_load_u32(uint64_t regs[32]) {
	uint32_t c = 0x11223344;
	regs[0] = c;
}

uint64_t jit_add(uint64_t a, uint64_t b) { return a  + b; }
uint64_t jit_sub(uint64_t a, uint64_t b) { return a  - b; }
uint64_t jit_mul(uint64_t a, uint64_t b) { return a  * b; }
uint64_t jit_sll(uint64_t a, uint64_t b) { return a << b; }
uint64_t jit_srl(uint64_t a, uint64_t b) { return a >> b; }
 int64_t jit_sra( int64_t a,  int64_t b) { return a >> b; }

