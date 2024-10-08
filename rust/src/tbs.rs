use crate::insts::*;

pub const TB_KICK_IN_JIT: i64 = 1_000;

#[allow(dead_code)]
pub struct TranslationBlock {
    pub start:      i64,
    pub exec_count: std::sync::atomic::AtomicI64,
    pub valid:      bool,
    pub label:      Option<std::rc::Rc<str>>,
    pub instrs:     Vec<(Inst, u8)>,

    pub jit_failed: bool,
    pub jit_fn: Option<extern "C" fn(regs: *mut u64, memory: *mut u8) -> u64>
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

    pub unsafe fn kick_in(&mut self) {
        use std::fmt::Write;
        use gccjit::ToRValue;

        /* libgccjit is a bad jit: Contexts cannot be reused, under the hood a shared object
         * is loaded for every .compile(), so we must compile in bulk!
         */
        let ctx = gccjit::Context::default();

        ctx.set_optimization_level(gccjit::OptimizationLevel::Limited);
        // ctx.set_dump_initial_gimple(true);
        ctx.set_print_errors_to_stderr(true);
        ctx.set_debug_info(false);

        let u8ty = ctx.new_type::<u8>();
        let u16ty = ctx.new_type::<u16>();
        let u32ty = ctx.new_type::<u32>();
        let i32ty = ctx.new_type::<i32>();
        let u64ty = ctx.new_type::<u64>();
        let i64ty = ctx.new_type::<i64>();

        let mut string_buf = String::new();
        // eprintln!("[simrv64i] JIT: kicking in...");

        let mut jitted_tbs: Vec<(String, i64)> = Vec::new();

        for tb in self.tbs.values_mut().filter(|tb|
                !tb.jit_failed && tb.jit_fn.is_none() &&
                tb.exec_count.load(std::sync::atomic::Ordering::Relaxed) > 100) {

            // eprintln!("[simrv64i] JIT: TB candidate: {:#08x} (freq={})",
            //     tb.start, tb.exec_count.load(std::sync::atomic::Ordering::Relaxed));

            /* In order to test this JIT without implemnting a ton of instructions,
             * just limit this to a very specific case:
             */

            string_buf.clear();
            write!(&mut string_buf, "jit_tb_{:08x}", tb.start).unwrap();
            jitted_tbs.push((string_buf.clone(), tb.start));

            /* The jit TB functions return the new PC and take as arguments:
             * - The register file
             * - The guest memory base pointer
             *
             * Future improvements: FPU registers and threading of successors branches,
             * calling the successors directly if possible.
             */
            let regs = ctx.new_parameter(None, u64ty.make_pointer(), "vm_regs");
            let memory_base = ctx.new_parameter(None, i64ty, "vm_memory");
            let f = ctx.new_function(
                None, gccjit::FunctionType::Exported, u64ty, &[regs, memory_base],
                string_buf.as_str(), false);
            let b = f.new_block("entry");
            let mut pc = tb.start;

            let register_lval = |reg: Reg| {
                ctx.new_array_access(None, regs, ctx.new_rvalue_from_int(u64ty, reg as i32))
            };
            let register_rval = |reg: Reg| {
                if reg == REG_ZR {
                    return ctx.new_rvalue_zero(u64ty);
                }
                ctx.new_array_access(None, regs, ctx.new_rvalue_from_int(u64ty, reg as i32)).to_rvalue()
            };
            let memory_addr = |base: Reg, offset: i32| {
                ctx.new_binary_op(None, gccjit::BinaryOp::Plus, i64ty,
                    memory_base,
                    ctx.new_binary_op(None, gccjit::BinaryOp::Plus, i64ty,
                        ctx.new_cast(None, register_rval(base), i64ty),
                        ctx.new_rvalue_from_int(i64ty, offset)))
            };

            for (inst, size) in &tb.instrs {
                string_buf.clear();
                write!(&mut string_buf, "{:?} (size={})", inst, size).unwrap();
                b.add_comment(None, string_buf.as_str());
                match inst.clone() {
                    Inst::NOP => continue,
                    Inst::ALUImm { op, dst, src1, imm } if op == ALU::AddW ||
                                                           op == ALU::SubW ||
                                                           op == ALU::SRAW => {
                        b.add_assignment(None, register_lval(dst),
                            ctx.new_cast(None, ctx.new_binary_op(None,
                                match op {
                                    ALU::AddW => gccjit::BinaryOp::Plus,
                                    ALU::SubW => gccjit::BinaryOp::Minus,
                                    ALU::SRAW => gccjit::BinaryOp::RShift,
                                    _ => todo!("{:?}", inst)
                                },
                                i32ty,
                                ctx.new_cast(None, register_rval(src1), i32ty),
                                ctx.new_rvalue_from_int(i32ty, imm as i32)), u64ty));
                    },
                    Inst::ALUImm { op, dst, src1, imm } if op == ALU::Add ||
                                                           op == ALU::SLL ||
                                                           op == ALU::SRL => {
                        let uimm64sext = imm as i32 as i64;
                        b.add_assignment(None, register_lval(dst),
                            ctx.new_binary_op(None,
                                match op {
                                    ALU::Add => gccjit::BinaryOp::Plus,
                                    ALU::SLL => gccjit::BinaryOp::LShift,
                                    ALU::SRL => gccjit::BinaryOp::RShift,
                                    _ => todo!("{:?}", inst)
                                },
                                u64ty,
                                register_rval(src1),
                                ctx.new_rvalue_from_long(u64ty, uimm64sext)));
                    },
                    Inst::ALUReg { op, dst, src1, src2 } if op == ALU::AddW || op == ALU::SubW => {
                        b.add_assignment(None, register_lval(dst),
                            ctx.new_cast(None, ctx.new_binary_op(None,
                                match op {
                                    ALU::AddW => gccjit::BinaryOp::Plus,
                                    ALU::SubW => gccjit::BinaryOp::Minus,
                                    _ => todo!("{:?}", inst)
                                },
                                u32ty,
                                ctx.new_cast(None, register_rval(src1), u32ty),
                                ctx.new_cast(None, register_rval(src2), u32ty)), u64ty));
                    },
                    Inst::ALUReg { op, dst, src1, src2 } => {
                        b.add_assignment(None,
                            register_lval(dst),
                            ctx.new_binary_op(None, match op {
                                ALU::Add => gccjit::BinaryOp::Plus,
                                ALU::And => gccjit::BinaryOp::BitwiseAnd,
                                ALU::Or => gccjit::BinaryOp::BitwiseOr,
                                ALU::XOr => gccjit::BinaryOp::BitwiseXor,
                                _ => todo!("{:?}", inst)
                            }, u64ty, register_rval(src1), register_rval(src2)));
                    },
                    Inst::Load { dst, width: 4, base, offset, signext: true } => {
                        let value = ctx.new_bitcast(None, memory_addr(base, offset), i32ty.make_pointer()).dereference(None);
                        b.add_assignment(None, register_lval(dst), ctx.new_cast(None, ctx.new_cast(None, value, i64ty), u64ty));
                    },
                    Inst::Load { dst, width: 8, base, offset, signext: _ } => {
                        let value = ctx.new_bitcast(None, memory_addr(base, offset), u64ty.make_pointer()).dereference(None);
                        b.add_assignment(None, register_lval(dst), value);
                    },
                    Inst::Store { src, width: 1, base, offset } => {
                        let lval = ctx.new_bitcast(None, memory_addr(base, offset), u8ty.make_pointer()).dereference(None);
                        b.add_assignment(None, lval, ctx.new_cast(None, register_rval(src), u8ty));
                    },
                    Inst::Store { src, width: 2, base, offset } => {
                        let lval = ctx.new_bitcast(None, memory_addr(base, offset), u16ty.make_pointer()).dereference(None);
                        b.add_assignment(None, lval, ctx.new_cast(None, register_rval(src), u16ty));
                    },
                    Inst::Store { src, width: 4, base, offset } => {
                        let lval = ctx.new_bitcast(None, memory_addr(base, offset), u32ty.make_pointer()).dereference(None);
                        b.add_assignment(None, lval, ctx.new_cast(None, register_rval(src), u32ty));
                    },
                    Inst::Store { src, width: 8, base, offset } => {
                        let lval = ctx.new_bitcast(None, memory_addr(base, offset), u64ty.make_pointer()).dereference(None);
                        b.add_assignment(None, lval, ctx.new_cast(None, register_rval(src), u64ty));
                    },
                    Inst::JumpAndLink { dst, offset } => {
                        if dst != REG_ZR {
                            b.add_assignment(None,
                                register_lval(dst),
                                ctx.new_rvalue_from_long(u64ty, pc + *size as i64));
                        }
                        b.end_with_return(None, ctx.new_rvalue_from_long(u64ty, pc + offset as i64));
                    },
                    Inst::JumpAndLinkReg { dst, base, offset } => {
                        if dst != REG_ZR {
                            b.add_assignment(None,
                                register_lval(dst),
                                ctx.new_rvalue_from_long(u64ty, pc + *size as i64));
                        }
                        let addr = ctx.new_binary_op(None, gccjit::BinaryOp::Plus, i64ty,
                            ctx.new_cast(None, register_rval(base), i64ty),
                            ctx.new_rvalue_from_long(i64ty, offset as i64));
                        b.end_with_return(None, ctx.new_cast(None, addr, u64ty));
                    },
                    Inst::Branch { pred, src1, src2, offset } => {
                        let src1 = ctx.new_array_access(None, regs,
                            ctx.new_rvalue_from_int(u64ty, src1 as i32));
                        let src2 = ctx.new_array_access(None, regs,
                            ctx.new_rvalue_from_int(u64ty, src2 as i32));
                        let cond = ctx.new_comparison(None,
                            match pred {
                                Predicate::EQ => gccjit::ComparisonOp::Equals,
                                Predicate::NE => gccjit::ComparisonOp::NotEquals,
                                Predicate::LTU => gccjit::ComparisonOp::LessThan,
                                Predicate::GEU => gccjit::ComparisonOp::GreaterThanEquals,
                                _ => todo!("{:?}", inst)
                            },
                            src1, src2);
                        let true_b = f.new_block("if_true");
                        let false_b = f.new_block("if_false");
                        b.end_with_conditional(None, cond, true_b, false_b);
                        true_b.end_with_return(None,
                            ctx.new_rvalue_from_long(u64ty, pc + offset as i64));
                        false_b.end_with_return(None,
                            ctx.new_rvalue_from_long(u64ty, pc + *size as i64));
                    },
                    _ => todo!("instr: {:?}", inst)
                }
                pc += *size as i64;
            }
        }

        /* After functions for TBs have been created, compile the module,
         * get the function pointer, and store it in the TB. */
        let res = ctx.compile();
        for (name, pc) in jitted_tbs {
            let tb = self.tbs.get_mut(&pc).unwrap();
            let fnptr = res.get_function(name.as_str());
            assert!(!fnptr.is_null());
            tb.jit_fn = Some(unsafe {
                std::mem::transmute(fnptr as usize)
            });
        }

        /* res must not be dropped! For whatever reason, if dropped, it unmaps
         * the shared library that contains the function itself.
         */
        std::mem::forget(res);
        std::mem::forget(ctx);
    }
}


