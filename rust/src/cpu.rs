use crate::insts::*;
use syscalls::{syscall, Sysno};

const MAX_ADDR: usize = 1 << 24;

pub struct CPU {
    pub pc: i64,
    pub regs: [u64; 32],
    pub fregs: [u64; 32],
    pub memory: Memory
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            pc: 0,
            regs: [0x0; 32],
            fregs: [0xffffffffffffffff; 32],
            memory: Memory {
                data: Box::new([0x0; MAX_ADDR])
            }
        }
    }

    pub fn get_reg(&self, reg: Reg) -> u64 {
        self.regs[reg as usize]
    }

    pub fn set_reg(&mut self, reg: Reg, val: u64) {
        if reg != REG_ZR {
            self.regs[reg as usize] = val;
        }
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


    pub unsafe fn ecall(&mut self) {
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
                syscall!(Sysno::read, a0, addr as usize, a2)
            },
            RISCV_SYSNO_WRITE => {
                let addr = self.memory.data.as_ptr().add(a1);
                syscall!(Sysno::write, a0, addr as usize, a2)
            },
            RISCV_SYSNO_NEWFSTAT => {
                let addr = self.memory.data.as_ptr().add(a1);
                syscall!(Sysno::newfstatat, a0, addr as usize)
            },
            RISCV_SYSNO_EXIT => syscall!(Sysno::exit, a0),
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
    }
}

pub struct Memory {
    data: Box<[u8; MAX_ADDR]>
}

impl Memory {
    pub fn size(&self) -> u64 { self.data.len() as u64 }

    pub fn copy_bulk(&mut self, addr: u64, data: &[u8]) {
        self.data[(addr as usize)..(addr as usize + data.len())].copy_from_slice(data);
    }

    pub fn load_u8(&self, addr: usize) -> u8 {
        self.data[addr]
    }

    pub fn load_u16(&self, addr: usize) -> u16 {
        (self.data[addr]) as u16 |
        ((self.data[addr + 1] as u16) << 8) as u16
    }

    pub fn load_u32(&self, addr: usize) -> u32 {
        (self.data[addr]) as u32 |
        ((self.data[addr + 1] as u32) << 8) as u32 |
        ((self.data[addr + 2] as u32) << 16) as u32 |
        ((self.data[addr + 3] as u32) << 24) as u32
    }

    pub fn load_u64(&self, addr: usize) -> u64 {
        (self.data[addr]) as u64 |
        ((self.data[addr + 1] as u64) << 8) as u64 |
        ((self.data[addr + 2] as u64) << 16) as u64 |
        ((self.data[addr + 3] as u64) << 24) as u64 |
        ((self.data[addr + 4] as u64) << 32) as u64 |
        ((self.data[addr + 5] as u64) << 40) as u64 |
        ((self.data[addr + 6] as u64) << 48) as u64 |
        ((self.data[addr + 7] as u64) << 56) as u64
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

