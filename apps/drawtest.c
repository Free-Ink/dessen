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
  CHECK(8, dsn_bitmap(w - 4, 10, 8, 8, smiley, 8, 8, 8, DSN_BITMAP_MASK, 0, DSN_BLACK, 0) == 0);     // partly off screen: clipped
  CHECK(15, dsn_bitmap(w + 10, 10, 8, 8, smiley, 8, 8, 8, DSN_BITMAP_MASK, 0, DSN_BLACK, 0) == -1);  // fully off screen: refused
  CHECK(9, dsn_bitmap(200, 10, 8, 8, smiley, 8, 8, 8, 9, 0, DSN_BLACK, 0) == -1);                    // unknown format

  static unsigned char large[32768];
  CHECK(12, dsn_bitmap(200, 10, 8, 8, large, sizeof large, 32768, 1, DSN_BITMAP_BW, DSN_BITMAP_CENTER, DSN_BLACK, 0) == -1);
  CHECK(13, dsn_bitmap(200, 10, 8, 8, large, sizeof large, 1, 32768, DSN_BITMAP_BW, DSN_BITMAP_CENTER, DSN_BLACK, 0) == -1);
  CHECK(14, dsn_bitmap(200, 10, 0, 0, smiley, sizeof smiley, 8, 8, DSN_BITMAP_BW, DSN_BITMAP_CENTER, DSN_BLACK, 0) == -1);

  // Fixed font IDs use the same selection for measurement and drawing.
  for (int font = DSN_FONT_NOTOSANS_12; font <= DSN_FONT_NOTOSANS_18; ++font) {
    int height = dsn_line_height(font);
    unsigned size = dsn_measure_text("fixed", 5, font, 0);
    CHECK(26, height > 0 && (int)(size & 0xffff) == height);
    CHECK(27, dsn_text_box(0, 200, w, height, "fixed", 5, font, 0, 1, 0) == 0);
  }
  CHECK(28, dsn_line_height(255) == dsn_line_height(DSN_FONT_SMALL));
  CHECK(29, dsn_text_box(0, 200, w, lh, "fallback", 8, 255, 0, 1, 0) == 0);

  // Button hints: the host's band fits the drawable area, and labels it
  // cannot draw are refused (a fifth line, a control byte, an overlong one).
  CHECK(21, d.hint_height <= h);
  if (d.hint_height > 0) dsn_fill(0, h - d.hint_height, 7, d.hint_height, DSN_BLACK); // marks the band for the test
  CHECK(22, dsn_button_hints((const uint8_t*)"Back\nSelect\n\nNext", 17) == 0);
  CHECK(23, dsn_button_hints((const uint8_t*)"a\nb\nc\nd\ne", 9) == -1);
  CHECK(24, dsn_button_hints((const uint8_t*)"a\tb", 3) == -1);
  CHECK(25, dsn_button_hints((const uint8_t*)"0123456789012345678901234567890123", 34) == -1);

  // Clipping: limited to a rectangle, lifted by a negative width, and a clip
  // entirely off screen lets nothing through rather than everything.
  CHECK(16, dsn_set_clip(20, 20, 100, 50) == 0);
  dsn_fill(0, 0, w, h, DSN_SCRIM);
  CHECK(17, dsn_set_clip(0, 0, -1, -1) == 0);
  CHECK(18, dsn_set_clip(w + 10, 0, 10, 10) == 0);
  CHECK(19, dsn_set_clip(0, 0, -1, 0) == 0);
  CHECK(20, dsn_set_clip(10, 10, 20, 20) == 0); // left set: the host restores its own clip at exit

  dsn_refresh();
  return 0;
}
