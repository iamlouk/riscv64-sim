use crate::insts::*;

fn reg_abi_name(reg: Reg) -> &'static str {
    match reg {
         0 => "zero",
         1 => "ra",   2 => "sp",   3 => "gp",   4 => "tp",
         5 => "t0",   6 => "t1",   7 => "t2",   8 => "s0",
         9 => "s1",  10 => "a0",  11 => "a1",  12 => "a2",
        13 => "a3",  14 => "a4",  15 => "a5",  16 => "a6",
        17 => "a7",  18 => "s2",  19 => "s3",  20 => "s4",
        21 => "s5",  22 => "s6",  23 => "s7",  24 => "s8",
        25 => "s9",  26 => "s10", 27 => "s11", 28 => "t3",
        29 => "t4",  30 => "t5",  31 => "t6",
         _ => panic!("RISC-V only has 32 registers")
    }
}

impl Inst {
    pub fn print<W: std::io::Write>(&self, w: &mut W, address: i64) -> std::io::Result<()> {
        match *self {
            Inst::Load { dst, width, base, offset, signext } =>
                write!(w, "l{}{}\t{},{}({})",
                    match width { 1 => "b", 2 => "h", 4 => "w", 8 => "d", _ => panic!() },
                    match signext { true => "", false => "u" },
                    reg_abi_name(dst), offset, reg_abi_name(base)),

            Inst::Store { src, width, base, offset } =>
                write!(w, "s{}\t{},{}({})",
                    match width { 1 => "b", 2 => "h", 4 => "w", 8 => "d", _ => panic!() },
                    reg_abi_name(src), offset, reg_abi_name(base)),

            Inst::JumpAndLink { dst: REG_ZR, offset } =>
                write!(w, "j\t{:x}", address - (offset as i64)),
            Inst::JumpAndLink { dst, offset } =>
                write!(w, "jal\t{},{:x}", reg_abi_name(dst), address + (offset as i64)),
            Inst::JumpAndLinkReg { dst: REG_ZR, base: REG_X1, offset: 0 } =>
                write!(w, "ret"),
            Inst::JumpAndLinkReg { dst: REG_ZR, base, offset: 0 } =>
                write!(w, "jr\t{}", reg_abi_name(base)),
            Inst::JumpAndLinkReg { dst: REG_X1, base, offset: 0 } =>
                write!(w, "jalr\t{}", reg_abi_name(base)),
            Inst::JumpAndLinkReg { dst, base, offset } =>
                write!(w, "jalr\t{},{},{:x}",
                    reg_abi_name(dst), reg_abi_name(base), address + (offset as i64)),

            Inst::Branch { pred: Predicate::EQ, src1, src2: REG_ZR, offset } =>
                write!(w, "beqz\t{},{:x}", reg_abi_name(src1), address + (offset as i64)),
            Inst::Branch { pred: Predicate::NE, src1, src2: REG_ZR, offset } =>
                write!(w, "bnez\t{},{:x}", reg_abi_name(src1), address + (offset as i64)),
            Inst::Branch { pred: Predicate::GE, src1: REG_ZR, src2, offset } =>
                write!(w, "blez\t{},{:x}", reg_abi_name(src2), address + (offset as i64)),
            Inst::Branch { pred: Predicate::GE, src1, src2: REG_ZR, offset } =>
                write!(w, "bgez\t{},{:x}", reg_abi_name(src1), address + (offset as i64)),
            Inst::Branch { pred: Predicate::LT, src1, src2: REG_ZR, offset } =>
                write!(w, "bltz\t{},{:x}", reg_abi_name(src1), address + (offset as i64)),
            Inst::Branch { pred: Predicate::LT, src1: REG_ZR, src2, offset } =>
                write!(w, "bgtz\t{},{:x}", reg_abi_name(src2), address + (offset as i64)),
            Inst::Branch { pred, src1, src2, offset } =>
                write!(w, "b{}\t{},{},{:x}",
                    match pred {
                        Predicate::EQ => "eq",
                        Predicate::NE => "ne",
                        Predicate::LT => "lt",
                        Predicate::LTU => "ltu",
                        Predicate::GE => "ge",
                        Predicate::GEU => "geu"
                    },
                    reg_abi_name(src1), reg_abi_name(src2), address + (offset as i64)),


            Inst::ECall { _priv } => write!(w, "ecall"),
            Inst::EBreak { _priv } => write!(w, "ebreak"),

            Inst::LoadUpperImmediate { dst, imm } =>
                write!(w, "lui\t{},{:#x}", reg_abi_name(dst), imm),
            Inst::AddUpperImmediateToPC { dst, imm } =>
                write!(w, "auipc\t{},{:#x}", reg_abi_name(dst), imm),

            Inst::CtrlStatusReg { .. } => write!(w, "csr ???"),

            Inst::ALUImm { op: ALU::Add, dst: REG_ZR, src1: REG_ZR, imm: 0 } =>
                write!(w, "nop"),
            Inst::ALUImm { op: ALU::Add, dst, src1: REG_ZR, imm } =>
                write!(w, "li\t{},{}", reg_abi_name(dst), imm as i32),
            Inst::ALUImm { op: ALU::Add, dst, src1, imm: 0 } =>
                write!(w, "mv\t{},{}", reg_abi_name(dst), reg_abi_name(src1)),
            Inst::ALUReg { op: ALU::Add, dst, src1: REG_ZR, src2 } =>
                write!(w, "mv\t{},{}", reg_abi_name(dst), reg_abi_name(src2)),
            Inst::ALUImm { op: ALU::XOr, dst, src1, imm: 0xffffffffu32 } =>
                write!(w, "not\t{},{}", reg_abi_name(dst), reg_abi_name(src1)),
            Inst::ALUReg { op: ALU::Sub, dst, src1: REG_ZR, src2 } =>
                write!(w, "neg\t{},{}", reg_abi_name(dst), reg_abi_name(src2)),
            Inst::ALUReg { op: ALU::SubW, dst, src1: REG_ZR, src2 } =>
                write!(w, "negw\t{},{}", reg_abi_name(dst), reg_abi_name(src2)),

            Inst::ALUReg { op, dst, src1, src2 } =>
                write!(w, "{}\t{},{},{}",
                    match op {
                        ALU::Add => "add", ALU::AddW => "addw",
                        ALU::Sub => "sub", ALU::SubW => "subw",
                        ALU::And => "and", ALU::Or   => "or", ALU::XOr => "xor",
                        ALU::SLT => "slt", ALU::SLTU => "sltu",
                        ALU::SLL => "sll", ALU::SLLW => "sllw",
                        ALU::SRL => "srl", ALU::SRLW => "srlw",
                        ALU::SRA => "sra", ALU::SRAW => "sraw",
                        ALU::Mul => "mul", ALU::MulW => "mulw",
                        ALU::Div => "div", ALU::DivW => "divw",
                        ALU::DivU => "divu", ALU::DivUW => "divuw",
                        ALU::Rem => "rem", ALU::RemW => "remw",
                        ALU::RemU => "remu", ALU::RemUW => "remuw",
                    },
                    reg_abi_name(dst),
                    reg_abi_name(src1), reg_abi_name(src2)),

            Inst::ALUImm { op, dst, src1, imm } =>
                write!(w, "{}\t{},{},{}",
                    match op {
                        ALU::Add => "addi", ALU::AddW => "addiw",
                        ALU::And => "andi", ALU::Or   => "ori", ALU::XOr => "xori",
                        ALU::SLT => "slti", ALU::SLTU => "sltiu",
                        ALU::SLL => "slli", ALU::SLLW => "slliw",
                        ALU::SRL => "srli", ALU::SRLW => "srliw",
                        ALU::SRA => "srai", ALU::SRAW => "sraiw",
                        _ => panic!("{:?} has a immediate variant?", op)
                    },
                    reg_abi_name(dst),
                    reg_abi_name(src1), imm as i32),

            Inst::Unknown =>
                write!(w, "???")
        }
    }
}

