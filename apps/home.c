// A stock home-screen app: a list of recent books with reading-progress bars.
// Tap a book to open it in the reader; the footer buttons open the file browser
// and settings. Set this as the device's home app and it replaces the firmware
// home screen. Buttons: Prev/Next move the selection, Confirm opens it, Back =
// browser. It re-reads recents every launch, so progress is always current.
#include "dessen.h"

#define MARGIN 24
#define HEADER 72
#define ROW_H 76
#define FOOT_H 64
#define MAX_BOOKS 16

struct Book {
  const char *path;
  const char *title;
  const char *author;
  int percent;
};

static char buf[4096];
static struct Book books[MAX_BOOKS];
static int nbooks, sel;
static int W, H, listTop, listBottom;

static int atoi_(const char *s);
static int slen(const char *s);

// Split the recents blob (lines of "path\ttitle\tauthor\tpercent") in place.
static int parse(void) {
  int n = dsn_recents(buf, sizeof buf - 1);
  if (n < 0) return 0;
  buf[n] = 0;
  int count = 0;
  char *p = buf;
  while (*p && count < MAX_BOOKS) {
    char *fields[4] = {p, 0, 0, 0};
    int f = 1;
    char *line_end = p;
    while (*line_end && *line_end != '\n') {
      if (*line_end == '\t' && f < 4) {
        *line_end = 0;
        fields[f++] = line_end + 1;
      }
      line_end++;
    }
    int more = (*line_end == '\n');
    *line_end = 0;
    if (fields[1]) {
      books[count].path = fields[0];
      books[count].title = fields[1];
      books[count].author = fields[2] ? fields[2] : "";
      books[count].percent = fields[3] ? atoi_(fields[3]) : -1;
      count++;
    }
    p = more ? line_end + 1 : line_end;
  }
  return count;
}

static int atoi_(const char *s) {
  int v = 0;
  while (*s >= '0' && *s <= '9') v = v * 10 + (*s++ - '0');
  return v;
}

static void pct_str(char *out, int pct) {
  if (pct < 0) { out[0] = 0; return; }
  char *p = out;
  if (pct >= 100) *p++ = '1', *p++ = '0', *p++ = '0';
  else {
    if (pct >= 10) *p++ = (char)('0' + pct / 10);
    *p++ = (char)('0' + pct % 10);
  }
  *p++ = '%';
  *p = 0;
}

static void draw(void) {
  dsn_clear();
  dsn_text_box(MARGIN, 20, W - 2 * MARGIN, 40, "Library", 7, DSN_FONT_TITLE, DSN_ALIGN_LEFT, 1, 0);

  if (nbooks == 0) {
    dsn_text_box(MARGIN, H / 2 - 20, W - 2 * MARGIN, 40, "No recent books. Open one from the file browser.", 48,
                 DSN_FONT_BODY, DSN_ALIGN_CENTER, 2, 0);
  }
  for (int i = 0; i < nbooks; i++) {
    int y = listTop + i * ROW_H;
    if (y + ROW_H > listBottom) break;
    if (i == sel) {  // selection frame
      dsn_fill(MARGIN - 6, y - 4, W - 2 * MARGIN + 12, ROW_H - 8, DSN_BLACK);
      dsn_fill(MARGIN - 3, y - 1, W - 2 * MARGIN + 6, ROW_H - 14, DSN_WHITE);
    }
    dsn_text_box(MARGIN, y, W - 2 * MARGIN - 70, 28, books[i].title, slen(books[i].title), DSN_FONT_BODY,
                 DSN_ALIGN_LEFT, 1, DSN_TEXT_BOLD);
    dsn_text_box(MARGIN, y + 26, W - 2 * MARGIN - 70, 22, books[i].author, slen(books[i].author), DSN_FONT_SMALL,
                 DSN_ALIGN_LEFT, 1, 0);
    // progress bar + percent
    int barY = y + 50, barW = W - 2 * MARGIN - 70;
    dsn_stroke(MARGIN, barY, barW, 8, DSN_BLACK, 1, 0, 0);
    if (books[i].percent > 0) dsn_fill(MARGIN, barY, barW * books[i].percent / 100, 8, DSN_BLACK);
    char pc[8];
    pct_str(pc, books[i].percent);
    dsn_text_box(W - MARGIN - 64, y + 20, 64, 28, pc, slen(pc), DSN_FONT_BODY, DSN_ALIGN_RIGHT, 1, 0);
  }

  // footer hints
  int fy = H - FOOT_H + 20;
  dsn_line(0, H - FOOT_H, W, H - FOOT_H, 1, DSN_BLACK);
  dsn_text_box(MARGIN, fy, (W - 2 * MARGIN) / 2, 28, "Back: Files", 11, DSN_FONT_SMALL, DSN_ALIGN_LEFT, 1, 0);
  dsn_text_box(W / 2, fy, (W - 2 * MARGIN) / 2, 28, "Hold Back: Settings", 19, DSN_FONT_SMALL, DSN_ALIGN_RIGHT, 1, 0);
  dsn_refresh();
}

static int slen(const char *s) {
  int n = 0;
  while (s[n]) n++;
  return n;
}

int main(void) {
  W = dsn_screen_width();
  H = dsn_screen_height();
  listTop = HEADER;
  listBottom = H - FOOT_H - 8;
  nbooks = parse();
  draw();

  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) {
      dsn_open_browser();  // ends the app
      return 0;
    }
    if (nbooks == 0) continue;
    if (type == DSN_EV_PREV) {
      sel = (sel + nbooks - 1) % nbooks;
    } else if (type == DSN_EV_NEXT) {
      sel = (sel + 1) % nbooks;
    } else if (type == DSN_EV_CONFIRM) {
      dsn_open_book(books[sel].path, slen(books[sel].path));
      return 0;
    } else if (type == DSN_EV_TAP) {
      int y = DSN_EV_Y(e);
      if (y >= listTop && y < listBottom) {
        int i = (y - listTop) / ROW_H;
        if (i < nbooks) {
          dsn_open_book(books[i].path, slen(books[i].path));
          return 0;
        }
      }
      continue;
    } else {
      continue;
    }
    draw();
  }
}
