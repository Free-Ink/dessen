// Lights Out: flipping a square also flips its four neighbours; turn the whole
// board white. Works by touch (tap a square) or buttons (Prev/Next move the
// cursor, Confirm flips, Back quits). Every level is solvable: it is built by
// pressing random squares starting from a solved board.
#include "dessen.h"

#define N 5
#define HEADER 64
#define FOOTER 110
#define GAP 8

static unsigned char lit[N][N];
static int cursor, moves, level = 1;
static int cell, grid_x, grid_y;
static unsigned rng;

static unsigned rnd(void) {
  rng = rng * 1664525u + 1013904223u;
  return rng >> 16;
}

static void press(int r, int c) {
  static const signed char d[5][2] = {{0, 0}, {1, 0}, {-1, 0}, {0, 1}, {0, -1}};
  for (int i = 0; i < 5; i++) {
    int rr = r + d[i][0], cc = c + d[i][1];
    if (rr >= 0 && rr < N && cc >= 0 && cc < N) lit[rr][cc] ^= 1;
  }
}

static int solved(void) {
  for (int i = 0; i < N * N; i++)
    if (lit[i / N][i % N]) return 0;
  return 1;
}

static void new_level(void) {
  int presses = 3 + level > 12 ? 12 : 3 + level;
  rng = (unsigned)level * 2654435761u;
  do {
    for (int i = 0; i < N * N; i++) lit[i / N][i % N] = 0;
    for (int i = 0; i < presses; i++) press(rnd() % N, rnd() % N);
  } while (solved());
  moves = 0;
}

static char *put_str(char *p, const char *s) {
  while (*s) *p++ = *s++;
  return p;
}

static char *put_num(char *p, unsigned n) {
  char tmp[10];
  int len = 0;
  do tmp[len++] = (char)('0' + n % 10);
  while (n /= 10);
  while (len) *p++ = tmp[--len];
  return p;
}

static void draw(void) {
  char buf[48], *p = buf;
  p = put_str(p, "Lights Out   Level ");
  p = put_num(p, (unsigned)level);
  p = put_str(p, "   Moves ");
  p = put_num(p, (unsigned)moves);
  *p = 0;

  dsn_clear();
  dsn_text(grid_x, 16, buf);
  for (int i = 0; i < N * N; i++) {
    int r = i / N, c = i % N, on = lit[r][c];
    int x = grid_x + c * cell, y = grid_y + r * cell, s = cell - GAP;
    dsn_fill(x, y, s, s, DSN_BLACK);
    if (!on) dsn_fill(x + 3, y + 3, s - 6, s - 6, DSN_WHITE);
    if (i == cursor) { // button cursor: a dot in the opposite colour
      int m = s * 3 / 8;
      dsn_fill(x + m, y + m, s - 2 * m, s - 2 * m, on ? DSN_WHITE : DSN_BLACK);
    }
  }
  int fy = grid_y + N * cell + 12;
  if (solved()) {
    dsn_text(grid_x, fy, "Solved! Tap or Confirm for the next level.");
  } else {
    dsn_text(grid_x, fy, "Turn every square white. Tap one to flip it");
    dsn_text(grid_x, fy + 36, "and its neighbours. Back quits.");
  }
  dsn_refresh();
}

int main(void) {
  int w = dsn_screen_width(), h = dsn_screen_height();
  int avail_w = w - 48, avail_h = h - HEADER - FOOTER;
  cell = (avail_w < avail_h ? avail_w : avail_h) / N;
  grid_x = (w - N * cell + GAP) / 2;
  grid_y = HEADER;

  new_level();
  draw();
  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) return 0;
    if (solved()) {
      if (type != DSN_EV_CONFIRM && type != DSN_EV_TAP) continue;
      level++;
      new_level();
    } else if (type == DSN_EV_PREV) {
      cursor = (cursor + N * N - 1) % (N * N);
    } else if (type == DSN_EV_NEXT) {
      cursor = (cursor + 1) % (N * N);
    } else if (type == DSN_EV_CONFIRM) {
      press(cursor / N, cursor % N);
      moves++;
    } else if (type == DSN_EV_TAP) {
      int x = DSN_EV_X(e) - grid_x, y = DSN_EV_Y(e) - grid_y;
      if (x < 0 || y < 0 || x >= N * cell || y >= N * cell) continue;
      cursor = (y / cell) * N + x / cell;
      press(cursor / N, cursor % N);
      moves++;
    } else {
      continue;
    }
    draw();
  }
}
