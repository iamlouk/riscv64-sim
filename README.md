# RISC-V Simulator

This repository contains two simulators, a old one in C at the root, and a new one in Rust.

### User-Space RISC-V Simulator in Rust

The Rust version can be found [here](./rust). It works a bit like user-mode QEMU: It simulates a user-space RISC-V binary (currently `rv64cim`), and when the executable makes a `ecall`, it translates the syscall number from the RISC-V version to the host version and let's the host kernel execute the syscall. *This only works on Linux*.

```
cd ./rust
cargo build

# Example (does two syscalls, a write(2) to stdout, and then a exit(2)):
cat ./tests/examples/hello-world.s

# Build the example:
riscv64-linux-gnu-as ./tests/examples/hello-world.s -o hello-world.o
riscv64-linux-gnu-ld -static hello-world.o -o hello-world.elf

# Run the RISC-V executable:
cargo run -- -f ./hello-world.elf -e

# Build a example using a libc (newlib):
pacman -S extra/riscv64-elf-newlib extra/riscv64-elf-gcc
riscv64-elf-gcc -O1 -static ./tests/examples/hello-world.c -o hello-world.newlib.elf
cargo run -- -f ./hello-world.newlib.elf -e
```

The rust version is actually capable of running a libc/newlib *Hello World* program. Only a select few of syscalls are implemented, so you might hit a limit soon. The C version does not support syscalls, only the most basic UART ever.

Be warned that *newlib* will, before it makes the `exit` syscall, close *stdout*. This will close the actual *stdout* of the simulator process, so that, in verbose mode, you will not see the `exit` syscall beeing called anymore.

### Bare-Metal WASM RISC-V Simulator in C

This project is deployed [here](https://louknr.net/projs/riscv64-sim/www/index.html) (That version is probably not up-to-date though). Everything is still very much __*work in progress...*__! The examples in `tests/progs` all work, you can build them by running `make all` in that directory. The root Makefile will build a CLI application and the `libriscvsim.wasm` used by the web-frontend.

Dependencies/Requirements: *riscv64-elf-gcc* for building the examples and LLVM/clang with a backend and linker for *wasm32-unkown-unkown* for building the wasm lib. Go look at the Makefile in `lib/` if you want to build using another WASM target.

- Features
  - `libriscvsim.wasm` has no external dependencies at all (Not even *libc*)
  - Usable via a CLI on x86/ARM/... and in a browser using [WebAssembly](https://developer.mozilla.org/en-US/docs/WebAssembly)
- TODOs
  - Be able to decode a lot more instructions
  - Be able to evaluate a lot more instructions
  - Proper UART
  - Make the loader be able to load data and bss sections
  - Step mode for Web-UI
  - Fix bugs (known bugs: `lui` does not work in WASM?)
- Inspiration
  - [c-to-webassembly](https://surma.dev/things/c-to-webassembly/)
  - [wasi](https://depth-first.com/articles/2019/10/16/compiling-c-to-webassembly-and-running-it-without-emscripten/)
  - [RISC-V spec](https://riscv.org/wp-content/uploads/2017/05/riscv-spec-v2.2.pdf) (Opcodes: Chap. 19)

