// Polyhedral dice roller for tabletop games. Tap a die to roll it, tap the
// result to roll the same die again. With buttons: Prev/Next pick a die,
// Confirm rolls, Back quits.
#include "dessen.h"

#define NDICE 7
#define MARGIN 16
#define GRID_Y 56
#define LABEL_H 28
#define HISTORY 5

static const unsigned short sides[NDICE] = {4, 6, 8, 10, 12, 20, 100};
static const char *const names[NDICE] = {"d4", "d6", "d8", "d10", "d12", "d20", "d100"};

// Outlines on a 16x16 grid: point count, then x,y pairs (closed polygon).
static const unsigned char outline[NDICE][17] = {
    {3, 8, 1, 15, 14, 1, 14},                               // d4: triangle
    {4, 2, 2, 14, 2, 14, 14, 2, 14},                        // d6: square
    {4, 8, 1, 15, 8, 8, 15, 1, 8},                          // d8: diamond
    {4, 8, 1, 15, 6, 8, 15, 1, 6},                          // d10: kite
    {5, 8, 1, 15, 6, 12, 15, 4, 15, 1, 6},                  // d12: pentagon
    {6, 8, 1, 14, 4, 14, 12, 8, 15, 2, 12, 2, 4},           // d20: hexagon
    {8, 5, 1, 11, 1, 15, 5, 15, 11, 11, 15, 5, 15, 1, 11, 1, 5}, // d100: octagon
};
// Inner facet lines: die, x0, y0, x1, y1.
static const unsigned char facets[][5] = {
    {2, 1, 8, 15, 8},  {3, 1, 6, 8, 10},  {3, 8, 10, 15, 6}, {3, 8, 10, 8, 15},
    {5, 8, 4, 13, 12}, {5, 13, 12, 3, 12}, {5, 3, 12, 8, 4},
};

static int w, h, cols, cell, result_y;
static int sel = 5, rolled; // rolled = 0 until the first roll
static unsigned char hist_die[HISTORY];
static unsigned short hist_val[HISTORY];
static int hist_n;

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

static void draw_die(int i, int x, int y, int s, int selected) {
  const unsigned char *o = outline[i];
  int n = o[0], width = selected ? 5 : 2;
  for (int k = 0; k < n; k++) {
    int a = 1 + 2 * k, b = 1 + 2 * ((k + 1) % n);
    dsn_line(x + o[a] * s / 16, y + o[a + 1] * s / 16, x + o[b] * s / 16, y + o[b + 1] * s / 16, width, DSN_BLACK);
  }
  for (unsigned k = 0; k < sizeof facets / sizeof facets[0]; k++) {
    const unsigned char *f = facets[k];
    if (f[0] == i) dsn_line(x + f[1] * s / 16, y + f[2] * s / 16, x + f[3] * s / 16, y + f[4] * s / 16, 1, DSN_BLACK);
  }
}

// Seven-segment digit, dw wide and 2*dw tall, so the result is large on any
// board without needing a big font.
static void draw_digit(int x, int y, int dw, int d) {
  static const unsigned char seg[10] = {0x3f, 0x06, 0x5b, 0x4f, 0x66, 0x6d, 0x7d, 0x07, 0x7f, 0x6f};
  int t = dw / 5 < 3 ? 3 : dw / 5, half = dw, m = seg[d];
  if (m & 0x01) dsn_fill(x, y, dw, t, DSN_BLACK);                     // top
  if (m & 0x02) dsn_fill(x + dw - t, y, t, half, DSN_BLACK);          // top right
  if (m & 0x04) dsn_fill(x + dw - t, y + half, t, half, DSN_BLACK);   // bottom right
  if (m & 0x08) dsn_fill(x, y + 2 * half - t, dw, t, DSN_BLACK);      // bottom
  if (m & 0x10) dsn_fill(x, y + half, t, half, DSN_BLACK);            // bottom left
  if (m & 0x20) dsn_fill(x, y, t, half, DSN_BLACK);                   // top left
  if (m & 0x40) dsn_fill(x, y + half - t / 2, dw, t, DSN_BLACK);      // middle
}

static void draw(void) {
  char buf[96], *p;
  dsn_clear();
  dsn_text(MARGIN, 16, "Dice: tap a die to roll");
  for (int i = 0; i < NDICE; i++) {
    int x = MARGIN + (i % cols) * cell, y = GRID_Y + (i / cols) * (cell + LABEL_H);
    draw_die(i, x + 8, y, cell - 16, i == sel);
    dsn_text(x + 8, y + cell - 12, names[i]);
  }

  if (!rolled) {
    dsn_text(MARGIN, result_y, "No rolls yet.");
  } else {
    p = put_str(buf, "Rolled ");
    p = put_str(p, names[sel]);
    if (sides[sel] == 20 && rolled == 20) p = put_str(p, ": critical hit!");
    if (sides[sel] == 20 && rolled == 1) p = put_str(p, ": critical fail.");
    *p = 0;
    dsn_text(MARGIN, result_y, buf);

    int digits = rolled >= 100 ? 3 : rolled >= 10 ? 2 : 1;
    int room = (h - result_y - 130) / 2, dw = w / 6 < room ? w / 6 : room;
    if (dw < 15) dw = 15;
    int gap = dw / 4, x = (w - digits * dw - (digits - 1) * gap) / 2, y = result_y + 44;
    for (int k = digits - 1, v = rolled; k >= 0; k--, v /= 10) draw_digit(x + k * (dw + gap), y, dw, v % 10);

    p = put_str(buf, "Last:");
    for (int k = 0; k < hist_n; k++) {
      p = put_str(p, "  ");
      p = put_str(p, names[hist_die[k]]);
      *p++ = '=';
      p = put_num(p, hist_val[k]);
    }
    *p = 0;
    dsn_text(MARGIN, y + 2 * dw + 20, buf);
  }
  dsn_refresh();
}

static void roll(void) {
  // Scale a 32-bit random number into 0..sides-1 without modulo bias.
  rolled = (int)(((unsigned long long)dsn_random() * sides[sel]) >> 32) + 1;
  for (int k = HISTORY - 1; k > 0; k--) {
    hist_die[k] = hist_die[k - 1];
    hist_val[k] = hist_val[k - 1];
  }
  hist_die[0] = (unsigned char)sel;
  hist_val[0] = (unsigned short)rolled;
  if (hist_n < HISTORY) hist_n++;
}

int main(void) {
  w = dsn_screen_width();
  h = dsn_screen_height();
  cols = w >= 700 ? NDICE : 4;
  cell = (w - 2 * MARGIN) / cols;
  int rows = (NDICE + cols - 1) / cols;
  result_y = GRID_Y + rows * (cell + LABEL_H) + 12;

  draw();
  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) return 0;
    if (type == DSN_EV_PREV) {
      sel = (sel + NDICE - 1) % NDICE;
      rolled = 0;
    } else if (type == DSN_EV_NEXT) {
      sel = (sel + 1) % NDICE;
      rolled = 0;
    } else if (type == DSN_EV_CONFIRM) {
      roll();
    } else if (type == DSN_EV_TAP) {
      int x = DSN_EV_X(e) - MARGIN, y = DSN_EV_Y(e);
      if (y >= result_y) {
        roll(); // tap the result: same die again
      } else if (y >= GRID_Y && x >= 0 && x < cols * cell) {
        int i = ((y - GRID_Y) / (cell + LABEL_H)) * cols + x / cell;
        if (i >= NDICE) continue;
        sel = i;
        roll();
      } else {
        continue;
      }
    } else {
      continue;
    }
    draw();
  }
}
