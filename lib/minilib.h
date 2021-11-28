#pragma once

typedef unsigned long size_t;

#define NULL ((void*) 0x0)

int strcmp(const char *a, const char *b);

void *memset(void *s, int c, size_t n);

void *memcpy(void *restrict dest, const void *restrict src, size_t n);

