use std::pin::Pin;

use crate::insts::*;
use crate::syms;
use crate::tbs::*;
use syscalls::{syscall, Sysno};

const MAX_ADDR: usize = 1 << 24;

#[repr(C)]
pub struct CPU {
    pub pc: i64,
    pub regs: [u64; 32],
    pub fregs: [u64; 32],
    pub memory: Memory,
    pub remapped_filenos: std::collections::HashMap<usize, usize>,
    pub debug_syscalls: bool,
    pub jit_enabled: bool
}

impl CPU {
    pub fn new(jit_enabled: bool) -> Self {
        CPU {
            pc: 0,
            regs: [0x0; 32],
            fregs: [0xffffffffffffffff; 32],
            memory: Memory::new(),
            remapped_filenos: std::collections::HashMap::new(),
            debug_syscalls: true,
            jit_enabled
        }
    }

    pub fn load_and_exec(
            &mut self,
            elf_file: &elf::ElfBytes<'_, elf::endian::AnyEndian>,
            argv: Option<Vec<&str>>) -> Result<(i32, JIT), Error> {
        self.pc = elf_file.ehdr.e_entry as i64;
        let (sections, _) = elf_file
            .section_headers_with_strtab()
            .map_err(|e| Error::ELF(format!("{}", e)))
            .map(|(sections, strtab)| (sections.unwrap(), strtab))?;

        let _symbols = syms::SymbolTreeNode::build(&syms::get_symbols(elf_file));

        for section in sections {
            if section.sh_flags & (elf::abi::SHF_ALLOC as u64) != 0 {
                let data = elf_file.section_data(&section)
                    .map_err(|e| Error::ELF(format!("{}", e)))
                    .and_then(|(data, compression)|
                        if let Some(c) = compression {
                            Err(Error::ELF(format!("unsupported compression: {:?}", c)))
                        } else {
                            Ok(data)
                        })?;
                self.memory.copy_bulk(section.sh_addr, data)
            }
        }

        /* TODO: There must be a better way... What address to use as TOS? */
        let top_of_stack = MAX_ADDR - (MAX_ADDR >> 2);
        self.set_reg(REG_SP, top_of_stack as u64);
        if let Some(argv) = argv {
            /*
             * Setup argv for the guest.
             * Layout (argv is not passed like a normal function call argument!):
             *   - memory[TOS] = argc;
             *   - memory[TOS + 8] = argv[0];
             *   - memory[TOS + 16] = argv[1];
             *   - memory[TOS + 24] = argv[2];
             *   - ...
             * The individual strings that make up argv are after argv itself,
             * so in addresses higher than TOS (stack grows downwards after all).
             */
            let argc = argv.len();
            let argv_size: usize = argv.iter().map(|s| s.len() + 1).sum();
            self.memory.store_u64(top_of_stack, argc as u64);

            let mut argv_pos = top_of_stack + (2  + argv.len()) * 8 + argv_size;
            for (i, arg) in argv.iter().enumerate() {
                self.memory.store_u64(top_of_stack + (1 + i) * 8, argv_pos as u64);
                for (j, c) in arg.as_bytes().iter().enumerate() {
                    self.memory.store_u8(argv_pos + j, *c);
                }
                argv_pos += arg.as_bytes().len();
                self.memory.store_u8(argv_pos, b'\0');
                argv_pos += 1;
            }
        }

        let mut jit = JIT::new();
        let exitcode = loop {
            match self.step(&mut jit, None) {
                Ok(_) => continue,
                Err(Error::Exit(exitcode)) => break exitcode,
                Err(e) => return Err(e)
            }
        };

        Ok((exitcode, jit))
    }

    pub fn step(&mut self, jit: &mut JIT, syms: Option<&syms::SymbolTreeNode>)
            -> Result<i64, Error> {
        /* Check if this TB was already executed:
         * TODO: Link TBs together, with successor pointers, so that we don't have
         * to do a lookup into a hashmap so often....
         */
        if let Some(tb) = jit.tbs.get(&self.pc) {
            let count = tb.exec_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if let Some(f) = tb.jit_fn {
                /* We have a JITed version of this TB! */
                let pc = f(self.regs.as_mut_ptr(), self.memory.data.as_mut_ptr()) as i64;
                self.pc = pc;
                return Ok(pc)
            }

            if !tb.jit_failed && count > TB_KICK_IN_JIT {
                unsafe {
                    /* Bypass the borrow checker: The better design that avoids this would
                     * be not to have the JIT own the TBs.
                     */
                    (*(jit as *const JIT as *mut JIT)).kick_in()
                };
            }

            for (inst, size) in &tb.instrs {
                inst.exec(*size as i64, self)?;
            }
            return Ok(self.pc)
        }

        jit.buffer.clear();
        let pc = self.pc;
        loop {
            let raw = self.memory.load_u32(self.pc as usize);
            let (instr, size) = Inst::parse(raw)?;
            let instr = instr.simplify();
            instr.exec(size as i64, self)?;
            let ends_tb = instr.is_terminator();
            jit.buffer.push((instr, size as u8));
            if ends_tb {
                break;
            }
        }

        let tb = TranslationBlock {
            start: pc,
            exec_count: std::sync::atomic::AtomicI64::new(1),
            valid: true,
            instrs: jit.buffer.clone(),
            label: syms.and_then(|s| s
                .lookup(pc)
                .filter(|(_, start)| *start == pc)
                .map(|(name, _)| std::rc::Rc::from(name))),

            jit_failed: !self.jit_enabled,
            jit_fn: None
        };

        jit.tbs.insert(pc, tb);
        Ok(pc)
    }

    pub fn get_reg(&self, reg: Reg) -> u64 {
        self.regs[reg as usize]
    }

    pub fn set_reg(&mut self, reg: Reg, val: u64) {
        if reg != REG_ZR {
            self.regs[reg as usize] = val;
        }
    }

    #[allow(dead_code)]
    pub fn get_reg_address(self: Pin<&Self>, reg: Reg) -> *const u64 {
        let reg_ref: &u64 = &self.regs[reg as usize];
        reg_ref as *const u64
    }

    #[allow(dead_code)]
    pub fn get_reg_offset(self: Pin<&Self>, reg: Reg) -> usize {
        let first_reg_addr = &self.regs[0] as *const u64 as usize;
        let reg_addr = &self.regs[reg as usize] as *const u64 as usize;
        reg_addr - first_reg_addr
    }

    pub fn get_freg_f32(&self, reg: FReg) -> f32 {
        f32::from_bits(self.fregs[reg as usize] as u32)
    }

    pub fn get_freg_f64(&self, reg: FReg) -> f64 {
        f64::from_bits(self.fregs[reg as usize])
    }

    pub fn set_freg_f32(&mut self, reg: FReg, val: f32) {
        self.fregs[reg as usize] = 0xffffffff00000000 | (val.to_bits() as u64);
    }

    pub fn set_freg_f64(&mut self, reg: FReg, val: f64) {
        self.fregs[reg as usize] = val.to_bits();
    }

    pub unsafe fn ecall(&mut self) -> Result<(), Error> {
        const RISCV_SYSNO_CLOSE:    u64 = 57;
        const RISCV_SYSNO_READ:     u64 = 63;
        const RISCV_SYSNO_WRITE:    u64 = 64;
        const RISCV_SYSNO_NEWFSTAT: u64 = 80;
        const RISCV_SYSNO_EXIT:     u64 = 93;
        const RISCV_SYSNO_BRK:      u64 = 214;
        const RISCV_SYSNO_OPEN:     u64 = 430;

        let a0 = self.get_reg(REG_A0) as usize;
        let a1 = self.get_reg(REG_A1) as usize;
        let a2 = self.get_reg(REG_A2) as usize;

        // Linux Syscall Numbers, for whatever reason, are different on different architectures.
        // See https://jborza.com/post/2021-05-11-riscv-linux-syscalls/. Let's hope at least
        // the arguments are the same and that the rust crate `syscalls` are those of the host.
        let syscall = self.get_reg(REG_A7);
        let res = match syscall {
            RISCV_SYSNO_CLOSE => {
                let fd = self.remapped_filenos.get(&a0).cloned().unwrap_or(a0);
                let res = syscall!(Sysno::close, fd);
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `close`({}) -> {:?}", fd, res);
                }
                res
            },
            RISCV_SYSNO_READ => {
                let fd = self.remapped_filenos.get(&a0).cloned().unwrap_or(a0);
                let addr = self.memory.data.as_ptr().add(a1);
                let res = syscall!(Sysno::read, fd, addr as usize, a2);
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `read`({}, {:?}, {}) -> {:?}",
                        fd, addr, a2, res);
                }
                res
            },
            RISCV_SYSNO_WRITE => {
                let fd = self.remapped_filenos.get(&a0).cloned().unwrap_or(a0);
                let addr = self.memory.data.as_ptr().add(a1);
                let res = syscall!(Sysno::write, fd, addr as usize, a2);
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `write`({}, {:?}, {}) -> {:?}",
                        fd, addr, a2, res);
                }
                res
            },
            RISCV_SYSNO_NEWFSTAT => {
                let fd = self.remapped_filenos.get(&a0).cloned().unwrap_or(a0);
                let addr = self.memory.data.as_ptr().add(a1);
                let res = syscall!(Sysno::newfstatat, fd, addr as usize);
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `newfstat`({}, {:?}) -> {:?}",
                        fd, addr, res);
                }
                res
            },
            RISCV_SYSNO_EXIT => {
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `exit`({:#08x}) -> !", a0);
                }
                return Err(Error::Exit(a0 as i32))
            },
            RISCV_SYSNO_BRK => {
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `brk` (ignored)");
                }
                Ok(0)
            },
            RISCV_SYSNO_OPEN => {
                let filepath = self.memory.data.as_ptr().add(a0);
                let res = syscall!(Sysno::open, filepath as usize, a1);
                if self.debug_syscalls {
                    eprintln!("[simrv64i] syscall: `open`({:?}, {}) -> {:?}",
                        std::ffi::CStr::from_ptr(filepath as *const i8), a1, res);
                }
                res
            },
            _ => todo!()
        };

        self.set_reg(REG_A0, match res {
            Ok(val) => val as u64,
            Err(errno) => -(errno.into_raw() as i64) as u64
        });
        Ok(())
    }
}

pub struct Memory {
    data: Vec<u8>
}

impl Memory {
    pub fn new() -> Memory {
        let mut data = Vec::<u8>::new();
        data.reserve_exact(MAX_ADDR);
        data.resize(MAX_ADDR, 0x0);
        Self { data }
    }

    pub fn copy_bulk(&mut self, addr: u64, src: &[u8]) {
        self.data[(addr as usize)..(addr as usize + src.len())].copy_from_slice(src);
    }

    pub fn load_u8(&self, addr: usize) -> u8 {
        self.data[addr]
    }

    pub fn load_u16(&self, addr: usize) -> u16 {
        (self.data[addr]) as u16 |
        ((self.data[addr + 1] as u16) << 8)
    }

    pub fn load_u32(&self, addr: usize) -> u32 {
        (self.data[addr]) as u32 |
        ((self.data[addr + 1] as u32) << 8) |
        ((self.data[addr + 2] as u32) << 16) |
        ((self.data[addr + 3] as u32) << 24)
    }

    pub fn load_u64(&self, addr: usize) -> u64 {
        (self.data[addr]) as u64 |
        ((self.data[addr + 1] as u64) << 8) |
        ((self.data[addr + 2] as u64) << 16) |
        ((self.data[addr + 3] as u64) << 24) |
        ((self.data[addr + 4] as u64) << 32) |
        ((self.data[addr + 5] as u64) << 40) |
        ((self.data[addr + 6] as u64) << 48) |
        ((self.data[addr + 7] as u64) << 56)
    }

    pub fn store_u8(&mut self, addr: usize, val: u8) {
        self.data[addr] = val;
    }

    pub fn store_u16(&mut self, addr: usize, val: u16) {
        self.data[addr] = (val & 0xff) as u8;
        self.data[addr + 1] = ((val >> 8) & 0xff) as u8;
    }

    pub fn store_u32(&mut self, addr: usize, val: u32) {
        self.data[addr] = (val & 0xff) as u8;
        self.data[addr + 1] = ((val >> 8) & 0xff) as u8;
        self.data[addr + 2] = ((val >> 16) & 0xff) as u8;
        self.data[addr + 3] = ((val >> 24) & 0xff) as u8;
    }

    pub fn store_u64(&mut self, addr: usize, val: u64) {
        self.data[addr] = (val & 0xff) as u8;
        self.data[addr + 1] = ((val >> 8) & 0xff) as u8;
        self.data[addr + 2] = ((val >> 16) & 0xff) as u8;
        self.data[addr + 3] = ((val >> 24) & 0xff) as u8;
        self.data[addr + 4] = ((val >> 32) & 0xff) as u8;
        self.data[addr + 5] = ((val >> 40) & 0xff) as u8;
        self.data[addr + 6] = ((val >> 48) & 0xff) as u8;
        self.data[addr + 7] = ((val >> 56) & 0xff) as u8;
    }
}

