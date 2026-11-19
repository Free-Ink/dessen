// A minimal home theme: a centered column of underlined text. "Continue
// Reading" opens your most recent book; the items below are the usual home
// destinations. Prev/Next move the selection, Confirm (or a tap) opens it.
// Mark the manifest "home": true to pick it as the Home Launcher in Settings.
#include "dessen.h"

#define ITEM_H 56
#define MAXCOL 8

struct Item {
  const char *label;
  int isBook;  // 1 = open the current book; else open_home(dest)
  int dest;
};

static char buf[1024];
static const char *bookPath;
static struct Item items[MAXCOL];
static int nitems, sel, W, H, top;

static int slen(const char *s) {
  int n = 0;
  while (s[n]) n++;
  return n;
}

// Pull just the first (most recent) book's path out of the recents blob.
static void loadCurrentBook(void) {
  int n = dsn_recents(buf, sizeof buf - 1);
  if (n <= 0) return;
  buf[n] = 0;
  for (int i = 0; i < n; i++)
    if (buf[i] == '\t' || buf[i] == '\n') { buf[i] = 0; break; }
  if (buf[0] == '/') bookPath = buf;
}

static void build(void) {
  nitems = 0;
  if (bookPath) items[nitems++] = (struct Item){"Continue Reading", 1, 0};
  items[nitems++] = (struct Item){"Library", 0, DSN_HOME_LIBRARY};
  items[nitems++] = (struct Item){"Files", 0, DSN_HOME_FILES};
  items[nitems++] = (struct Item){"Plugins", 0, DSN_HOME_PLUGINS};
  items[nitems++] = (struct Item){"OPDS", 0, DSN_HOME_OPDS};
  items[nitems++] = (struct Item){"File Transfer", 0, DSN_HOME_TRANSFER};
  items[nitems++] = (struct Item){"Settings", 0, DSN_HOME_SETTINGS};
  top = (H - nitems * ITEM_H) / 2;
}

static void draw(void) {
  dsn_clear();
  int lh = dsn_line_height(DSN_FONT_TITLE);
  for (int i = 0; i < nitems; i++) {
    int y = top + i * ITEM_H;
    int on = (i == sel);
    int flags = on ? DSN_TEXT_BOLD : 0;
    const char *s = items[i].label;
    int len = slen(s);
    // centered text, with an underline the width of the text
    dsn_text_box(0, y, W, lh, s, len, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, flags);
    unsigned m = dsn_measure_text(s, len, DSN_FONT_TITLE, flags);
    int tw = (int)(m >> 16);
    int ux = (W - tw) / 2, uy = y + lh + 3;
    dsn_line(ux, uy, ux + tw, uy, on ? 3 : 1, DSN_BLACK);
    if (on) {  // selection carets flanking the item
      dsn_text_box(ux - 40, y, 28, lh, "\xE2\x80\xA2", 3, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, 0);
    }
  }
  dsn_refresh();
}

static void activate(void) {
  if (items[sel].isBook) dsn_open_book(bookPath, slen(bookPath));
  else dsn_open_home(items[sel].dest);
}

int main(void) {
  W = dsn_screen_width();
  H = dsn_screen_height();
  loadCurrentBook();
  build();
  draw();

  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_PREV) {
      sel = (sel + nitems - 1) % nitems;
    } else if (type == DSN_EV_NEXT) {
      sel = (sel + 1) % nitems;
    } else if (type == DSN_EV_CONFIRM) {
      activate();
      return 0;
    } else if (type == DSN_EV_TAP) {
      int ty = DSN_EV_Y(e);
      int i = (ty - top) / ITEM_H;
      if (i >= 0 && i < nitems && ty >= top) {
        sel = i;
        activate();
        return 0;
      }
      continue;
    } else {
      continue;  // Back is ignored: this is the home root
    }
    draw();
  }
}
