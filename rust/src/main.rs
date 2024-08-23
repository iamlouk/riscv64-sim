#![allow(clippy::just_underscores_and_digits)]
#![allow(clippy::upper_case_acronyms)]

mod cpu;
mod dbg;
mod insts;
mod syms;
mod tbs;

use std::io::Write;

use crate::insts::Inst;
use clap::Parser;

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
    jit: bool,

    #[arg(short, long)]
    tb_stats: bool,

    args: Vec<String>,
}

fn dump_hottest_tbs(elf_file: &elf::ElfBytes<'_, elf::endian::AnyEndian>, jit: &tbs::JIT) {
    let _ = elf_file;
    const MIN_TB_FREQ: i64 = 5;
    let mut tbs = jit
        .tbs
        .values()
        .filter_map(|tb| {
            let freq = tb.exec_count.load(std::sync::atomic::Ordering::Relaxed);
            if freq > MIN_TB_FREQ {
                Some((freq, tb))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    tbs.sort_by(|(f1, _), (f2, _)| f2.cmp(f1));
    eprintln!(
        "[simrv64i] JIT: TBs, #total={}, #freq-above-{}={}",
        jit.tbs.len(),
        MIN_TB_FREQ,
        tbs.len()
    );
    for (i, (freq, tb)) in tbs.into_iter().take(25).enumerate() {
        eprintln!(
            "[simrv64i] JIT: Top-TB #{:08?}: {:#08x?} (freq={})",
            i, tb.start, freq
        );
    }
}

fn execute(args: &Args, elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>, _: &Vec<u8>) {
    let _ = args;
    let mut cpu = cpu::CPU::new(args.jit);

    /* Avoid that the guest closes stderr. */
    let stderr_dupped = unsafe { libc::dup(2) };
    assert!(stderr_dupped != -1);
    cpu.remapped_filenos.insert(2, stderr_dupped as usize);

    let mut argv: Vec<&str> = args.args.iter().map(|s| s.as_str()).collect();
    argv.insert(0, args.file.as_str());

    match cpu.load_and_exec(&elf_file, Some(argv)) {
        Ok((exitcode, jit)) => {
            if args.tb_stats {
                dump_hottest_tbs(&elf_file, &jit);
            }
            std::process::exit(exitcode);
        }
        Err(e) => {
            eprintln!("[simrv64i]: error(pc={:#08x?}): {:?}", cpu.pc, e);
            std::process::exit(1);
        }
    }
}

fn dump_text_section(
    elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian>,
    _: &Vec<u8>,
) -> std::io::Result<()> {
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
        let raw = (bytes[offset] as u32)
            | ((bytes[offset + 1] as u32) << 8)
            | ((bytes[offset + 2] as u32) << 16)
            | ((bytes[offset + 3] as u32) << 24);
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
            let raw = (bytes[offset] as u32) | ((bytes[offset + 1] as u32) << 8);
            write!(&mut stdout, "{:8x}:\t{:04x}     \t", addr, raw)?;
        } else if size == 4 {
            let raw = (bytes[offset] as u32)
                | ((bytes[offset + 1] as u32) << 8)
                | ((bytes[offset + 2] as u32) << 16)
                | ((bytes[offset + 3] as u32) << 24);
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
            eprintln!("[simrv64i]: error reading {:?}: {}", &args.file, e);
            std::process::exit(1);
        }
    };

    let elf_file: elf::ElfBytes<'_, elf::endian::AnyEndian> =
        match elf::ElfBytes::minimal_parse(raw_file.as_slice()) {
            Ok(elf_file) => elf_file,
            Err(e) => {
                eprintln!("[simrv64i]: error parsing ELF file {:?}: {}", &args.file, e);
                std::process::exit(1);
            }
        };

    if elf_file.ehdr.class != elf::file::Class::ELF64
        || elf_file.ehdr.e_type != elf::abi::ET_EXEC
        || elf_file.ehdr.e_machine != elf::abi::EM_RISCV
    {
        eprintln!(
            "[simrv64i]: error processing ELF file {:?}: Not a RV64 executable",
            &args.file
        );
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
    use std::io::{Read, Write};
    use std::os::fd::FromRawFd;
    use std::path::PathBuf;

    fn run_example(
        filename: &str,
        argv: Option<Vec<&str>>,
        stdin: Option<&[u8]>,
        jit_enabled: bool,
    ) -> (String, i32) {
        let mut filepath = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        filepath.push(filename);

        if !filepath.exists() {
            let cmd = format!(
                "make -C {} {}",
                filepath.parent().unwrap().to_str().unwrap(),
                filepath.file_name().unwrap().to_str().unwrap()
            );
            assert_eq!(unsafe { libc::system(cmd.as_ptr() as *const i8) }, 0);
        }

        let binary_file = std::fs::read(filepath).unwrap();
        let elf_file =
            elf::ElfBytes::<'_, elf::endian::AnyEndian>::minimal_parse(&binary_file).unwrap();
        assert!(
            elf_file.ehdr.class == elf::file::Class::ELF64
                && elf_file.ehdr.e_type == elf::abi::ET_EXEC
                && elf_file.ehdr.e_machine == elf::abi::EM_RISCV
        );

        let mut cpu = crate::cpu::CPU::new(jit_enabled);

        /* Avoid that the guest closes stderr. */
        let stderr_dupped = unsafe { libc::dup(2) };
        assert!(stderr_dupped != -1);
        cpu.remapped_filenos.insert(2, stderr_dupped as usize);

        /* Redirect stdout to a pipe so that we can capture it.
         * Note that if the guest writes more than the kernel is willing
         * to buffer for us, the guest could block. Maybe read from the
         * pipe in a parallel thread? */
        let mut stdout_pipe: [i32; 2] = [-1, -1];
        assert!(0 == unsafe { libc::pipe(stdout_pipe.as_mut_ptr()) });
        cpu.remapped_filenos
            .insert(/*stdout:*/ 1, stdout_pipe[1] as usize);

        if let Some(stdin) = stdin {
            let mut stdin_pipe: [i32; 2] = [-1, -1];
            assert!(0 == unsafe { libc::pipe(stdin_pipe.as_mut_ptr()) });
            cpu.remapped_filenos
                .insert(/*stdin:*/ 0, stdin_pipe[0] as usize);

            /* If the input is larger than what the kernel is willing to buffer in
             * the kernel, then this will block and the test will never finish.
             * Solution: Write to the pipe in a parallel thread?
             */
            let mut example_stdin_file = unsafe { std::fs::File::from_raw_fd(stdin_pipe[1]) };
            example_stdin_file.write_all(stdin).unwrap();
        }

        let (exitcode, _) = cpu.load_and_exec(&elf_file, argv).unwrap();

        let mut example_stdout_file = unsafe { std::fs::File::from_raw_fd(stdout_pipe[0]) };
        let mut example_stdout = String::new();
        example_stdout_file
            .read_to_string(&mut example_stdout)
            .unwrap();

        (example_stdout, exitcode)
    }

    #[test]
    fn example_hello_world() {
        let argv = vec!["hello-world.elf", "foo"];
        let (stdout, exitcode) = run_example("./examples/hello-world.elf", Some(argv), None, false);
        assert_eq!(exitcode, 42);
        assert_eq!(
            stdout.as_str(),
            "Hello, World! (argc=2)\nargv[0] = 'hello-world.elf'\nargv[1] = 'foo'\n"
        );
    }

    #[test]
    fn example_bubblesort() {
        let input = "8\n3\n5\n6\n9\n1\n4\n2\n7\n";
        let (stdout, exitcode) = run_example(
            "./examples/bubblesort.elf",
            None,
            Some(input.as_bytes()),
            false,
        );
        assert_eq!(exitcode, 0);
        assert_eq!(stdout.as_str(), "1\n2\n3\n4\n5\n6\n7\n8\n9\n");
    }

    #[test]
    fn example_nqueens() {
        let argv = vec!["nqueens.elf", "8"];
        let (stdout, exitcode) = run_example("./examples/nqueens.elf", Some(argv), None, false);
        assert_eq!(exitcode, 0);
        assert_eq!(stdout.as_str(), "#solutions: 92 (grid_size=8)\n");
    }

    #[test]
    fn example_nqueens_jitted() {
        let argv = vec!["nqueens.elf", "8"];
        let (stdout, exitcode) = run_example("./examples/nqueens.elf", Some(argv), None, true);
        assert_eq!(exitcode, 0);
        assert_eq!(stdout.as_str(), "#solutions: 92 (grid_size=8)\n");
    }
}
