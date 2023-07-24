mod insts;
mod cpu;
mod dbg;

use std::io::Write;

use elf;
use clap::{self, Parser};
use insts::{Inst, parse_instruction};

#[derive(clap::Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    file: String,

    #[arg(short, long)]
    dump: bool,

    #[arg(short, long)]
    exec: bool
}

fn execute(elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>, _: &Vec<u8>) {
    let mut cpu = cpu::CPU::new();
    cpu.pc = elf_file.ehdr.e_entry as i64;
    let sections = elf_file.section_headers().expect("no sections");
    for section in sections {
        if section.sh_type != elf::abi::SHT_PROGBITS || section.sh_size == 0 {
            continue;
        }

        let bytes = match elf_file.section_data(&section) {
            Ok((data, None)) => data,
            Ok((_, Some(_))) => {
                eprintln!("[simrv64i]: failed to read section because its compressed");
                std::process::exit(1);
            },
            Err(e) => {
                eprintln!("[simrv64i]: failed to read section: {:?}", e);
                std::process::exit(1);
            }
        };

        if section.sh_addr + section.sh_size >= cpu.memory.size() {
            eprintln!("[simrv64i]: VM memory not large enough");
            std::process::exit(1);
        }

        cpu.memory.copy_bulk(section.sh_addr, bytes);
    }

    loop {
        match cpu.step() {
            Ok(_) => continue,
            Err(e) => {
                eprintln!("[simrv64i]: failed to execute instruction (at PC={:?}): {:?}",
                          cpu.pc, e);
                std::process::exit(1);
            }
        }
    }
}

fn dump_text_section(elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>, _: &Vec<u8>) -> std::io::Result<()> {
    let textsectionhdr = match elf_file.section_header_by_name(".text") {
        Ok(Some(hdr)) => hdr,
        Ok(None) => {
            eprintln!("[simrv64i]: no '.text' section");
            std::process::exit(1);
        },
        Err(e) => {
            eprintln!("[simrv64i]: failed to read '.text' section: {:?}", e);
            std::process::exit(1);
        }
    };

    let bytes = match elf_file.section_data(&textsectionhdr) {
        Ok((data, None)) => data,
        Ok((_, Some(_))) => {
            eprintln!("[simrv64i]: failed to read '.text' section because its compressed");
            std::process::exit(1);
        },
        Err(e) => {
            eprintln!("[simrv64i]: failed to read '.text' section: {:?}", e);
            std::process::exit(1);
        }
    };

    let mut stdout = std::io::stdout().lock();
    let mut offset: usize = 0;
    while offset + 2 < textsectionhdr.sh_size as usize {
        let addr = textsectionhdr.sh_addr as usize + offset;
        let raw =
            ((bytes[offset + 0] as u32) <<  0) |
            ((bytes[offset + 1] as u32) <<  8) |
            ((bytes[offset + 2] as u32) << 16) |
            ((bytes[offset + 3] as u32) << 24);
        let (inst, size) = match parse_instruction(raw) {
            Ok(res) => res,
            Err(_) => {
                // eprintln!("[simrv64i]: failed to parse instruction: {:?}", e);
                // eprintln!("            binary: {:032b} as address {:#x}", raw, addr);
                // std::process::exit(1);

                if (bytes[offset] & 0b11) == 0b11 {
                    (Inst::Unknown, 4)
                } else {
                    (Inst::Unknown, 2)
                }
            }
        };

        if size == 2 {
            let raw =
                ((bytes[offset + 0] as u32) << 0) |
                ((bytes[offset + 1] as u32) << 8);
            write!(&mut stdout, "{:8x}:\t{:04x}        ", addr, raw)?;
        } else if size == 4 {
            let raw =
                ((bytes[offset + 0] as u32) << 0) |
                ((bytes[offset + 1] as u32) << 8) |
                ((bytes[offset + 2] as u32) << 16) |
                ((bytes[offset + 3] as u32) << 24);
            write!(&mut stdout, "{:8x}:\t{:08x}    ", addr, raw)?;
        } else {
            panic!()
        }

        inst.print(&mut stdout, addr as i64)?;
        write!(&mut stdout, "\n")?;
        // stdout.flush().expect("stdout flush failed");
        offset += size;
    }

    Ok(())
}

fn main() {
    let args = Args::parse();
    let path = std::path::PathBuf::from(&args.file);
    let raw_file = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => {
            eprintln!("[simrv64i]: error reading {:?}: {:?}", &args.file, e);
            std::process::exit(1);
        }
    };

    let elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian> = match
            elf::ElfBytes::minimal_parse(raw_file.as_slice()) {
        Ok(elf_file) => elf_file,
        Err(e) => {
            eprintln!("[simrv64i]: error parsing ELF file {:?}: {:?}", &args.file, e);
            std::process::exit(1);
        }
    };

    if elf_file.ehdr.class != elf::file::Class::ELF64
        || elf_file.ehdr.e_type != elf::abi::ET_EXEC
        || elf_file.ehdr.e_machine != elf::abi::EM_RISCV {
        eprintln!("[simrv64i]: error processing ELF file {:?}: {}", &args.file,
            "class needs to be 64, type needs to be executable, and machine needs to be RISC-V");
        std::process::exit(1);
    }

    if args.dump {
        dump_text_section(elf_file, &raw_file).expect("I/O error");
        return;
    }

    if args.exec {
        execute(elf_file, &raw_file);
        return;
    }
}

