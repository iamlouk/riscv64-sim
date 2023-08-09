#include <stdio.h>
#include <stdlib.h>

int main(int argc, const char *argv[], const char **environ) {
	printf("Hello, World! (argc=%d)\n", argc);

	for (int i = 0; i < argc; i++)
		printf("argv[%d] = '%s'\n", i, argv[i]);
	for (int i = 0; environ[i] != NULL; i++)
		printf("environ[%d] = '%s'\n", i, environ[i]);

	return EXIT_SUCCESS;
}

