use std::pin::Pin;

use crate::insts::*;
use crate::syms;
use crate::tbs::*;
use syscalls::{syscall, Sysno};

const MAX_ADDR: usize = 1 << 24;

#[repr(C)]
pub struct CPU<'io> {
    pub pc: i64,
    pub regs: [u64; 32],
    pub fregs: [u64; 32],
    pub memory: Memory,
    pub capture_filenos: std::collections::HashMap<usize,
            (Option<&'io mut dyn std::io::Write>, Option<&'io mut dyn std::io::Read>)>
}

impl<'io> CPU<'io> {
    pub fn new() -> Self {
        CPU {
            pc: 0,
            regs: [0x0; 32],
            fregs: [0xffffffffffffffff; 32],
            memory: Memory::new(),
            capture_filenos: std::collections::HashMap::new()
        }
    }

    pub fn load_and_exec(
            &mut self, elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>) -> Result<i32, Error> {
        self.pc = elf_file.ehdr.e_entry as i64;
        let (sections, _) = elf_file
            .section_headers_with_strtab()
            .map_err(|e| Error::ELF(format!("{}", e)))
            .map(|(sections, strtab)| (sections.unwrap(), strtab))?;

        let _symbols = syms::SymbolTreeNode::build(&syms::get_symbols(&elf_file));

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

        /* TODO: There must be a better way... */
        let top_of_stack = 0x10000u64;
        self.set_reg(REG_SP, top_of_stack);

        let mut jit = JIT::new();
        loop {
            match self.step(&mut jit, None) {
                Ok(_) => continue,
                Err(Error::Exit(exitcode)) => return Ok(exitcode),
                Err(e) => return Err(e)
            }
        }
    }

    pub fn step(&mut self, jit: &mut JIT, syms: Option<&syms::SymbolTreeNode>)
            -> Result<u64, Error> {
        if let Some(tb) = jit.tbs.get(&self.pc) {
            tb.exec_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            // print!("[simrv64i] re-running TB at {:#08x}, count: {}\n",
            //        self.pc, *tb.exec_count.borrow());
            for (inst, size) in &tb.instrs {
                inst.exec(*size as i64, self)?;
            }
            return Ok(self.pc as u64)
        }

        jit.buffer.clear();
        let start_pc = self.pc as u64;
        let mut tb_size = 0;
        loop {
            let raw = self.memory.load_u32(self.pc as usize + tb_size);
            let (instr, size) = Inst::parse(raw)?;
            tb_size += size;
            let ends_tb = instr.is_terminator();
            jit.buffer.push((instr, size as u8));
            if ends_tb {
                break;
            }
        }

        let tb = TranslationBlock {
            start: start_pc,
            size: tb_size as u64,
            exec_count: std::sync::atomic::AtomicI64::new(1),
            valid: true,
            instrs: jit.buffer.clone(),
            label: syms.and_then(|s| s
                .lookup(start_pc)
                .filter(|(_, start)| *start == start_pc)
                .map(|(name, _)| name.to_owned())),
        };

        for (inst, size) in &tb.instrs {
            inst.exec(*size as i64, self)?;
        }

        // print!("[simrv64i] new TB at {:#08x}, instrs: {}, size: {}\n",
        //        start_pc, tb.instrs.len(),  tb.size);
        jit.tbs.insert(start_pc as i64, tb);
        Ok(start_pc)
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
            RISCV_SYSNO_CLOSE => syscall!(Sysno::close, a0),
            RISCV_SYSNO_READ => {
                let addr = self.memory.data.as_ptr().add(a1);
                if let Some((_, Some(r))) = self.capture_filenos.get(&a0) {
                    let _ = r;
                    todo!()
                }
                syscall!(Sysno::read, a0, addr as usize, a2)
            },
            RISCV_SYSNO_WRITE => {
                if let Some((Some(w), _)) = self.capture_filenos.get_mut(&a0) {
                    /* TODO: Transform std::io::Error back to a errno? */
                    match w.write(&self.memory.data[a1..(a1 + a2)]) {
                        Ok(n) => Ok(n),
                        Err(e) => return Err(Error::IO(e))
                    }
                } else {
                    let addr = self.memory.data.as_ptr().add(a1);
                    syscall!(Sysno::write, a0, addr as usize, a2)
                }
            },
            RISCV_SYSNO_NEWFSTAT => {
                let addr = self.memory.data.as_ptr().add(a1);
                syscall!(Sysno::newfstatat, a0, addr as usize)
            },
            RISCV_SYSNO_EXIT => return Err(Error::Exit(a0 as i32)),
            RISCV_SYSNO_BRK => {
                eprintln!("[simrv64i]: VM did syscall `BRK` (ignored)");
                Ok(0)
            },
            RISCV_SYSNO_OPEN => {
                let filepath = self.memory.data.as_ptr().add(a0);
                syscall!(Sysno::open, filepath as usize, a1)
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

