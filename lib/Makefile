CC = clang
CFLAGS = -Wall -Wextra -Wno-unused-parameter -std=c11 -nostdlib -Og -g -fsanitize=address -fsanitize=undefined
WASM_CFLAGS = -Wall -Wextra -O3 -Wno-unused-parameter -Wno-format --target=wasm32-unkown-unkown -nostdlib
WASM_LDFLAGS = -Wl,--no-entry -Wl,--export-all -Wl,--strip-all -Wl,-allow-undefined-file,imports.syms -Wl,--lto-O2 -Wl,--export-dynamic

.PHONY: all clean

all: decode-test libriscvsim.wasm

%.wasm: %.c
	$(CC) $(WASM_CFLAGS) -D TARGET_WASM -c -o $@ $<

%.native.o: %.c
	$(CC) $(CFLAGS) -D TARGET_NATIVE -c -o $@ $<

libriscvsim.wasm: ./cpu.c ./decode.c ./loader.c ./wasm-interface.c ./minilib.c
	$(CC) $(WASM_CFLAGS) $(WASM_LDFLAGS) -o $@ $^

decode-test: ./decode.test.c ./decode.c
	$(CC) -Wall -Werror -Wno-unused-parameter -std=c11 -o $@ $^

./decode.c: ./decode.h

clean:
	rm -f libriscvsim.so libriscvsim.wasm
	rm -f ./*.o
	rm -f ./*.wasm


