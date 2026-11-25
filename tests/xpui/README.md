# Tests for the xpui apps

The safety net for changing how `apps/xpui` draws its screens, above all for
moving them to a new xpui core: the screens must come out where they were,
whatever draws them. Two layers, both part of `make test` (via `make xpui`):

| Layer | What it holds still | Where |
| --- | --- | --- |
| **Screen tests** | Every screen's picture, where each text and button is, and the screen standards, at three sizes | `*.scenarios`, `golden/`, `tools/screen_test.py` |
| **Unit tests** | The logic under the screens: the reading log, the clock, puzzles, dice, menus, the recents shelf | `unit/` |

## Testing a new xpui app (checklist)

Every xpui app ships with screen tests, and with unit tests if it has logic
that draws nothing. An app is not done until all of this is in and
`make test` passes. The same list is in `AGENTS.md` for agents.

1. **Build it into the suite.** Add the app to `XPUI_APPS` in the `Makefile`
   and to the workspace in `apps/xpui/Cargo.toml`, so `make xpui` builds
   `build/<app>_xpui.bin`.
2. **Write `tests/xpui/<app>.scenarios`.** Copy `nook.scenarios`, the
   reference, and set `app = <app>_xpui` (plus `manifest =` if the app has
   `requires`, and `board = touch` if it is touch-only).
3. **Add a walk for every screen and state** a user can reach:
   - the first screen, on each board that draws it differently
     (`board=touch`, `board=buttons`);
   - every pushed screen (mark it `pushed`) and the way back from it, by the
     back arrow (`@back`) and by Back (`b`);
   - every dialog, and each of its answers;
   - every empty, error and "needs newer firmware" state (`fs=empty`, no `fs=`);
   - each step of the main loop (a move, a press, a roll), the key path
     (`c` `n` `p`) and the touch path (`@Label`, or pixels);
   - what leaves the app: Back on the root screen, launches (`[launch ...]`).

   Give each walk an `expect="..."` naming text only that screen shows. Tap
   by label wherever there is a label. A pixel tap needs one walk per size
   and an `expect=`.
4. **Declare the standards it meets.** Run `make xpui-standards`. Put every
   standard the app passes on its `standards =` line. A new app should meet
   all three (`bar taps actions`); if it cannot yet, say why in a comment.
5. **Make the goldens.** Run `make xpui-screens`. The first run writes
   `tests/xpui/golden/<app>/*.png` and `*.trace` and fails on purpose. **Open
   every new PNG** and check it is the screen the walk is named for. Run again:
   it must pass. Commit the PNGs and traces with the app.
6. **Unit-test the logic.** If the app has files that draw nothing (rules,
   parsing, scoring, a generator), add a crate in `tests/xpui/unit/` that
   includes them with `#[path]` (copy `unit/lightsout/`), add it to
   `unit/Cargo.toml`'s members, and test the rules. Run `make xpui-unit`.
7. **Run `make test`.** Everything passes before the app is done.

## Changing or migrating an app's screens

When you change how an app draws (a new xpui core, a new component, a
refactor), its screens must come out the same:

1. Run `make xpui-screens` before you start; it must pass.
2. Make the change, then run `make xpui-screens` again.
3. For each failure, read the element list and open the diff image in
   `build/screen-diffs/<app>/`:
   - **Nothing should have moved:** fix the code until it passes. The
     element list says what moved and by how much.
   - **The change is meant** (the design asked for it): run `make xpui-bless`,
     open the changed PNGs in `tests/xpui/golden/`, and commit them in the
     same commit as the code. The reviewer then sees the picture change next
     to its cause.
4. When an app meets another screen standard after the change, add it to its
   `standards =` line.

Never edit a golden by hand, never bless without looking at the pictures,
and never raise `SCREEN_TOLERANCE` to make a failure go away: a screen that
moved more than 1 px moved.

## Screen tests

Each app has a `<app>.scenarios` file of *walks*. A walk is a list of events
that brings the app to one screen or state. Every walk runs on the desktop
runner (`build/dessen`) at 480×800, 800×480 and 1448×1072. Each run happens in
a fixed world: the clock stopped at 2025-11-24 16:00 UTC (`DESSEN_NOW`), UTC
(`TZ`), and seeded random numbers (`DESSEN_SEED`).

It is drawn through FreeInk's own framebuffer target, the one the SDK renders
with (`DisplayTarget`: real glyphs, dithered grays, icons), with real fonts
(`DESSEN_SNAPSHOT`). The screen it ends on is compared with two goldens in
`golden/<app>/`:

| Golden | Compared how | Fails when |
| --- | --- | --- |
| `<walk>@<size>.png`, the picture | Ink matched to ink within **1 px** | Any ink moved further, appeared or disappeared. A diff image goes to `build/screen-diffs/`: red is what broke, black is unchanged, blue is only in the golden, grey is only now. |
| `<walk>@<size>.trace`, the draw calls, with where each text's words land | Every **text** and **button** found again by its label | One moved or resized by more than 1 px, is missing, or is new. The report names each one: `text 'How to Play' moved +0,+2`. |

The draw calls themselves are not a test: a new framework is expected to
draw the same screen with different calls, and when only the calls changed it
says so in a note. The tolerance is `SCREEN_TOLERANCE` (1 by default; 0 makes
it exact once the migration is done).

```
walk plant sizes=480x800 expect="1 of 7 plants" : t173,291
walk help pushed expect="How to Play" : @?
```

| Event | Meaning |
| --- | --- |
| `c` `b` `p` `n` | Confirm, Back, Prev (Up), Next (Down) |
| `^` `v` `<` `>` | Swipe up, down, left, right |
| `t240,400` / `t50%,40%` | A tap, in pixels or as a share of the screen |
| `T240,400` | A long press |
| `@Label` / `@"Two Words"` | A tap on the words of the text that reads that, on the screen as it is then |
| `@@Label` | A long press on it |
| `@back` | A tap on the back arrow |
| `type:secret` | An answer to a text prompt |

Walk options:

| Option | Meaning |
| --- | --- |
| `expect="text"` | Text the final screen must show. A walk that missed its target fails as "did not reach its screen", not as a changed picture. Give it to every walk with a pixel tap. |
| `board=touch\|buttons\|both` | The board's input. |
| `fs=empty\|readstats\|sync` | A storage root: empty, the session log of `tools/readstats_fixture.py`, or a CrossPoint Sync account with its server's fixtures. |
| `sizes=480x800,...` | Run at these sizes only. |
| `pushed` | The screen is pushed, so it needs a back arrow. |
| `skip=fit,bar` | Checks not run on this walk; say why in a comment. |

Prefer `@Label` taps to coordinates: they find their target wherever it is
drawn, so the walk reaches the same screen after the layout changes. Where
nothing labelled is there (a sudoku cell, a cubby, a die's result), use pixels,
one walk per size, with `expect=`.

### When a screen fails

- **It moved and you meant it** (the new component looks better 4 px lower):
  look at the diff image and the element list, then `make xpui-bless`. Commit
  the goldens with the change, so a reviewer sees the picture change next to
  the code that made it.
- **It moved and you did not mean it**: that is the regression this is for.
  The element list says what moved and by how much.

A new walk writes its goldens on the first run and **fails**, so nobody
commits a picture they never looked at. Open the PNG, then run again.

`python3 tools/screen_test.py --only nook_xpui/help tests/xpui/nook.scenarios`
runs one walk (`--only nook_xpui` one app; `--show` prints its draw calls).

### Rules every screen keeps, and the screen standards

Every frame of every walk must:

- **`screen`:** draw nothing off the screen;
- **`fit`:** give every text a box at least its font's line tall. Shorter draws
  nothing on a device: the "blank UI" bug (`docs/writing-apps.md`), measured
  here with the real fonts.

An app's `standards = ...` line lists the [screen standards](../../docs/writing-apps.md#screen-standards)
it meets, checked on the screen each walk ends on. These are exact: the bar
starts at 8 px, not 9.

| Name | Checks |
| --- | --- |
| `bar` | The bar starts 8 px from the top and is a body line + 16 px tall; a `pushed` screen has a back arrow (three 3 px strokes) and a rule under the bar |
| `taps` | Buttons (capsules) are at least the bar's height and 8 px apart, and none is within 2 px of a rule |
| `actions` | At most three actions in the bar, at most one solid (primary) button |

Only `nook` meets them today. `make xpui-standards` lists what every other app
still breaks. It is the checklist for the migration: when an app passes, add
the standard to its `standards =` line, and from then on it cannot regress.

The checks see draw calls, so they rely on how things are drawn: a capsule is
a rect rounded by half its height, a rule is a 1 px line or fill. A badge drawn
as a capsule counts as a button.

## Unit tests

`unit/` is a host-only Cargo workspace (outside `apps/xpui`, so the riscv
default target does not apply). Each crate compiles an app's own source files
with `#[path]`, so nothing is copied and the apps are not changed. They build
against `dessen-host`, a stand-in for the dessen SDK. Its constants are the
real SDK's, and its calls answer from a `World` each test scripts: files, the
clock, the time, random numbers, recents, the shelf, the battery, and a record
of what the app asked to open.

```sh
make xpui-unit                 # or: cd tests/xpui/unit && cargo test
```

Only files that draw nothing are here; the screens cover the rest. To test
another file, add a `#[path]` module to its crate's `lib.rs` and, if it needs a
call `dessen-host` lacks, add the call there.

## In CI

`.github/workflows/test.yml` runs `make test` on every pull request, every
push to `master`, and on demand. `make test` includes the
screen tests and the unit tests (through `make xpui`) and exits non-zero on
any failure. When a screen test fails, the workflow uploads the diff images
as the `screen-diffs` artifact. Open it from the run's summary page.

The workflow runs on Ubuntu, with Ubuntu's RISC-V GCC
(`RISCV=riscv64-unknown-elf-`; on a Mac, plain `make test` uses PlatformIO's)
and with Cargo `--locked`. The goldens are made on macOS. That
is fine: FreeInk's renderer is integer-only and the pictures are compared
pixel by pixel, not byte by byte. To run CI elsewhere, it needs:

| Need | Why |
| --- | --- |
| A C++17 compiler, zlib and OpenSSL (libcrypto) | The desktop runner (`build/dessen`) |
| The FreeInk SDK checked out; pass `FREEINK_SDK=<path>` if it is not `../freeink-sdk` | The runner draws with its `DisplayTarget` and fonts |
| The RISC-V GCC (`~/.platformio/tools/toolchain-riscv32-esp/`) | The C apps and `objcopy` |
| Rust 1.97.1 with the `riscv32im-unknown-none-elf` target (rustup reads `apps/xpui/rust-toolchain.toml`) | The xpui apps, and the unit tests on the host |
| Network for the first Cargo build | xpui and xpui-backends are git dependencies |
| Python 3 (standard library only) | `tools/screen_test.py` |

```sh
make FREEINK_SDK=freeink-sdk test             # everything
make FREEINK_SDK=freeink-sdk xpui-screens     # only the screen tests (builds what they need)
make xpui-unit                                # only the unit tests
```

On failure, keep `build/screen-diffs/`: it holds a diff image for every
screen that changed. The log says what moved.

The results do not depend on the machine: the clock, the time zone and the
random numbers are fixed by the tool, and the pictures come from the SDK's own
renderer, not the OS's fonts. Two things do change them:

- **The FreeInk SDK commit.** It draws the pictures. CI checks out the
  commit in `abi/.freeink-sync`, so a `make sync-accept` that moves it must
  come with `make xpui-screens` run against the new SDK, and with the goldens
  re-blessed if the pictures moved.
- **xpui and xpui-backends.** They are git dependencies on `main`, pinned to a
  commit by the committed `apps/xpui/Cargo.lock`. A framework bump is a change
  of its own: `cd apps/xpui && cargo update -p xpui -p xpui-dessen -p xpui-chrome`,
  then `make xpui-screens`, then commit the lockfile with any re-blessed goldens
  and say which commits it moves to. CI builds `--locked`, so it fails if a
  `Cargo.toml` changes without its lockfile. A build with `apps/xpui/.cargo/local.toml`
  (local xpui checkouts) rewrites the lockfile to point at them: do not commit
  it from such a build.

A golden that is missing fails in CI as it does locally ("new golden
written"), so a walk added without its goldens cannot pass.

## What this cannot catch

- **The firmware's own fonts.** The pictures use FreeInk's bundled fonts:
  Noto Sans at 34 px for body text (a device's body size), its 18 px for small
  and the 43 px gallery font for titles. Those are real glyph metrics, so text
  that stops fitting shows, but they are not CrossPoint's typefaces. The
  bitmap fonts have no bold, so bold text draws regular, and they cover ASCII
  only.
- **The panel.** The picture is the framebuffer, before the e-paper refresh.
- **Unreached screens.** A solved puzzle (sudoku, Lights Out, nook's next
  shelf) is not walked to. Its logic is unit-tested, but not its screen.
