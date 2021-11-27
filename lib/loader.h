#pragma once

#include <stdlib.h>
#include <stdint.h>
#include <stdio.h>

#include "cpu.h"
#include "elf.h"

int load_binary(struct cpu *cpu, const char *binary);

