// Sudoku. Every puzzle is generated on the device and has exactly one solution.
// Touch: tap a cell, then tap a number below the grid (X erases).
// Buttons: Prev/Next move the cursor, Confirm steps the cell through 1-9 and
// blank, Back quits.
#include "dessen.h"

#define MARGIN 16
#define GRID_Y 48
#define TARGET_CLUES 30

static unsigned char grid[81], solution[81], given[81];
static unsigned short row_used[9], col_used[9], box_used[9]; // bit d-1 set = digit d placed
static int sel = 40, cell, grid_x, pad_y, pad_w;

#define ROW(i) ((i) / 9)
#define COL(i) ((i) % 9)
#define BOX(i) (ROW(i) / 3 * 3 + COL(i) / 3)

static unsigned below(unsigned n) { return (unsigned)(((unsigned long long)dsn_random() * n) >> 32); }

static void shuffle(unsigned char *a, int n) {
  for (int i = n - 1; i > 0; i--) {
    int j = (int)below((unsigned)i + 1);
    unsigned char t = a[i];
    a[i] = a[j];
    a[j] = t;
  }
}

static void set_cell(int i, int d) { // d = 0 clears
  unsigned short bit;
  if (grid[i]) {
    bit = (unsigned short)~(1u << (grid[i] - 1));
    row_used[ROW(i)] &= bit, col_used[COL(i)] &= bit, box_used[BOX(i)] &= bit;
  }
  grid[i] = (unsigned char)d;
  if (d) {
    bit = (unsigned short)(1u << (d - 1));
    row_used[ROW(i)] |= bit, col_used[COL(i)] |= bit, box_used[BOX(i)] |= bit;
  }
}

// Counts solutions of the current grid, stopping at `limit`. Always fills the
// cell with the fewest candidates next, which keeps the search small.
static int count_solutions(int limit) {
  int best = -1, best_n = 10;
  unsigned best_mask = 0;
  for (int i = 0; i < 81; i++) {
    if (grid[i]) continue;
    unsigned mask = ~(unsigned)(row_used[ROW(i)] | col_used[COL(i)] | box_used[BOX(i)]) & 0x1ff;
    int n = 0;
    for (unsigned m = mask; m; m &= m - 1) n++;
    if (n < best_n) best = i, best_n = n, best_mask = mask;
    if (n <= 1) break;
  }
  if (best < 0) return 1; // no empty cell: this is a solution
  int total = 0;
  for (int d = 1; d <= 9 && total < limit; d++) {
    if (!(best_mask & (1u << (d - 1)))) continue;
    set_cell(best, d);
    total += count_solutions(limit - total);
    set_cell(best, 0);
  }
  return total;
}

static void new_puzzle(void) {
  // A valid solved grid: a fixed pattern with digits, rows and columns
  // shuffled in ways that keep it valid (rows within a band, bands, same for columns).
  unsigned char digit[9], rows[9], cols[9], band[3], order[81];
  for (int i = 0; i < 9; i++) digit[i] = (unsigned char)(i + 1);
  shuffle(digit, 9);
  for (int pass = 0; pass < 2; pass++) {
    unsigned char *out = pass ? cols : rows, in[3];
    for (int i = 0; i < 3; i++) band[i] = (unsigned char)i;
    shuffle(band, 3);
    for (int b = 0; b < 3; b++) {
      for (int i = 0; i < 3; i++) in[i] = (unsigned char)i;
      shuffle(in, 3);
      for (int i = 0; i < 3; i++) out[b * 3 + i] = (unsigned char)(band[b] * 3 + in[i]);
    }
  }
  for (int i = 0; i < 9; i++) row_used[i] = col_used[i] = box_used[i] = 0;
  for (int i = 0; i < 81; i++) {
    int r = rows[ROW(i)], c = cols[COL(i)];
    grid[i] = 0;
    solution[i] = digit[(r * 3 + r / 3 + c) % 9];
    set_cell(i, solution[i]);
  }

  // Remove cells in random order, keeping each removal only if the puzzle
  // still has a single solution.
  for (int i = 0; i < 81; i++) order[i] = (unsigned char)i;
  shuffle(order, 81);
  int clues = 81;
  for (int k = 0; k < 81 && clues > TARGET_CLUES; k++) {
    int i = order[k];
    set_cell(i, 0);
    if (count_solutions(2) == 1) clues--;
    else set_cell(i, solution[i]);
  }
  for (int i = 0; i < 81; i++) given[i] = grid[i] != 0;
}

static int empty_cells(void) {
  int n = 0;
  for (int i = 0; i < 81; i++) n += !grid[i];
  return n;
}

static int is_solved(void) {
  for (int i = 0; i < 81; i++)
    if (grid[i] != solution[i]) return 0;
  return 1;
}

// Seven-segment digit dw wide and 2*dw tall with stroke t.
static void draw_digit(int x, int y, int dw, int t, int d) {
  static const unsigned char seg[10] = {0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f};
  int m = seg[d];
  if (m & 0x01) dsn_fill(x, y, dw, t, DSN_BLACK);
  if (m & 0x02) dsn_fill(x + dw - t, y, t, dw, DSN_BLACK);
  if (m & 0x04) dsn_fill(x + dw - t, y + dw, t, dw, DSN_BLACK);
  if (m & 0x08) dsn_fill(x, y + 2 * dw - t, dw, t, DSN_BLACK);
  if (m & 0x10) dsn_fill(x, y + dw, t, dw, DSN_BLACK);
  if (m & 0x20) dsn_fill(x, y, t, dw, DSN_BLACK);
  if (m & 0x40) dsn_fill(x, y + dw - t / 2, dw, t, DSN_BLACK);
}

static char *put_str(char *p, const char *s) {
  while (*s) *p++ = *s++;
  return p;
}

static void draw(void) {
  int size = 9 * cell, dw = cell * 3 / 10;
  dsn_clear();
  dsn_text(grid_x, 12, "Sudoku");

  // Selected cell: a solid frame inside the cell.
  int sx = grid_x + COL(sel) * cell, sy = GRID_Y + ROW(sel) * cell;
  dsn_fill(sx, sy, cell, cell, DSN_BLACK);
  dsn_fill(sx + 5, sy + 5, cell - 10, cell - 10, DSN_WHITE);

  for (int k = 0; k <= 9; k++) { // box borders are heavier
    int width = k % 3 ? 1 : 4;
    dsn_line(grid_x + k * cell, GRID_Y, grid_x + k * cell, GRID_Y + size, width, DSN_BLACK);
    dsn_line(grid_x, GRID_Y + k * cell, grid_x + size, GRID_Y + k * cell, width, DSN_BLACK);
  }
  for (int i = 0; i < 81; i++) { // printed clues are bold, your entries thin
    if (!grid[i]) continue;
    int t = given[i] ? dw / 3 : dw / 7;
    draw_digit(grid_x + COL(i) * cell + (cell - dw) / 2, GRID_Y + ROW(i) * cell + (cell - 2 * dw) / 2, dw,
               t < 2 ? 2 : t, grid[i]);
  }

  // Number pad: 1-9 then X (erase).
  int pdw = pad_w * 3 / 10;
  for (int k = 0; k < 10; k++) {
    int x = grid_x + k * pad_w;
    dsn_fill(x + 2, pad_y, pad_w - 4, pad_w, DSN_BLACK);
    dsn_fill(x + 4, pad_y + 2, pad_w - 8, pad_w - 4, DSN_WHITE);
    if (k < 9) {
      draw_digit(x + (pad_w - pdw) / 2, pad_y + (pad_w - 2 * pdw) / 2, pdw, pdw / 4 < 2 ? 2 : pdw / 4, k + 1);
    } else {
      dsn_line(x + pad_w / 4, pad_y + pad_w / 4, x + pad_w * 3 / 4, pad_y + pad_w * 3 / 4, 3, DSN_BLACK);
      dsn_line(x + pad_w * 3 / 4, pad_y + pad_w / 4, x + pad_w / 4, pad_y + pad_w * 3 / 4, 3, DSN_BLACK);
    }
  }

  char buf[64], *p = buf;
  int left = empty_cells();
  if (is_solved()) {
    p = put_str(p, "Solved! Tap or Confirm for a new puzzle.");
  } else if (left == 0) {
    p = put_str(p, "Full, but something is wrong.");
  } else {
    if (left >= 10) *p++ = (char)('0' + left / 10);
    *p++ = (char)('0' + left % 10);
    p = put_str(p, " cells left. Tap a cell, then a number.");
  }
  *p = 0;
  dsn_text(grid_x, pad_y + pad_w + 12, buf);
  dsn_refresh();
}

int main(void) {
  int w = dsn_screen_width(), h = dsn_screen_height();
  // Leave room under the grid for the pad (about a cell tall) and a status line.
  int by_w = (w - 2 * MARGIN) / 9, by_h = (h - GRID_Y - 70) / 10;
  cell = by_w < by_h ? by_w : by_h;
  grid_x = (w - 9 * cell) / 2;
  pad_w = 9 * cell / 10;
  pad_y = GRID_Y + 9 * cell + 14;

  dsn_clear();
  dsn_text(MARGIN, 12, "Sudoku: making a puzzle...");
  dsn_refresh();
  new_puzzle();
  draw();

  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) return 0;
    if (is_solved()) {
      if (type != DSN_EV_CONFIRM && type != DSN_EV_TAP) continue;
      new_puzzle();
    } else if (type == DSN_EV_PREV) {
      sel = (sel + 80) % 81;
    } else if (type == DSN_EV_NEXT) {
      sel = (sel + 1) % 81;
    } else if (type == DSN_EV_CONFIRM) {
      if (given[sel]) continue;
      set_cell(sel, (grid[sel] + 1) % 10);
    } else if (type == DSN_EV_TAP) {
      int x = DSN_EV_X(e) - grid_x, y = DSN_EV_Y(e);
      if (x < 0 || x >= 9 * cell) continue;
      if (y >= GRID_Y && y < GRID_Y + 9 * cell) {
        sel = (y - GRID_Y) / cell * 9 + x / cell;
      } else if (y >= pad_y && y < pad_y + pad_w && !given[sel]) {
        int k = x / pad_w;
        set_cell(sel, k < 9 ? k + 1 : 0);
      } else {
        continue;
      }
    } else {
      continue;
    }
    draw();
  }
}
