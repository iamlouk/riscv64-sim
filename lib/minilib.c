#include <stdbool.h>

#include "minilib.h"

int strcmp(const char *a, const char *b) {
	int i = 0;
	for (;;) {
		if (a[i] == b[i]) {
			if (a[i] == '\0')
				return 0;

			i++;
			continue;
		}

		return (a[i] < b[i]) ? -1 : 1;
	}
}

void *memset(void *s, int c, size_t n) {
	char *cs = s;
	for (size_t i = 0; i < n; i++)
		cs[i] = c;

	return s;
}

void *memcpy(void *restrict dest, const void *restrict src, size_t n) {
	char *restrict cdest = dest;
	const char *restrict csrc = src;
	for (size_t i = 0; i < n; i++)
		cdest[i] = csrc[i];
	return dest;
}

