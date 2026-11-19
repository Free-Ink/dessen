# dessen

**Run sandboxed apps inside e-paper reader firmware — tiny RISC-V binaries
launched from the SD card, no reflashing.**

Dessen lets a [CrossPoint](https://github.com/crosspoint-reader/crosspoint-reader) /
[FreeInk](https://github.com/Free-Ink/freeink-sdk) e-paper reader run small apps
without flashing new firmware. An app is a tiny RISC-V binary that the firmware
runs inside a ~1.3 KB interpreter, sandboxed so it can only touch the screen,
buttons, and the capabilities the firmware hands it — never the flash, the
radio, or anything else. Apps draw through the reader's own renderer and fonts,
so they look native and work across every board; they launch straight from the
SD card alongside the existing plugins. The whole app-facing API is a thin,
generated projection of the FreeInk SDK, so games, tools, and utilities ship as
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
engine. It is a small virtual CPU (RISC-V RV32IM) with a sandbox, plus a set of
system calls that map onto the FreeInk SDK.

## Demo apps

| App | What it shows |
|---|---|
| [`hello`](apps/hello.c) | The minimum: draw, refresh, wait for a button |
| [`dice`](apps/dice.c) | Polyhedral dice roller (d4–d100), touch + buttons, hardware random |
| [`lightsout`](apps/lightsout.c) | A touch/button grid puzzle with generated solvable levels |
| [`sudoku`](apps/sudoku.c) | On-device puzzle generation with a unique solution |

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

Flash cost on an ESP32-C3 (`-Os`): ~1.3 KB interpreter + ~2.8 KB binding. Zero
when the firmware build doesn't enable it.

## Alternatives we evaluated

The question was how to run user-supplied "apps" on a reader without reflashing.
Flash cost was the deciding factor, because CrossPoint's rule is that an unused
feature must cost nothing, and the firmware image already fills most of its
slot. The figures below are **engine code size** (`text+data`) compiled for the
ESP32-C3 at `-Os` and measured directly (see Method), not quoted.

| Approach | App is | Engine flash | Authoring | Why not |
|---|---|---:|---|---|
| **dessen** (this repo) | compiled RV32IM `.bin` | **~1.3 KB** + ~2.8 KB binding | needs a compiler (a tool can hide it) | — chosen |
| Lua 5.4 | `.lua` text file | ~119 KB¹ (141 KB full) | edit on the SD card, no toolchain | 30× the flash; GC on a 380 KB heap |
| MicroQuickJS | `.js` text file | ~109 KB² | edit on the SD card; same language as plugin.js | 27× the flash; heavier RAM |
| QuickJS (full) | `.js` text file | ~377 KB | full modern JS | won't fit the headroom |
| wasm3 (WASM) | `.wasm` from C/Rust/Zig | ~74 KB | needs a compiler *and* a wasm toolchain | 18× the flash, still needs a compiler |
| Berry | `.be` text file | ~70 KB core³ (129 KB full) | edit on the SD card | 17×+ the flash |
| Chain-load whole firmware `.bin`s | a full firmware image | ~0 engine | build a whole firmware per app | reflash per launch; MBs per app; partition/bootloader work; no sandbox |
| Emulate an ESP32 in the ESP32 | an unmodified firmware `.bin` | n/a | — | infeasible: RAM, closed radio blobs, ~20–50× slowdown |
| Extend `device.json` (declarative) | a JSON manifest | ~0 | edit on the SD card | can't compute anything (no games, tools, logic) |

Every scripting-engine row would **also** need the same ~2.8 KB FreeInk binding
on top, plus its own interpreter heap in RAM; the compiled-bin approach needs
neither. The one thing the scripting engines win is authoring — editing a text
file straight on the card, no compiler — which is what a host-side
[authoring tool](docs/writing-apps.md) is meant to close for dessen.

<sup>¹ Lua core+parser+useful stdlibs, dropping io/os/debug/package. ² mquickjs +
dtoa + libm + cutils (Bellard's own figure is ~100 KB ROM). ³ Berry core without
the optional `*lib` modules, which are needed for a usable embed.</sup>

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
docs/          writing-apps.md
AGENTS.md      guide for building apps / capabilities with an AI assistant
```

The firmware side (the activity that runs an app, the plugin-menu hook) lives in
the CrossPoint repo, behind `-DCROSSPOINT_DESSEN_APPS`.

## Contributing

`make test` must pass. New capabilities are added to `abi/freeink.api` (one
line) and must sanitize all app input — see [AGENTS.md](AGENTS.md) for the rules
and recipes. `make sync` reports FreeInk SDK surface not yet exposed.
