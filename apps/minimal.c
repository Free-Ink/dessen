// A minimal, declarative home launcher. Each row in MENU below is one entry:
// a label, an optional icon, and what it opens — a BUILTIN firmware screen or
// an installed dessen APP (by its app.bin path). Edit MENU to build your own
// home. Mark the plugin manifest "home": true to pick it under
// Settings -> Display -> Home Launcher. Prev/Next move the selection,
// Confirm (or a tap) opens the entry.
#include "dessen.h"

#define ITEM_H 56
#define ICON_PX 32

// What a launcher entry opens. The macros fill the {kind, dest, app} fields.
enum { OPEN_BUILTIN, OPEN_APP, OPEN_BOOK };
#define BUILTIN(screen) OPEN_BUILTIN, (screen), 0  // a firmware screen (DSN_HOME_*)
#define APP(path)       OPEN_APP, 0, (path)        // an installed dessen app's app.bin
#define BOOK            OPEN_BOOK, 0, 0            // the most recently read book

typedef struct {
  const char *label;
  const unsigned char *icon;  // optional ICON_PX-square 1-bit mask bitmap; 0 = text only
  int kind;                   // OPEN_BUILTIN / OPEN_APP / OPEN_BOOK (set by the macros)
  int dest;                   // DSN_HOME_* for BUILTIN
  const char *app;            // ".../app.bin" for APP
} Item;

// ---------------------------------------------------------------------------
// Your home. Reorder, add, or remove rows. BUILTIN(screen) opens a firmware
// screen; APP("/plugins/<name>/app.bin") opens one of your installed dessen
// apps. Set the 2nd field to a bitmap for an icon, or 0 for text only.
// ---------------------------------------------------------------------------
static const Item MENU[] = {
    {"Library",  0, BUILTIN(DSN_HOME_LIBRARY)},
    {"Files",    0, BUILTIN(DSN_HOME_FILES)},
    {"Dice",     0, APP("/plugins/dice/app.bin")},
    {"Settings", 0, BUILTIN(DSN_HOME_SETTINGS)},
};
#define MENU_N ((int)(sizeof MENU / sizeof MENU[0]))

static char buf[1024];
static const char *bookPath;
static Item items[MENU_N + 1];  // MENU, optionally preceded by a Continue Reading row
static int nitems, sel, W, H, top;

static int slen(const char *s) {
  int n = 0;
  while (s[n]) n++;
  return n;
}

// No libc: tiny unsigned-to-decimal, returns the number of chars written.
static int putn(char *b, int v) {
  char tmp[8];
  int i = 0;
  if (v == 0) { b[0] = '0'; return 1; }
  while (v > 0 && i < 8) { tmp[i++] = '0' + v % 10; v /= 10; }
  for (int j = 0; j < i; j++) b[j] = tmp[i - 1 - j];
  return i;
}

// A home theme draws its own chrome: a centered "H:MM AM  87%" line up top,
// from the clock and battery capabilities.
static char status[32];
static void drawStatus(void) {
  int n = 0;
  dsn_clock_t c;
  if (dsn_clock(&c, sizeof c) == 0) {
    int h = c.hour % 12;
    if (h == 0) h = 12;
    n += putn(status + n, h);
    status[n++] = ':';
    status[n++] = '0' + c.minute / 10;
    status[n++] = '0' + c.minute % 10;
    status[n++] = ' ';
    status[n++] = c.hour < 12 ? 'A' : 'P';
    status[n++] = 'M';
  }
  int b = dsn_battery();
  if (b >= 0) {
    if (n) { status[n++] = ' '; status[n++] = ' '; }
    n += putn(status + n, b & 0xff);
    status[n++] = '%';
  }
  if (n) {
    int lh = dsn_line_height(DSN_FONT_SMALL);
    dsn_text_box(0, 24, W, lh, status, n, DSN_FONT_SMALL, DSN_ALIGN_CENTER, 1, 0);
  }
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

// Assemble the visible list: the static MENU, preceded by Continue Reading when
// there is a recent book to resume.
static void build(void) {
  nitems = 0;
  if (bookPath) items[nitems++] = (Item){"Continue Reading", 0, BOOK};
  for (int i = 0; i < MENU_N; i++) items[nitems++] = MENU[i];
  top = (H - nitems * ITEM_H) / 2;
}

static void draw(void) {
  dsn_clear();
  drawStatus();
  int lh = dsn_line_height(DSN_FONT_TITLE);
  for (int i = 0; i < nitems; i++) {
    int y = top + i * ITEM_H;
    int on = (i == sel);
    int flags = on ? DSN_TEXT_BOLD : 0;
    const char *s = items[i].label;
    int len = slen(s);
    // centered label, with an underline the width of the text
    dsn_text_box(0, y, W, lh, s, len, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, flags);
    unsigned m = dsn_measure_text(s, len, DSN_FONT_TITLE, flags);
    int tw = (int)(m >> 16);
    int ux = (W - tw) / 2, uy = y + lh + 3;
    dsn_line(ux, uy, ux + tw, uy, on ? 3 : 1, DSN_BLACK);
    if (items[i].icon)  // optional icon, just left of the label
      dsn_bitmap(ux - ICON_PX - 10, y, ICON_PX, ICON_PX, items[i].icon, (ICON_PX / 8) * ICON_PX, ICON_PX, ICON_PX,
                 DSN_BITMAP_MASK, DSN_BITMAP_CENTER, DSN_BLACK, 0);
    if (on)  // selection caret flanking the item
      dsn_text_box(ux - 40, y, 28, lh, "\xE2\x80\xA2", 3, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, 0);
  }
  dsn_refresh();
}

// Open whatever the selected row points at. open_* end this app; the firmware
// returns here (re-running the launcher) when that screen or app exits.
static void activate(void) {
  const Item *it = &items[sel];
  if (it->kind == OPEN_APP) dsn_open_app(it->app, slen(it->app));
  else if (it->kind == OPEN_BOOK) { if (bookPath) dsn_open_book(bookPath, slen(bookPath)); }
  else dsn_open_home(it->dest);
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
