# Writing a dessen app

A dessen app is a small program that runs **inside** CrossPoint firmware, in a
sandbox, without being flashed to the chip. You compile it to a tiny `.bin`,
drop it on the SD card, and launch it from the Plugins menu. It draws through
the firmware's own renderer and fonts, so it looks like part of the device.

This guide gets you from nothing to an app running on a real reader.

## What an app is (and isn't)

- It is a **RISC-V (RV32IM) binary** that the firmware runs with a ~1.3 KB
  interpreter. The interpreter bounds-checks every memory access, so a buggy or
  hostile app cannot touch firmware memory, the SD card, or anything else it
  was not handed.
- It talks to the device **only through system calls** (`dsn_*`). It has no
  direct hardware access: no GPIO, no Wi-Fi radio, no flash. Capabilities exist
  only where the firmware chose to expose one.
- It is **not** a firmware image, a plugin.js browser script, or a Lua script.
  It is compiled C (or Rust/Zig — anything that targets bare RV32IM), linked
  against the dessen SDK in [`sdk/`](../sdk).

If you have not already, skim the four example apps in [`apps/`](../apps):
`hello.c` (minimum), `dice.c` (drawing + random), `lightsout.c` (grid + touch),
`sudoku.c` (heavier logic). They are the best reference for real usage.

## Prerequisites

- A RISC-V GCC. If you have PlatformIO with an ESP32-C3 or S3 platform
  installed, you already have one at
  `~/.platformio/tools/toolchain-riscv32-esp/bin/`. The Makefile points there.
- A C++17 host compiler (`cc`/`c++`) for the desktop runner and tests.
- A checkout of [`freeink-sdk`](https://github.com/Free-Ink/freeink-sdk) next to
  this repo (the binding reads its UI headers). Override with
  `make FREEINK_SDK=/path/to/freeink-sdk`.

Build everything and run the tests:

```sh
make test     # builds the VM, the apps, and runs the suite
make sd       # stages launchable plugin folders under build/sd/plugins/
```

## Your first app

Create `apps/clock.c`:

```c
#include "dessen.h"

int main(void) {
  int w = dsn_screen_width(), h = dsn_screen_height();
  dsn_clear();
  dsn_text_box(0, h / 2 - 20, w, 40, "Hello", 5, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, 0);
  dsn_refresh();              // show it on the panel
  return dsn_wait_event();    // sleep until a button/tap, then exit
}
```

Add it to the `APPS` (and optionally `SD_APPS`) list in the
[`Makefile`](../Makefile), then:

```sh
make build/clock.bin          # compile
build/dessen build/clock.bin  # run it on the desktop
```

The desktop runner prints each drawing call and reads a key from stdin as an
event: `c` confirm, `b` back, `p` prev, `n` next, any other key is passed
through. That is your fast edit-run loop — no device needed.

## The shape of every app

Apps are **event-driven**, not frame-driven. The pattern is: draw, refresh,
block on `dsn_wait_event()`, react, repeat. On e-ink you redraw only when
something changed, because every refresh is a visible flash. Blocking on
`wait_event` also lets the device sleep between presses.

```c
#include "dessen.h"

int main(void) {
  draw();
  for (;;) {
    int e = dsn_wait_event();
    switch (DSN_EV_TYPE(e)) {
      case DSN_EV_BACK: return 0;                 // leave the app
      case DSN_EV_CONFIRM: /* ... */ break;
      case DSN_EV_PREV: case DSN_EV_NEXT: /* move a cursor */ break;
      case DSN_EV_TAP: {
        int x = DSN_EV_X(e), y = DSN_EV_Y(e);     // where, in app coordinates
        /* ... */
        break;
      }
      default: continue;                          // nothing to redo
    }
    draw();                                        // only after a real change
  }
}
```

### Events are semantic, not physical

You get `CONFIRM / BACK / PREV / NEXT / TAP`, never raw buttons. The firmware
maps each board's keys to these, so the **same binary** works on a
touch-and-button reader and a buttons-only one. Always offer a button path *and*
a touch path (as the examples do) so your app is usable everywhere. Holding
**Back** for one second always force-quits, so the user is never trapped.

### Coordinates respect the bezel automatically

`(0,0)` is the top-left of the device's **safe area**, and `dsn_screen_width/height()`
return the safe area's size — the bezel and rounded-corner insets are already
excluded. Draw to those bounds and nothing lands under a bezel. `dsn_clear()`
still wipes the whole panel. Taps on the bezel are not delivered. Call
`dsn_device()` if you want the raw panel size, the insets, the minimum
comfortable tap size, or whether the board has touch.

## The API

Fonts, colors, and text are the firmware's own, so your app matches the device.
All calls are declared in [`sdk/dessen_calls.h`](../sdk/dessen_calls.h)
(generated) with constants in [`sdk/dessen_abi.h`](../sdk/dessen_abi.h). Current
surface:

| Call | Does |
|---|---|
| `dsn_screen_width() / _height()` | Safe-area size |
| `dsn_device(&info, sizeof info)` | Panel size, insets, touch/button flags, orientation |
| `dsn_wait_event() -> e` | Block for the next event; decode with `DSN_EV_TYPE/X/Y(e)` |
| `dsn_exit(code)` | Quit (also happens if `main` returns) |
| `dsn_random() -> u32` | Hardware random number |
| `dsn_clear()` | White the whole panel |
| `dsn_refresh()` | Push the frame to the display |
| `dsn_fill(x,y,w,h,color)` | Filled rect; `dsn_fill_round(...,radius,corners)` for rounded |
| `dsn_stroke(x,y,w,h,color,width,radius,corners)` | Outlined rect |
| `dsn_line(x0,y0,x1,y1,width,color)` | Line, width 1–8 |
| `dsn_triangle(ax,ay,bx,by,cx,cy,color)` | Filled triangle |
| `dsn_text(x,y,s)` | One line, small font, to the right edge (convenience) |
| `dsn_text_box(x,y,w,h,s,len,font,align,max_lines,flags)` -> 0/-1 | Wrapped, aligned, bold/inverted text |
| `dsn_measure_text(s,len,font,flags) -> u32` | `width<<16 \| height` before drawing |
| `dsn_line_height(font) -> px` | Line height of a font |
| `dsn_bitmap(x,y,w,h,data,len,bw,bh,format,mode,color,rotation)` -> 0/-1 | 1-bit image |
| `dsn_list_row(x,y,w,h,label,len,flags)` -> 0/-1 | One menu row, drawn like the device's own lists |
| `dsn_grid_cell(x,y,w,h,cols,rows,gap,col,row,colspan,rowspan,&cell,sizeof cell)` -> 0/-1 | One cell of a grid |
| `dsn_split(x,y,w,h,axis,gap,lens,lens_len,slots,slots_len)` -> count/-1 | Divide a rect into flex/fixed slots |
| `dsn_ui_begin()` / `dsn_ui_event()` -> packed | Start a widget render pass / block and route input |
| `dsn_button / dsn_checkbox(x,y,w,h,label,len,action,flags)` | Interactive controls (real fui widgets) |
| `dsn_toggle(x,y,w,h,action,flags)` / `dsn_progress_bar(x,y,w,h,permille)` | Toggle switch / progress bar |

Colors: `DSN_WHITE / DSN_BLACK / DSN_LIGHT_GRAY / DSN_DARK_GRAY`.
Fonts: `DSN_FONT_SMALL / _BODY / _TITLE`. Text flags: `DSN_TEXT_BOLD / _INVERTED`.

**Menus: use `dsn_list_row`, don't hand-roll one.** For any list of choices,
draw each entry with `dsn_list_row` instead of composing your own
fill + text + chevron. It renders the row the way the firmware's own menus do —
the system selection highlight, the trailing chevron, the spacing and the
font — so your app looks like part of the device, and tracks the UI if the
theme changes. You still own layout, scrolling and hit-testing: lay the rows out
down the screen, remember each row's `y`/height to turn a tap back into an index,
and pass the row's state in `flags` (`DSN_ROW_SELECTED` for the highlighted row,
`DSN_ROW_CHEVRON` for a row that opens another screen, `DSN_ROW_DISABLED` to dim
one and drop its chevron). See `draw_list` in the Protected Content app, or the
`dsn_list_row` check in `apps/drawtest.c`. Reach for the raw `dsn_fill` /
`dsn_text_box` primitives only for chrome a row can't express.

**Positioning: let the layout engine place things.** `dsn_grid_cell` and
`dsn_split` are the firmware's own layout math, so a grid of tiles or a row of
panels lines up the way the rest of the device does, at any screen size, without
you hand-computing pixels. They compute rects and draw nothing — you get a
`dsn_rect_t` back and draw into it with the normal calls:

```c
dsn_rect_t c;                       // a 3x3 board, 4px gutters, inside the safe area
for (int i = 0; i < 9; i++) {
  dsn_grid_cell(0, 0, w, w, 3, 3, 4, i % 3, i / 3, 1, 1, &c, sizeof c);
  dsn_fill(c.x, c.y, c.w, c.h, DSN_LIGHT_GRAY);
}
```

`dsn_split` divides a rect along an axis into slots that are either `DSN_LEN_FIXED`
(exact px) or `DSN_LEN_FLEX` (share the leftover by weight) — a fixed sidebar
next to a flexible body, a toolbar of equal buttons, and so on. Each slot is 4
bytes `{u8 kind, i16 value, u8 flex}`; it writes one `dsn_rect_t` per slot.

### Interactive widgets (buttons, checkboxes, toggles)

For controls the user operates, use the widget calls instead of drawing your own
and hit-testing taps by hand. They draw the firmware's real controls *and* route
input the native way: a tap goes to the control under it, and on button-only
boards PREV/NEXT move a focus highlight and CONFIRM fires the focused control —
all handled for you. The shape is a render loop bracketed by `dsn_ui_begin` and
`dsn_ui_event`:

```c
enum { A_SAVE = 1, A_WRAP = 2 };          // your own nonzero action ids
int wrap = 0;
for (;;) {
  dsn_clear();
  dsn_ui_begin();                          // start the pass: clears the hit table
  dsn_checkbox(16, 70, w - 32, 48, "Wrap", 4, A_WRAP, wrap ? DSN_W_CHECKED : 0);
  dsn_button(16, 140, w - 32, 56, "Save", 4, A_SAVE, 0);
  dsn_refresh();
  unsigned e = dsn_ui_event();             // blocks; routes the next input
  int a = DSN_UI_ACTION(e);
  if (a == A_SAVE) break;
  else if (a == A_WRAP) wrap = !wrap;
  else if (a == 0 && DSN_UI_SYS(e) == DSN_UI_BACK) break;  // Back pressed
  // (a == 0 && DSN_UI_SYS == DSN_UI_REDRAW: focus moved — just loop and redraw)
}
```

Give each interactive widget a **nonzero** action id. `dsn_ui_event` returns a
packed value: when `DSN_UI_ACTION(e)` is nonzero a widget fired (with
`DSN_UI_VALUE(e)` its value, e.g. a list index); otherwise `DSN_UI_SYS(e)` is a
system code — `DSN_UI_BACK` (Back, nothing consumed it: usually quit) or
`DSN_UI_REDRAW` (focus moved, so repaint to show the new highlight). Flags:
`DSN_W_CHECKED` (checkbox ticked / toggle on), `DSN_W_DISABLED` (dimmed). See
`apps/widgets.c`. `dsn_progress_bar` is display-only (no action). Mixing modes:
an app that calls `dsn_ui_event` shouldn't also call `dsn_wait_event` in the same
loop — pick one input model per screen.

Calls that can fail on bad input (`text_box`, `bitmap`) return `-1`; check it
during development. The host silently refuses anything out of range — it never
trusts an app value — so a bug draws nothing rather than corrupting the device.

## Constraints to design around

- **Memory:** one fixed block, 64 KB today (see `sdk/app.ld`). No `malloc` is
  wired up; use static and stack storage. Keep big arrays modest. (On PSRAM
  boards this cap can be raised per-app later; ask if you need it.)
- **No libc:** the SDK is freestanding. You have the `dsn_*` calls and whatever
  you write yourself. There is no `printf`, `memcpy` from a header, or 64-bit
  divide helper unless you add one. Integer math and your own small helpers are
  the norm — see how the examples format numbers by hand.
- **Speed:** interpreted, so fine for UI and game logic but not for pixel-level
  image work or text layout — those belong in host calls. In practice the e-ink
  refresh dominates everything anyway.
- **Draw only on change:** redrawing every loop makes the screen flash
  constantly. Track state and redraw after a real change.

## Testing

Non-trivial apps should leave one runnable check behind. There are two styles
in this repo:

1. **Self-checking app:** a build that runs entirely in the VM and `return`s 0
   on pass or a failing check number — see `apps/selftest.c` and
   `apps/drawtest.c`. Run with `build/dessen build/selftest.bin`.
2. **Binding test:** drive the real app against a recording draw target and
   assert on the calls it makes — see `freeink/test_host.cpp`. This is how
   `dice`, `lightsout`, and `sudoku` are checked.

Wire either into the `test:` target so `make test` covers your app.

## Getting it onto a device

```sh
make sd
```

stages `build/sd/plugins/<app>/` folders, each with `app.bin` and a
`manifest.json` (copied from `apps/<app>.json` — `title` and `description` show
in the menu). Copy the folder into `/plugins/` on the reader's SD card, then:

1. **Settings → Connectivity → Plugins** (enable).
2. **Home → Plugins → <your app>**.

The firmware launches any plugin folder that contains an `app.bin`. Holding
**Back** quits back to the menu.

> The firmware side lives behind `-DCROSSPOINT_DESSEN_APPS` and is **off by
> default**. The device must be running a CrossPoint build with it enabled (see
> the `feat/dessen-apps` branch and the `[dessen_apps]` section in its
> `platformio.ini`). Firmware and apps are built from the same ABI, so rebuild
> both together when the call list changes.

## Adding a capability

If your app needs something the firmware can do but no call exposes yet (a new
shape, frontlight, a file read), you add **one line** to
[`abi/freeink.api`](../abi/freeink.api) and run `python3 tools/gen_api.py`. That
regenerates both the app-side wrapper and the firmware-side dispatch. See
`AGENTS.md` for the recipe and the rules every new call must follow (clip, pin,
bounds-check — every app value is untrusted). Network and file access need a
"call in progress" mechanism that does not exist yet; ask before reaching for
those.
