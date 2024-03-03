#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::upper_case_acronyms)]

mod insts;
mod cpu;
mod dbg;
mod syms;
mod tbs;

use std::io::Write;

use clap::{self, Parser};
use crate::insts::Inst;

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
    let _ = args;
    let mut cpu = cpu::CPU::new();
    match cpu.load_and_exec(elf_file) {
        Ok(exitcode) => {
            std::process::exit(exitcode);
        },
        Err(e) => {
            eprintln!("[simrv64i]: error: {:?}", e);
            std::process::exit(1);
        }
    }
}

fn dump_text_section(elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>, _: &Vec<u8>)
        -> std::io::Result<()> {
    let text_section = match elf_file.section_header_by_name(".text") {
        Ok(Some(hdr)) => hdr,
        Ok(None) | Err(_) => {
            eprintln!("[simrv64i]: failed to find '.text' section");
            std::process::exit(1);
        }
    };

    let bytes = match elf_file.section_data(&text_section) {
        Ok((data, None)) => data,
        Ok((_, Some(_))) | Err(_) => {
            eprintln!("[simrv64i]: failed to read '.text' section");
            std::process::exit(1);
        }
    };

    let mut stdout = std::io::stdout().lock();
    let mut offset: usize = 0;
    while offset + 2 < text_section.sh_size as usize {
        let addr = text_section.sh_addr as usize + offset;
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
        eprintln!("[simrv64i]: error processing ELF file {:?}: Not a RV64 executable",
                  &args.file);
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

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    const EXAMPLES: [(&'static str, i32, &'static str); 1] = [
        ("hello-world.elf", 42, "Hello, World! (argc=0)\n")
    ];

    #[test]
    fn examples() {
        let mut root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        root_dir.push("examples/foo.txt");
        for (filename, exitcode, expected_stdout) in EXAMPLES {
            let binary_path = root_dir.with_file_name(filename);
            let binary_file = std::fs::read(binary_path).unwrap();
            let elf_file =
                elf::ElfBytes::<'_, elf::endian::AnyEndian>::minimal_parse(&binary_file).unwrap();
            assert!(elf_file.ehdr.class == elf::file::Class::ELF64
                && elf_file.ehdr.e_type == elf::abi::ET_EXEC
                && elf_file.ehdr.e_machine == elf::abi::EM_RISCV);

            let mut cpu = crate::cpu::CPU::new();
            let mut raw_stdout: Vec<u8> = Vec::new();
            cpu.capture_filenos.insert(1, (Some(&mut raw_stdout), None));

            assert_eq!(cpu.load_and_exec(elf_file).unwrap(), exitcode);
            assert_eq!(expected_stdout.as_bytes(), raw_stdout.as_slice());
        }
    }
}

