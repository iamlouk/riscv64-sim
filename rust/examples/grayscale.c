#include <assert.h>
#include <errno.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

static void die(const char msg[]) {
  fprintf(stderr, "error(%s): %s\n", msg, strerror(errno));
  exit(EXIT_FAILURE);
}

static inline uint8_t *encode_uint32(uint8_t *pos, uint32_t value) {
  *(pos++) = value & 0xff;
  *(pos++) = (value >> 8) & 0xff;
  *(pos++) = (value >> 16) & 0xff;
  *(pos++) = (value >> 24) & 0xff;
  return pos;
}

struct color_palette {
  uint8_t R;
  uint8_t G;
  uint8_t B;
  uint8_t reserved;
};

static void print_grayscale_bmp(const char *filepath, size_t N, size_t M,
                                const uint8_t image[N][M]) {
  FILE *f = NULL;
  if (strcmp(filepath, "-") == 0) {
    f = stdout;
  } else {
    f = fopen(filepath, "w");
    if (!f)
      die(filepath);
  }

  uint8_t header[54];
  uint8_t *pos = &header[0];

  /* BMP header */
  *(pos++) = 'B';
  *(pos++) = 'M';
  pos = encode_uint32(pos, sizeof(header) +
                               N * M * sizeof(uint8_t)); // BMP file size
  pos = encode_uint32(pos, 0x0);                         // unused
  pos = encode_uint32(
      pos,
      sizeof(header) + 256 * sizeof(struct color_palette)); // pixel data offset

  /* DIB header */
  pos = encode_uint32(pos, 40); // DIB header length
  pos = encode_uint32(pos, N);  // height
  pos = encode_uint32(pos, M);  // width
  *(pos++) = 1;                 // color Planes
  *(pos++) = 0;
  *(pos++) = 8; // bits per Pixel
  *(pos++) = 0;
  pos = encode_uint32(pos, 0);                       // Compression
  pos = encode_uint32(pos, N * M * sizeof(uint8_t)); // RAW data size
  pos = encode_uint32(pos, 0); // horizontal print resolution
  pos = encode_uint32(pos, 0); // vertical print resolution
  pos =
      encode_uint32(pos, 0); // number of colors (0 is the default, means 2**N)
  pos = encode_uint32(pos, 0); // important colors
  assert(pos - 1 == &header[sizeof(header) - 1]);

  if (fwrite(&header[0], sizeof(uint8_t), sizeof(header), f) != sizeof(header))
    die("fwrite failed");

  /* The color map. */
  struct color_palette color_palette[256];
  for (size_t i = 0; i < 256; i++) {
    color_palette[i] = (struct color_palette){.R = i, .G = i, .B = i};
  }

  if (fwrite(&color_palette[0], sizeof(struct color_palette), 256, f) != 256)
    die("fwrite failed");

  /* The pixel data. */
  if (fwrite(&image[0][0], sizeof(uint8_t), N * M, f) != N * M)
    die("fwrite failed");

  if (fflush(f) != 0)
    die("fflush failed");

  if (f != stdout && fclose(f) != 0)
    die(filepath);
}

static int sqrti(int x) {
  if (x == 0 || x == 1)
    return x;

  int i = 1, result = 1;
  while (result <= x) {
    i++;
    result = i * i;
  }
  return i - 1;
}

static void draw_circle(size_t radius, int center_x, int center_y, size_t N,
                        size_t M, uint8_t image[N][M], uint8_t circle_color) {
  for (size_t i = 0; i < N; i++) {
    for (size_t j = 0; j < M; j++) {
      int x = ((int)i - (int)(N / 2)) + center_x;
      int y = ((int)j - (int)(M / 2)) + center_y;
      int distance = sqrti(x * x + y * y);
      if (distance < radius) {
        image[i][j] = circle_color;
      }
    }
  }
}

static void blur(size_t N, size_t M, const uint8_t image_src[N][M],
                 uint8_t image_dst[N][M]) {
  for (size_t i = 1; i < N - 1; i++) {
    for (size_t j = 1; j < M - 1; j++) {
      uint64_t pixel = image_src[i - 1][j - 1] + image_src[i - 1][j - 0] +
                       image_src[i - 1][j + 1] + image_src[i - 0][j - 1] +
                       image_src[i - 0][j - 0] + image_src[i - 0][j + 1] +
                       image_src[i + 1][j - 1] + image_src[i + 1][j - 0] +
                       image_src[i + 1][j + 1];
      image_dst[i][j] = (uint8_t)(pixel / 9);
    }
  }
}

__attribute__((noinline)) static void init(size_t N, size_t M,
                                           uint8_t (*image)[M], uint8_t val) {
  for (size_t i = 0; i < N; i++)
    for (size_t j = 0; j < M; j++)
      image[i][j] = val;
}

int main() {
  // fprintf(stderr, "debug: main started...\n");

  size_t N = 500;
  size_t M = 500;
  uint8_t(*image)[M] = malloc(N * M * sizeof(uint8_t));

  init(N, M, image, 0x00);

  // fprintf(stderr, "debug: image allocated...\n");
  // fprintf(stderr, "debug: image initialized...\n");

#if 1
  draw_circle(150, 100, 100, N, M, image, 0xff);
  draw_circle(100, -50, -100, N, M, image, 0xb0);
  draw_circle(200, 200, -100, N, M, image, 0x80);
  draw_circle(100, -50, 100, N, M, image, 0x40);
  draw_circle(50, 50, 200, N, M, image, 0xb0);
  draw_circle(200, -250, -250, N, M, image, 0x80);
  fprintf(stderr, "debug: circles drawn...\n");

  uint8_t(*tmp_image)[M] = malloc(N * M * sizeof(uint8_t));
  for (size_t i = 0; i < 10; i++) {
    blur(N, M, image, tmp_image);
    blur(N, M, tmp_image, image);
  }
  fprintf(stderr, "debug: blured...\n");

  print_grayscale_bmp("-", N, M, image);
  fprintf(stderr, "debug: done!\n");
#endif
  return EXIT_SUCCESS;
}
