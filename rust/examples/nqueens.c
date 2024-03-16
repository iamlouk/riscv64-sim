#include <stdio.h>
#include <stdlib.h>

static unsigned solve(unsigned N, unsigned col, unsigned hist[N]) {
	if (col == N) {
		return 1;
	}

	unsigned solutions = 0;
	for (unsigned i = 0; i < N; i++) {
		unsigned j = 0;
		while (j < col) {
			if (hist[j] == i || abs((signed)hist[j] - (signed)i) == col - j)
				break;

			j++;
		}

		if (j < col) continue;

		hist[col] = i;
		solutions += solve(N, col + 1, hist);
	}
	return solutions;
}

int main(int argc, const char *argv[]) {
	unsigned N = 8;
	if (argc > 1)
		N = atoi(argv[1]);

	unsigned hist[N];
	unsigned solutions = solve(N, 0, hist);
	printf("#solutions: %u (grid_size=%u)\n", solutions, N);
}

