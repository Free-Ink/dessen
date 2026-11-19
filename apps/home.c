// A home screen showing recently opened books, then recently added books.
// Tap a book or select it with Prev/Next and Confirm. Back opens Files.
#include "dessen.h"
#include "home_icons.h"

#define MARGIN 24
#define HEADER 72
#define FOOT_H 92
#define VIS DSN_SHELF_PAGE_MAX  // cover buffers are per visible row

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
static int rowH, headingLh, titleLh, authorLh;

struct Book {
  const char *path;
  const char *title;
  const char *author;
  int percent;
};

// Keep the displayed strings intact if the host fails while reading the next page.
static char pages[2][VIS * DSN_SHELF_RECORD_MAX + 1];
static struct Book books[VIS];
static int activePage, pageStart, pageRows, shown, sel, perPage;
static int W, H, listTop, listBottom;

// Covers: packed {u16 w, u16 h} + MSB-first 1-bit rows. Kept per visible SLOT
// (not per book) so a long library needs only a few buffers; a slot is reloaded
// when the book under it changes (scroll). Loaded one per redraw, since
// dsn_book_cover decodes lazily. cover_slot[r] = the book index slot r holds.
#define COVER_MAX 1024
#define COVER_H 92   // cover thumbnail height (the row is built around it)
#define GUTTER 28    // side margin for the list
static unsigned char covers[VIS][COVER_MAX];
static int cover_n[VIS], cover_slot[VIS];

static int slen(const char *s);

// Split the returned records in place so the visible rows can refer to buf.
static int parse(char *buf, int n) {
  if (n < 0 || n >= (int)sizeof pages[0]) return 0;
  buf[n] = 0;
  int count = 0;
  char *p = buf;
  while (*p && count < perPage) {
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
      const int percent = dsn_book_progress(fields[0], slen(fields[0]));
      books[count].percent = percent <= 100 ? percent : -1;
      count++;
    }
    p = more ? line_end + 1 : line_end;
  }
  return count;
}

static int show_page(int start, int minRows) {
  if (start < 0) return 0;
  const int nextPage = 1 - activePage;
  char *buf = pages[nextPage];
  int n = dsn_recent_shelf(start, perPage, buf, sizeof pages[0] - 1);
  if (n < 0 && start == 0 && !pageRows) n = dsn_recents(buf, sizeof pages[0] - 1);
  if (n <= 0 || n >= (int)sizeof pages[0]) return 0;
  int rows = 0;
  for (int i = 0; i < n && rows < perPage; ++i) rows += buf[i] == '\n';
  if (rows < minRows) return 0;
  pageRows = rows;
  shown = parse(buf, n);
  if (sel >= shown) sel = shown ? shown - 1 : 0;
  activePage = nextPage;
  for (int i = 0; i < VIS; ++i) cover_slot[i] = -1;
  pageStart = start;
  return 1;
}

static int next_page(void) {
  if (pageRows < perPage) return show_page(pageStart, pageRows + 1);
  if (!show_page(pageStart + pageRows, 1)) return 0;
  sel = 0;
  return 1;
}

static int put_number(char *out, int value) {
  int n = 0, place = 1;
  while (value / place >= 10) place *= 10;
  for (; place > 0; place /= 10) out[n++] = '0' + value / place % 10;
  return n;
}

static int nav_edge(int i) {
  return i * W / NAV_N;
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

// Nav glyphs are real icons bundled in the app binary (home_icons.h) and blitted
// with dsn_bitmap — the way any app adds its own icons.
static const unsigned char *const nav_bmp[NAV_N] = {ic_folder, ic_globe, ic_puzzle, ic_transfer, ic_settings};
#define ICON_BYTES (((ICON_SZ + 7) / 8) * ICON_SZ)

static void draw_glyph(int idx, int cx, int cy) {
  dsn_bitmap(cx - ICON_SZ / 2, cy - ICON_SZ / 2, ICON_SZ, ICON_SZ, nav_bmp[idx], ICON_BYTES, ICON_SZ, ICON_SZ,
             DSN_BITMAP_BW, DSN_BITMAP_CENTER, DSN_BLACK, 0);
}

static void draw(void) {
  dsn_clear();
  const int rangeW = 128;
  dsn_text_box(GUTTER, 24, W - 2 * GUTTER - rangeW, headingLh, "Recents", 7, DSN_FONT_TITLE, DSN_ALIGN_LEFT, 1, 0);
  dsn_line(GUTTER, HEADER - 14, W - GUTTER, HEADER - 14, 1, DSN_LIGHT_GRAY);
  if (pageRows) {
    char range[24];
    int n = put_number(range, pageStart + 1);
    if (pageRows > 1) {
      range[n++] = '-';
      n += put_number(range + n, pageStart + pageRows);
    }
    dsn_text_box(W - GUTTER - rangeW, 24, rangeW, authorLh, range, n, DSN_FONT_SMALL, DSN_ALIGN_RIGHT, 1, 0);
  } else {
    const char *msg = "No recent books. Open a book or add books and update the Library.";
    dsn_text_box(GUTTER, H / 2 - 40, W - 2 * GUTTER, 4 * titleLh,
                 msg, slen(msg), DSN_FONT_BODY, DSN_ALIGN_CENTER, 4, 0);
  }
  for (int r = 0; r < shown; r++) {
    int idx = pageStart + r, y = listTop + r * rowH;
    int selected = r == sel;
    if (selected)  // soft rounded highlight behind the whole row
      dsn_fill_round(GUTTER - 12, y + 6, W - 2 * GUTTER + 24, rowH - 12, DSN_LIGHT_GRAY, 18, DSN_CORNERS_ALL);

    int cy = y + (rowH - COVER_H) / 2;  // cover / content top
    int tx = GUTTER;
    if (cover_slot[r] == idx && cover_n[r] > 4) {
      int cw = covers[r][0] | covers[r][1] << 8, ch = covers[r][2] | covers[r][3] << 8;
      dsn_bitmap(GUTTER, cy, cw, ch, covers[r] + 4, cover_n[r] - 4, cw, ch, DSN_BITMAP_BW, DSN_BITMAP_CENTER,
                 DSN_BLACK, 0);
      dsn_stroke(GUTTER, cy, cw, ch, DSN_BLACK, 1, 0, 0);  // crisp cover edge
      tx = GUTTER + cw + 20;
    }
    int tw = W - GUTTER - tx;
    // Vertically center the title/author/progress block against the cover.
    int blockH = titleLh + 6 + authorLh + 18 + 10;
    int ty = cy + (COVER_H - blockH) / 2;
    if (ty < cy) ty = cy;
    dsn_text_box(tx, ty, tw, titleLh, books[r].title, slen(books[r].title), DSN_FONT_BODY, DSN_ALIGN_LEFT, 1,
                 DSN_TEXT_BOLD);
    dsn_text_box(tx, ty + titleLh + 6, tw, authorLh, books[r].author, slen(books[r].author), DSN_FONT_SMALL,
                 DSN_ALIGN_LEFT, 1, 0);
    // Progress: a rounded track with the read portion filled, percent beside it.
    int barY = ty + titleLh + 6 + authorLh + 18, barW = tw - 70;
    if (barW > 60) {
      // the track must contrast the row: white on the gray selection, else gray
      dsn_fill_round(tx, barY, barW, 10, selected ? DSN_WHITE : DSN_LIGHT_GRAY, 5, DSN_CORNERS_ALL);
      if (books[r].percent > 0) {
        int fw = barW * books[r].percent / 100;
        dsn_fill_round(tx, barY, fw < 10 ? 10 : fw, 10, DSN_BLACK, 5, DSN_CORNERS_ALL);
      }
      char pc[8];
      pct_str(pc, books[r].percent);
      if (pc[0])
        dsn_text_box(tx + barW + 12, barY + 5 - authorLh / 2, 56, authorLh, pc, slen(pc), DSN_FONT_SMALL,
                     DSN_ALIGN_LEFT, 1, 0);
    }
  }

  // Nav bar: one tappable cell per home destination (glyph over a label). Touch
  // devices have no Back key, so this is the only way to reach the rest.
  int navTop = H - FOOT_H;
  dsn_line(0, navTop, W, navTop, 1, DSN_LIGHT_GRAY);
  for (int i = 0; i < NAV_N; i++) {
    int left = nav_edge(i), width = nav_edge(i + 1) - left;
    draw_glyph(i, left + width / 2, navTop + 28);
    dsn_text_box(left, navTop + 48, width, authorLh, nav_label[i], slen(nav_label[i]), DSN_FONT_SMALL,
                 DSN_ALIGN_CENTER, 1, 0);
  }
  dsn_refresh();
}

static int slen(const char *s) {
  int n = 0;
  while (s[n]) n++;
  return n;
}

// Fetch one visible cover that isn't loaded for the book now under its slot;
// returns 1 if it did work (caller redraws and comes back), 0 when the visible
// window is fully covered.
static int load_one_cover(void) {
  for (int r = 0; r < shown; r++) {
    int idx = pageStart + r;
    if (cover_slot[r] == idx) continue;  // already the right cover
    int n = dsn_book_cover(books[r].path, slen(books[r].path), COVER_H, covers[r], COVER_MAX);
    cover_n[r] = n > 4 ? n : 0;  // 0 = no cover / not supported (no PSRAM)
    cover_slot[r] = idx;
    return 1;
  }
  return 0;
}

int main(void) {
  W = dsn_screen_width();
  H = dsn_screen_height();
  headingLh = dsn_line_height(DSN_FONT_TITLE);
  titleLh = dsn_line_height(DSN_FONT_BODY);
  authorLh = dsn_line_height(DSN_FONT_SMALL);
  rowH = COVER_H + 28;  // cover height plus row padding
  listTop = HEADER;
  listBottom = H - FOOT_H - 8;
  perPage = listBottom > listTop ? (listBottom - listTop) / rowH : 1;
  if (perPage > VIS) perPage = VIS;
  if (perPage < 1) perPage = 1;
  show_page(0, 1);
  draw();
  while (load_one_cover()) draw();  // covers pop in one per redraw

  for (;;) {
    int e = dsn_wait_event(), type = DSN_EV_TYPE(e);
    if (type == DSN_EV_BACK) {
      dsn_open_browser();  // ends the app (button devices)
      return 0;
    }
    if (type == DSN_EV_TAP) {
      int x = DSN_EV_X(e), y = DSN_EV_Y(e);
      if (y >= H - FOOT_H) {
        for (int i = 0; i < NAV_N; i++) {
          if (x < nav_edge(i + 1)) {
            dsn_open_home(nav_dest[i]);
            break;
          }
        }
      } else if (y >= listTop && y < listBottom) {
        int r = (y - listTop) / rowH;
        if (r < shown) {
          const char *path = books[r].path;
          dsn_open_book(path, slen(path));
        }
      }
      continue;
    }
    if (!pageRows) {
      if ((type == DSN_EV_NEXT || type == DSN_EV_CONFIRM || type == DSN_EV_SWIPE_UP) && show_page(0, 1)) {
        draw();
        while (load_one_cover()) draw();
      }
      continue;
    }
    if (type == DSN_EV_NEXT) {
      if (sel + 1 < shown) sel++;
      else if (!next_page()) continue;
    } else if (type == DSN_EV_PREV) {
      if (sel > 0) sel--;
      else if (show_page(pageStart - perPage, 1)) sel = shown ? shown - 1 : 0;
      else continue;
    } else if (type == DSN_EV_SWIPE_UP) {
      if (!next_page()) continue;
    } else if (type == DSN_EV_SWIPE_DOWN) {
      if (!show_page(pageStart - perPage, 1)) continue;
      sel = shown ? shown - 1 : 0;
    } else if (type == DSN_EV_CONFIRM && shown) {
      const char *path = books[sel].path;
      dsn_open_book(path, slen(path));
      continue;
    } else {
      continue;
    }
    draw();
    while (load_one_cover()) draw();  // reload covers for the newly visible books
  }
}
