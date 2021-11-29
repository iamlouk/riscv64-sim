# RISC-V Simulator

__*work in progress...*__ 

The examples in `tests/progs` all work. Build the examples by running `make <example-name>.elf` in that directory. The command line application can be
build by calling the root `Makefile`. Call `make libriscvsim.wasm` in `lib/` to build the WebAssembly library.

Requirements: *riscv64-elf-gcc* and LLVM/clang with a backend that is able to target *wasm32-unkown-unkown*. Makefiles can be changed if different compilers are used.

## Goals

- Have a *zero-dependencies* core in C which can be extended and used as a library
    - Library takes in an ELF executable and loads it into the virt. memory and then executes it
    - Provide callbacks/configuration for mapped memory regions such as URAT
    - Virt. extensible CPU that can execute RISC-V instructions directly (Extend to JIT and/or binary translation later)
- Make core compileable to WASM, build a frontend for the Web
    - Small glue C code (callbacks), provide API to JS
    - UI for uploading ELF file, starting execution, UART in/output
- Make CLI wrapper
    - Pass it the executable, it gets executed, done

- Inspiration:
    - [c-to-webassembly](https://surma.dev/things/c-to-webassembly/)
    - [wasi](https://depth-first.com/articles/2019/10/16/compiling-c-to-webassembly-and-running-it-without-emscripten/)
    - [RISC-V spec](https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf) (Opcodes: Chap. 19)

