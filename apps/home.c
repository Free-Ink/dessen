// A home screen showing books from the Library, with Recents as a fallback.
// Tap a book or select it with Prev/Next and Confirm. Back opens Files.
#include "dessen.h"
#include "home_icons.h"

#define MARGIN 24
#define HEADER 72
#define FOOT_H 92
#define MAX_BOOKS 64
#define VIS 8  // max rows that can fit on screen (cover buffers are per-slot)

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

static char buf[8192];
static struct Book books[MAX_BOOKS];
static int nbooks, sel, top, perPage;  // sel = selection, top = first visible, perPage = rows shown
static int W, H, listTop, listBottom;

// Covers: packed {u16 w, u16 h} + MSB-first 1-bit rows. Kept per visible SLOT
// (not per book) so a long library needs only a few buffers; a slot is reloaded
// when the book under it changes (scroll). Loaded one per redraw, since
// dsn_book_cover decodes lazily. cover_slot[r] = the book index slot r holds.
#define COVER_MAX 1024
#define COVER_H 92   // cover thumbnail height (the row is built around it)
#define GUTTER 28    // side margin for the list
static unsigned char covers[VIS][COVER_MAX];
static int cover_n[VIS], cover_slot[VIS], coverH;

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
  // Header with a hairline rule under it, and a "page / total" hint when it scrolls.
  dsn_text_box(GUTTER, 24, W - 2 * GUTTER - 90, titleLh + 6, "Library", 7, DSN_FONT_TITLE, DSN_ALIGN_LEFT, 1, 0);
  dsn_line(GUTTER, HEADER - 14, W - GUTTER, HEADER - 14, 1, DSN_LIGHT_GRAY);
  if (nbooks > perPage) {
    char pg[16];
    int k = 0, cur = top / perPage + 1, tot = (nbooks + perPage - 1) / perPage;
    if (cur >= 10) pg[k++] = '0' + cur / 10;
    pg[k++] = '0' + cur % 10;
    pg[k++] = ' ', pg[k++] = '/', pg[k++] = ' ';
    if (tot >= 10) pg[k++] = '0' + tot / 10;
    pg[k++] = '0' + tot % 10;
    dsn_text_box(W - GUTTER - 90, 24, 90, titleLh + 6, pg, k, DSN_FONT_SMALL, DSN_ALIGN_RIGHT, 1, 0);
  }

  if (nbooks == 0) {
    dsn_text_box(GUTTER, H / 2 - 40, W - 2 * GUTTER, 120,
                 "No books yet. Add books, then open the Library once to index them.", 66, DSN_FONT_BODY,
                 DSN_ALIGN_CENTER, 4, 0);
  }
  int shown = nbooks - top < perPage ? nbooks - top : perPage;
  for (int r = 0; r < shown; r++) {
    int idx = top + r, y = listTop + r * rowH;
    int selected = idx == sel;
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
    dsn_text_box(tx, ty, tw, titleLh, books[idx].title, slen(books[idx].title), DSN_FONT_BODY, DSN_ALIGN_LEFT, 1,
                 DSN_TEXT_BOLD);
    dsn_text_box(tx, ty + titleLh + 6, tw, authorLh, books[idx].author, slen(books[idx].author), DSN_FONT_SMALL,
                 DSN_ALIGN_LEFT, 1, 0);
    // Progress: a rounded track with the read portion filled, percent beside it.
    int barY = ty + titleLh + 6 + authorLh + 18, barW = tw - 70;
    if (barW > 60) {
      // the track must contrast the row: white on the gray selection, else gray
      dsn_fill_round(tx, barY, barW, 10, selected ? DSN_WHITE : DSN_LIGHT_GRAY, 5, DSN_CORNERS_ALL);
      if (books[idx].percent > 0) {
        int fw = barW * books[idx].percent / 100;
        dsn_fill_round(tx, barY, fw < 10 ? 10 : fw, 10, DSN_BLACK, 5, DSN_CORNERS_ALL);
      }
      char pc[8];
      pct_str(pc, books[idx].percent);
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

// Clamp a scroll target and apply it, invalidating cover slots so the newly
// visible books reload.
static void scroll_to(int t) {
  int maxTop = nbooks - perPage;
  if (maxTop < 0) maxTop = 0;
  top = t < 0 ? 0 : t > maxTop ? maxTop : t;
}

// Fetch one visible cover that isn't loaded for the book now under its slot;
// returns 1 if it did work (caller redraws and comes back), 0 when the visible
// window is fully covered.
static int load_one_cover(void) {
  int shown = nbooks - top < perPage ? nbooks - top : perPage;
  for (int r = 0; r < shown; r++) {
    int idx = top + r;
    if (cover_slot[r] == idx) continue;  // already the right cover
    int n = dsn_book_cover(books[idx].path, slen(books[idx].path), coverH, covers[r], COVER_MAX);
    cover_n[r] = n > 4 ? n : 0;  // 0 = no cover / not supported (no PSRAM)
    cover_slot[r] = idx;
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
  perPage = listBottom > listTop ? (listBottom - listTop) / rowH : 1;
  if (perPage > VIS) perPage = VIS;
  if (perPage < 1) perPage = 1;
  nbooks = parse(MAX_BOOKS);  // load the whole library; the window scrolls
  coverH = COVER_H;           // the firmware returns a book-aspect width at this height
  for (int r = 0; r < VIS; r++) cover_slot[r] = -1;
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
      if (y >= H - FOOT_H) {  // nav bar: open the tapped destination
        int i = x / (W / NAV_N);
        if (i >= 0 && i < NAV_N) dsn_open_home(nav_dest[i]);
        return 0;
      }
      if (nbooks && y >= listTop && y < listBottom) {  // tap a book to open it
        int r = (y - listTop) / rowH;
        if (r < perPage && top + r < nbooks) {
          dsn_open_book(books[top + r].path, slen(books[top + r].path));
          return 0;
        }
      }
      continue;
    }
    if (nbooks == 0) continue;
    // Up/Down buttons move the selection (scrolling to keep it in view). Swipes
    // page the list. Confirm opens the selection; a tap opens what it hit.
    if (type == DSN_EV_NEXT) {
      if (sel >= nbooks - 1) continue;
      sel++;
      if (sel >= top + perPage) scroll_to(sel - perPage + 1);
    } else if (type == DSN_EV_PREV) {
      if (sel <= 0) continue;
      sel--;
      if (sel < top) scroll_to(sel);
    } else if (type == DSN_EV_SWIPE_UP) {  // content moves up -> next page
      if (top + perPage >= nbooks) continue;
      scroll_to(top + perPage);
      if (sel < top) sel = top;
    } else if (type == DSN_EV_SWIPE_DOWN) {  // -> previous page
      if (top == 0) continue;
      scroll_to(top - perPage);
      if (sel >= top + perPage) sel = top + perPage - 1;
    } else if (type == DSN_EV_CONFIRM) {
      dsn_open_book(books[sel].path, slen(books[sel].path));
      return 0;
    } else {
      continue;
    }
    draw();
    while (load_one_cover()) draw();  // reload covers for the newly visible books
  }
}
