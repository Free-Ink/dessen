# dessen

**Run sandboxed apps inside device firmware without reflashing. They are tiny
RISC-V binaries loaded from storage. The engine is device-agnostic. The first
binding runs on e-paper readers.**

Dessen lets a [CrossPoint](https://github.com/crosspoint-reader/crosspoint-reader) /
[FreeInk](https://github.com/Free-Ink/freeink-sdk) e-paper reader run small apps
without flashing new firmware. An app is a tiny RISC-V binary that the firmware
runs inside a ~1.3 KB interpreter, sandboxed so it only reaches the device
through calls the firmware hands it. It never touches the flash or the radio
directly, but the firmware can offer it the screen, buttons, storage, network,
crypto and the board sensors as curated, bounds-checked calls. Apps draw through
the reader's own renderer and fonts, so they look native and run on every board,
and they launch straight from the SD card alongside the existing plugins. You can
write them in C, Rust or Zig. An app can be a game or a tool, replace the whole
home screen, react to firmware events like reaching the end of a book, or build
reading-stats cards from the device's own session log. The app-facing API is a
thin, generated projection of the FreeInk SDK, so all of this ships as
self-contained files instead of firmware forks.

> **Status: pre-release / experimental.** The app ABI is still changing freely.
> Firmware and apps must be built from the same version. Not yet in anyone's
> hands.

## Why

A reader's firmware is a single image you flash to the chip. Adding a feature
means forking and reflashing. Dessen turns "an app" into a file on the SD card:
write it, drop it in `/plugins/`, launch it from the menu. The firmware stays
one build; the apps are independent.

It is **not** a VM that emulates an ESP32, a browser-script plugin, or a Lua
engine. It is an interpreter for the RISC-V RV32IM instruction set with a
sandbox, plus a set of system calls that map onto the FreeInk SDK. It runs real
compiled machine code the way a RISC-V chip would, so there is no bytecode of its
own and no garbage collector.

## Demo apps

| App | What it shows |
|---|---|
| [`hello`](apps/hello.c) | The minimum: draw, refresh, wait for a button |
| [`dice`](apps/dice.c) | Polyhedral dice roller (d4–d100), touch + buttons, hardware random |
| [`lightsout`](apps/lightsout.c) | A touch/button grid puzzle with generated solvable levels |
| [`sudoku`](apps/sudoku.c) | On-device puzzle generation with a unique solution |
| [`minimal`](apps/minimal.c) / [`home`](apps/home.c) | Home launchers that replace the device home screen, mixing built-in screens and other apps |
| [`readstats`](apps/readstats.rs) | Reading-stats cards (heatmap, charts) from the session log, written in Rust |

## Quick start

Needs a RISC-V GCC (PlatformIO's `toolchain-riscv32-esp` works), a C++17 host
compiler, and a [`freeink-sdk`](https://github.com/Free-Ink/freeink-sdk)
checkout next to this repo.

```sh
make test                      # build the VM + apps, run the suite
build/dessen build/dice.bin    # run an app on the desktop (keys c/b/p/n = events)
make sd                        # stage build/sd/plugins/<app>/ for an SD card
```

To run on a device, copy a staged `build/sd/plugins/<app>/` folder into
`/plugins/` on the reader's SD card, enable **Settings → Connectivity →
Plugins**, and open it from **Home → Plugins**. (The firmware must be a
CrossPoint build with dessen support enabled — see
[Running on a device](docs/writing-apps.md#getting-it-onto-a-device).)

## Write an app

Full walkthrough in **[docs/writing-apps.md](docs/writing-apps.md)**. The shape:

```c
#include "dessen.h"

int main(void) {
  int w = dsn_screen_width(), h = dsn_screen_height();
  dsn_clear();
  dsn_text_box(0, h / 2 - 20, w, 40, "Hello", 5, DSN_FONT_TITLE, DSN_ALIGN_CENTER, 1, 0);
  dsn_refresh();
  return dsn_wait_event();   // sleep until a tap or button, then exit
}
```

Apps are event-driven (draw → refresh → `wait_event`), get semantic events
(`CONFIRM/BACK/PREV/NEXT/TAP`) so one binary works on touch and button boards,
and draw inside the device's safe area so nothing lands under a bezel.

That is C, but an app is just a RISC-V binary, so Rust and Zig work too. The same
call wrappers are generated for all three. Run `make rust` or `make zig` to build
the `hello` example in each, and see [`apps/readstats.rs`](apps/readstats.rs) for
a full Rust app.

## How it works

```
app.c ──(RISC-V GCC)──▶ app.bin ──┐
                                  │  loaded from the SD card
   abi/freeink.api ──(gen_api.py)─┤
        │                         ▼
        ├─▶ sdk/*.h   (app side)  ┌─────────────────────────────┐
        └─▶ dispatch  (host side) │ firmware                    │
                                  │  ┌────────────┐   ┌────────┐│
                                  │  │ dessen VM  │──▶│ binding││──▶ FreeInk SDK
                                  │  │ (RV32IM)   │◀──│ (calls)││    (draw, input, …)
                                  │  └────────────┘   └────────┘│
                                  └─────────────────────────────┘
```

- **The VM** ([`vm/`](vm)) is a portable RV32IM interpreter. It bounds-checks
  every memory access, so an app can never reach outside its own block.
- **The call list** ([`abi/freeink.api`](abi/freeink.api)) is the single source
  of truth. [`tools/gen_api.py`](tools/gen_api.py) turns it into both the
  app-side wrappers and the firmware-side dispatch, so exposing a new capability
  is one line.
- **The binding** ([`freeink/`](freeink)) maps each call onto FreeInk SDK types,
  sanitizing every untrusted app value before it reaches the renderer.
- Apps carry their own logic; the firmware provides the device. The app API is a
  projection of the SDK, not a reimplementation.

Flash cost on an ESP32-C3 (`-Os`): ~1.3 KB interpreter + ~6.7 KB binding ≈ **8 KB**.
Most of the binding is the capability set (storage, crypto, network, board
sensors, text input), and drawing is a small fraction of it. The crypto, network
and storage *work* is not in that number. It reuses firmware code that is already
there. Zero when the firmware build doesn't enable it.

## Beyond e-paper

The e-paper reader is the first binding. It is not the whole project. Take it
away and what is left is reusable: a ~1.3 KB sandboxed RISC-V interpreter plus a
generator that turns one `.api` file into both the app-side wrappers and the
firmware-side sanitizing dispatch. The VM is portable C with no dependency but
`memcpy`. Only [`freeink/`](freeink) is device-specific.

So the same pattern fits any constrained device that wants to run untrusted
third-party apps without reflashing. A smartwatch running faces and widgets. A
mechanical keyboard loading macros instead of recompiling firmware. A synth or
pedal running user patches against an audio buffer. A home-automation hub running
local rules. A handheld running tiny games. Each new target is its own `.api`
plus a binding onto its hardware, and the core, the sandbox, and the codegen stay
the same.

The tradeoff carries over too. An app is a compiled binary, so you need a
compiler rather than a text file you edit on the device. A host-side authoring
tool is what closes that gap.

## Alternatives we evaluated

The question was how to run user-supplied "apps" on a reader without reflashing.
Flash cost was the deciding factor, because CrossPoint's rule is that an unused
feature must cost nothing, and the firmware image already fills most of its
slot. The figures below are **engine code size** (`text+data`) compiled for the
ESP32-C3 at `-Os` and measured directly (see Method), not quoted.

| Approach | App is | Engine flash | Authoring | Why not |
|---|---|---:|---|---|
| **dessen** (this repo) | compiled RV32IM `.bin` | **~8 KB** (1.3 KB VM + 6.7 KB binding) | needs a compiler (a tool can hide it) | — chosen |
| Lua 5.4 | `.lua` text file | ~119 KB¹ (141 KB full) | edit on the SD card, no toolchain | 15× the flash, GC on a 380 KB heap |
| MicroQuickJS | `.js` text file | ~109 KB² | edit on the SD card, same language as plugin.js | 14× the flash, heavier RAM |
| QuickJS (full) | `.js` text file | ~377 KB | full modern JS | won't fit the headroom |
| JerryScript | `.js` text file | ~565 KB⁴ | edit on the SD card | largest of the lot; even reduced profiles stay in the hundreds of KB |
| wasm3 (WASM) | `.wasm` from C/Rust/Zig | ~74 KB | needs a compiler *and* a wasm toolchain | 9× the flash, still needs a compiler |
| Berry | `.be` text file | ~70 KB core³ (129 KB full) | edit on the SD card | 9×+ the flash |
| zForth | Forth text file | ~5 KB⁸ | edit on the SD card | smallest engine here, but Forth is hard for most app authors; small enough to run as a dessen app instead |
| Jim Tcl | `.tcl` text file | ~89 KB core⁵ | edit on the SD card | 11× the flash before useful extensions; allocates from the shared heap with no cap |
| uLisp | Lisp text file | not measured⁶ | edit on the SD card | 72 KB fixed workspace on the C3; built on the Arduino core, so pin and Wi-Fi calls must be removed to sandbox it |
| s7 Scheme | `.scm` text file | ~1.13 MB⁷ | edit on the SD card | 140× the flash; a desktop-sized engine |
| Chain-load whole firmware `.bin`s | a full firmware image | ~0 engine | build a whole firmware per app | reflash per launch; MBs per app; partition/bootloader work; no sandbox |
| Emulate an ESP32 in the ESP32 | an unmodified firmware `.bin` | n/a | — | infeasible: RAM, closed radio blobs, ~20–50× slowdown |
| Extend `device.json` (declarative) | a JSON manifest | ~0 | edit on the SD card | can't compute anything (no games, tools, logic) |

Every scripting-engine row would **also** need the same ~6.7 KB capability
binding on top, plus its own interpreter heap in RAM, where the compiled-bin
approach needs only the binding. The one thing the scripting engines win is authoring — editing a text
file straight on the card, no compiler — which is what a host-side
[authoring tool](docs/writing-apps.md) is meant to close for dessen.

<sup>¹ Lua core+parser+useful stdlibs, dropping io/os/debug/package. ² mquickjs +
dtoa + libm + cutils (Bellard's own figure is ~100 KB ROM). ³ Berry core without
the optional `*lib` modules, which are needed for a usable embed. ⁴ JerryScript
default profile, all 200 core TUs; it is configurable downward but stays large.
⁵ Jim Tcl `jim.c` + `jim-subcmd.c` + `jim-format.c` + `utf8.c`, with UTF-8,
math functions and every `jim-*.c` extension (file, aio, regexp, clock) left
out. ⁶ uLisp is a single Arduino sketch that needs the full Arduino-ESP32 core
to compile, so it was not measured; the 72 KB is its own C3 default
(`WORKSPACESIZE` 9216 objects × 8 bytes). ⁷ s7 with the C loader and system
extras off; compiling it needed stub `utsname`/`rusage` headers and
`sigsetjmp` mapped to `setjmp`. Its default heap starts at 64,000 cells.
⁸ zForth with `int32_t` cells, chosen in the port's own `zfconf.h`. Its Linux
example uses `float` cells, which is 6.1 KB and pulls in the C3's software float
routines.</sup>

**Method:** each engine's C sources were compiled with
`riscv32-esp-elf-gcc -march=rv32imc_zicsr_zifencei -mabi=ilp32 -Os
-ffunction-sections -fdata-sections` and summed with `size` (`text+data`). These
are code size only, before link-time garbage collection, and exclude the C
library each engine pulls in. dessen's own number is reproduced by `make size`.

## Repo layout

```
vm/            the RV32IM interpreter
sdk/           what an app links: dessen.h, generated headers, crt0.S, app.ld
abi/freeink.api  the call list (source of truth); gen_api.py generates from it
freeink/       the host binding onto FreeInk SDK types
apps/          demo + test apps (.c) and their menu manifests (.json)
host/          desktop runner + a recording draw target for tests
tools/         gen_api.py (codegen), sync_check.py (SDK drift report)
docs/          writing-apps.md, proposed-capabilities.md
AGENTS.md      guide for building apps / capabilities with an AI assistant
```

The firmware side (the activity that runs an app, the plugin-menu hook) lives in
the CrossPoint repo, behind `-DCROSSPOINT_DESSEN_APPS`.

## Why "dessen"

Named after **Jason Dessen**, the physicist at the center of Blake Crouch's
*Dark Matter*. He builds a chamber — the Box — that holds a person in quantum
superposition: inside it you are every version of yourself at once, in the box
and not in the box, and nothing is real until a choice collapses it into a
single world.

A dessen app lives the same way. The `.bin` on the SD card is both an app and
not an app — inert bytes until the firmware opens the box and runs it. It
executes *inside* the firmware yet was never flashed *into* it; it is present on
the device yet separable from it; it is, at rest, a superposition of every app
the reader could become. Launching one is the observation that collapses it: of
all the versions waiting in `/plugins/`, this is the one the device is, right
now, until you leave and it returns to being just a file.

And the sandbox is the Box itself — a sealed space where an app runs in its own
world, able to see out through narrow windows (the system calls) but never to
reach out and touch the world that contains it. Whatever happens in there stays
in there. Close the lid and the device is exactly as it was.

> *"Are you happy with your life?"* — the question that opens the Box. Every app
> is a different answer.

## Contributing

`make test` must pass. New capabilities are added to `abi/freeink.api` (one
line) and must sanitize all app input — see [AGENTS.md](AGENTS.md) for the rules
and recipes. `make sync` reports FreeInk SDK surface not yet exposed.
