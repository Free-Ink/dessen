// dessen app SDK. Freestanding: no libc. The dsn_* calls come from
// dessen_calls.h, generated from abi/freeink.api; this file only adds a few
// conveniences on top.
#pragma once
#include "dessen_abi.h"
#include "dessen_calls.h"

static inline int dsn_screen_width(void) { return (int)(dsn_screen() >> 16); }
static inline int dsn_screen_height(void) { return (int)(dsn_screen() & 0xffff); }

// One line of text in the small font, from (x, y) to the right screen edge.
static inline void dsn_text(int x, int y, const char *s) {
  int len = 0;
  while (s[len]) len++;
  dsn_text_box(x, y, 0, 0, s, len, DSN_FONT_SMALL, DSN_ALIGN_LEFT, 1, 0);
}

// Load a custom font from the app's folder; returns DSN_FONT_CUSTOM on success,
// or a DSN_ERR_*. Pass the result as the font argument to dsn_text_box.
static inline int dsn_font(const char *path, int size) {
  int n = 0;
  while (path[n]) n++;
  return dsn_font_load(path, n, size);
}
