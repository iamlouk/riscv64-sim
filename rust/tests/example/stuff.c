#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>
#include <assert.h>

static const char test_data_1[] = "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";

extern uint64_t hash_string(uint64_t seed, uint64_t length, const char data[length]) {
	uint64_t h = seed ^ 201326611ull;
	for (uint64_t i = 0; i < length; i++) {
		uint64_t byte = data[i];

		assert(byte != 0);

		h = h + byte;
		h = h ^ (h << 10);
		h = h ^ byte;
		h = h * 17;
	}
	return h;
}

int main(int argc, const char *argv[]) {
	printf("Hello World!\n");

	uint64_t seed = 0;
	unsigned repeats = 500;
	for (unsigned repeat = 0; repeat < repeats; repeat++) {
		seed += repeat;
		uint64_t hash = 0;
		unsigned len = sizeof(test_data_1) - 1;
		hash = hash_string(seed, len, test_data_1);
		hash = hash_string(hash, len, test_data_1);
		hash = hash_string(hash, len, test_data_1);
		hash = hash_string(hash, len, test_data_1);
		hash = hash_string(hash, len, test_data_1);
		if (repeat % 32 == 0)
			printf("seed: %#018lx, hash: %#018lx, len: %d\n", seed, hash, len);
		seed += hash;
	}

	return EXIT_SUCCESS;
}




