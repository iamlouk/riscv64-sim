use crate::cpu;

pub type Reg = u8;
pub type FReg = u8;
pub const REG_ZR: Reg = 0;
pub const REG_RA: Reg = 1;
pub const REG_SP: Reg = 2;
pub const REG_A0: Reg = 10;
pub const REG_A1: Reg = 11;
pub const REG_A2: Reg = 12;
pub const REG_A7: Reg = 17;


fn sign_extend(x: u32, nbits: u32) -> u32 {
    let notherbits = std::mem::size_of_val(&x) as u32 * 8 - nbits;
    (x as i32).wrapping_shl(notherbits).wrapping_shr(notherbits) as u32
}

#[derive(Debug, Clone, Copy)]
pub enum Predicate { EQ, NE, LT, LTU, GE, GEU }

#[derive(Debug, Clone, Copy)]
pub enum CSR { RW, RS, RC, RWI, RSI, RCI }

#[derive(Debug, Clone, Copy)]
pub enum ALU {
    Add, AddW, Sub, SubW, And, Or, XOr,
    SLT, SLTU,
    SLL, SLLW, SRL, SRLW, SRA, SRAW,
    Mul, MulW,
    Div, DivW, DivU, DivUW,
    Rem, RemW, RemU, RemUW
}

#[derive(Debug, Clone, Copy)]
pub enum FPU {
    Add, Sub, Mul, Div, Min, Max, Sqrt
}

#[derive(Debug, Clone, Copy)]
pub enum RoundingMode {
    RNE, RTZ, RDN, RUP, RMM
}

#[derive(Debug, Clone)]
pub enum Inst {
    Unknown,
    Load { dst: Reg, width: u8, base: Reg, offset: i32, signext: bool },
    Store { src: Reg, width: u8, base: Reg, offset: i32 },
    JumpAndLink { dst: Reg, offset: i32 },
    JumpAndLinkReg { dst: Reg, base: Reg, offset: i32 },
    Branch { pred: Predicate, src1: Reg, src2: Reg, offset: i32 },
    CtrlStatusReg { op: CSR, dst: Reg, src: Reg, csr: u16 },
    ECall { _priv: u8 },
    EBreak { _priv: u8 },
    ALUImm { op: ALU, dst: Reg, src1: Reg, imm: u32 },
    ALUReg { op: ALU, dst: Reg, src1: Reg, src2: Reg },
    LoadUpperImmediate { dst: Reg, imm: u32 },
    AddUpperImmediateToPC { dst: Reg, imm: u32 },

    // "F" and "D" extension instructions:
    LoadFP { dst: FReg, width: u8, base: Reg, offset: i32 },
    StoreFP { src: FReg, width: u8, base: Reg, offset: i32 },
    FComp { op: FPU, dst: FReg, src1: FReg, dst2: FReg, rm: RoundingMode, width: u8 },
    FMADD { dst: FReg, src1: FReg, src2: FReg, src3: FReg,
            rm: RoundingMode, width: u8, negate: bool },
    FMSUB { dst: FReg, src1: FReg, src2: FReg, src3: FReg,
            rm: RoundingMode, width: u8, negate: bool },
    // TODO...
}

#[derive(Debug)]
pub enum Error {
    Illegal,
    InvalidEncoding(&'static str),
    Unimplemented(&'static str),
}

// The C extension really contains some fucked-up encodings:
fn parse_compressed_instruction(raw: u16) -> Result<Inst, Error> {
    fn get_reg3_bits987(raw: u16) -> Reg { (((raw >> 7) & 0b111) + 8) as Reg }
    fn get_reg3_bits432(raw: u16) -> Reg { (((raw >> 2) & 0b111) + 8) as Reg }

    fn get_reg5_bits1110987(raw: u16) -> Reg { ((raw >> 7) & 0b11111) as Reg }

    Ok(match ((raw >> 13) & 0b111, raw & 0b11) {
        (0b000, 0b00) if raw == 0 => return Err(Error::Illegal),
        (0b000, 0b00) => Inst::ALUImm {
            op: ALU::Add, dst: get_reg3_bits432(raw), src1: REG_SP,
            imm: (((raw & 0b0000000000100000) >> ( 5 - 3)) |
                  ((raw & 0b0000000001000000) >> ( 6 - 2)) |
                  ((raw & 0b0000011110000000) >> ( 7 - 6)) |
                  ((raw & 0b0001100000000000) >> (11 - 4))) as u32
        },
        (0b001, 0b00) => return Err(Error::Unimplemented("C.FLD")),
        (0b010, 0b00) => Inst::Load {
            dst: get_reg3_bits432(raw), width: 4, base: get_reg3_bits987(raw),
            offset: (((raw & 0b0001110000000000) >> (10 - 3)) |
                     ((raw & 0b0000000001000000) >> ( 6 - 2)) |
                     ((raw & 0b0000000000100000) << ( 6 - 5))) as i32,
            signext: true
        },
        (0b011, 0b00) => Inst::Load {
            dst: get_reg3_bits432(raw), width: 8, base: get_reg3_bits987(raw),
            offset: (((raw & 0b0001110000000000) >> (10 - 3)) |
                     ((raw & 0b0000000001100000) << ( 6 - 5))) as i32,
            signext: true
        },
        (0b100, 0b00) => return Err(Error::InvalidEncoding("C extension reserved space")),
        (0b101, 0b00) => return Err(Error::Unimplemented("C.FSD")),
        (0b110, 0b00) => Inst::Store {
            src: get_reg3_bits432(raw), width: 4, base: get_reg3_bits987(raw),
            offset: (((raw & 0b0001110000000000) >> (10 - 3)) |
                     ((raw & 0b0000000001000000) >> ( 6 - 2)) |
                     ((raw & 0b0000000000100000) << ( 6 - 5))) as i32
        },
        (0b111, 0b00) => Inst::Store {
            src: get_reg3_bits432(raw), width: 8, base: get_reg3_bits987(raw),
            offset: (((raw & 0b0001110000000000) >> (10 - 3)) |
                     ((raw & 0b0000000001100000) << ( 6 - 5))) as i32
        },
        (0b000, 0b01) => Inst::ALUImm { // C.ADDI
            op: ALU::Add,
            dst: get_reg5_bits1110987(raw),
            src1: get_reg5_bits1110987(raw),
            imm: sign_extend((((raw & 0b0001000000000000) >> (12 - 5)) |
                              ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32, 6)
        },
        (0b001, 0b01) => Inst::ALUImm { // C.ADDIW
            op: ALU::AddW,
            dst: get_reg5_bits1110987(raw),
            src1: get_reg5_bits1110987(raw),
            imm: sign_extend((((raw & 0b0001000000000000) >> (12 - 5)) |
                              ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32, 6)
        },
        (0b010, 0b01) => Inst::ALUImm { // C.LI
            op: ALU::Add,
            dst: get_reg5_bits1110987(raw),
            src1: REG_ZR,
            imm: sign_extend((((raw & 0b0001000000000000) >> (12 - 5)) |
                              ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32, 6)
        },
        (0b011, 0b01) => match get_reg5_bits1110987(raw) {
            2 => Inst::ALUImm { // C.ADDI16SP
                op: ALU::Add,
                dst: REG_SP,
                src1: REG_SP,
                imm: sign_extend((
                    ((raw & 0b0001000000000000) >> (12 - 9)) |
                    ((raw & 0b0000000001000000) >> ( 6 - 4)) |
                    ((raw & 0b0000000000100000) << ( 6 - 5)) |
                    ((raw & 0b0000000000011000) << ( 7 - 3)) |
                    ((raw & 0b0000000000000100) << ( 5 - 2))) as u32, 10)
            },
            0 => return Err(Error::InvalidEncoding("C extension reserved space")),
            rd => {
                let rd = rd as Reg;
                let imm = ((raw as u32 & 0b0001000000000000) << (17 - 12)) |
                          ((raw as u32 & 0b0000000001111100) << (12 -  2));
                Inst::LoadUpperImmediate {
                    dst: rd,
                    imm: sign_extend(imm, 18)
                }
            }
        },
        (0b100, 0b01) => match ((raw >> 10) & 0b11, get_reg3_bits987(raw)) {
            (0b00, rd) => Inst::ALUImm {
                op: ALU::SRL, dst: rd, src1: rd,
                imm: (((raw & 0b0001000000000000) >> (12 - 5)) |
                      ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32
            },
            (0b01, rd) => Inst::ALUImm {
                op: ALU::SRA, dst: rd, src1: rd,
                imm: (((raw & 0b0001000000000000) >> (12 - 5)) |
                      ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32
            },
            (0b10, rd) => Inst::ALUImm {
                op: ALU::And,
                dst: rd, src1: rd,
                imm: sign_extend((((raw & 0b0001000000000000) >> (12 - 5)) |
                                  ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32, 6)
            },
            (0b11, rd) => match ((raw >> 12) & 0b1, (raw >> 5) & 0b11) {
                (0b0, 0b00) => Inst::ALUReg {
                    op: ALU::Sub,
                    dst: rd, src1: rd, src2: get_reg3_bits432(raw)
                },
                (0b0, 0b01) => Inst::ALUReg {
                    op: ALU::XOr,
                    dst: rd, src1: rd, src2: get_reg3_bits432(raw)
                },
                (0b0, 0b10) => Inst::ALUReg {
                    op: ALU::Or,
                    dst: rd, src1: rd, src2: get_reg3_bits432(raw)
                },
                (0b0, 0b11) => Inst::ALUReg {
                    op: ALU::And,
                    dst: rd, src1: rd, src2: get_reg3_bits432(raw)
                },
                (0b1, 0b00) => Inst::ALUReg {
                    op: ALU::SubW,
                    dst: rd, src1: rd, src2: get_reg3_bits432(raw)
                },
                (0b1, 0b01) => Inst::ALUReg {
                    op: ALU::AddW,
                    dst: rd, src1: rd, src2: get_reg3_bits432(raw)
                },
                _ => return Err(Error::InvalidEncoding("C extension reserved space"))
            },
            _ => panic!("impossible?")
        },
        (0b101, 0b01) => Inst::JumpAndLink {
            dst: REG_ZR,
            offset: sign_extend((
                    ((raw & 0b0001000000000000) >> (12 - 11)) |
                    ((raw & 0b0000100000000000) >> (11 -  4)) |
                    ((raw & 0b0000011000000000) >> ( 9 -  8)) |
                    ((raw & 0b0000000100000000) << (10 -  8)) |
                    ((raw & 0b0000000010000000) >> ( 7 -  6)) |
                    ((raw & 0b0000000001000000) << ( 7 -  6)) |
                    ((raw & 0b0000000000111000) >> ( 3 -  1)) |
                    ((raw & 0b0000000000000100) << ( 5 -  2))) as u32, 12) as i32
        },
        (0b110, 0b01) => Inst::Branch {
            pred: Predicate::EQ,
            src1: get_reg3_bits987(raw),
            src2: REG_ZR,
            offset: sign_extend(
                (((raw & 0b0001000000000000) >> (12 - 8)) |
                 ((raw & 0b0000110000000000) >> (10 - 3)) |
                 ((raw & 0b0000000001100000) << ( 6 - 5)) |
                 ((raw & 0b0000000000011000) >> ( 3 - 1)) |
                 ((raw & 0b0000000000000100) << ( 5 - 2))) as u32, 9) as i32
        },
        (0b111, 0b01) => Inst::Branch {
            pred: Predicate::NE,
            src1: get_reg3_bits987(raw),
            src2: REG_ZR,
            offset: sign_extend(
                (((raw & 0b0001000000000000) >> (12 - 8)) |
                 ((raw & 0b0000110000000000) >> (10 - 3)) |
                 ((raw & 0b0000000001100000) << ( 6 - 5)) |
                 ((raw & 0b0000000000011000) >> ( 3 - 1)) |
                 ((raw & 0b0000000000000100) << ( 5 - 2))) as u32, 9) as i32
        },
        (0b000, 0b10) => Inst::ALUImm {
            op: ALU::SLL,
            dst: get_reg5_bits1110987(raw), src1: get_reg5_bits1110987(raw),
            imm: (((raw & 0b0001000000000000) >> (12 - 5)) |
                  ((raw & 0b0000000001111100) >> ( 2 - 0))) as u32
        },
        (0b001, 0b10) => return Err(Error::Unimplemented("C.FLDSP")),
        (0b010, 0b10) => Inst::Load {
            dst: get_reg5_bits1110987(raw),
            width: 4, base: REG_SP,
            offset: (((raw & 0b0001000000000000) >> (12 - 5)) |
                     ((raw & 0b0000000001110000) >> ( 4 - 2)) |
                     ((raw & 0b0000000000001100) << ( 6 - 2))) as i32,
            signext: true
        },
        (0b011, 0b10) => Inst::Load {
            dst: get_reg5_bits1110987(raw),
            width: 8, base: REG_SP,
            offset: (((raw & 0b0001000000000000) >> (12 - 5)) |
                     ((raw & 0b0000000001100000) >> ( 5 - 3)) |
                     ((raw & 0b0000000000011100) << ( 8 - 4))) as i32,
            signext: true
        },
        (0b100, 0b10) => match ((raw >> 12) & 1, (raw >> 7) & 0x1f, (raw >> 2) & 0x1f) {
            (0, rs1, 0) if rs1 != 0 => Inst::JumpAndLinkReg {
                dst: REG_ZR, base: rs1 as Reg, offset: 0
            },
            (0, rd, rs2) if rd != 0 && rs2 != 0 => Inst::ALUReg {
                op: ALU::Add, dst: rd as Reg, src1: REG_ZR, src2: rs2 as Reg
            },
            (1, 0, 0) => Inst::EBreak { _priv: 0 },
            (1, rs1, 0) if rs1 != 0 => Inst::JumpAndLinkReg {
                dst: REG_RA, base: rs1 as Reg, offset: 0
            },
            (1, rd, rs2) if rd != 0 && rs2 != 0 => Inst::ALUReg {
                op: ALU::Add, dst: rd as Reg, src1: rd as Reg, src2: rs2 as Reg
            },
            _ => return Err(Error::InvalidEncoding("C extension reserved space"))
        },
        (0b101, 0b10) => return Err(Error::Unimplemented("C.FSDSP")),
        (0b110, 0b10) => Inst::Store {
            src: ((raw >> 2) & 0x1f) as Reg, width: 4, base: REG_SP,
            offset: (((raw & 0b0001111000000000) >> (9 - 2)) |
                     ((raw & 0b0000000110000000) >> (7 - 6))) as i32
        },
        (0b111, 0b10) => Inst::Store {
            src: ((raw >> 2) & 0x1f) as Reg, width: 8, base: REG_SP,
            offset: (((raw & 0b0001110000000000) >> (10 - 3)) |
                     ((raw & 0b0000001110000000) >> ( 7 - 6))) as i32
        },
        (_____, 0b11) => panic!("this is not a compressed instruction"),
        (_____, ____) => panic!("impossible?")
    })
}

fn parse_instruction(raw: u32) -> Result<(Inst, usize), Error> {
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
                ((raw & 0x000ff000) >> (12 - 12)), 20) as i32
        },
        0b1100111 if get_funct3(raw) == 0 => Inst::JumpAndLinkReg {
            dst: get_rd(raw),
            base: get_rs1(raw),
            offset: sign_extend((raw & 0xfff00000) >> 20, 12) as i32
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
                ((raw & 0x00000080) << (11 -  7)), 13) as i32
        },
        0b0000011 => {
            let (width, signext) = match get_funct3(raw) {
                0b000 => (1, true),
                0b001 => (2, true),
                0b010 => (4, true),
                0b011 => (8, true),
                0b100 => (1, false),
                0b101 => (2, false),
                0b110 => (4, false),
                _ => return Err(Error::InvalidEncoding("invalid load width/sign extension"))
            };
            Inst::Load {
                dst: get_rd(raw), width, base: get_rs1(raw),
                offset: sign_extend((raw & 0xfff00000) >> 20, 12) as i32,
                signext
            }
        },
        0b0100011 => Inst::Store {
            src: get_rs2(raw),
            width: match get_funct3(raw) {
                0b000 => 1,
                0b001 => 2,
                0b010 => 4,
                0b011 => 8,
                _ => return Err(Error::InvalidEncoding("invalid store length"))
            },
            base: get_rs1(raw),
            offset: sign_extend(
                ((raw & 0xfe000000) >> (25 - 5)) |
                ((raw & 0x00000f80) >> ( 7 - 0)), 12) as i32
        },
        0b0010011 => {
            let dst = get_rd(raw);
            let src1 = get_rs1(raw);
            let imm12 = sign_extend((raw & 0xfff00000) >> 20, 12);
            let funct7 = get_funct7(raw) & !1;
            match get_funct3(raw) {
                0b000 => Inst::ALUImm { op: ALU::Add,  dst, src1, imm: imm12 },
                0b010 => Inst::ALUImm { op: ALU::SLT,  dst, src1, imm: imm12 },
                0b011 => Inst::ALUImm { op: ALU::SLTU, dst, src1, imm: imm12 },
                0b100 => Inst::ALUImm { op: ALU::XOr,  dst, src1, imm: imm12 },
                0b110 => Inst::ALUImm { op: ALU::Or,   dst, src1, imm: imm12 },
                0b111 => Inst::ALUImm { op: ALU::And,  dst, src1, imm: imm12 },
                0b001 if funct7 == 0b0000000 => Inst::ALUImm {
                    op: ALU::SLL, dst, src1, imm: (raw >> 20) & 0x3f },
                0b101 if funct7 == 0b0000000 => Inst::ALUImm {
                    op: ALU::SRL, dst, src1, imm: (raw >> 20) & 0x3f },
                0b101 if funct7 == 0b0100000 => Inst::ALUImm {
                    op: ALU::SRA, dst, src1, imm: (raw >> 20) & 0x3f },
                _ => return Err(Error::Unimplemented("ALU instruction extensions"))
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
                (0b001, 0b0000001) => return Err(Error::Unimplemented("mulh")),
                (0b010, 0b0000000) => Inst::ALUReg { op: ALU::SLT,  dst, src1, src2 },
                (0b010, 0b0000001) => return Err(Error::Unimplemented("mulhsu")),
                (0b011, 0b0000000) => Inst::ALUReg { op: ALU::SLTU, dst, src1, src2 },
                (0b011, 0b0000001) => return Err(Error::Unimplemented("mulhu")),
                (0b100, 0b0000000) => Inst::ALUReg { op: ALU::XOr,  dst, src1, src2 },
                (0b100, 0b0000001) => Inst::ALUReg { op: ALU::Div,  dst, src1, src2 },
                (0b101, 0b0000000) => Inst::ALUReg { op: ALU::SRL,  dst, src1, src2 },
                (0b101, 0b0000001) => Inst::ALUReg { op: ALU::DivU, dst, src1, src2 },
                (0b101, 0b0100000) => Inst::ALUReg { op: ALU::SRA,  dst, src1, src2 },
                (0b110, 0b0000000) => Inst::ALUReg { op: ALU::Or,   dst, src1, src2 },
                (0b110, 0b0000001) => Inst::ALUReg { op: ALU::Rem,  dst, src1, src2 },
                (0b111, 0b0000000) => Inst::ALUReg { op: ALU::And,  dst, src1, src2 },
                (0b111, 0b0000001) => Inst::ALUReg { op: ALU::RemU, dst, src1, src2 },
                _ => return Err(Error::Unimplemented("ALU instruction extensions"))
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
        0b0011011 => match (get_funct7(raw), get_funct3(raw)) {
            (_, 0b000) => Inst::ALUImm {
                op: ALU::AddW,
                dst: get_rd(raw), src1: get_rs1(raw),
                imm: sign_extend((raw >> 20) & 0xfff, 12)
            },
            (0b0000000, 0b001) => Inst::ALUImm {
                op: ALU::SLLW,
                dst: get_rd(raw), src1: get_rs1(raw),
                imm: get_rs2(raw) as u32
            },
            (0b0000000, 0b101) => Inst::ALUImm {
                op: ALU::SRLW,
                dst: get_rd(raw), src1: get_rs1(raw),
                imm: get_rs2(raw) as u32
            },
            (0b0100000, 0b101) => Inst::ALUImm {
                op: ALU::SRAW,
                dst: get_rd(raw), src1: get_rs1(raw),
                imm: get_rs2(raw) as u32
            },
            _ => return Err(Error::Unimplemented("0b0011011 opcode space"))
        },
        0b0111011 => match (get_funct7(raw), get_funct3(raw)) {
            (0b0000000, 0b000) => Inst::ALUReg {
                op: ALU::AddW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0100000, 0b000) => Inst::ALUReg {
                op: ALU::SubW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000000, 0b001) => Inst::ALUReg {
                op: ALU::SLLW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000000, 0b101) => Inst::ALUReg {
                op: ALU::SRLW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0100000, 0b101) => Inst::ALUReg {
                op: ALU::SRAW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000001, 0b000) => Inst::ALUReg {
                op: ALU::MulW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000001, 0b100) => Inst::ALUReg {
                op: ALU::DivW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000001, 0b101) => Inst::ALUReg {
                op: ALU::DivUW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000001, 0b110) => Inst::ALUReg {
                op: ALU::RemW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            (0b0000001, 0b111) => Inst::ALUReg {
                op: ALU::RemUW,
                dst: get_rd(raw), src1: get_rs1(raw), src2: get_rs2(raw)
            },
            _ => return Err(Error::Unimplemented("0b0111011 opcode space"))
        },
        _ => return Err(Error::InvalidEncoding("unknown opcode"))
    }, 4))
}

fn execute_instruction(cpu: &mut cpu::CPU, inst: Inst, inst_size: i64) {
    match inst.clone() {
        Inst::LoadUpperImmediate { dst, imm } => {
            cpu.set_reg(dst, imm as i32 as i64 as u64);
        },
        Inst::AddUpperImmediateToPC { dst, imm } => {
            let imm = imm as i32 as i64;
            cpu.set_reg(dst, (cpu.pc + imm) as u64);
        },
        Inst::JumpAndLink { dst, offset } => {
            cpu.set_reg(dst, (cpu.pc + inst_size) as u64);
            cpu.pc += offset as i64;
            return
        },
        Inst::JumpAndLinkReg { dst, base, offset } => {
            cpu.set_reg(dst, (cpu.pc + inst_size) as u64);
            cpu.pc = (cpu.get_reg(base) as i64) + offset as i64;
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
                cpu.pc += offset as i64;
                return
            }
        },
        Inst::Load { dst, width, base, offset, signext: false } => {
            let addr = ((cpu.get_reg(base) as i64) + offset as i64) as usize;
            cpu.set_reg(dst, match width {
                1 => cpu.memory.load_u8(addr)  as u64,
                2 => cpu.memory.load_u16(addr) as u64,
                4 => cpu.memory.load_u32(addr) as u64,
                8 => cpu.memory.load_u64(addr) as u64,
                _ => unimplemented!()
            });
        },
        Inst::Load { dst, width, base, offset, signext: true } => {
            let addr = ((cpu.get_reg(base) as i64) + offset as i64) as usize;
            cpu.set_reg(dst, match width {
                1 => cpu.memory.load_u8(addr) as i8 as i64 as u64,
                2 => cpu.memory.load_u16(addr) as i16 as i64 as u64,
                4 => cpu.memory.load_u32(addr) as i32 as i64 as u64,
                8 => cpu.memory.load_u64(addr) as i64 as i64 as u64,
                _ => unimplemented!()
            });
        },
        Inst::Store { src, width, base, offset } => {
            let addr = (cpu.get_reg(base) as i64 + offset as i64) as usize;
            let val = cpu.get_reg(src);
            match width {
                1 => cpu.memory.store_u8(addr, val as u8),
                2 => cpu.memory.store_u16(addr, val as u16),
                4 => cpu.memory.store_u32(addr, val as u32),
                8 => cpu.memory.store_u64(addr, val as u64),
                _ => unimplemented!()
            }
        },
        Inst::ALUReg { op, dst, src1, src2 } => {
            let a = cpu.get_reg(src1);
            let b = cpu.get_reg(src2);
            cpu.set_reg(dst, match op {
                ALU::Add  => a.wrapping_add(b),
                ALU::AddW => (a as u32).wrapping_add(b as u32) as i32 as i64 as u64,
                ALU::Sub  => a.wrapping_sub(b),
                ALU::SubW => (a as u32).wrapping_sub(b as u32) as i32 as i64 as u64,
                ALU::And  => a  & b,
                ALU::Or   => a  | b,
                ALU::XOr  => a  ^ b,
                ALU::SLL  => a << b,
                ALU::SLLW => (a as u32).wrapping_shl(b as u32) as i32 as i64 as u64,
                ALU::SRL  => a >> b,
                ALU::SRLW => (a as u32).wrapping_shr(b as u32) as i32 as i64 as u64,
                ALU::SRA  => (a as i64).wrapping_shr(b as u32) as u64,
                ALU::SRAW => (a as i32).wrapping_shr(b as u32) as i64 as u64,
                ALU::SLT  => if (a as i64) < (b as i64) { 1 } else { 0 },
                ALU::SLTU => if a < b { 1 } else { 0 },
                ALU::Mul  => (a as i64).wrapping_mul(b as i64) as u64,
                ALU::Div  => (a as i64).wrapping_div(b as i64) as u64,
                ALU::Rem  => (a as i64).wrapping_rem(b as i64) as u64,
                ALU::MulW => (a as i32).wrapping_mul(b as i32) as i64 as u64,
                ALU::DivW => (a as i32).wrapping_div(b as i32) as i64 as u64,
                ALU::RemW => (a as i32).wrapping_rem(b as i32) as i64 as u64,
                ALU::DivU => a.wrapping_div(b),
                ALU::RemU => a.wrapping_rem(b),
                ALU::DivUW => (a as u32).wrapping_div(b as u32) as i32 as i64 as u64,
                ALU::RemUW => (a as u32).wrapping_rem(b as u32) as i32 as i64 as u64,
            })
        },
        Inst::ALUImm { op, dst, src1, imm: uimm32 } => {
            let a = cpu.get_reg(src1);
            let (uimm64sext, simm64) = (uimm32 as i32 as i64 as u64, uimm32 as i32 as i64);
            cpu.set_reg(dst, match op {
                ALU::Add  => a.wrapping_add(uimm64sext),
                ALU::AddW => (a as u32).wrapping_add(uimm32) as i32 as i64 as u64,
                ALU::Sub  => a.wrapping_sub(uimm64sext),
                ALU::SubW => (a as u32).wrapping_sub(uimm32) as i32 as i64 as u64,
                ALU::And  => a  & uimm64sext,
                ALU::Or   => a  | uimm64sext,
                ALU::XOr  => a  ^ uimm64sext,
                ALU::SLL  => a << uimm64sext,
                ALU::SLLW => (a as u32).wrapping_shl(uimm32) as i32 as i64 as u64,
                ALU::SRL  => a >> uimm64sext,
                ALU::SRLW => (a as u32).wrapping_shr(uimm32) as i32 as i64 as u64,
                ALU::SRA  => (a as i64).wrapping_shr(uimm32) as u64,
                ALU::SRAW => (a as i32).wrapping_shr(uimm32) as i64 as u64,
                ALU::SLT  => if (a as i64) < simm64 { 1 } else { 0 },
                ALU::SLTU => if a < uimm64sext { 1 } else { 0 },

                ALU::Mul | ALU::MulW |
                ALU::Div | ALU::DivW | ALU::DivU | ALU::DivUW |
                ALU::Rem | ALU::RemW | ALU::RemU | ALU::RemUW =>
                    panic!("there is no valid encoding for this instruction")
            })
        },
        Inst::ECall { _priv } => unsafe { cpu.ecall(); },
        _ => unimplemented!()
    };
    cpu.pc += inst_size;
}

impl Inst {
    pub fn parse(raw: u32) -> Result<(Self, usize), Error> {
        parse_instruction(raw)
    }

    pub fn exec(&self, inst_size: i64, cpu: &mut cpu::CPU) {
        execute_instruction(cpu, self.clone(), inst_size)
    }

    pub fn is_call(&self) -> bool {
        match self {
            Inst::JumpAndLink { dst: REG_RA, offset: _ } => true,
            Inst::JumpAndLinkReg { dst: REG_RA, base: _, offset: _ } => true,
            _ => false
        }
    }

    pub fn is_ret(&self) -> bool {
        match self {
            Inst::JumpAndLinkReg { dst: REG_ZR, base: REG_RA, offset: 0 } => true,
            _ => false
        }
    }
}

