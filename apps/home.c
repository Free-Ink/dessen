// A home screen showing books from the Library, with Recents as a fallback.
// Tap a book or select it with Prev/Next and Confirm. Back opens Files.
#include "dessen.h"

#define MARGIN 24
#define HEADER 72
#define FOOT_H 64
#define MAX_BOOKS 16

// Row height and text-line heights are resolved from the firmware's fonts at
// startup (main). A text box shorter than the font's line height draws nothing,
// so these must never be hardcoded below the real font size.
static int rowH, titleLh, authorLh;

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

static int slen(const char *s);

// Split a book-list blob (lines of "path\ttitle\tauthor\tpercent") in place.
// Prefer the whole library; fall back to recent books if the index isn't built.
static int parse(int maxBooks) {
  int n = dsn_library(buf, sizeof buf - 1);
  if (n <= 0) n = dsn_recents(buf, sizeof buf - 1);
  if (n < 0 || n >= (int)sizeof buf) return 0;
  buf[n] = 0;
  int count = 0;
  char *p = buf;
  while (*p && count < maxBooks) {
    char *fields[3] = {p, 0, 0};
    int f = 1;
    char *line_end = p;
    while (*line_end && *line_end != '\n') {
      if (*line_end == '\t') {
        *line_end = 0;
        if (f < 3) fields[f++] = line_end + 1;
      }
      line_end++;
    }
    int more = (*line_end == '\n');
    *line_end = 0;
    if (fields[1]) {
      books[count].path = fields[0];
      books[count].title = fields[1];
      books[count].author = fields[2] ? fields[2] : "";
      books[count].percent = dsn_book_progress(fields[0], slen(fields[0]));
      count++;
    }
    p = more ? line_end + 1 : line_end;
  }
  return count;
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
    dsn_text_box(MARGIN, H / 2 - 20, W - 2 * MARGIN, 60, "No books yet. Add books, then open the Library once to index them.",
                 66, DSN_FONT_BODY, DSN_ALIGN_CENTER, 3, 0);
  }
  for (int i = 0; i < nbooks; i++) {
    int y = listTop + i * rowH;
    if (y + rowH > listBottom) break;
    if (i == sel) {  // selection frame
      dsn_fill(MARGIN - 6, y - 4, W - 2 * MARGIN + 12, rowH - 8, DSN_BLACK);
      dsn_fill(MARGIN - 3, y - 1, W - 2 * MARGIN + 6, rowH - 14, DSN_WHITE);
    }
    dsn_text_box(MARGIN, y, W - 2 * MARGIN - 70, titleLh, books[i].title, slen(books[i].title), DSN_FONT_BODY,
                 DSN_ALIGN_LEFT, 1, DSN_TEXT_BOLD);
    dsn_text_box(MARGIN, y + titleLh, W - 2 * MARGIN - 70, authorLh, books[i].author, slen(books[i].author),
                 DSN_FONT_SMALL, DSN_ALIGN_LEFT, 1, 0);
    // progress bar + percent
    int barY = y + titleLh + authorLh + 6, barW = W - 2 * MARGIN - 70;
    dsn_stroke(MARGIN, barY, barW, 8, DSN_BLACK, 1, 0, 0);
    if (books[i].percent > 0) dsn_fill(MARGIN, barY, barW * books[i].percent / 100, 8, DSN_BLACK);
    char pc[8];
    pct_str(pc, books[i].percent);
    dsn_text_box(W - MARGIN - 64, y, 64, titleLh, pc, slen(pc), DSN_FONT_BODY, DSN_ALIGN_RIGHT, 1, 0);
  }

  // Footer: two tappable buttons (touch devices have no Back key, so never rely
  // on it). On button devices Back also opens Files.
  int fy = H - FOOT_H, bh = FOOT_H - 12, half = W / 2;
  dsn_line(0, fy, W, fy, 1, DSN_BLACK);
  dsn_stroke(MARGIN, fy + 6, half - MARGIN - 6, bh, DSN_BLACK, 1, 8, DSN_CORNERS_ALL);
  dsn_text_box(MARGIN, fy + 6, half - MARGIN - 6, bh, "Files", 5, DSN_FONT_BODY, DSN_ALIGN_CENTER, 1, 0);
  dsn_stroke(half + 6, fy + 6, half - MARGIN - 6, bh, DSN_BLACK, 1, 8, DSN_CORNERS_ALL);
  dsn_text_box(half + 6, fy + 6, half - MARGIN - 6, bh, "Settings", 8, DSN_FONT_BODY, DSN_ALIGN_CENTER, 1, 0);
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
  titleLh = dsn_line_height(DSN_FONT_BODY);
  authorLh = dsn_line_height(DSN_FONT_SMALL);
  rowH = titleLh + authorLh + 8 + 14;  // title + author + progress bar + padding
  listTop = HEADER;
  listBottom = H - FOOT_H - 8;
  const int visibleRows = listBottom > listTop ? (listBottom - listTop) / rowH : 0;
  nbooks = parse(visibleRows < MAX_BOOKS ? visibleRows : MAX_BOOKS);
  draw();

  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) {
      dsn_open_browser();  // ends the app (button devices)
      return 0;
    }
    if (type == DSN_EV_TAP) {
      int y = DSN_EV_Y(e);
      if (y >= H - FOOT_H) {  // footer: Files (left) / Settings (right)
        if (DSN_EV_X(e) < W / 2) dsn_open_browser();
        else dsn_open_settings();
        return 0;
      }
      if (nbooks && y >= listTop && y < listBottom) {
        int i = (y - listTop) / rowH;
        if (i < nbooks) {
          dsn_open_book(books[i].path, slen(books[i].path));
          return 0;
        }
      }
      continue;
    }
    if (nbooks == 0) continue;
    if (type == DSN_EV_PREV) {
      sel = (sel + nbooks - 1) % nbooks;
    } else if (type == DSN_EV_NEXT) {
      sel = (sel + 1) % nbooks;
    } else if (type == DSN_EV_CONFIRM) {
      dsn_open_book(books[sel].path, slen(books[sel].path));
      return 0;
    } else {
      continue;
    }
    draw();
  }
}
