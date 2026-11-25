---
name: dessen-app
description: >-
  Author a dessen app — a small sandboxed program (C, Rust, or Zig) that runs
  inside CrossPoint / FreeInk e-paper reader firmware via the ~1.3 KB RV32IM
  interpreter in this repo. Use when someone wants to write, build, or debug a
  dessen app, add a new capability/syscall, make a custom home launcher, react
  to firmware events (reader.open/exit/end, book.downloaded, sleep.enter), read
  reading-stats from the session log, or draw a UI on the e-reader. Covers the
  app contract, the full API surface, the draw→refresh→wait_event loop, home
  launchers, events, reading stats, other languages, and `make test`.
---

# Authoring a dessen app

A dessen app is a freestanding **RV32IM** binary (no compressed/`C` ext, no
float, ≤64 KB flat memory) that the firmware runs in-process. It draws through
the FreeInk SDK and reaches the device only through system calls. Not flashed,
not a browser script.

**Read first:** [`AGENTS.md`](../../AGENTS.md) is the rules you must not break
(the ABI is pre-release; sanitize every app value; generated files are
generated; keep it lazy; leave a test). [`docs/writing-apps.md`](../../docs/writing-apps.md)
is the full human guide. This skill is the working map.

## Decide what you're doing

- **A normal app** (game, tool, viewer) → Recipe below. Study `apps/hello.c`
  (minimum), `apps/dice.c` (drawing + random), `apps/lightsout.c` / `apps/sudoku.c`
  (grids, touch + buttons).
- **A home launcher** (replaces the device home screen) → mark the manifest
  `"home": true`; study `apps/minimal.c` (declarative `MENU` of
  `BUILTIN(screen)` / `APP("/plugins/x/app.bin")` rows) and `apps/home.c`
  (book-grid launcher). Launch built-in screens with `dsn_open_home(DSN_HOME_*)`,
  other apps with `dsn_open_app(path)`, books with `dsn_open_book(path)`.
- **React to a firmware event** (sync on reader.exit, act at end of book, custom
  end-of-book screen) → see *Firmware events* below and `docs/writing-apps.md#reacting-to-firmware-events`.
- **Reading stats / anything reading the session log** → see *Reading stats* below.
- **Expose something the SDK can do but no call does** → *Add a capability* below.
- **In Rust or Zig** → the wrappers are generated too; see *Other languages*.

## Recipe: build an app

1. Write `apps/<name>.c`. The shape of every app:
   ```c
   #include "dessen.h"
   int main(void) {
     for (;;) {
       dsn_clear();
       /* draw with dsn_fill / dsn_text_box / dsn_bitmap / ... */
       dsn_refresh();
       int e = dsn_wait_event();           // blocks
       int t = DSN_EV_TYPE(e);             // DSN_EV_TAP/CONFIRM/BACK/PREV/NEXT/SWIPE_*
       if (t == DSN_EV_BACK) return 0;     // exit code = DSN_EV_* or your own
       /* DSN_EV_X(e)/DSN_EV_Y(e) carry tap coords */
     }
   }
   ```
   Redraw only on change. Offer **both** a touch path and a button path
   (CONFIRM/BACK activate, PREV/NEXT move a selection, swipes page) so it works
   on every board.
2. Draw to `dsn_screen_width/height()` (the safe area — bezels/corners already
   excluded); never hardcode a panel size. Use the firmware fonts
   (`DSN_FONT_SMALL/BODY/TITLE`). **Size every text box with `dsn_line_height()`**:
   a box shorter than the line height draws *nothing* — the #1 "blank UI" bug,
   and it only shows on a real device (the desktop test font is tiny).
3. Limits: 64 KB fixed memory, no libc, no `malloc`, no 64-bit divide helper.
   Hand-write small helpers like the examples.
4. Add `<name>` to `APPS` (and `SD_APPS` to launch it on a device) in the
   `Makefile`; add `apps/<name>.json` with `title` + `description` (+ `"home": true`
   for a launcher; + `"requires": ["touch"]` or `["buttons"]` only if the app is
   unusable without it).
5. `build/dessen build/<name>.bin` runs it on the desktop (`c/b/p/n` = events;
   `DESSEN_BOARD=touch|buttons|both`, `DESSEN_MANIFEST=apps/<name>.json`).
6. Add a self-check and run `make test`.

## Building screens

Every screen follows the repo's screen standards. The full text is
[`docs/writing-apps.md#screen-standards`](../../../docs/writing-apps.md#screen-standards)
and the reference implementation is `apps/xpui/nook` (Book Nook). In short:

- **Bar, content, line**, inside one margin. The bar is one line of body text
  + 16 px high. No host title band when the bar says where you are.
- **The bar:** on the root screen a bold label and detail at the left; on a
  pushed screen a **back arrow** (three 3 px strokes, not xpui's thin
  chevron), its title, and a rule under the bar (`Bare::pushed` in
  `apps/xpui/nook/src/view.rs`). Both start 8 px from the top; do not use
  xpui's taller `NavigationScreen` header. Actions at the right as
  outlined capsules: at most three, `?` (help) last.
- **Navigation is a stack:** a second page is a pushed screen
  (`xpui::present`), not a flag swapping the body. The back arrow pops it
  (`xpui::finish_screen`), and so does the system Back. Never "tap anywhere to
  go back".
- **One primary button** (solid ink) per screen; the rest outlined. Ask before
  destroying work, with `Modal::confirm`, only when there is work to lose.
- **Tap targets** at least the bar's height, 8 px apart.
- **Words:** firmware fonts; Title Case for titles, headings, buttons and
  dialog choices (`Next Shelf`), sentence case for sentences; one-line labels
  of at most about 36 characters;
  the line under the content gives the most useful next step for the current
  state. Rules and explanations go on a help screen behind `?`.
- **State on the thing itself:** outlined = to do, solid = done, grey = wrong,
  a badge on whatever breaks a rule.
- **Measured layout:** sizes from the screen and `dsn_line_height`, good in
  portrait and landscape from 480 px to 1448 px.
- **E-paper:** redraw only on change, same state same picture, no animation.
- **Game graphics** (pieces, boards, state marks): follow
  [`docs/drawing-icons.md`](../../../docs/drawing-icons.md). Give every
  graphic a role (piece, fixture, structure, mark); solid ink shapes layered
  back to front with a paper edge; one grey for material and shadow; sizes
  measured from the box; tens of draw calls per piece.
- **Done means looked at:** render each screen in the simulator, and leave a
  screen-test walk for every screen (below).

## Testing an xpui app

An xpui app is not done without screen tests and, for its logic, unit tests.
The recipe is in [`AGENTS.md`](../../../AGENTS.md#recipe-test-an-xpui-app) and the
reference in [`tests/xpui/README.md`](../../../tests/xpui/README.md). In short:

- `tests/xpui/<app>.scenarios` (copy `nook.scenarios`): a walk per screen and
  state, each with `expect="..."`, tapping by label (`@Label`) where it can.
- `make xpui-screens` writes the goldens on the first run and fails; open every
  PNG, then run again. Commit the goldens with the app.
- When a screen fails after a change: read what moved, look at
  `build/screen-diffs/`, fix it or (if meant) `make xpui-bless APP=<app>`
  and say so. Change and bless one app at a time.
  Never bless unseen; never raise `SCREEN_TOLERANCE`.
- Logic that draws nothing: a crate in `tests/xpui/unit/` (copy `lightsout/`).

## The API surface

Every call is a thin wrapper in `sdk/dessen_calls.h` (generated) over one
`ecall`; constants are in `sdk/dessen_abi.h`. The one source of truth is
[`abi/freeink.api`](../../abi/freeink.api); the per-call host behaviour and
intent are in [`docs/proposed-capabilities.md`](../../docs/proposed-capabilities.md).
Full grouped list in `docs/writing-apps.md` under **The API** → **Full reference**.
Groups an app can use:

- **Core:** `dsn_screen_width/height`, `dsn_device`, `dsn_wait_event`, `dsn_exit`,
  `dsn_random`, `dsn_time`, `dsn_clock`, `dsn_device_id`, `dsn_random_bytes`,
  `dsn_locale` (BCP47 UI language, e.g. `"en"`).
- **Draw:** `dsn_clear`, `dsn_fill` / `dsn_fill_round`, `dsn_stroke`, `dsn_line`,
  `dsn_triangle`, `dsn_text` / `dsn_text_box`, `dsn_measure_text`,
  `dsn_line_height`, `dsn_bitmap`.
- **Text input:** `dsn_text_input` (the firmware runs its own on-screen keyboard).
- **Storage** (scoped to the SD card): `dsn_fs_read` / `dsn_fs_read_at`
  (ranged, for files bigger than app memory) / `dsn_fs_write` / `dsn_fs_list` /
  `dsn_fs_delete` / `dsn_fs_app_dir` / `dsn_keystore`.
- **Network** (async): `dsn_net_request`, `dsn_fetch_to_sd`.
- **Crypto:** `dsn_hash`, `dsn_aes_cbc`, `dsn_rsa_keygen/encrypt/decrypt/sign`,
  `dsn_pkcs12_parse`.
- **Board:** `dsn_battery` (percent + power state in the 2nd byte, `DSN_PWR_*`),
  `dsn_backlight` / `dsn_backlight_get`, `dsn_haptic`, `dsn_accel`.
- **Launcher / home:** `dsn_open_book`, `dsn_open_app`, `dsn_open_settings`,
  `dsn_open_browser`, `dsn_open_home(DSN_HOME_*)`, `dsn_recents`,
  `dsn_recent_shelf`, `dsn_book_progress`, `dsn_book_cover`, `dsn_wifi_connect`.

Calls that can fail (`text_box`, `bitmap`, storage, …) return `-1`/`DSN_ERR_*`;
check it. The host silently refuses out-of-range values, so a bug draws nothing
rather than corrupting the device.

## Reading stats

The firmware keeps an append-only log of measured reading sessions at
`/.crosspoint/reading_sessions.jsonl` — **one JSON object per line**:

| key | meaning |
|---|---|
| `doc` | 32-hex sync document id |
| `start` | Unix time of the first counted page |
| `secs` | active reading seconds |
| `tz` | UTC offset in minutes at the start |
| `from` / `to` | whole-book progress in basis points (0..10000) |

The log is never trimmed, so **stream it with `dsn_fs_read_at(path, offset, buf,
len)`** a chunk at a time — do not load it whole (it outgrows the 64 KB app
memory). Parse line by line; a plain substring search for `"key":` is sound
because quotes inside strings are escaped. Aggregate into per-day seconds (a
heatmap), per-hour, streaks, pages, etc. The full worked example is
[`apps/readstats.rs`](../../apps/readstats.rs) (a Rust home launcher that renders
heatmap + bar-chart cards). The schema is owned by crosspoint
`src/sync/ReadingSessionLog.h`.

## Firmware events

A plugin's `device.json` `"events"` section subscribes to firmware events
(`reader.open/exit/session/end`, `book.downloaded`, `sleep.enter`). A handler is
a `request` (HTTPS), a `download`, or an `app` (`"mode": "read"` queues to
`app-events.jsonl` for the app to read on its next run; `"mode": "launch"` opens
it now). A `reader.end` handler with `"replaces": "end of book"` opens the app
full-screen in place of the firmware's end-of-book menu. Details + the `vars`
each event carries: `docs/writing-apps.md#reacting-to-firmware-events`.

## Other languages

Nothing is C-specific — an app is just an RV32IM image reached through `ecall`.
`make rust` / `make zig` build `apps/hello.rs` / `apps/hello.zig` against the
**generated** wrappers `sdk/dessen.rs` / `sdk/dessen.zig` (same crt0.S + app.ld
as C). Target plain `im`, never `imc` — a compressed instruction faults the VM.
`readstats.rs` is a full Rust example.

## Add a capability (one new call)

Only when a real app needs something no call exposes. Append one line to
`abi/freeink.api`, add any enum to `sdk/dessen_abi.h`, run `python3 tools/gen_api.py`
(or `make test`), and **sanitize every app value** in the host statement (`clip`
rects, `pin` points, range-check enums, bounds-check buffers via `buf`/`out`).
Full recipe + the "when NOT to proceed without asking" list (network, filesystem,
Bluetooth, raising the 64 KB cap) are in `AGENTS.md`. A service-backed call
(frontlight, files, net) needs firmware wiring in crosspoint-reader — flag it.

## Build & test

```sh
make test     # generate, build VM + apps, run the suite  ← after any change
make sd       # stage build/sd/plugins/<app>/ for an SD card
make rust     # build + run the Rust example (needs the Rust toolchain)
make zig      # build + run the Zig example (needs Zig)
make size     # ESP32-C3 flash cost of the VM + binding
make xpui-screens   # xpui screen tests (pictures + positions, 3 sizes)
make xpui-bless APP=nook   # accept one app's deliberate screen changes, after looking
make xpui-unit      # xpui host unit tests
```
