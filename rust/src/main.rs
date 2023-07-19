mod insts;
mod cpu;

use elf;
use clap::{self, Parser};

#[derive(clap::Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    file: String
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

    // println!("[simrv64i]: ELF file header: {:?}", &elf_file.ehdr);
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
                eprintln!("[simrv64i]: failed to execute instruction (at PC={:?}): {:?}", cpu.pc, e);
                std::process::exit(1);
            }
        }
    }
}

