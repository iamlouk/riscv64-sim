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

	static unsigned known_solutions[20] = {
		[ 0] = 1,
		[ 1] = 1,
		[ 2] = 0,
		[ 3] = 0,
		[ 4] = 2,
		[ 5] = 10,
		[ 6] = 4,
		[ 7] = 40,
		[ 8] = 92,
		[ 9] = 352,
		[10] = 724,
		[11] = 2680,
		[12] = 14200,
		[13] = 73712,
		[14] = 365596,
		[15] = 2279184,
		[16] = 14772512,
		[17] = 95815104,
		[18] = 666090624,
	};
	if (N > 18 || known_solutions[N] != solutions) {
		printf("\n---> This is wrong!!!\n");
		return EXIT_FAILURE;
	}
	return EXIT_SUCCESS;
}

