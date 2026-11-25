# dessen — agent guide

This file orients an AI assistant (Claude Code or similar) working in this repo.
If a human asked you to "help me build a dessen app" or "add a capability,"
start here. Read [`docs/writing-apps.md`](docs/writing-apps.md) for the full
human-facing guide; this file is the working checklist and the rules you must
not break.

## What this repo is

A sandboxed app runtime for CrossPoint / FreeInk e-paper readers. An app is a
small RV32IM binary run by a ~1.3 KB interpreter **inside** the firmware — not
flashed, not a browser script. Apps draw through the FreeInk SDK and reach the
device only through system calls.

```
vm/            the interpreter (dessen_vm.c/.h) — portable C, no deps but memcpy
sdk/           what an app links: dessen.h, generated headers, crt0.S, app.ld
abi/freeink.api  THE SINGLE SOURCE OF TRUTH for the call list
tools/gen_api.py generates sdk/dessen_calls.h, sdk/dessen_sysnums.h,
                 freeink/dessen_dispatch.inc, and the Rust/Zig app wrappers
                 (sdk/dessen.rs, sdk/dessen.zig) from the .api file
freeink/       the host binding: maps calls onto freeink::ui types (FreeInkHost)
apps/          example + test apps (.c) with menu manifests (.json)
apps/xpui/     Rust versions of the apps (and launchers) on the xpui framework (make xpui)
host/          desktop runner (main.cpp) + TraceTarget for printing/asserting
```

The firmware side (the activity that runs an app, the plugin-menu hook) lives in
the **crosspoint-reader** repo on branch `feat/dessen-apps`, behind
`-DCROSSPOINT_DESSEN_APPS`. It is not in this repo.

## Golden rules

1. **The ABI is pre-release.** There are no users yet. Renumber, rename, and
   reshape calls freely. Do **not** bump `DSN_API_VERSION` or add backward-compat
   handling until the user says apps are published. Just rebuild and note that
   firmware + apps must be rebuilt together. (Once released, `abi/freeink.api`
   numbers become append-only.)
2. **Every value from an app is untrusted.** A call's host statement must clip
   rectangles (`clip`), pin points (`pin`), bounds-check buffers (`dvm_ptr` via
   the `buf`/`out` types), and range-check enums before anything reaches the
   draw target or SDK. The sandbox stops memory escapes; *you* stop a bad
   coordinate or length from corrupting a draw. Never pass a raw app integer to
   an SDK call.
3. **Generated files are generated.** Never hand-edit `sdk/dessen_calls.h`,
   `sdk/dessen_sysnums.h`, `freeink/dessen_dispatch.inc`, `sdk/dessen.rs`, or
   `sdk/dessen.zig`. Edit `abi/freeink.api` and run `python3 tools/gen_api.py`.
4. **Keep it lazy.** Match CrossPoint's "unused features cost nothing" rule. The
   interpreter and binding are tiny on purpose. Add a call only when a real app
   needs it; don't speculatively mirror the whole SDK.
5. **Leave a test.** Non-trivial logic gets one runnable check (a self-checking
   app or a `test_host.cpp` assertion) wired into `make test`. An **xpui app**
   gets screen tests for every screen and unit tests for its logic; see
   [Recipe: test an xpui app](#recipe-test-an-xpui-app). Never bless a golden
   you have not opened, and never raise the tolerance to make a test pass.

## Build & test

```sh
make test     # generate, build VM + C and Rust apps, run the suite  ← run this after any change
make sd       # stage build/sd/plugins/<app>/ (C, Rust and xpui apps) for an SD card
make test RUST=no   # C only, when Rust (riscv32im target) or xpui-backends is absent
make size     # ESP32-C3 flash cost of the VM and the binding
zig build     # the Zig apps with Zig alone (build.zig, also read by zls); zig build run -Dapp=hello
make FREEINK_SDK=/path/to/freeink-sdk test   # if the SDK isn't a sibling dir
make xpui-screens   # the xpui screen tests only: pictures + element positions, 3 sizes
make xpui-bless     # rewrite the goldens after a deliberate change (look first!)
make xpui-standards # what each xpui app still breaks of the screen standards
make xpui-unit      # the xpui apps' host unit tests only
```

`make test` is what CI runs (`.github/workflows/test.yml`, on every pull
request); it fails on any screen, element, standard or unit test failure, and
uploads the screen diffs as an artifact. More in
[`tests/xpui/README.md`](tests/xpui/README.md#in-ci).

The RISC-V compiler is expected at `~/.platformio/tools/toolchain-riscv32-esp/`.
Note PlatformIO sometimes removes the copy under `packages/`; the Makefile uses
the stable `tools/` path.

## Recipe: build an app

1. Write `apps/<name>.c`. Study `apps/hello.c` (minimum), `apps/dice.c`
   (drawing + random), `apps/lightsout.c` / `apps/sudoku.c` (grids, touch +
   buttons). Follow the draw → refresh → `wait_event` loop; redraw only on
   change.
2. Offer **both** a touch path and a button path so the app works on every
   board: CONFIRM/BACK activate, PREV/NEXT (up/down keys) move a selection,
   swipes (`SWIPE_UP/DOWN/…`) page, taps/long-press carry `DSN_EV_X/Y`. Boards
   with keys *and* touch deliver all of them, distinctly.
3. Build every screen to the **screen standards** below (bar, content, line; a
   back arrow on every pushed screen; one primary button).
   Draw to `dsn_screen_width/height()` (the safe area); never hardcode a panel
   size. Use the firmware fonts (`DSN_FONT_*`), not your own glyphs, unless you
   deliberately want bitmap digits (see `dice.c`). Follow the layout rules in
   [`docs/writing-apps.md`](docs/writing-apps.md#laying-out-a-screen) — above all,
   **size every text box with `dsn_line_height()`**: a box shorter than the font's
   line height draws *nothing* (the #1 "blank UI" bug), and it only shows up on a
   real device, since the desktop test font is tiny.
4. Remember the limits: 64 KB fixed memory, no libc, no `malloc`, no 64-bit
   divide helper unless you add one. Hand-write small helpers like the examples.
5. Add `<name>` to `APPS` (and `SD_APPS` if it should launch on a device) in the
   `Makefile`; add `apps/<name>.json` with `title` + `description`. Add
   `"requires": ["touch"]` (or `["buttons"]`) only if the app is unusable
   without it; an app with both paths needs no `requires`.
6. `build/dessen build/<name>.bin` to run on the desktop (`c/b/p/n` = events;
   `DESSEN_BOARD=touch|buttons|both` picks the board, `DESSEN_MANIFEST` the
   manifest it checks; exit 103 = the board cannot drive the app).
7. Add a check and run `make test`.

## Recipe: test an xpui app

Every xpui app (new or changed) is covered by **screen tests**: walks that
drive it to each screen, drawn with FreeInk's real renderer and fonts at
480×800, 800×480 and 1448×1072, compared with committed PNG goldens and
element positions within 1 px. Pure logic gets **unit tests** on the host.
Full reference: [`tests/xpui/README.md`](tests/xpui/README.md).

For a **new** app:

1. Add it to `XPUI_APPS` in the `Makefile` and to `apps/xpui/Cargo.toml`.
2. Write `tests/xpui/<app>.scenarios`, copying `tests/xpui/nook.scenarios`.
   One walk per reachable screen and state: the root screen (per board if it
   differs), every pushed screen (`pushed`) and both ways back (`@back`, `b`),
   every dialog and each answer, empty/error/old-firmware states, a step of
   the main loop by keys and by touch, and leaving the app.
3. Give every walk `expect="..."`: text only its final screen shows. Tap by
   label (`@New`, `@"Keep Playing"`); a pixel tap (`t173,291`) only where
   nothing is labelled, one walk per size (`sizes=480x800`).
4. Run `make xpui-standards` and list what it passes on `standards =` (a new
   app should pass `bar taps actions`).
5. Run `make xpui-screens`. The first run writes the goldens and **fails on
   purpose**. Open every new PNG in `tests/xpui/golden/<app>/` and confirm it
   shows the screen the walk names; run again, it must pass.
6. If the app has files that draw nothing, add a crate in `tests/xpui/unit/`
   (copy `unit/lightsout/`; it `#[path]`-includes the app's files) and add it
   to the workspace members.
7. `make test`. Commit the scenarios, the goldens and the unit tests with the
   app.

For a **change** to how an app draws (a refactor, a new xpui component, a
framework migration):

1. `make xpui-screens` passes before you start.
2. After the change, each failure names what moved (`text 'How to Play' moved
   +0,+2`) and writes a diff image to `build/screen-diffs/<app>/` (red = moved
   beyond tolerance). Look at it.
3. Unintended: fix the code. Intended: tell the user what moved and why, then
   `make xpui-bless`, open the changed PNGs, and commit them with the code.
4. A walk that now fails "did not reach its screen" means its tap missed the
   new layout: re-aim the tap, don't delete the walk.

## Recipe: add a capability (one new call)

When an app needs something FreeInk/CrossPoint can do but no call exposes:

1. Pick the next free number in [`abi/freeink.api`](abi/freeink.api) (append;
   group by range: core 0–15, DrawTarget 16–31, leave room for UI/storage/net
   blocks later).
2. Add one line:
   `N name(type arg, ...) [-> i32|u32] : <host C++ statement>`
   - Types: `i32 u32` (one register each); `i16 u16 u8` (packed, neighbours
     share a register); `str` (NUL-terminated host copy); `buf` (read-only
     bytes); `out` (bytes the host writes back). Max 7 registers per call.
   - The statement sees args as locals plus `target`, `screen` (a `View` =
     safe-area size+origin), `self` (the `FreeInkHost`), and `vm`. Set `result`
     to return.
   - **Sanitize**: `clip(x,y,w,h,screen)` for rects, `pin(x,y,screen)` for
     points, range-check every enum/flag, and reject (`result = -1`) rather than
     trust. Look at calls 17–25 for the pattern.
3. If the SDK side needs a helper (like `drawText`/`drawBitmap`/`writeDeviceInfo`),
   add it to the anonymous namespace in `freeink/DessenFreeInk.cpp`.
4. Add matching constants to `sdk/dessen_abi.h` if the call takes enums/flags.
5. `python3 tools/gen_api.py` (or just `make test`, which runs it).
6. Use it from a test app and assert on it in `freeink/test_host.cpp` via the
   recording `TraceTarget`.
7. The firmware side needs no change for a new drawing call — it already passes
   its `DrawTarget`. A capability backed by a *service* (frontlight, files,
   network) is different: it needs firmware wiring and, for anything that
   blocks (TLS, large reads), a "call-in-progress / resume" state that does not
   exist yet. Flag that to the user before starting — don't invent a blocking
   call inside the render task.

## Screen standards

Every screen you build follows these. The full version, with the reasons and a
checklist, is [`docs/writing-apps.md`](docs/writing-apps.md#screen-standards);
[`apps/xpui/nook`](apps/xpui/nook) is the reference implementation.

1. **Bar, content, line.** A bar across the top (one line of body text + 16 px
   high), the content, and one line under it; all inside one margin. No host
   title band when the bar says where you are.
2. **The bar says where you are and what you can do.** Root screen: a bold
   label and detail on the left. Pushed screen: a **back arrow** (three 3 px
   strokes, not xpui's thin chevron), its title, and a rule under the bar.
   Both start 8 px from the top; do not use xpui's taller `NavigationScreen`
   header. Actions on the right as outlined capsules, at most three, `?` last.
3. **Navigation is a stack.** A second page is a pushed screen
   (`xpui::present`), never a flag that swaps a body. Its back arrow pops it
   (`xpui::finish_screen`) and so does the system Back. Never "tap anywhere to
   go back".
4. **One primary button.** Solid ink capsule for the screen's one next step,
   outlined for everything else. Ask before destroying work, with the host's
   dialog, and only when there is work to lose.
5. **Tap targets are at least the bar's height**, 8 px apart.
6. **Words.** Firmware fonts; Title Case for titles, headings, buttons and
   dialog choices (`Next Shelf`), sentence case for sentences; a one-line
   label is at most about 36 characters;
   the line under the content says the most useful next thing for the current
   state, not the rules. Rules go on a help screen behind `?`.
7. **State is shown on the thing itself** (outlined = to do, solid = done,
   grey = wrong, a badge on what breaks a rule), not described in a sentence.
8. **Everything is measured** from the screen size and `dsn_line_height`;
   it must hold in portrait and landscape from 480 px to 1448 px.
9. **E-paper.** Redraw only on change; the same state draws the same picture;
   no animation.
10. **Look at it.** Check each screen in the simulator before calling it done,
    and leave a screen-test walk for every screen
    ([Recipe: test an xpui app](#recipe-test-an-xpui-app)).
    Game graphics follow [`docs/drawing-icons.md`](docs/drawing-icons.md).

## When NOT to proceed without asking

- Exposing **network, filesystem, or Bluetooth** — these need a firmware-side
  async mechanism and a permission model that aren't built yet.
- Anything that would let an app read outside its block or call an SDK method
  with an unchecked app value.
- Raising the 64 KB memory cap (fine on PSRAM boards, needs a per-app size
  mechanism and a board-aware cap).
- Touching the crosspoint-reader repo — that's a separate branch and build; say
  so and keep changes there isolated.

## Useful references

- [`docs/writing-apps.md`](docs/writing-apps.md) — the human guide, API table,
  constraints.
- [`docs/drawing-icons.md`](docs/drawing-icons.md) — how to draw e-paper
  graphics for a mini-game in code: pieces, boards and state marks that are
  legible, cheap to draw and kind to the panel. Read it before drawing any.
- `abi/freeink.api` — the whole call list, with the format spec at the top.
- `freeink/DessenFreeInk.cpp` — the sanitizers (`clip`, `pin`, `guestStr`,
  `drawText`, `drawBitmap`) every call relies on.
- FreeInk SDK: `freeink-sdk/libs/ui/FreeInkUI/include/` — `DrawTarget`,
  `DeviceContext`, `TextStyle`, the component library you're projecting from.

## Staying in sync with the FreeInk SDK

`make sync` reports FreeInk surface dessen does not yet expose, and whether the
SDK commit moved since the last review. It is a **report, not a generator** —
exposing a method still means writing the sanitized call by hand (rule 2), so
this cannot be fully automated. What it gives you:

- **New un-exposed methods** on a tracked SDK class (currently `DrawTarget`) —
  candidates to wrap via the recipe above.
- **Stale calls** — a `.api` statement calling a method the SDK renamed or
  removed, which would break the build.
- **Pin drift** — the SDK commit changed since `abi/.freeink-sync`.

Deliberately-skipped methods go in `abi/.sync-ignore` (with a reason) so the
report stays quiet until something genuinely new appears. After reviewing new
surface, `make sync-accept` records the current SDK commit.

To run it automatically, call `make sync` from CI, or add it to the firmware
repo's SDK-bump workflow — a submodule update to freeink-sdk is exactly when new
components appear. Wiring a local git hook is left to the user (don't install
one without asking). Widen coverage by adding classes to `SURFACES` in
`tools/sync_check.py` (e.g. a frontlight or storage manager once those are
exposed).
