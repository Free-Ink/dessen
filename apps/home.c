// A home screen showing books from the Library, with Recents as a fallback.
// Tap a book or select it with Prev/Next and Confirm. Back opens Files.
#include "dessen.h"

#define MARGIN 24
#define HEADER 72
#define FOOT_H 92
#define MAX_BOOKS 16

// Bottom nav: reach every home destination. Each cell draws its own little glyph
// (from the drawing primitives — an app can equally bundle a bitmap and
// dsn_bitmap it) above a label.
#define NAV_N 5
static const int nav_dest[NAV_N] = {DSN_HOME_FILES, DSN_HOME_OPDS, DSN_HOME_PLUGINS, DSN_HOME_TRANSFER,
                                    DSN_HOME_SETTINGS};
static const char *const nav_label[NAV_N] = {"Files", "OPDS", "Plugins", "Transfer", "Settings"};

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

// Covers: each packed cover is {u16 w, u16 h} + MSB-first 1-bit rows. Loaded one
// per redraw (dsn_book_cover decodes+caches lazily, so a whole shelf at once
// would block the firmware). cover_n[i]: -1 untried, 0 none, >4 = packed bytes.
#define COVER_MAX 1024
#define COVER_H 92   // cover thumbnail height (the row is built around it)
#define GUTTER 28    // side margin for the list
static unsigned char covers[MAX_BOOKS][COVER_MAX];
static int cover_n[MAX_BOOKS], coverH;

static int slen(const char *s);

// Split a book-list blob (lines of "path\ttitle\tauthor\tpercent") in place.
// Prefer the whole library; fall back to recent books if the index isn't built.
static int parse(int maxBooks) {
  int n = dsn_library(buf, sizeof buf - 1);
  const int useRecents = n <= 0;
  if (useRecents) n = dsn_recents(buf, sizeof buf - 1);
  if (n < 0 || n >= (int)sizeof buf) return 0;
  buf[n] = 0;
  int count = 0;
  char *p = buf;
  while (*p && count < maxBooks) {
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
      int percent = -1;
      if (!useRecents) {
        percent = dsn_book_progress(fields[0], slen(fields[0]));
      } else if (fields[3] && *fields[3] >= '0' && *fields[3] <= '9') {
        percent = 0;
        for (const char *digit = fields[3]; *digit >= '0' && *digit <= '9' && percent <= 100; digit++)
          percent = percent * 10 + *digit - '0';
      }
      books[count].percent = percent <= 100 ? percent : -1;
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

// A little nav glyph for item `idx`, centered on (cx, cy). Drawn from the
// primitives — a worked example of making icons without bundling a bitmap.
static void draw_glyph(int idx, int cx, int cy) {
  if (idx == 0) {  // Files: a folder
    dsn_fill_round(cx - 13, cy - 6, 26, 16, DSN_BLACK, 3, DSN_CORNERS_ALL);
    dsn_fill_round(cx - 13, cy - 10, 12, 8, DSN_BLACK, 3, DSN_CORNERS_ALL);
  } else if (idx == 1) {  // OPDS: a globe
    dsn_stroke(cx - 12, cy - 12, 24, 24, DSN_BLACK, 2, 12, DSN_CORNERS_ALL);
    dsn_stroke(cx - 6, cy - 12, 12, 24, DSN_BLACK, 1, 6, DSN_CORNERS_ALL);
    dsn_line(cx - 12, cy, cx + 12, cy, 1, DSN_BLACK);
  } else if (idx == 2) {  // Plugins: a 2x2 grid
    for (int r = 0; r < 2; r++)
      for (int c = 0; c < 2; c++) dsn_fill_round(cx - 11 + c * 13, cy - 11 + r * 13, 9, 9, DSN_BLACK, 2, DSN_CORNERS_ALL);
  } else if (idx == 3) {  // Transfer: a down and an up arrow
    dsn_line(cx - 6, cy - 10, cx - 6, cy + 4, 2, DSN_BLACK);
    dsn_triangle(cx - 11, cy + 2, cx - 1, cy + 2, cx - 6, cy + 11, DSN_BLACK);
    dsn_line(cx + 6, cy + 10, cx + 6, cy - 4, 2, DSN_BLACK);
    dsn_triangle(cx + 1, cy - 2, cx + 11, cy - 2, cx + 6, cy - 11, DSN_BLACK);
  } else {  // Settings: a gear
    dsn_fill_round(cx - 11, cy - 11, 22, 22, DSN_BLACK, 11, DSN_CORNERS_ALL);
    dsn_fill_round(cx - 4, cy - 4, 8, 8, DSN_WHITE, 4, DSN_CORNERS_ALL);
    dsn_fill(cx - 2, cy - 14, 4, 5, DSN_BLACK);
    dsn_fill(cx - 2, cy + 9, 4, 5, DSN_BLACK);
    dsn_fill(cx - 14, cy - 2, 5, 4, DSN_BLACK);
    dsn_fill(cx + 9, cy - 2, 5, 4, DSN_BLACK);
  }
}

static void draw(void) {
  dsn_clear();
  // Header with a hairline rule under it.
  dsn_text_box(GUTTER, 24, W - 2 * GUTTER, titleLh + 6, "Library", 7, DSN_FONT_TITLE, DSN_ALIGN_LEFT, 1, 0);
  dsn_line(GUTTER, HEADER - 14, W - GUTTER, HEADER - 14, 1, DSN_LIGHT_GRAY);

  if (nbooks == 0) {
    dsn_text_box(GUTTER, H / 2 - 40, W - 2 * GUTTER, 120,
                 "No books yet. Add books, then open the Library once to index them.", 66, DSN_FONT_BODY,
                 DSN_ALIGN_CENTER, 4, 0);
  }
  for (int i = 0; i < nbooks; i++) {
    int y = listTop + i * rowH;
    if (y + rowH > listBottom) break;
    int selected = i == sel;
    if (selected)  // soft rounded highlight behind the whole row
      dsn_fill_round(GUTTER - 12, y + 6, W - 2 * GUTTER + 24, rowH - 12, DSN_LIGHT_GRAY, 18, DSN_CORNERS_ALL);

    int cy = y + (rowH - COVER_H) / 2;  // cover / content top
    int tx = GUTTER;
    if (cover_n[i] > 4) {
      int cw = covers[i][0] | covers[i][1] << 8, ch = covers[i][2] | covers[i][3] << 8;
      dsn_bitmap(GUTTER, cy, cw, ch, covers[i] + 4, cover_n[i] - 4, cw, ch, DSN_BITMAP_BW, DSN_BITMAP_CENTER,
                 DSN_BLACK, 0);
      dsn_stroke(GUTTER, cy, cw, ch, DSN_BLACK, 1, 0, 0);  // crisp cover edge
      tx = GUTTER + cw + 20;
    }
    int tw = W - GUTTER - tx;
    // Vertically center the title/author/progress block against the cover.
    int blockH = titleLh + 6 + authorLh + 18 + 10;
    int ty = cy + (COVER_H - blockH) / 2;
    if (ty < cy) ty = cy;
    dsn_text_box(tx, ty, tw, titleLh, books[i].title, slen(books[i].title), DSN_FONT_BODY, DSN_ALIGN_LEFT, 1,
                 DSN_TEXT_BOLD);
    dsn_text_box(tx, ty + titleLh + 6, tw, authorLh, books[i].author, slen(books[i].author), DSN_FONT_SMALL,
                 DSN_ALIGN_LEFT, 1, 0);
    // Progress: a rounded track with the read portion filled, percent beside it.
    int barY = ty + titleLh + 6 + authorLh + 18, barW = tw - 70;
    if (barW > 60) {
      // the track must contrast the row: white on the gray selection, else gray
      dsn_fill_round(tx, barY, barW, 10, selected ? DSN_WHITE : DSN_LIGHT_GRAY, 5, DSN_CORNERS_ALL);
      if (books[i].percent > 0) {
        int fw = barW * books[i].percent / 100;
        dsn_fill_round(tx, barY, fw < 10 ? 10 : fw, 10, DSN_BLACK, 5, DSN_CORNERS_ALL);
      }
      char pc[8];
      pct_str(pc, books[i].percent);
      if (pc[0])
        dsn_text_box(tx + barW + 12, barY + 5 - authorLh / 2, 56, authorLh, pc, slen(pc), DSN_FONT_SMALL,
                     DSN_ALIGN_LEFT, 1, 0);
    }
  }

  // Nav bar: one tappable cell per home destination (glyph over a label). Touch
  // devices have no Back key, so this is the only way to reach the rest.
  int navTop = H - FOOT_H, cellW = W / NAV_N;
  dsn_line(0, navTop, W, navTop, 1, DSN_LIGHT_GRAY);
  for (int i = 0; i < NAV_N; i++) {
    draw_glyph(i, i * cellW + cellW / 2, navTop + 28);
    dsn_text_box(i * cellW, navTop + 48, cellW, authorLh, nav_label[i], slen(nav_label[i]), DSN_FONT_SMALL,
                 DSN_ALIGN_CENTER, 1, 0);
  }
  dsn_refresh();
}

static int slen(const char *s) {
  int n = 0;
  while (s[n]) n++;
  return n;
}

// Fetch one not-yet-tried cover; returns 1 if it did work (so the caller
// redraws and comes back for the next), 0 when every cover has been tried.
static int load_one_cover(void) {
  for (int i = 0; i < nbooks; i++) {
    if (cover_n[i] != -1) continue;
    int n = dsn_book_cover(books[i].path, slen(books[i].path), coverH, covers[i], COVER_MAX);
    cover_n[i] = n > 4 ? n : 0;  // 0 = no cover / not supported (no PSRAM)
    return 1;
  }
  return 0;
}

int main(void) {
  W = dsn_screen_width();
  H = dsn_screen_height();
  titleLh = dsn_line_height(DSN_FONT_BODY);
  authorLh = dsn_line_height(DSN_FONT_SMALL);
  rowH = COVER_H + 28;  // cover height plus row padding
  listTop = HEADER;
  listBottom = H - FOOT_H - 8;
  const int visibleRows = listBottom > listTop ? (listBottom - listTop) / rowH : 0;
  nbooks = parse(visibleRows < MAX_BOOKS ? visibleRows : MAX_BOOKS);
  coverH = COVER_H;  // the firmware returns a book-aspect width at this height
  for (int i = 0; i < nbooks; i++) cover_n[i] = -1;
  draw();
  // Load covers one per redraw so the firmware never decodes a whole shelf in a
  // single blocking call; each pops in as it's ready.
  while (load_one_cover()) draw();

  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) {
      dsn_open_browser();  // ends the app (button devices)
      return 0;
    }
    if (type == DSN_EV_TAP) {
      int y = DSN_EV_Y(e);
      if (y >= H - FOOT_H) {  // nav bar: open the tapped destination
        int i = DSN_EV_X(e) / (W / NAV_N);
        if (i >= 0 && i < NAV_N) dsn_open_home(nav_dest[i]);
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
