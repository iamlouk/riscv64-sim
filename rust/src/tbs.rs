use crate::insts::*;

pub const TB_KICK_IN_JIT: i64 = 1000;

#[allow(dead_code)]
pub struct TranslationBlock {
    pub start:      u64,
    pub size:       u64,
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

        /* libgccjit is a bad jit: Contexts cannot be reused, under the hood a shared object
         * is loaded for every .compile(), so we must compile in bulk!
         */
        let ctx = gccjit::Context::default();

        ctx.set_optimization_level(gccjit::OptimizationLevel::Limited);
        // ctx.set_dump_initial_gimple(true);
        ctx.set_print_errors_to_stderr(true);
        ctx.set_allow_unreachable_blocks(false);
        ctx.set_debug_info(true);
        ctx.set_program_name("tb-jit");

        let u8ty = ctx.new_type::<u8>();
        let u32ty = ctx.new_type::<u32>();
        let u64ty = ctx.new_type::<u64>();

        let mut string_buf = String::new();
        eprintln!("[simrv64i] JIT: kicking in...");

        let mut jitted_tbs: Vec<(String, u64)> = Vec::new();

        for tb in self.tbs.values_mut().filter(|tb|
                !tb.jit_failed && tb.jit_fn.is_none() &&
                tb.exec_count.load(std::sync::atomic::Ordering::Relaxed) > 500) {

            // eprintln!("[simrv64i] JIT: TB candidate: {:#08x} (freq={})",
            //     tb.start, tb.exec_count.load(std::sync::atomic::Ordering::Relaxed));

            /* In order to test this JIT without implemnting a ton of instructions,
             * just limit this to a very specific case:
             */
            if tb.start != 0x01021c {
                tb.jit_failed = true;
                continue;
            }

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
            let regs = ctx.new_parameter(None, u64ty.make_pointer(), "regs");
            let memory = ctx.new_parameter(None, u8ty.make_pointer(), "memory");
            let f = ctx.new_function(
                None, gccjit::FunctionType::Exported, u64ty, &[regs, memory],
                string_buf.as_str(), false);
            let b = f.new_block("entry");
            let mut pc = tb.start;
            for (inst, size) in &tb.instrs {
                // string_buf.clear();
                // write!(&mut string_buf, "{:?} (size={})", inst, size).unwrap();
                // b.add_comment(None, string_buf.as_str());
                match inst {
                    Inst::NOP => continue,
                    Inst::ALUImm { op: ALU::AddW, dst, src1, imm } => {
                        let dst = ctx.new_array_access(None, regs,
                            ctx.new_rvalue_from_int(u64ty, *dst as i32));
                        b.add_assignment(None, dst,
                            ctx.new_cast(None, ctx.new_binary_op(None,
                                gccjit::BinaryOp::Plus,
                                u32ty,
                                ctx.new_cast(None,
                                    ctx.new_array_access(None, regs,
                                        ctx.new_rvalue_from_int(u64ty, *src1 as i32)), u32ty),
                                ctx.new_rvalue_from_int(u32ty, *imm as i32)), u64ty));
                    },
                    Inst::Branch { pred, src1, src2, offset } => {
                        let src1 = ctx.new_array_access(None, regs,
                            ctx.new_rvalue_from_int(u64ty, *src1 as i32));
                        let src2 = ctx.new_array_access(None, regs,
                            ctx.new_rvalue_from_int(u64ty, *src2 as i32));
                        let cond = ctx.new_comparison(None,
                            match pred {
                                Predicate::EQ => gccjit::ComparisonOp::Equals,
                                Predicate::NE => gccjit::ComparisonOp::NotEquals,
                                _ => unimplemented!()
                            },
                            src1, src2);
                        let true_b = f.new_block("if_true");
                        let false_b = f.new_block("if_false");
                        b.end_with_conditional(None, cond, true_b, false_b);
                        true_b.end_with_return(None,
                            ctx.new_rvalue_from_long(u64ty, pc as i64 + *offset as i64));
                        false_b.end_with_return(None,
                            ctx.new_rvalue_from_long(u64ty, (pc + *size as u64) as i64));
                    },
                    _ => todo!("instr: {:?}", inst)
                }
                pc += *size as u64;
            }
        }

        /* After functions for TBs have been created, compile the module,
         * get the function pointer, and store it in the TB. */
        let res = ctx.compile();
        for (name, pc) in jitted_tbs {
            let tb = self.tbs.get_mut(&(pc as i64)).unwrap();
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


