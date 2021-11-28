#include <stdint.h>

#include "minilib.h"
#include "elf.h"
#include "cpu.h"
#include "loader.h"

/*
 * The worst ELF loader ever!
 * DO NOT USE IT!
 *
 * Searches for a section called `.text` and copies its
 * contents to the entry address in cpu->memory;
 */
int load_binary(struct cpu *cpu, const char *binary) {
	const Elf64_Ehdr *ehdr = (const Elf64_Ehdr *)&binary[0];
	if (ehdr->e_ident[EI_MAG0] != ELFMAG0
		|| ehdr->e_ident[EI_MAG1] != ELFMAG1
		|| ehdr->e_ident[EI_MAG2] != ELFMAG2
		|| ehdr->e_ident[EI_MAG3] != ELFMAG3
		|| ehdr->e_ident[EI_CLASS] != ELFCLASS64) {
		return -1;
	}

	if (ehdr->e_type != ET_EXEC
		|| ehdr->e_machine != EM_RISCV) {
		return -2;
	}

	const Elf64_Shdr *nameshdr = (const Elf64_Shdr *)
		&binary[ehdr->e_shoff + ehdr->e_shstrndx * ehdr->e_shentsize];
	const char *names = &binary[nameshdr->sh_offset];

	const Elf64_Shdr *textshdr = NULL;
	for (int i = 0; i < ehdr->e_shnum; i++) {
		const Elf64_Shdr *shdr = (const Elf64_Shdr *)
			&binary[ehdr->e_shoff + i * ehdr->e_shentsize];
		const char *name = &names[shdr->sh_name];
		if (strcmp(name, ".text") == 0)
			textshdr = shdr;
	}

	if (textshdr == NULL) {
		return -3;
	}

	if (ehdr->e_entry + textshdr->sh_size >= cpu->mem_size)
		return -4;

	memcpy(&(cpu->mem[ehdr->e_entry]), &binary[textshdr->sh_offset], textshdr->sh_size);
	cpu->pc = ehdr->e_entry;
	return 0;
}

