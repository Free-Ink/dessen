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
  int lh = dsn_line_height(DSN_FONT_TITLE);
  dsn_text_box(0, (h - lh) / 2, w, lh, "Hello", 5, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, 0);
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
event: `c` confirm, `b` back, `p` prev, `n` next. The desktop runner enables file
access when you set `DESSEN_FS_ROOT` to the directory the app may use. Set
`DESSEN_NET_ROOT` to enable local HTTP fixtures. A request for
`https://example.test/hello` reads `example.test/hello` inside that root. An
optional `hello.status` file supplies the HTTP status. Fetching to a file
requires both roots. Choose directories that contain only data intended for the app.

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

You get semantic events, never raw buttons. The firmware maps each board's keys
and gestures to these, so the **same binary** works on a touch-and-button reader
and a buttons-only one:

- `CONFIRM` / `BACK` — the confirm/back keys (activate / go back).
- `PREV` / `NEXT` — the up/down keys (and focus-prev/next). Use them to **move a
  selection**, not to page. Left and Right arrive as `PREV` and `NEXT` too, with
  `DSN_EV_SIDEWAYS` set in the event, so an app that only reads `DSN_EV_TYPE(e)`
  treats both pairs alike, and one that wants a second axis can tell them apart.
  (The firmware has to mark them; until it does, the flag never appears.)
- `TAP` / `LONG_PRESS` — a touch and a press-and-hold; both carry `x, y`
  (`DSN_EV_X/Y(e)`).
- `SWIPE_UP` / `SWIPE_DOWN` / `SWIPE_LEFT` / `SWIPE_RIGHT` — the four touch
  swipes (no coordinates). Use vertical swipes to **page/scroll**.

A board with up/down keys *and* touch (most of them) delivers both, so these stay
distinct: keys move the selection, a swipe pages, a tap opens. Always offer a
button path *and* a touch path (as the examples do) so your app is usable
everywhere. Holding **Back** for one second always force-quits, so the user is
never trapped.

### Coordinates respect the bezel automatically

`(0,0)` is the top-left of the device's **safe area**, and `dsn_screen_width/height()`
return the safe area's size — the bezel and rounded-corner insets are already
excluded. Draw to those bounds and nothing lands under a bezel. `dsn_clear()`
still wipes the whole panel. Taps on the bezel are not delivered. Call
`dsn_device()` if you want the raw panel size, the insets, the minimum
comfortable tap size, or whether the board has touch.

Button hints are the host's to draw, in its own look and over its own keys:
call `dsn_button_hints("Back\nSelect\nUp\nDown", len)` with one label per
action (back, confirm, previous, next; an empty line for none), after drawing
the screen and before `dsn_refresh()`. `dsn_device()` reports `hint_height`,
the band along the bottom it draws in: keep that clear. It is 0 where the
device shows no hints (a touch board, no keys, or turned), and then the call
does nothing, so an app can make it everywhere.

## The API

Fonts, colors, and text are the firmware's own, so your app matches the device.
All calls are declared in [`sdk/dessen_calls.h`](../sdk/dessen_calls.h)
(generated) with constants in [`sdk/dessen_abi.h`](../sdk/dessen_abi.h). Current
surface:

| Call | Does |
|---|---|
| `dsn_screen_width() / _height()` | Safe-area size |
| `dsn_device(&info, sizeof info)` | Panel size, insets, touch/button flags, orientation, button-hint band |
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
| `dsn_set_clip(x,y,w,h)` -> 0/error | Limit every later draw to a rectangle, until the next call; a negative width lifts it. Kept between events, never applied to the firmware's own drawing |
| `dsn_button_hints(labels, len)` -> 0/error | The host draws its own button hints: back, confirm, previous, next, one per line (empty: none). Nothing where `hint_height` is 0; -1 for over four lines, a control byte, or a label over `DSN_HINT_LABEL_MAX` bytes |
| `dsn_recent_shelf(first,count,list,len)` -> bytes or error | Recently opened books first, then recently added books. Use a zero-based `first` and a `count` up to `DSN_SHELF_PAGE_MAX`. Each row is `path<TAB>title<TAB>author<TAB>-1<LF>` or a blank line for an unreadable entry. Zero means end of shelf. |

Advance `first` by the number of lines you received, including blank lines.
Allow `DSN_SHELF_RECORD_MAX` bytes per requested row to fit a full page. Use
`dsn_book_progress()` for the reading progress of visible books. Rebuild the
firmware and app binaries together when migrating to the recent shelf call.

Colors: `DSN_WHITE / DSN_BLACK / DSN_LIGHT_GRAY / DSN_DARK_GRAY`, and for
`dsn_fill` only `DSN_SCRIM`: light-gray ink over what is already there, for
dimming behind a dialog. Whether the background shows through depends on the
firmware's renderer; on one that cannot, it looks like `DSN_LIGHT_GRAY`.
Fonts: `DSN_FONT_SMALL / _BODY / _TITLE`. Text flags: `DSN_TEXT_BOLD / _INVERTED`.

Calls that can fail on bad input (`text_box`, `bitmap`) return `-1`; check it
during development. The host silently refuses anything out of range — it never
trusts an app value — so a bug draws nothing rather than corrupting the device.

### Full reference

The table above is the drawing core. The rest are **capabilities** — each
feature-detectable (it returns `DSN_ERR_UNAVAILABLE` when the host doesn't grant
it, so you can fall back). The complete list lives in
[`abi/freeink.api`](../abi/freeink.api) (the source of truth), with per-call host
behaviour in [`docs/proposed-capabilities.md`](proposed-capabilities.md). By group:

- **Core:** `dsn_time` (Unix seconds), `dsn_clock` (local time struct),
  `dsn_device_id` (stable printable id), `dsn_random_bytes`,
  `dsn_locale` (BCP47 UI language code, e.g. `"en"` — for localizing your own strings).
- **Text input:** `dsn_text_input(prompt, out, len)` — the firmware runs its own
  on-screen keyboard and hands back the text.
- **Storage** (the SD card; path rules enforced): `dsn_fs_read`,
  `dsn_fs_read_at(path, offset, out, len)` (ranged read — for files bigger than
  app memory), `dsn_fs_write`, `dsn_fs_list`, `dsn_fs_delete`,
  `dsn_fs_app_dir` (your own plugin folder), `dsn_keystore`.
- **Network:** `dsn_net_request` (one HTTPS request), `dsn_fetch_to_sd` (stream a
  URL straight to a file).
- **Crypto:** `dsn_hash`, `dsn_aes_cbc`, `dsn_rsa_keygen` / `_encrypt` /
  `_decrypt` / `_sign`, `dsn_pkcs12_parse`.
- **Board:** `dsn_battery` (percent, with the power state `DSN_PWR_*` in the 2nd
  byte), `dsn_backlight` / `dsn_backlight_get`, `dsn_haptic`, `dsn_accel`.
- **Launcher / home:** `dsn_open_book`, `dsn_open_app`, `dsn_open_settings`,
  `dsn_open_browser`, `dsn_open_home(DSN_HOME_*)`, `dsn_recents`,
  `dsn_recent_shelf`, `dsn_book_progress`, `dsn_book_cover`, `dsn_wifi_connect`.

Text input, network, and key generation are **async**, but they look synchronous
to your app: the firmware parks the VM while the work runs and resumes you with
the result — just read the return value.

### Reading stats

The firmware keeps an append-only log of measured reading sessions at
`/.crosspoint/reading_sessions.jsonl` — **one JSON object per line**:

| key | meaning |
|---|---|
| `doc` | 32-hex sync document id of the book |
| `start` | Unix time of the first counted page |
| `secs` | active reading seconds |
| `tz` | UTC offset in minutes at `start` |
| `from` / `to` | whole-book progress in basis points (0..10000) |

The log is never trimmed, so **stream it with `dsn_fs_read_at`** a chunk at a
time — never load it whole, it outgrows the 64 KB app memory. Parse line by line;
a plain substring search for `"key":` is sound (quotes inside strings are
escaped). Aggregate into per-day seconds (a heatmap), per-hour, streaks, pages,
etc. The full worked example is [`apps/readstats.rs`](../apps/readstats.rs) — a
Rust home launcher that renders heatmap + bar-chart cards. (The schema is owned
by crosspoint `src/sync/ReadingSessionLog.h`.)

## Laying out a screen

The firmware's fonts and components give you the device's look for free, but a
few rules keep spacing honest. `apps/home.c` follows all of them.

- **Size text boxes by line height — never hardcode a height.** `dsn_text_box`
  draws **nothing** (returns `-1`) if its rect is shorter than one line of the
  font. A box that looked fine with a 16px test font vanishes on a device whose
  body font is ~30px. Height a row or box with `dsn_line_height(font)` (× the
  line count), not a guessed pixel value.
- **Let the box center the text.** The firmware vertically centers text in the
  rect you pass, so give a row its full height and a single `y` — don't add your
  own offset on top, or it double-centers.
- **Measure before you place.** For content that sits below variable or wrapped
  text, use `dsn_measure_text` rather than a magic gap, so the layout holds for
  any string and any board font size.
- **One gutter, everywhere.** Pick a single side margin (~28px) and draw every
  screen to `GUTTER … width-GUTTER`. A title row with a hairline rule
  (`dsn_line` in `DSN_LIGHT_GRAY`) under it gives structure cheaply.
- **Selection = an inset rounded pill.** `dsn_fill_round(... , radius, corners)`
  inset from the gutter reads better than an edge-to-edge gray bar. Keep
  contrast: don't draw a gray element (e.g. a progress bar) on the gray
  selection fill — swap it to white when the row is selected.

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

## Reacting to firmware events

The firmware emits a small set of events (`reader.open`, `reader.exit`,
`reader.session`, `book.downloaded`, `sleep.enter`, `reader.end`). Your plugin
can hook one by adding an `events` section to its `device.json` that names your
app:

```jsonc
{
  "events": {
    "reader.exit":    { "app": "app.bin", "mode": "read"   },
    "book.downloaded":{ "app": "app.bin", "mode": "launch" }
  }
}
```

- **`read`** — the firmware appends the event (one JSON line) to
  `app-events.jsonl` in your plugin folder. Nothing happens on screen; your app
  processes the queue the next time it runs.
- **`launch`** — same queue, and the firmware also opens your app full-screen
  right away (except for `sleep.enter`, where there is no UI). Opening a new app
  mid-flow is intrusive, so prefer `read` unless the event really should
  interrupt the reader.

`reader.end` fires once when the reader reaches the last page. A `reader.end`
handler may also set `"replaces": "end of book"` — reaching the end then opens
your app full-screen *in place of* the firmware's end-of-book menu (this implies
`launch`). Your app owns that screen: offer your own actions and exit (or launch
another book) through the usual syscalls. Paging to the end again re-opens it.

```jsonc
"reader.end": { "app": "app.bin", "mode": "launch", "replaces": "end of book" }
```

Read the queue from your own app directory and clear it when done — it is plain
text, one JSON object per line:

```c
char dir[128];
dsn_fs_app_dir(dir, sizeof dir);          // e.g. "/plugins/myapp"
char path[160];
int plen = /* build dir + "/app-events.jsonl" into path */;
char buf[1024];
int n = dsn_fs_read(path, plen, buf, sizeof buf);
if (n > 0) { /* parse the lines you care about */ dsn_fs_delete(path, plen); }
```

The `vars` object carries the event's fields (`book`, `percent`, `title`, …).
Events route to `app-events.jsonl`, never the network outbox, so the firmware's
own event delivery never touches or deletes your queue.

## Which boards an app needs

Most apps need nothing in particular: they offer a touch path and a button path
and run on every board. An app that is unusable without one kind of input says
so in its manifest:

```json
{ "title": "Sketch", "description": "Draw with a finger", "requires": ["touch"] }
```

| `requires` | meaning |
|---|---|
| absent or `[]` | runs on any board (the norm) |
| `["touch"]` | needs a touch panel |
| `["buttons"]` | needs physical keys |

A launcher reads this **before** the app runs, to decide whether to offer it on
this board; a host that meets a word it does not know treats the app as not
runnable rather than guess. It is not how a running app adapts. That comes from
`dsn_device`: `has_touch` and `has_buttons` for navigation, `hint_height` for
whether button hints show. The same app gets hints on a buttons-only board and
none on a touch board, which no fixed manifest value could say.

On the desktop, `DESSEN_BOARD=touch|buttons|both` picks the board (default:
buttons, no touch) and `DESSEN_MANIFEST=apps/<app>.json` the manifest to check;
a `manifest.json` beside the image is checked too. When the board falls short,
the runner exits with 103 instead of starting the app.

## Getting it onto a device

```sh
make sd
```

stages `build/sd/plugins/<app>/` folders, each with `app.bin` and a
`manifest.json` (copied from `apps/<app>.json` — `title` and `description` show
in the menu, and `requires`, below, says which boards can drive the app). Copy the folder into `/plugins/` on the reader's SD card, then:

1. **Settings → Connectivity → Plugins** (enable).
2. **Home → Plugins → <your app>**.

To use the bundled `home` app as your launcher, open the built-in Library
to create or refresh its index. Then choose **Recents** under
**Settings → Display → Home Launcher**. Home shows recently opened books first and fills out the shelf
with recently added books from the Library. A book already shown as opened
appears once. Without an index, Home still shows recently opened books.

The firmware launches any plugin folder that contains an `app.bin`. Holding
**Back** quits back to the menu.

A home-launcher app can offer **shortcuts to other dessen apps**: call
`dsn_open_app(path, len)` with a plugin's `/plugins/<name>/app.bin`. It opens
that app full-screen (like `dsn_open_book` opens the reader); when the app exits,
the firmware returns to your launcher. See the `Dice` entry in `apps/minimal.c`.

> The firmware side lives behind `-DCROSSPOINT_DESSEN_APPS` and is **off by
> default**. The device must be running a CrossPoint build with it enabled (see
> the `feat/dessen-apps` branch and the `[dessen_apps]` section in its
> `platformio.ini`). Firmware and apps are built from the same ABI, so rebuild
> both together when the call list changes.

## Other languages

Nothing here is C-specific. An app is just a freestanding **RV32IM** image (no
compressed/`C` extension, no float, ≤64 KB, `_start` → `main` → `exit`) that
reaches the host through one instruction: `ecall` with the call number in `a7`,
args in `a0`–`a6`, result in `a0`. Any language with a freestanding RV32IM
backend works — the runtime never learns what produced the bytes.

The C `hello` is rewritten verbatim in two other languages as worked examples,
each linking against the *same* `sdk/crt0.S` and `sdk/app.ld` as the C apps. The
`ecall` wrappers are **generated** from `abi/freeink.api` by `tools/gen_api.py` —
the same single source of truth as the C header — so every call is first-class
in all three languages, with the ABI's arg-packing handled for you:

- [`apps/hello.rs`](../apps/hello.rs) — **Rust** (`#![no_std]`, no allocator).
  `include!("../sdk/dessen.rs")`, then call `dsn_clear()`, `dsn_text_box(…)`, …
  `make rust` (one-time: `rustup target add riscv32im-unknown-none-elf`).
- [`apps/readstats.rs`](../apps/readstats.rs) — a full Rust app: reading-stats
  cards (heatmap, bar charts) from the firmware's session log, streamed with
  `dsn_fs_read_at` because the log outgrows app memory. Any `apps/<name>.rs`
  builds with `make build/<name>.bin`.
- [`apps/hello.zig`](../apps/hello.zig) — **Zig** (freestanding, no std).
  `@import` the `dsn` module (`sdk/dessen.zig`), then `dsn.clear()`, `dsn.text_box(…)`.
  `make zig` (one-time: install Zig). Or with Zig alone: `zig build` writes
  `zig-out/bin/hello_zig.bin` (the same image, linked by Zig instead of gcc) and
  `zig build run` runs it; a new Zig app is one entry in `apps` in
  [`build.zig`](../build.zig). zls reads the same file, so `dsn.*` resolves in the editor.

Rust is part of the normal build: `make test` builds and checks the Rust apps
(and the xpui ones below) along with the C ones, and `make sd` stages them
(`hello_rs/`, `readstats/`, `<name>_xpui/`). Both stop with the install command
if the riscv32im target is missing; `make test RUST=no` / `make sd RUST=no`
leaves Rust out. Zig is staged as `hello_zig/` whenever Zig is installed. To
pick the staged apps by hand, pass `make sd SD_LANG_APPS="hello_rs"`, or
`SD_LANG_APPS=""` for C only.

The output is the same kind of image as a C app: RV32IM code for the dessen VM,
not for the reader's chip. One `app.bin` runs on every board, ESP32-C3 (RISC-V)
and ESP32-S3 (Xtensa) alike. Only the firmware is built per chip.

### The Rust SDK crate

For a Rust app built with `cargo`, depend on the `dessen` crate in
[`sdk/rust/`](../sdk/rust) instead of `include!`-ing `sdk/dessen.rs`. It has no
dependencies, and it gives you:

- the calls, from the same generated `sdk/dessen.rs`;
- every `DSN_*` constant from `sdk/dessen_abi.h` (events, colours, fonts,
  errors), generated at build time, so nothing is copied by hand;
- with the default `rt` feature, the image header, `_start` and a panic
  handler (the Rust counterpart of `crt0.S`; a panic exits with
  `dessen::PANIC_EXIT_CODE`, `0x70`), plus the link script;
- with the `heap` feature, `dessen::heap!(SIZE)`: an allocator over a static
  array, for apps that use `alloc`. The heap counts against the app's memory.

```toml
# Cargo.toml
[dependencies]
dessen = { path = "../dessen/sdk/rust", features = ["heap"] }
```

```toml
# .cargo/config.toml
[build]
target = "riscv32im-unknown-none-elf"
[target.riscv32im-unknown-none-elf]
rustflags = ["-C", "link-arg=-Tdessen.ld"]   # add "-C", "link-arg=--defsym=APP_MEM=131072" for 128 KB,
                                             # and "-C", "link-arg=--defsym=APP_STACK=8192" to ask only
                                             # for code + data + heap + that stack
```

Or size the stack in the app itself, next to its heap: `dessen::stack!(4096)`
makes the header ask for code, data and heap plus exactly that much stack, and
wins over `APP_STACK`. A stack that runs out writes over the heap below it with
no fault to say so, so size it from a measured peak with room to spare.

`cargo build --release` then links with Rust's own `rust-lld`; no gcc or
`crt0.S` is involved. `objcopy -O binary` the ELF into `app.bin`.
[`sdk/rust/examples/selftest_rs.rs`](../sdk/rust/examples/selftest_rs.rs) is a
complete app, and `make rust-sdk` builds and runs it along with the heap's unit
tests.

### Apps built with xpui

[`apps/xpui/`](../apps/xpui) holds Rust versions of hello, dice, Lights Out,
Sudoku and the three home launchers (minimal, Recents and Reading Stats),
written against
[xpui](https://github.com/XPUI-Framework/xpui-framework) and drawn through its
dessen backend, `xpui-dessen` (in xpui-backends). The C apps stay; these are
parallel versions. A screen is a type with `body()` and `update()`; xpui lays
it out, draws the header, moves focus and routes taps, and the backend turns
each dessen event into the input xpui polls. Button hints are the host's: the
backend keeps `hint_height` clear and hands each action's label to
`dsn_button_hints`, and the host draws them as its own screens do. A touch
board reports no band, so it shows no hints and its content keeps that space.

```rust
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use xpui::{App, NavigationScreen, Screen, Text, View, vstack};

dessen::heap!(1024);
dessen::stack!(2560);

struct Hello;

impl Screen for Hello {
    type Message = ();
    fn title(&self) -> Option<&'static str> { Some("Hello") }
    fn body(&self) -> impl View<()> {
        NavigationScreen::new(vstack![16; Text::new("Hello from xpui")])
    }
    fn update(&mut self, _message: ()) {}
}

fn app() -> App {
    App::new(Hello)
}

xpui_dessen::main!(app); // returns when Back leaves the screen
```

**An app needs no `unsafe`.** `xpui_dessen::main!` writes the entry point the
image's `_start` calls (a plain `dessen` app uses `dessen::main!(run)` for
the same), installs the backend and runs the app, so every app in
`apps/xpui/` and the shared `kit` crate carries `#![forbid(unsafe_code)]`.
State too big for the stack and with no reason to be on the heap goes in a
`dessen::ConstStaticCell`, which hands out `&'static mut` to its value once:

```rust
static STATS: dessen::ConstStaticCell<Stats> = dessen::ConstStaticCell::new(Stats::ZERO);
let stats: &'static mut Stats = STATS.take().expect("taken once");
```

The apps build with `--cfg xpui_static_host` (in `apps/xpui/.cargo/config.toml`):
xpui then calls the backend through named functions instead of a table of
function pointers, so the linker drops every host method an app never calls
(hello keeps 24 of 43). Without it every image carried all of them, about
7 KB.

Taps on the desktop runner need a board with touch: `DESSEN_BOARD=both`
(or `touch`), since the default board has keys only and drops `t<x>,<y>;`.

`make xpui` builds all seven into `build/<name>_xpui.bin` and checks each on the
VM, the launchers with the same checks as their C and Rust versions; `make test`
and `make sd` include them. Cargo fetches xpui and its dessen backend from
git the first time, so that build needs the network.

Each app asks only for the memory it uses, which matters on an ESP32-C3: the
firmware needs the whole block in one piece of internal RAM, and on dessen the
code is RAM too, since the VM loads the whole image. Each app's `main.rs` sets
its heap with `dessen::heap!` and its stack with `dessen::stack!`, from the
peaks measured on the VM (covers at full size, portrait screens up to
1072×1448, keys and taps): the heap gets 40% or 512 bytes more, whichever is
larger, and the stack 25% or 1 KB, rounded up to 256 bytes; `make xpui-peaks`
measures them again and `make xpui-sizes` prints what each image asks for.
`APP_MEM=65536` makes the link fail for an app past the 64 KB every dessen
app must fit in.

| App | Asks for | Code + data | Heap (peak) | Stack (deepest) |
|---|---|---|---|---|
| hello | 19.5 KB | 16.2 KB | 1.0 KB (0.3 KB) | 2.2 KB (1.2 KB) |
| dice | 27.3 KB | 23.5 KB | 1.2 KB (0.6 KB) | 2.5 KB (1.4 KB) |
| lightsout | 23.7 KB | 19.9 KB | 1.2 KB (0.6 KB) | 2.5 KB (1.4 KB) |
| minimal | 25.2 KB | 20.7 KB | 1.8 KB (1.0 KB) | 2.8 KB (1.7 KB) |
| sudoku | 30.3 KB | 23.1 KB | 1.8 KB (1.0 KB) | 5.5 KB (4.3 KB) |
| home (Recents) | 41.3 KB | 27.0 KB | 11.8 KB (8.4 KB) | 2.5 KB (1.4 KB) |
| readstats | 61.4 KB | 43.5 KB | 9.2 KB (6.5 KB) | 6.0 KB (4.7 KB, importing) |

About 15 KB of every image is xpui itself, its dessen backend and the chrome
it draws; hello's own code is a few hundred bytes. What keeps the rest small:

- **No `format!`.** It links Rust's formatting machinery, 3 to 5 KB of every
  image that uses it. `kit::join!("Level ", level)` puts words and numbers
  together without it.
- **No interaction per cell.** xpui keeps 48 bytes for each one, so an
  `xpui::CellGrid` declares one for the whole board and works out the cell
  from where the tap landed.
- **No 64-bit division.** It is a library routine on RV32, over a kilobyte;
  the chrome scales its bars with a 32-bit long division instead, and the
  apps keep their own arithmetic in 32 bits.
- **Images go to the firmware as they are.** xpui's 1-bit images are already
  dessen's `DSN_BITMAP_MASK` format, so the backend passes the app's bytes
  straight to `dsn_bitmap`, with no copy.
- **One buffer per cover.** `kit::Bitmap::cover` sizes its buffer from the
  height asked for, decodes the cover in it, and cuts it down in place: the
  SDK heap shrinks a block where it is, and grows one into free space right
  after it, so neither needs room for a second copy.
- **Reading Stats keeps titles in the log.** Its tally of 64 books holds each
  title's offset in the session log, and reads back only the three it shows.

Things to know:

- **Back on the root screen ends the app.** A home launcher claims it in
  `on_key` instead, as `minimal` does; Recents opens Files with it.
- **The apps share `apps/xpui/kit`:** `kit::shelf` reads the recents
  shelf, `kit::Bitmap` draws a cover or a bundled icon (dessen's 1-bit format),
  `kit::text` and `kit::Face` place a line of text in a box in any of the
  firmware's three faces, `kit::status` builds the clock/battery title, and
  `kit::join!` builds strings.
- **Covers load before the page draws.** The C Recents shows a page, then pops
  each cover in on its own redraw; xpui only repaints after an event, so the
  xpui version fetches a page's covers first (slow only the first time the
  firmware decodes one).
- **Custom drawing is a `View`.** The puzzles draw their boards with
  `xpui::CellGrid`, which sizes square cells to the space given, paints each
  through one closure and sends `on_cell_tap(index)` for a tap;
  `kit::digit` draws seven-segment numbers.
- **Keys reach the screen first** through `on_key`, so a game can claim Up,
  Down and Confirm for its cursor. `kit::step` moves a cursor along a row with
  Left/Right, when the board sends them, and through every cell with Up/Down.
- **Clipping is real** through `dsn_set_clip`, so a `ScrollView` stays inside
  its frame. A dialog's dimmed backdrop uses `DSN_SCRIM`, and dims only where
  the firmware's renderer supports it.
- **The apps build from git:** xpui, xpui-chrome and xpui-backends'
  `dessen-support` branch. To build against local checkouts instead, copy
  `apps/xpui/.cargo/local.toml.example` to `local.toml` (git ignores it) and
  fix its paths: `.cargo/config.toml` includes it when it exists. A patch only
  replaces a branch that is on the remote, so push a branch before patching it.

Freestanding **C++** works the same way. The one hard rule: target plain `im`,
never `imc` — the VM only decodes 32-bit instructions, so a compressed
instruction faults immediately.

## Adding a capability

If your app needs something the firmware can do but no call exposes yet (a new
shape, frontlight, a file read), you add **one line** to
[`abi/freeink.api`](../abi/freeink.api) and run `python3 tools/gen_api.py`. That
regenerates both the app-side wrapper and the firmware-side dispatch. See
`AGENTS.md` for the recipe and the rules every new call must follow (clip, pin,
bounds-check — every app value is untrusted).
