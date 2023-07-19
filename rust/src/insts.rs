use crate::cpu;

pub type Reg = u8;
pub const REG_ZR: Reg = 0;

fn sign_extend(x: u32, sign_bit: usize) -> u64 {
    let m = 1usize << (sign_bit - 1usize);
    let x = (x as usize) & ((1usize << sign_bit) - 1usize);
    (x ^ m).wrapping_sub(m) as u64
}

pub enum Predicate { EQ, NE, LT, LTU, GE, GEU }

pub enum CSR { RW, RS, RC, RWI, RSI, RCI }

pub enum ALU {
    Add, Sub, SLT, SLTU, And, Or, XOr, SLL, SRL, SRA,
    Mul, Div, DivU, Rem, RemU
}

pub enum Inst {
    Load { dst: Reg, width: u8, base: Reg, offset: i64 },
    Store { src: Reg, width: u8, base: Reg, offset: i64 },
    JumpAndLink { dst: Reg, offset: i64 },
    JumpAndLinkReg { dst: Reg, base: Reg, offset: i64 },
    Branch { pred: Predicate, src1: Reg, src2: Reg, offset: i64 },
    CtrlStatusReg { op: CSR, dst: Reg, src: Reg, csr: u16 },
    ECall { _priv: u8 },
    EBreak { _priv: u8 },
    ALUImm { op: ALU, dst: Reg, src1: Reg, imm: u64 },
    ALUReg { op: ALU, dst: Reg, src1: Reg, src2: Reg },
    LoadUpperImmediate { dst: Reg, imm: u32 },
    AddUpperImmediateToPC { dst: Reg, imm: u32 }
}

#[derive(Debug)]
pub enum Error {
    Illegal,
    InvalidEncoding(&'static str),
    Unimplemented(&'static str),
}

pub fn parse_compressed_instruction(raw: u16) -> Result<Inst, Error> {
    fn get_reg3_bit4(raw: u16) -> Reg { (((raw >> 7) & 0b111) + 8) as Reg }
    fn get_reg3_bit9(raw: u16) -> Reg { (((raw >> 2) & 0b111) + 8) as Reg }
    fn get_bit12(raw: u16) -> u16 { (raw & 0b0001000000000000) >> 12 }

    fn get_reg5_rs1(raw: u16) -> Reg { ((raw >> 7) & 0x1f) as Reg }
    fn get_reg5_rs2(raw: u16) -> Reg { ((raw >> 2) & 0x1f) as Reg }

    Ok(match ((raw >> 13) & 0b111, raw & 0b11) {
        (0b100, 0b10) => match (get_bit12(raw), get_reg5_rs1(raw), get_reg5_rs2(raw)) {
            (0, base, 0) if base != 0 =>
                Inst::JumpAndLinkReg { dst: REG_ZR, base, offset: 0x0 },
            (0, dst, src1) if dst != 0 && src1 != 0 =>
                Inst::ALUReg { op: ALU::Add, dst, src1, src2: REG_ZR },
            (0, 0, 0) => Inst::EBreak { _priv: 0 },
            (1, base, 0) if base != 0 =>
                Inst::JumpAndLinkReg { dst: 1, base, offset: 1 },
            (1, dst, src) if dst != 0 && src != 0 =>
                Inst::ALUReg { op: ALU::Add, dst, src1: dst, src2: src },
            (_, _, _) => unimplemented!()
        },
        (0b000, 0b00) if raw == 0 => return Err(Error::Illegal),
        (0b100, 0b00) => return Err(Error::InvalidEncoding("reserved compressed instruction")),
        _ => unimplemented!()
    })
}

pub fn parse_instruction(raw: u32) -> Result<(Inst, usize), Error> {
    fn get_rd(raw: u32) -> Reg { ((raw >>  7) & 0x0000001f) as Reg }
    fn get_rs1(raw: u32) -> Reg { ((raw >> 15) & 0x0000001f) as Reg }
    fn get_rs2(raw: u32) -> Reg { ((raw >> 20) & 0x0000001f) as Reg }
    fn get_funct3(raw: u32) -> u8 { ((raw >> 12) & 0x00000007) as u8 }
    fn get_funct7(raw: u32) -> u8 { ((raw >> 25) & 0x0000007f) as u8 }

    if raw & 0b11 != 0b11 {
        return Ok((parse_compressed_instruction(raw as u16)?, 2));
    }

    Ok((match raw & 0x0000007f {
        0b0110111 => Inst::LoadUpperImmediate {
            dst: get_rd(raw),
            imm: (raw & 0xfffff000) as u32
        },
        0b0010111 => Inst::AddUpperImmediateToPC {
            dst: get_rd(raw),
            imm: (raw & 0xfffff000) as u32
        },
        0b1101111 => Inst::JumpAndLink {
            dst: get_rd(raw),
            offset: sign_extend(
                ((raw & 0x80000000) >> (31 - 20)) |
                ((raw & 0x7fe00000) >> (21 -  1)) |
                ((raw & 0x00100000) >> (20 - 11)) |
                ((raw & 0x000ff000) >> (12 - 12)), 20) as i64
        },
        0b1100111 if get_funct3(raw) == 0 => Inst::JumpAndLinkReg {
            dst: get_rd(raw),
            base: get_rs1(raw),
            offset: sign_extend((raw & 0xfff00000) >> 20, 12) as i64
        },
        0b1100011 => Inst::Branch {
            pred: match get_funct3(raw) {
                0b000 => Predicate::EQ,
                0b001 => Predicate::NE,
                0b100 => Predicate::LT,
                0b101 => Predicate::GE,
                0b110 => Predicate::LTU,
                0b111 => Predicate::GEU,
                _ => return Err(Error::InvalidEncoding("unknown predicate for branch"))
            },
            src1: get_rs1(raw),
            src2: get_rs2(raw),
            offset: sign_extend(
                ((raw & 0x80000000) >> (31 - 12)) |
                ((raw & 0x7e000000) >> (25 -  5)) |
                ((raw & 0x00000f00) >> ( 8 -  1)) |
                ((raw & 0x00000080) << 4), 12) as i64
        },
        0b0000011 => Inst::Load {
            dst: get_rd(raw),
            width: match get_funct3(raw) {
                0b000 => 1,
                0b001 => 2,
                0b010 => 4,
                0b011 => 8,
                _ => return Err(Error::Unimplemented("sign-extending loads"))
            },
            base: get_rs1(raw),
            offset: sign_extend((raw & 0xfff00000) >> 20, 12) as i64
        },
        0b0100011 => Inst::Store {
            src: get_rs2(raw),
            width: match get_funct3(raw) {
                0b000 => 1,
                0b001 => 2,
                0b010 => 4,
                0b011 => 8,
                _ => return Err(Error::InvalidEncoding("store of invalid length"))
            },
            base: get_rs1(raw),
            offset: sign_extend(
                ((raw & 0xfe000000) >> (25 - 5)) |
                ((raw & 0x00000f80) >> ( 7 - 0)), 12) as i64
        },
        0b0010011 => {
            let dst = get_rd(raw);
            let src1 = get_rs1(raw);
            let imm12 = sign_extend((raw & 0xfff00000) >> 20, 12);
            match get_funct3(raw) {
                0b000 => Inst::ALUImm { op: ALU::Add,  dst, src1, imm: imm12 },
                0b010 => Inst::ALUImm { op: ALU::SLT,  dst, src1, imm: imm12 },
                0b011 => Inst::ALUImm { op: ALU::SLTU, dst, src1, imm: imm12 },
                0b100 => Inst::ALUImm { op: ALU::XOr,  dst, src1, imm: imm12 },
                0b110 => Inst::ALUImm { op: ALU::Or,   dst, src1, imm: imm12 },
                0b111 => Inst::ALUImm { op: ALU::And,  dst, src1, imm: imm12 },
                0b001 if get_funct7(raw) == 0 => Inst::ALUImm {
                    op: ALU::SLL, dst, src1, imm: get_rs2(raw) as u64 },
                0b101 if get_funct7(raw) == 0 => Inst::ALUImm {
                    op: ALU::SRL, dst, src1, imm: get_rs2(raw) as u64 },
                0b101 if get_funct7(raw) == 0b0100000 => Inst::ALUImm {
                    op: ALU::SRL, dst, src1, imm: get_rs2(raw) as u64 },
                _ => return Err(Error::Unimplemented("shifts"))
            }
        },
        0b0110011 => {
            let dst = get_rd(raw);
            let src1 = get_rs1(raw);
            let src2 = get_rs2(raw);
            match (get_funct3(raw), get_funct7(raw)) {
                (0b000, 0b0000000) => Inst::ALUReg { op: ALU::Add,  dst, src1, src2 },
                (0b000, 0b0100000) => Inst::ALUReg { op: ALU::Sub,  dst, src1, src2 },
                (0b000, 0b0000001) => Inst::ALUReg { op: ALU::Mul,  dst, src1, src2 },
                (0b001, 0b0000000) => Inst::ALUReg { op: ALU::SLL,  dst, src1, src2 },
                (0b010, 0b0000000) => Inst::ALUReg { op: ALU::SLT,  dst, src1, src2 },
                (0b011, 0b0000000) => Inst::ALUReg { op: ALU::SLTU, dst, src1, src2 },
                (0b100, 0b0000000) => Inst::ALUReg { op: ALU::XOr,  dst, src1, src2 },
                (0b100, 0b0000001) => Inst::ALUReg { op: ALU::Div,  dst, src1, src2 },
                (0b101, 0b0000000) => Inst::ALUReg { op: ALU::SRL,  dst, src1, src2 },
                (0b101, 0b0000001) => Inst::ALUReg { op: ALU::DivU, dst, src1, src2 },
                (0b101, 0b0100000) => Inst::ALUReg { op: ALU::SRA,  dst, src1, src2 },
                (0b110, 0b0000000) => Inst::ALUReg { op: ALU::Or,   dst, src1, src2 },
                (0b110, 0b0000001) => Inst::ALUReg { op: ALU::Rem,  dst, src1, src2 },
                (0b111, 0b0000000) => Inst::ALUReg { op: ALU::And,  dst, src1, src2 },
                (0b111, 0b0000001) => Inst::ALUReg { op: ALU::RemU, dst, src1, src2 },
                _ => return Err(Error::InvalidEncoding("ALU opcode space"))
            }
        },
        0b1110011 => {
            let dst = get_rd(raw);
            let src = get_rs1(raw);
            let csr = ((raw & 0xfff00000) >> 20) as u16;
            match get_funct3(raw) {
                0b000 if dst == 0 && src == 0 && csr == 0 => Inst::ECall { _priv: 0 },
                0b000 if dst == 0 && src == 0 && csr == 1 => Inst::EBreak { _priv: 0 },
                0b001 => Inst::CtrlStatusReg { op: CSR::RW, dst, src, csr },
                0b010 => Inst::CtrlStatusReg { op: CSR::RS, dst, src, csr },
                0b011 => Inst::CtrlStatusReg { op: CSR::RC, dst, src, csr },
                0b101 => Inst::CtrlStatusReg { op: CSR::RWI, dst, src, csr },
                0b110 => Inst::CtrlStatusReg { op: CSR::RSI, dst, src, csr },
                0b111 => Inst::CtrlStatusReg { op: CSR::RCI, dst, src, csr },
                _ => return Err(Error::InvalidEncoding("system instruction"))
            }
        },
        _ => return Err(Error::InvalidEncoding("unknown opcode"))
    }, 4))
}

pub fn execute_instruction(cpu: &mut cpu::CPU, inst: Inst, inst_size: i64) {
    match inst {
        Inst::LoadUpperImmediate { dst, imm } => {
            cpu.set_reg(dst, imm as u64);
        },
        Inst::AddUpperImmediateToPC { dst, imm } => {
            cpu.set_reg(dst, (cpu.pc + (imm as i64)) as u64);
        },
        Inst::JumpAndLink { dst, offset } => {
            cpu.set_reg(dst, (cpu.pc + inst_size) as u64);
            cpu.pc += offset;
            return
        },
        Inst::JumpAndLinkReg { dst, base, offset } => {
            cpu.set_reg(dst, (cpu.pc + inst_size) as u64);
            cpu.pc = (cpu.get_reg(base) as i64) + offset;
            cpu.pc &= !1;
            return
        },
        Inst::Branch { pred, src1, src2, offset } => {
            let a = cpu.get_reg(src1);
            let b = cpu.get_reg(src2);
            let branch = match pred {
                Predicate::EQ => a == b,
                Predicate::NE => a != b,
                Predicate::LT => (a as i64) < (b as i64),
                Predicate::LTU => a < b,
                Predicate::GE => (a as i64) >= (b as i64),
                Predicate::GEU => a >= b,
            };
            if branch {
                cpu.pc += offset;
                return
            }
        },
        Inst::Load { dst, width, base, offset } => {
            let addr = ((cpu.get_reg(base) as i64) + offset) as usize;
            cpu.set_reg(dst, match width {
                1 => cpu.memory.load_u8(addr) as u64,
                2 => cpu.memory.load_u16(addr) as u64,
                4 => cpu.memory.load_u32(addr) as u64,
                8 => cpu.memory.load_u64(addr) as u64,
                _ => unimplemented!()
            });
        },
        Inst::Store { src, width, base, offset } => {
            let addr = ((cpu.get_reg(base) as i64) + offset) as usize;
            let val = cpu.get_reg(src);
            match width {
                1 => cpu.memory.store_u8(addr, val as u8),
                2 => cpu.memory.store_u16(addr, val as u16),
                4 => cpu.memory.store_u32(addr, val as u32),
                8 => cpu.memory.store_u64(addr, val as u64),
                _ => unimplemented!()
            }
        },
        Inst::ALUImm { op, dst, src1, imm } => {
            let val = cpu.get_reg(src1);
            cpu.set_reg(dst, match op {
                ALU::Add => val.wrapping_add(imm),
                ALU::Sub => val.wrapping_sub(imm),
                ALU::And => val & imm,
                ALU::Or => val | imm,
                ALU::XOr => val ^ imm,
                ALU::SLL => val << imm,
                ALU::SRL => val >> imm,
                ALU::SRA => ((val as i64) >> (imm as i64)) as u64,
                ALU::SLT => if (val as i64) < (imm as i64) { 1 } else { 0 },
                ALU::SLTU => if val < imm { 1 } else { 0 },
                _ => unimplemented!()
            });
        },
        Inst::ALUReg { op, dst, src1, src2 } => {
            let a = cpu.get_reg(src1);
            let b = cpu.get_reg(src2);
            cpu.set_reg(dst, match op {
                ALU::Add => a.wrapping_add(b),
                ALU::Sub => a.wrapping_sub(b),
                ALU::And => a & b,
                ALU::Or => a | b,
                ALU::XOr => a ^ b,
                ALU::SLL => a << b,
                ALU::SRL => a >> b,
                ALU::SRA => ((a as i64) >> (b as i64)) as u64,
                ALU::SLT => if (a as i64) < (b as i64) { 1 } else { 0 },
                ALU::SLTU => if a < b { 1 } else { 0 },
                ALU::Mul => a.wrapping_mul(b),
                ALU::Div => ((a as i64) / (b as i64)) as u64,
                ALU::DivU => a / b,
                ALU::Rem => ((a as i64) % (b as i64)) as u64,
                ALU::RemU => a % b
            });
        },
        Inst::ECall { _priv } => unsafe { cpu::ecall(cpu) },
        _ => unimplemented!()
    };
    cpu.pc += inst_size;
}

