#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <stdbool.h>

static void bubblesort(size_t count, uint64_t *data) {
	bool change = true;
	while (change) {
		change = false;
		for (size_t i = 0; i < count - 1; i++) {
			if (data[i + 1] < data[i]) {
				uint64_t tmp = data[i];
				data[i] = data[i + 1];
				data[i + 1] = tmp;
				change = true;
			}
		}
	}
}

int main(int argc, const char *argv[], const char **environ) {
	(void) argc;
	(void) argv;
	(void) environ;

	size_t count = 0;
	size_t capacity = 64;
	uint64_t *data = malloc(capacity * sizeof(uint64_t));
	for (;;) {
		char buf[64];
		if (!fgets(buf, sizeof(buf) - 1, stdin) || buf[0] == '\0')
			break;

		if (count + 1 > capacity) {
			capacity *= 2;
			data = realloc(data, capacity * sizeof(uint64_t));
		}

		data[count] = strtol(buf, NULL, 10);
		count += 1;
	}

	bubblesort(count, data);

	for (size_t i = 0; i < count; i++)
		printf("%ld\n", data[i]);

	return EXIT_SUCCESS;
}

