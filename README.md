# RISC-V Simulator

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

