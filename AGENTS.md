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
                 and freeink/dessen_dispatch.inc from the .api file
freeink/       the host binding: maps calls onto freeink::ui types (FreeInkHost)
apps/          example + test apps (.c) with menu manifests (.json)
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
   `sdk/dessen_sysnums.h`, or `freeink/dessen_dispatch.inc`. Edit
   `abi/freeink.api` and run `python3 tools/gen_api.py`.
4. **Keep it lazy.** Match CrossPoint's "unused features cost nothing" rule. The
   interpreter and binding are tiny on purpose. Add a call only when a real app
   needs it; don't speculatively mirror the whole SDK.
5. **Leave a test.** Non-trivial logic gets one runnable check (a self-checking
   app or a `test_host.cpp` assertion) wired into `make test`.

## Build & test

```sh
make test     # generate, build VM + apps, run the suite  ← run this after any change
make sd       # stage build/sd/plugins/<app>/ for copying to an SD card
make size     # ESP32-C3 flash cost of the VM and the binding
make FREEINK_SDK=/path/to/freeink-sdk test   # if the SDK isn't a sibling dir
```

The RISC-V compiler is expected at `~/.platformio/tools/toolchain-riscv32-esp/`.
Note PlatformIO sometimes removes the copy under `packages/`; the Makefile uses
the stable `tools/` path.

## Recipe: build an app

1. Write `apps/<name>.c`. Study `apps/hello.c` (minimum), `apps/dice.c`
   (drawing + random), `apps/lightsout.c` / `apps/sudoku.c` (grids, touch +
   buttons). Follow the draw → refresh → `wait_event` loop; redraw only on
   change.
2. Offer **both** a touch path and a button path (CONFIRM/BACK/PREV/NEXT) so the
   app works on every board. Decode taps with `DSN_EV_X/Y`.
3. Draw to `dsn_screen_width/height()` (the safe area); never hardcode a panel
   size. Use the firmware fonts (`DSN_FONT_*`), not your own glyphs, unless you
   deliberately want bitmap digits (see `dice.c`). For a **menu**, draw each
   entry with `dsn_list_row` (flags: `DSN_ROW_SELECTED/_CHEVRON/_DISABLED`) so it
   looks like the device's own lists — don't hand-roll a selection bar from
   `fill` + `text`. The app still owns layout, scroll and tap→index math; see
   `draw_list` in the Protected Content app.
4. Remember the limits: 64 KB fixed memory, no libc, no `malloc`, no 64-bit
   divide helper unless you add one. Hand-write small helpers like the examples.
5. Add `<name>` to `APPS` (and `SD_APPS` if it should launch on a device) in the
   `Makefile`; add `apps/<name>.json` with `title` + `description`.
6. `build/dessen build/<name>.bin` to run on the desktop (`c/b/p/n` = events).
7. Add a check and run `make test`.

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

## Recipe: expose an fui component

The binding links the real FreeInk UI library (`build/fui.o`, built with the
tiny `host/shim/Arduino.h` so it compiles off-device), so a widget call projects
the *actual* `freeink::ui` component rather than re-drawing it. `list_row`,
`button`, `checkbox`, `toggle`, `progress_bar` are done; the rest of
`components/` follows the same shape.

- **Draw-only** component (header, status-bar, metric-card, divider…): a helper
  that `clip()`s the rect, builds a `Frame<kUiMaxInteractions>` over the host's
  `interactions_` (a default `InputSnapshot` — drawing doesn't route), fills the
  component's `*Props` from the call args, and calls the component. See
  `drawProgress` in `DessenFreeInk.cpp`. Include its header in the control
  block at the top (the control headers are Arduino-free; `media/` ones aren't).
- **Interactive** component (button, list, dropdown, slider, tabs…): same, plus
  give it a nonzero `ActionId` from the app so it registers a hit. The app
  brackets its pass with `ui_begin` (clears `interactions_`) and `ui_event`
  (parks; `routeUi` maps the next tap / PREV-NEXT focus / CONFIRM to a widget
  action). Don't add app-side tap math — routing is the whole point.
- Keep the app's value (list index, slider position) in the `value` field; it
  comes back in `DSN_UI_VALUE`. Leave a `widgets.c`-style check in
  `test_host.cpp` driving both a tap and button-nav.

Only the `media/` components need an Arduino symbol (`min`) in their header;
expose those behind the shim or inline the geometry. Don't speculatively mirror
the whole catalog — add a component when an app reaches for it.

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
