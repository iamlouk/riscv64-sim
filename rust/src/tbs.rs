use crate::insts::*;

#[allow(dead_code)]
pub struct TranslationBlock {
    pub start:      u64,
    pub size:       u64,
    pub exec_count: std::sync::atomic::AtomicI64,
    pub valid:      bool,
    pub label:      Option<String>,
    pub instrs:     Vec<(Inst, u8)>,
}

pub struct JIT {
    pub tbs: std::collections::HashMap<i64, TranslationBlock>,
    pub buffer: Vec<(Inst, u8)>,
}

impl JIT {
    pub fn new() -> Self {
        Self {
            tbs: std::collections::HashMap::with_capacity(1024),
            buffer : Vec::with_capacity(32),
        }
    }
}

impl TranslationBlock {

}

/*
 * TODO:
 *
 * - Have a JIT-TB block prologue and epilogue (push/pop regs, return at end)
 * - Have pointer to CPU struct in RBX
 *
 * - For ALU instrs:
 *   - Load op0 to RDX (as imm or from CPU reg. using mem. access, or load imm zero if its r0)
 *   - Load op1 to RCX (as imm or from CPU reg. using mem. access, or load imm zero if its r0)
 *   - Have one `extern "C" fn (_: u64, _: u64, op0: u64, op1: u64) -> u64` per ALU op.
 *   - Result is returned and stored to the dst register (unless its reg. 0)
 *
 * - uncond. direct jump, rd = r0:
 *   - Load RAX with imm. of next PC
 *   - Store RAX to mem[RBX] (pc of CPU is first field)
 *   - end TB
 * - uncond. ind. jump, rd = r0 (ret instr.):
 *   - Load RAX with base reg. value
 *   - Store RAX to mem[RBX] (pc of CPU is first field)
 *   - end TB
 * - branch:
 *   - Load op0 to RDX (from CPU reg. using mem. access, or load imm zero if its r0)
 *   - Load op1 to RDX (from CPU reg. using mem. access, or load imm zero if its r0)
 *   - Load CPU struct ptr to RDI
 *   - Load imm of PC of branch itself to RSI
 *   - Have one `extern "C" fn (cpu: *mut CPU, pc: u64, op0: u64, op1: u64) -> u64` per predicate,
 *     which also updates the PC
 *   - end TB
 *
 * - Loads/Stores and others: unsupported (too many args.: base, offset, width, ...)
 * - See https://c9x.me/x86/html/file_module_x86_id_26.html:
 *   - Instead of calling a sub-routine using `callq %r?`, if the text
 *     section is not too far from the heap, use a relative call?
 */

type x86Reg = u8;
const REG_RAX: x86Reg = 0; // Ret. Val, Caller-Save,
const REG_RCX: x86Reg = 1; // Arg. 4,   Caller-Save,
const REG_RDX: x86Reg = 2; // Arg. 3,   Caller-Save,
const REG_RBX: x86Reg = 3; //           Callee-Save,
const REG_RSP: x86Reg = 4; //           Callee-Save,
const REG_RBP: x86Reg = 5; //           Callee-Save,
const REG_RSI: x86Reg = 6; // Arg. 2,   Caller-Save,
const REG_RDI: x86Reg = 7; // Arg. 1,   Caller-Save,

fn x86_load_u64_imm(dst_reg: x86Reg, imm: u64, code: &mut Vec<u8>) {
    assert!(dst_reg < 16);
    code.extend_from_slice(&[
        0x48 + (dst_reg >> 3),
        0xb8 + (dst_reg & 0b0111),
        (imm >> 56) as u8, (imm >> 48) as u8,
        (imm >> 40) as u8, (imm >> 32) as u8,
        (imm >> 24) as u8, (imm >> 16) as u8,
        (imm >>  8) as u8, (imm >>  0) as u8,
    ]);
}

fn x86_load_u32_imm(dst_reg: x86Reg, imm: u32, code: &mut Vec<u8>) {
    assert!(dst_reg < 16);
    if dst_reg < 8 {
        code.extend_from_slice(&[
            0xb8 + dst_reg,
            (imm >> 24) as u8, (imm >> 16) as u8,
            (imm >>  8) as u8, (imm >>  0) as u8,
        ]);
    } else {
        code.extend_from_slice(&[
            0x41, 0xb8 + (dst_reg & 0b0111),
            (imm >> 24) as u8, (imm >> 16) as u8,
            (imm >>  8) as u8, (imm >>  0) as u8,
        ]);
    }
}

fn x86_load_from_memory(dst_reg: x86Reg, base_reg: x86Reg,
        offset: u32, code: &mut Vec<u8>) {
    assert!(dst_reg <= 4 && base_reg <= 4);
    if offset == 0 {
        code.extend_from_slice(&[ 0x48, 0x8b, ((dst_reg << 3) | base_reg) ]);
        return
    }

    code.extend_from_slice(&[
        0x48, 0x8b, 0x80 | (dst_reg << 3) | base_reg,
        (offset >> 24) as u8, (offset >> 16) as u8,
        (offset >>  8) as u8, (offset >>  0) as u8,
    ]);
}

fn x86_store_to_memory(src_reg: x86Reg, base_reg: x86Reg,
        offset: u32, code: &mut Vec<u8>) {
    assert!(src_reg <= 4 && base_reg <= 4);
    if offset == 0 {
        code.extend_from_slice(&[ 0x48, 0x89, ((src_reg << 3) | base_reg) ]);
        return
    }

    code.extend_from_slice(&[
        0x48, 0x89, 0x80 | (src_reg << 3) | base_reg,
        (offset >> 24) as u8, (offset >> 16) as u8,
        (offset >>  8) as u8, (offset >>  0) as u8,
    ]);
}

// pub extern "C" fn jit_

