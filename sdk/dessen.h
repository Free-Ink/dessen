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
