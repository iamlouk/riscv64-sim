use crate::insts::*;
use syscalls::{syscall, Sysno};

const MAX_ADDR: usize = 2usize.pow(20);

pub struct CPU {
    pub pc: i64,
    pub regs: [u64; 32],
    pub memory: Memory
}

impl CPU {
    pub fn new() -> Self {
        CPU {
            pc: 0,
            regs: [0x0; 32],
            memory: Memory {
                data: Box::new([0x0; MAX_ADDR])
            }
        }
    }

    pub fn step(&mut self) -> Result<(), Error> {
        let raw = self.memory.load_u32(self.pc as usize);
        let (inst, size) = parse_instruction(raw)?;
        execute_instruction(self, inst, size as i64);
        Ok(())
    }

    pub fn get_reg(&self, reg: Reg) -> u64 {
        if reg == REG_ZR {
            0x0
        } else {
            self.regs[reg as usize]
        }
    }

    pub fn set_reg(&mut self, reg: Reg, val: u64) {
        if reg != REG_ZR {
            self.regs[reg as usize] = val;
        }
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

const RISCV_SYSNO_OPEN:  u64 = 430;
const RISCV_SYSNO_READ:  u64 = 63;
const RISCV_SYSNO_WRITE: u64 = 64;
const RISCV_SYSNO_CLOSE: u64 = 57;
const RISCV_SYSNO_EXIT:  u64 = 93;

pub unsafe fn ecall(cpu: &mut CPU) {
    const A0: Reg = 10;
    const A1: Reg = 11;
    const A2: Reg = 12;
    const _A3: Reg = 13;
    const _A4: Reg = 14;
    const _A5: Reg = 15;
    const _A6: Reg = 16;
    const A7: Reg = 17;


    // Linux Syscall Numbers, for whatever reason, are different on different architectures.
    // See https://jborza.com/post/2021-05-11-riscv-linux-syscalls/. Let's hope at least
    // the arguments are the same and that the rust crate `syscalls` are those of the host.
    let syscall = cpu.get_reg(A7);
    let res = match syscall {
        RISCV_SYSNO_OPEN => {
            let filepath = cpu.memory.data.as_ptr().add(cpu.get_reg(A0) as usize);
            let flags = cpu.get_reg(A1);
            syscall!(Sysno::open, filepath as usize, flags)
        },
        RISCV_SYSNO_READ => {
            let fd = cpu.get_reg(A0) as usize;
            let count = cpu.get_reg(A2) as usize;
            let addr = cpu.memory.data.as_ptr().add(cpu.get_reg(A1) as usize);
            syscall!(Sysno::read, fd, addr as usize, count)
        },
        RISCV_SYSNO_WRITE => {
            let fd = cpu.get_reg(A0) as usize;
            let count = cpu.get_reg(A2) as usize;
            let addr = cpu.memory.data.as_ptr().add(cpu.get_reg(A1) as usize);
            syscall!(Sysno::write, fd, addr as usize, count)
        },
        RISCV_SYSNO_CLOSE => {
            let fd = cpu.get_reg(A0) as usize;
            syscall!(Sysno::close, fd)
        },
        RISCV_SYSNO_EXIT => {
            let status = cpu.get_reg(A0) as usize;
            syscall!(Sysno::exit, status)
        },
        _ => todo!()
    };

    cpu.set_reg(A0, match res {
        Ok(val) => val as u64,
        Err(errno) => -(errno.into_raw() as i64) as u64
    });
}


