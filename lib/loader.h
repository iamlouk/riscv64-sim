#pragma once

#include <stdint.h>

#include "cpu.h"
#include "elf.h"

int load_binary(struct cpu *cpu, const char *binary);

