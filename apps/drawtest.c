// Exercises every drawing call once and checks the values that come back.
// Exits 0 on pass, or the number of the first failed check. Used by the tests.
#include "dessen.h"

static const unsigned char smiley[8] = {0x3c, 0x42, 0xa5, 0x81, 0xa5, 0x99, 0x42, 0x3c};

#define CHECK(n, cond) \
  if (!(cond)) return n

int main(void) {
  int w = dsn_screen_width(), h = dsn_screen_height();
  CHECK(1, w > 0 && h > 0);
  dsn_device_info d;
  CHECK(10, dsn_device(&d, sizeof d) == sizeof d && d.width == w && d.height == h);
  CHECK(11, d.panel_width == w + d.inset_left + d.inset_right && d.panel_height == h + d.inset_top + d.inset_bottom);
  dsn_clear();
  dsn_fill_round(10, 10, 50, 30, DSN_BLACK, 6, DSN_CORNERS_ALL);
  dsn_stroke(-20, -20, 60, 60, DSN_BLACK, 3, 0, DSN_CORNERS_ALL); // partly off screen: clipped
  dsn_triangle(100, 10, 140, 60, 60, 60, DSN_DARK_GRAY);
  dsn_line(-5, -5, w + 100, h + 100, 2, DSN_BLACK); // pinned to the screen

  int lh = dsn_line_height(DSN_FONT_BODY);
  unsigned m = dsn_measure_text("abcd", 4, DSN_FONT_BODY, 0);
  CHECK(2, lh > 0 && (int)(m & 0xffff) == lh && (m >> 16) > 0);
  CHECK(3, dsn_text_box(0, 100, w, 3 * lh, "wrapped and centred", 19, DSN_FONT_BODY, DSN_ALIGN_CENTER, 3, DSN_TEXT_BOLD) == 0);
  CHECK(4, dsn_text_box(0, h + 50, 0, 0, "off screen", 10, 0, 0, 1, 0) == -1);
  CHECK(5, dsn_text_box(0, 0, 0, 0, (const char *)0x7fffffff, 4, 0, 0, 1, 0) == -1); // bad pointer

  CHECK(6, dsn_bitmap(200, 10, 8, 8, smiley, sizeof smiley, 8, 8, DSN_BITMAP_MASK, DSN_BITMAP_CENTER, DSN_BLACK, 0) == 0);
  CHECK(7, dsn_bitmap(200, 10, 8, 8, smiley, 4, 8, 8, DSN_BITMAP_MASK, 0, DSN_BLACK, 0) == -1);      // buffer too short
  CHECK(8, dsn_bitmap(w - 4, 10, 8, 8, smiley, 8, 8, 8, DSN_BITMAP_MASK, 0, DSN_BLACK, 0) == -1);    // leaves the screen
  CHECK(9, dsn_bitmap(200, 10, 8, 8, smiley, 8, 8, 8, 9, 0, DSN_BLACK, 0) == -1);                    // unknown format

  // a native menu row: selected (gray fill) with a trailing chevron (two lines)
  CHECK(12, dsn_list_row(8, 300, w - 16, 52, "Menu", 4, DSN_ROW_SELECTED | DSN_ROW_CHEVRON) == 0);
  CHECK(13, dsn_list_row(0, h + 10, w, 52, "off screen", 10, 0) == -1); // fully clipped away

  // layout engine: a 2x2 grid cell, and a fixed+flex row split.
  dsn_rect_t cell;
  CHECK(14, dsn_grid_cell(0, 0, 100, 100, 2, 2, 0, 1, 1, 1, 1, &cell, sizeof cell) == 0);
  CHECK(15, cell.x == 50 && cell.y == 50 && cell.w == 50 && cell.h == 50);
  CHECK(16, dsn_grid_cell(0, 0, 100, 100, 2, 2, 0, 2, 0, 1, 1, &cell, sizeof cell) == -1); // column out of range
  unsigned char lens[8] = {DSN_LEN_FIXED, 30, 0, 0, DSN_LEN_FLEX, 0, 0, 1}; // 30px, then the rest
  dsn_rect_t slots[2];
  CHECK(17, dsn_split(0, 0, 100, 20, DSN_AXIS_ROW, 0, lens, sizeof lens, (unsigned char *)slots, sizeof slots) == 2);
  CHECK(18, slots[0].w == 30 && slots[1].x == 30 && slots[1].w == 70);
  dsn_refresh();
  return 0;
}
