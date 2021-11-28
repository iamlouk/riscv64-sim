#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>

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

char *strcpy(char *restrict dest, const char *src) {
	int i;
	for (i = 0; src[i] != '\0'; i++)
		dest[i] = src[i];

	dest[i] = '\0';
	return dest;
}

void *memcpy(void *restrict dest, const void *restrict src, size_t n) {
	char *restrict cdest = dest;
	const char *restrict csrc = src;
	for (size_t i = 0; i < n; i++)
		cdest[i] = csrc[i];
	return dest;
}

static const char char_table[] = {
	'0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'
};

static int int64_to_str(char *dest, int64_t x, int64_t base) {
	char c = char_table[x % base];
	int n = 0;
	x = x / base;
	if (x > 0)
		n = int64_to_str(dest, x, base);

	dest[n] = c;
	return n + 1;
}

int sprintf(char *dest, const char *fmt, ...) {
	va_list ap;
	va_start(ap, fmt);

	int n = 0;
	for (const char *c = fmt; *c != '\0'; c++) {
		if (*c != '%') {
			dest[n++] = *c;
			continue;
		}

		c += 1;
		if (*c == '%') {
			dest[n++] = '%';
			continue;
		}

		if (*c == 'l' && *(c + 1) == 'x') {
			c += 1;
			int64_t x = va_arg(ap, int64_t);
			if (x < 0) {
				x = -x;
				dest[n++] = '-';
			}

			dest[n++] = '0';
			dest[n++] = 'x';
			n += int64_to_str(&dest[n], x, 16);
			continue;
		}

		if (*c == 'l' && *(c + 1) == 'u') {
			c += 1;
			int64_t x = va_arg(ap, int64_t);
			n += int64_to_str(&dest[n], x, 10);
			continue;
		}

		if (*c == 's') {
			const char *s = va_arg(ap, const char *);
			while (*s != '\0') {
				dest[n++] = *s;
				s += 1;
			}
			continue;
		}

		dest[n++] = '%';
		dest[n++] = *c;
	}

	va_end(ap);
	dest[n] = '\0';
	return n;
}

