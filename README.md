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
