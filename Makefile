CC = clang
CFLAGS = -Wall -Wextra -Wno-unused-parameter -std=c11 -Og -g

.PHONY: clean

sim: sim.c lib/cpu.native.o lib/decode.native.o lib/loader.native.o
	$(CC) $(CFLAGS) -o $@ $^

lib/%.native.o: lib/%.c
	$(MAKE) -C ./lib $(notdir $@)

clean:
	rm -f sim
	rm -f *.o
	$(MAKE) -C ./lib clean

