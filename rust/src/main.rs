#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::upper_case_acronyms)]

mod insts;
mod cpu;
mod dbg;
mod syms;
mod tbs;

use std::io::Write;

use clap::{self, Parser};
use crate::insts::{Inst, REG_SP};

#[derive(clap::Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    file: String,

    #[arg(short, long)]
    dump: bool,

    #[arg(short, long)]
    exec: bool,

    #[arg(short, long)]
    verbose: bool,

    #[arg(short, long)]
    jit: bool
}

fn execute(args: &Args, elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>, _: &Vec<u8>) {
    let mut cpu = cpu::CPU::new();
    cpu.pc = elf_file.ehdr.e_entry as i64;

    let (sections, string_table) = match elf_file.section_headers_with_strtab() {
        Ok((sections, string_table)) => match (sections, string_table) {
            (Some(sections), Some(string_table)) => (sections, string_table),
            (None, _) => {
                eprintln!("[simrv64i]: no section headers");
                std::process::exit(1);
            },
            (_, None) => {
                eprintln!("[simrv64i]: no section headers");
                std::process::exit(1);
            }
        },
        Err(e) => {
            eprintln!("[simrv64i]: failed to parse headers and string table: {}", e);
            std::process::exit(1);
        }
    };

    for section in sections {
        // TODO: There must be a better way of knowing if this section is important...!
        // TODO: Read ELF spec and so on?
        let name = string_table.get(section.sh_name as usize).unwrap_or("<unnamed>");
        let skip = !matches!(name,
            ".rela.dyn" | ".text" | ".rodata" | ".data.rel.ro" | ".data" | ".sdata" | ".tdata" |
            ".eh_frame" | ".gcc_except_table" | ".init_array" | ".fini_array" | ".preinit_array" |
            "__libc_freeres_fn" | "__libc_subfreeres" | "__libc_IO_vtables" | "__libc_atexit" |
            ".got" | "__libc_freeres_ptrs");


        if args.verbose {
            eprintln!("[simrv64i]:{} {:#08x} - {:#08x}: section {:?} ({} bytes)",
                if skip { " skipped:" } else { "" },
                section.sh_addr, section.sh_addr + section.sh_size, name, section.sh_size);
        }
        if skip {
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

    let symbols = syms::get_symbols(&elf_file);
    let symbol_tree = syms::SymbolTreeNode::build(&symbols).unwrap();
    eprintln!("[simrv64i]: start-pc={:#08x}, number-of-symbols: {}", cpu.pc, symbol_tree.count());

    let top_of_stack = 0x10000u64;
    cpu.set_reg(REG_SP, top_of_stack);

    let mut call_stack: Vec<&str> = vec![];
    if let Some((name, _)) = symbol_tree.lookup(cpu.pc as u64) {
        call_stack.push(name);
    }

    let mut jit = tbs::JIT::new();
    if args.jit {
        loop {
            match cpu.step(&mut jit, &symbol_tree) {
                Ok(_) => continue,
                Err(e) => {
                    eprintln!("[simrv64i]: at PC={:08x}: {:?}", cpu.pc, e);
                    std::process::exit(1);
                }
            }
        }
    }

    if args.verbose {
        eprintln!("[simrv64i]: starting VM...");
    }
    let mut stdout = std::io::stdout().lock();
    loop {
        let raw = cpu.memory.load_u32(cpu.pc as usize);
        let (inst, inst_size) = match Inst::parse(raw) {
            Ok((inst, inst_size)) => (inst, inst_size as i64),
            Err(e) => {
                eprintln!("[simrv64i]: failed to execute instruction (at PC={:08x}): {:?}",
                          cpu.pc, e);
                std::process::exit(1);
            }
        };

        let old_pc = cpu.pc;
        if args.verbose {
            write!(&mut stdout, "[{:8x}]:\t", cpu.pc).unwrap();
            inst.print(&mut stdout, cpu.pc).unwrap();
            writeln!(&mut stdout).unwrap();
        }
        inst.exec(inst_size, &mut cpu);
        if args.verbose && cpu.pc != old_pc + inst_size {
            if inst.is_ret() && !call_stack.is_empty() {
                call_stack.pop();
            }

            let (name, start) = symbol_tree.lookup(cpu.pc as u64).unwrap_or(("???", 0));
            if inst.is_call() {
                call_stack.push(name);
            }
            writeln!(&mut stdout, "[simrv64i]: {} + {:#x}",
                   name, cpu.pc as u64 - start).unwrap();
            if (inst.is_call() && start == cpu.pc as u64) || inst.is_ret() {
                writeln!(&mut stdout, "[simrv64i]: call-stack: {:?}", call_stack).unwrap();
            }
        }
    }
}

fn dump_text_section(elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>, _: &Vec<u8>)
        -> std::io::Result<()> {
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
            (bytes[offset] as u32) |
            ((bytes[offset + 1] as u32) <<  8) |
            ((bytes[offset + 2] as u32) << 16) |
            ((bytes[offset + 3] as u32) << 24);
        let (inst, size) = match Inst::parse(raw) {
            Ok(res) => res,
            Err(_) => {
                if (bytes[offset] & 0b11) == 0b11 {
                    (Inst::Unknown, 4)
                } else {
                    (Inst::Unknown, 2)
                }
            }
        };

        if size == 2 {
            let raw =
                (bytes[offset] as u32) |
                ((bytes[offset + 1] as u32) << 8);
            write!(&mut stdout, "{:8x}:\t{:04x}     \t", addr, raw)?;
        } else if size == 4 {
            let raw =
                (bytes[offset] as u32) |
                ((bytes[offset + 1] as u32) << 8) |
                ((bytes[offset + 2] as u32) << 16) |
                ((bytes[offset + 3] as u32) << 24);
            write!(&mut stdout, "{:8x}:\t{:08x} \t", addr, raw)?;
        } else {
            panic!()
        }

        inst.print(&mut stdout, addr as i64)?;
        writeln!(&mut stdout)?;
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
        eprintln!("[simrv64i]: error processing ELF file {:?}: Not a RV64 executable", &args.file);
        std::process::exit(1);
    }

    if args.dump {
        dump_text_section(elf_file, &raw_file).expect("I/O error");
        return;
    }

    if args.exec {
        execute(&args, elf_file, &raw_file);
    }
}

