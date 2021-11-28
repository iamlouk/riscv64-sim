#pragma once

typedef unsigned long size_t;

#define NULL ((void*) 0x0)

int strcmp(const char *a, const char *b);

void *memset(void *s, int c, size_t n);

char *strcpy(char *restrict dest, const char *src);

void *memcpy(void *restrict dest, const void *restrict src, size_t n);

int sprintf(char *dest, const char *fmt, ...);

