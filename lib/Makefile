CC = clang
CFLAGS = -Wall -Wextra -Wno-unused-parameter -std=c11 -g -nostdlib
WASM_CFLAGS = -Wall -Wextra -Wno-unused-parameter --target=wasm32-unkown-unkown -nostdlib
WASM_LDFLAGS = -Wl,--no-entry -Wl,--export-all -Wl,--lto-O2

SRCFILES = $(find . -name "*.c")

.PHONY: all clean

all: decode-test libriscvsim.wasm libriscvsim.so

%.wasm: %.c
	$(CC) $(WASM_CFLAGS) -D TARGET_WASM -c -o $@ $<

%.native.o: %.c
	$(CC) $(CFLAGS) -D TARGET_NATIVE -c -o $@ $<

libriscvsim.so: $(SRCFILES:.c=.native.o)
	$(CC) $(CFLAGS) -o $@ $^

libriscvsim.wasm: ./cpu.c ./decode.c ./loader.c ./wasm-interface.c ./minilib.c
	$(CC) $(WASM_CFLAGS) $(WASM_LDFLAGS) -o $@ $^

decode-test: ./decode.test.c ./decode.c
	$(CC) -Wall -Werror -Wno-unused-parameter -std=c11 -o $@ $^

./decode.c: ./decode.h

clean:
	rm -f libriscvsim.so libriscvsim.wasm
	rm -f ./*.o
	rm -f ./*.wasm


