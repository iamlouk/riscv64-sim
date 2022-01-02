CC = clang
CFLAGS = -Wall -Wextra -Wno-unused-parameter -std=c11 -Og -g -fsanitize=address -fsanitize=undefined

.PHONY: clean all

all: sim lib/libriscvsim.wasm

lib/libriscvsim.wasm: $(find lib/*.c) $(find lib/*.h)
	$(MAKE) -C ./lib libriscvsim.wasm

sim: sim.c lib/cpu.native.o lib/decode.native.o lib/loader.native.o lib/minilib.native.o
	$(CC) $(CFLAGS) -o $@ $^

lib/%.native.o: lib/%.c
	$(MAKE) -C ./lib $(notdir $@)

clean:
	rm -f sim
	rm -f *.o
	$(MAKE) -C ./lib clean

