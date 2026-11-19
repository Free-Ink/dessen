// Shows the interactive widgets and the fui interaction model: draw the screen
// between dsn_ui_begin() and dsn_ui_event(), and the firmware routes a tap or
// button navigation (PREV/NEXT move focus, CONFIRM fires the focused control)
// to the control's action. One code path works on touch and button boards.
#include "dessen.h"

enum { A_OK = 1, A_WRAP = 2, A_DARK = 3 };  // action ids the widgets carry

static int wrap, dark, prog = 350;

static void render(void) {
  int w = dsn_screen_width();
  dsn_clear();
  dsn_text_box(20, 12, w - 40, 40, "Settings", 8, DSN_FONT_TITLE, DSN_ALIGN_LEFT, 1, 0);
  dsn_ui_begin();
  dsn_checkbox(16, 70, w - 32, 48, "Wrap long lines", 15, A_WRAP, wrap ? DSN_W_CHECKED : 0);
  dsn_checkbox(16, 126, w - 32, 48, "Dark mode", 9, A_DARK, dark ? DSN_W_CHECKED : 0);
  dsn_progress_bar(16, 196, w - 32, 16, prog);
  dsn_button(16, 236, w - 32, 56, "Done", 4, A_OK, 0);
  dsn_refresh();
}

int main(void) {
  render();
  for (;;) {
    unsigned e = dsn_ui_event();
    int a = DSN_UI_ACTION(e);
    if (a == A_OK) return 7;  // Done: exit 7 (the test asserts this)
    if (a == A_WRAP) wrap = !wrap;
    else if (a == A_DARK) dark = !dark;
    else if (a == 0 && DSN_UI_SYS(e) == DSN_UI_BACK) return 0;  // Back quits
    // action handled, or focus moved (DSN_UI_REDRAW): repaint to reflect it
    render();
  }
}
