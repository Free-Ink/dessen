# Any RV32 GCC works. PlatformIO keeps its copy under tools/ permanently; the
# packages/ link comes and goes depending on which board was built last.
RISCV ?= $(HOME)/.platformio/tools/toolchain-riscv32-esp/bin/riscv32-esp-elf-
# ponytail: no libgcc/libc linked, so 64-bit division and memcpy-style builtins
# will fail to link. Add a small runtime to sdk/ when an app first needs one.
APPFLAGS = -march=rv32im -mabi=ilp32 -Os -ffreestanding -nostdlib -mno-relax \
           -fno-tree-loop-distribute-patterns -Wall -Isdk -T sdk/app.ld -Wl,--no-warn-rwx-segments
APPS = hello selftest oob bench drawtest lightsout dice sudoku captest nettest asynctest home minimal
SD_APPS = hello lightsout dice sudoku home minimal

# Rust is part of `make test` and `make sd`: the Rust SDK, the hand-linked Rust
# apps (hello_rs, readstats) and the xpui versions of the C apps (apps/xpui) are
# built, checked and staged alongside the C ones. It needs the riscv32im target
# (`make rust-check` says how to add it); the xpui apps fetch xpui and its
# dessen backend from git. RUST=no leaves Rust out.
# Zig joins `make sd` when its toolchain is installed.
RUST ?= yes
HAVE_RUST := $(shell rustup target list --installed 2>/dev/null | grep -qx riscv32im-unknown-none-elf && echo y)
HAVE_ZIG := $(shell command -v zig >/dev/null 2>&1 && echo y)
XPUI_APPS = hello dice lightsout sudoku nook minimal home readstats
WITH_RUST := $(filter yes,$(RUST))
SD_LANG_APPS ?= $(if $(WITH_RUST),hello_rs readstats $(XPUI_APPS:%=%_xpui)) $(if $(HAVE_ZIG),hello_zig)

# FreeInk SDK checkout, used read-only for its UI headers.
FREEINK_SDK ?= ../freeink-sdk

# OpenSSL (libcrypto) backs the DESKTOP crypto service only — it is a test tool,
# not firmware. Found via Homebrew or pkg-config.
OPENSSL_PREFIX ?= $(shell brew --prefix openssl@3 2>/dev/null)
ifneq ($(OPENSSL_PREFIX),)
  OPENSSL_CFLAGS := -I$(OPENSSL_PREFIX)/include
  OPENSSL_LIBS := -L$(OPENSSL_PREFIX)/lib -lcrypto
else
  OPENSSL_CFLAGS := $(shell pkg-config --cflags libcrypto 2>/dev/null)
  OPENSSL_LIBS := $(shell pkg-config --libs libcrypto 2>/dev/null || echo -lcrypto)
endif

GEN = sdk/dessen_sysnums.h sdk/dessen_calls.h freeink/dessen_dispatch.inc
HOSTFLAGS = -std=c++17 -O2 -Wall -Wextra -Ivm -Isdk -Ifreeink -Ihost $(OPENSSL_CFLAGS) -I$(FREEINK_SDK)/libs/ui/FreeInkUI/include
# The desktop runner's snapshots (host/Snapshot.h) draw with FreeInk's
# framebuffer target and its gallery fonts, and write PNGs with zlib.
SNAPFLAGS = -I$(FREEINK_SDK)/libs/ui/FreeInkUI/tools
BINDING = sdk/dessen_abi.h freeink/DessenFreeInk.cpp freeink/DessenFreeInk.h freeink/DessenServices.h host/DesktopServices.cpp host/DesktopServices.h host/TraceTarget.h host/Manifest.h vm/dessen_vm.h $(GEN)

all: build/dessen build/test_freeink $(APPS:%=build/%.bin)

# The whole call list, both sides, comes from one description file.
sdk/dessen_calls.h: abi/freeink.api tools/gen_api.py
	python3 tools/gen_api.py
ifneq ($(filter-out $(wildcard $(GEN)),$(GEN)),)
.PHONY: missing-generated
sdk/dessen_calls.h: missing-generated
endif
sdk/dessen_sysnums.h freeink/dessen_dispatch.inc: sdk/dessen_calls.h

build/dessen_vm.o: vm/dessen_vm.c vm/dessen_vm.h sdk/dessen_abi.h $(GEN)
	@mkdir -p build
	cc -O2 -Wall -Wextra -Ivm -Isdk -c vm/dessen_vm.c -o $@

build/dessen: host/main.cpp host/Snapshot.h build/dessen_vm.o $(BINDING)
	c++ $(HOSTFLAGS) $(SNAPFLAGS) host/main.cpp freeink/DessenFreeInk.cpp host/DesktopServices.cpp \
	    $(FREEINK_SDK)/libs/ui/FreeInkUI/src/FreeInkUI.cpp build/dessen_vm.o $(OPENSSL_LIBS) -lz -o $@

build/test_freeink: freeink/test_host.cpp build/dessen_vm.o $(BINDING)
	c++ $(HOSTFLAGS) freeink/test_host.cpp freeink/DessenFreeInk.cpp host/DesktopServices.cpp build/dessen_vm.o $(OPENSSL_LIBS) -o $@

build/home.bin: apps/home_icons.h

build/%.bin: apps/%.c sdk/crt0.S sdk/rt.c sdk/app.ld sdk/dessen.h sdk/dessen_abi.h $(GEN)
	@mkdir -p build
	$(RISCV)gcc $(APPFLAGS) sdk/crt0.S sdk/rt.c $< -o build/$*.elf
	$(RISCV)objcopy -O binary build/$*.elf $@

# Exit codes: selftest 0 = all checks pass; hello returns the event it got
# (c = confirm = 1); oob must fault (101 = memory fault).
test: $(if $(WITH_RUST),rust-check) all $(if $(WITH_RUST),rust xpui)
	build/dessen build/selftest.bin
	printf c | build/dessen build/hello.bin; test $$? -eq 1
	build/dessen build/oob.bin; test $$? -eq 101
	build/dessen build/bench.bin
	printf ncb | build/dessen build/lightsout.bin | grep -q 'Moves 1'
	printf cb | build/dessen build/dice.bin | grep -q 'Last:  d20='
	printf b | build/dessen build/sudoku.bin | grep -q 'cells left'
	printf c | build/dessen build/home.bin | grep -q 'launch reader /books/dune.epub'
	printf nc | build/dessen build/home.bin | grep -q 'launch reader /books/hyperion.epub'
	printf nnc | build/dessen build/home.bin | grep -q 'launch reader /books/neuromancer.epub'
	build/dessen build/home.bin </dev/null | grep -q '37%'
	printf nnnc | build/dessen build/minimal.bin | grep -q 'launch app /plugins/dice/app.bin'
	@for m in apps/*.json; do python3 -c 'import json,sys; r=json.load(open(sys.argv[1])).get("requires",[]); \
	  sys.exit(0 if isinstance(r,list) and set(r)<={"touch","buttons"} else sys.argv[1]+": requires is a list of touch/buttons")' $$m || exit 1; done
	@tmp=$$(mktemp -d); printf '{"title":"T","requires":["touch"]}' > $$tmp/m.json; \
	  DESSEN_MANIFEST=$$tmp/m.json build/dessen build/hello.bin </dev/null 2>/dev/null; a=$$?; \
	  printf c | DESSEN_BOARD=both DESSEN_MANIFEST=$$tmp/m.json build/dessen build/hello.bin >/dev/null 2>&1; b=$$?; \
	  mkdir $$tmp/sd && cp build/hello.bin $$tmp/sd/app.bin && printf '{"requires":["buttons"]}' > $$tmp/sd/manifest.json; \
	  DESSEN_BOARD=touch build/dessen $$tmp/sd/app.bin </dev/null 2>/dev/null; c=$$?; rm -rf $$tmp; \
	  test $$a -eq 103 -a $$b -eq 1 -a $$c -eq 103 && echo 'manifest requires: PASS'
	@tmp=$$(mktemp -d); DESSEN_FS_ROOT=$$tmp build/dessen build/captest.bin; rc=$$?; rm -rf $$tmp; test $$rc -eq 0 && echo 'captest: PASS'
	@nr=$$(mktemp -d); fs=$$(mktemp -d); mkdir -p $$nr/example.test; \
	  printf 'hello world' > $$nr/example.test/hello; \
	  printf 'nope' > $$nr/example.test/missing; printf '404' > $$nr/example.test/missing.status; \
	  printf 'the book bytes here' > $$nr/example.test/book.bin; \
	  DESSEN_NET_ROOT=$$nr DESSEN_FS_ROOT=$$fs build/dessen build/nettest.bin; rc=$$?; \
	  rm -rf $$nr $$fs; test $$rc -eq 0 && echo 'nettest: PASS'
	build/test_freeink build/hello.bin build/oob.bin build/lightsout.bin build/dice.bin build/sudoku.bin build/drawtest.bin build/asynctest.bin build/home.bin build/captest.bin
	@echo PASS

# Fails early, with the fix, when `test`/`sd` would need Rust that is not here.
rust-check:
	@test -n "$(HAVE_RUST)" || { echo "Rust with the riscv32im target is needed (or pass RUST=no):"; \
	  echo "  rustup target add riscv32im-unknown-none-elf"; exit 1; }

# Plugin folders ready to copy into /plugins on the SD card.
sd: $(if $(WITH_RUST),rust-check) all $(SD_LANG_APPS:%=build/%.bin)
	@for a in $(SD_APPS) $(SD_LANG_APPS); do mkdir -p build/sd/plugins/$$a && cp build/$$a.bin build/sd/plugins/$$a/app.bin \
	  && cp apps/$$a.json build/sd/plugins/$$a/manifest.json; done; ls build/sd/plugins

# What each xpui app costs in code, data and memory (flags any over 64 KB), and
# the heap and stack peaks its heap!/stack! are sized from.
xpui-sizes: xpui-build $(XPUI_APPS:%=build/%_xpui.bin)
	@tools/sizes.sh
xpui-peaks: xpui-build $(XPUI_APPS:%=build/%_xpui.bin) build/dessen
	@tools/peaks.sh

# Report FreeInk SDK surface that dessen does not yet expose, and SDK pin drift.
# Does not change code — exposing a method is a judgment call (see AGENTS.md).
sync:
	python3 tools/sync_check.py --freeink-sdk $(FREEINK_SDK)

# Record the current SDK commit as reviewed, after looking at `make sync`.
sync-accept:
	git -C $(FREEINK_SDK) rev-parse HEAD > abi/.freeink-sync
	@echo "pinned $(FREEINK_SDK) @ $$(cat abi/.freeink-sync | cut -c1-10)"

# Flash cost of the interpreter on an ESP32-C3, per crosspoint's size rule.
size: $(GEN)
	@mkdir -p build
	$(RISCV)gcc -march=rv32imc_zicsr_zifencei -mabi=ilp32 -Os -Ivm -Isdk -c vm/dessen_vm.c -o build/dessen_vm.c3.o
	$(RISCV)g++ -std=gnu++2a -march=rv32imc_zicsr_zifencei -mabi=ilp32 -Os -ffunction-sections -fdata-sections -fno-exceptions -fno-rtti \
	    -Ivm -Isdk -Ifreeink -I$(FREEINK_SDK)/libs/ui/FreeInkUI/include -c freeink/DessenFreeInk.cpp -o build/binding.c3.o
	$(RISCV)size build/dessen_vm.c3.o build/binding.c3.o

# A dessen app written in Rust, to show the runtime is language-agnostic: it
# links against the same crt0.S + app.ld as the C apps, nothing else changes.
# Part of `test` (see RUST above); on its own: make rust
#   rustup target add riscv32im-unknown-none-elf   # one-time
build/hello_rs.bin: apps/hello.rs sdk/dessen.rs sdk/crt0.S sdk/app.ld sdk/dessen_abi.h
	@mkdir -p build
	rustc --target riscv32im-unknown-none-elf --crate-type staticlib \
	    -C panic=abort -C opt-level=s -o build/libhello_rs.a $<
	$(RISCV)gcc -march=rv32im -mabi=ilp32 -nostdlib -mno-relax -Isdk -ffunction-sections \
	    -T sdk/app.ld sdk/crt0.S build/libhello_rs.a -o build/hello_rs.elf \
	    -Wl,--gc-sections,--no-warn-rwx-segments
	$(RISCV)objcopy -O binary build/hello_rs.elf $@

# Any other Rust app: apps/<name>.rs -> build/<name>.bin (same recipe as above).
build/readstats.bin: apps/readstats_icons.rs

build/%.bin: apps/%.rs sdk/crt0.S sdk/app.ld sdk/dessen_abi.h $(GEN)
	@mkdir -p build
	rustc --target riscv32im-unknown-none-elf --crate-type staticlib \
	    -C panic=abort -C opt-level=s -o build/lib$*.a $<
	$(RISCV)gcc -march=rv32im -mabi=ilp32 -nostdlib -mno-relax -Isdk -ffunction-sections \
	    -T sdk/app.ld sdk/crt0.S build/lib$*.a -o build/$*.elf \
	    -Wl,--gc-sections,--no-warn-rwx-segments
	$(RISCV)objcopy -O binary build/$*.elf $@

# The Rust SDK crate (sdk/rust): its heap's unit tests on the host, then its
# self-checking example on the VM. Pure cargo: rust-lld links with sdk/app.ld,
# so no crt0.S or gcc step; only objcopy is borrowed from the RISC-V toolchain.
RUST_SDK = cargo --quiet --color never
RUST_SDK_OUT = build/rust/riscv32im-unknown-none-elf/release/examples
build/selftest_rs.bin: sdk/rust/Cargo.toml sdk/rust/build.rs $(wildcard sdk/rust/src/*.rs) \
                       sdk/rust/examples/selftest_rs.rs sdk/app.ld sdk/dessen_abi.h $(GEN)
	cd sdk/rust && $(RUST_SDK) build --release --target riscv32im-unknown-none-elf \
	    --features heap --example selftest_rs --target-dir ../../build/rust
	$(RISCV)objcopy -O binary $(RUST_SDK_OUT)/selftest_rs $@

rust-sdk: build/selftest_rs.bin build/dessen
	cd sdk/rust && $(RUST_SDK) test --features heap --lib --target-dir ../../build/rust
	build/dessen build/selftest_rs.bin
	@echo "rust sdk: PASS"

# The xpui apps: one cargo build for the workspace, then each image cut out
# of its ELF. Every app is checked on the VM with scripted keys.
XPUI_OUT = build/xpui/riscv32im-unknown-none-elf/release
# XPUI_LOCKED=--locked (CI) builds exactly the xpui commits in the committed
# apps/xpui/Cargo.lock, the ones the screen goldens were drawn with, and fails
# if a Cargo.toml changed without its lockfile. Only the xpui workspaces have a
# committed lockfile; sdk/rust has no dependencies and none.
XPUI_LOCKED ?=
xpui-build: $(GEN)
	cd apps/xpui && $(RUST_SDK) build $(XPUI_LOCKED) --release --target-dir ../../build/xpui

$(XPUI_APPS:%=build/%_xpui.bin): build/%_xpui.bin: xpui-build
	$(RISCV)objcopy -O binary $(XPUI_OUT)/$*_xpui $@

xpui: $(XPUI_APPS:%=build/%_xpui.bin) build/dessen
	printf b | build/dessen build/hello_xpui.bin | grep -q 'Hello from xpui'
	printf ncb | build/dessen build/lightsout_xpui.bin | grep -q 'Moves 1'
	printf cb | build/dessen build/dice_xpui.bin | grep -q 'Last:  d20='
	printf ncb | build/dessen build/sudoku_xpui.bin | grep -q 'cells left'
	rustc --edition 2024 --test apps/xpui/nook/src/board.rs -o build/nook_board_test && build/nook_board_test -q
	printf 't240,400;' | DESSEN_BOARD=touch DESSEN_MANIFEST=apps/nook_xpui.json build/dessen build/nook_xpui.bin | grep -q '[01] of 7 plants'
	@# ? opens the help page; its back arrow returns to the game.
	printf 't448,32;t30,32;' | DESSEN_BOARD=touch DESSEN_SCREEN=480x800 build/dessen build/nook_xpui.bin | awk '/How to Play/{h=1} h&&/ plants$$/{ok=1} END{exit !ok}'
	printf c | build/dessen build/minimal_xpui.bin | grep -q 'launch reader /books/dune.epub'
	printf nnnc | build/dessen build/minimal_xpui.bin | grep -q 'launch app /plugins/dice/app.bin'
	printf b | build/dessen build/minimal_xpui.bin | grep -q 'Library'
	printf c | build/dessen build/home_xpui.bin | grep -q 'launch reader /books/dune.epub'
	printf nc | build/dessen build/home_xpui.bin | grep -q 'launch reader /books/hyperion.epub'
	printf nnc | build/dessen build/home_xpui.bin | grep -q 'launch reader /books/neuromancer.epub'
	build/dessen build/home_xpui.bin </dev/null | grep -q '37%'
	$(call readstats_check,build/readstats_xpui.bin)
	@$(MAKE) --no-print-directory xpui-screens xpui-unit
	@echo "xpui apps: PASS"

# The safety net for changing the xpui apps' screens (tests/xpui/README.md):
# every walk in tests/xpui/*.scenarios, at three screen sizes, drawn with real
# fonts and compared with its committed picture and element positions (1 px of
# tolerance) and checked against the screen standards the app declares; and
# the apps' pure logic, unit-tested on the host. After a deliberate change,
# look at build/screen-diffs/, then `make xpui-bless`. `make xpui-standards`
# lists what each app still breaks of the screen standards.
# APP=nook runs (or blesses) one app's walks only.
XPUI_SCENARIOS = $(wildcard tests/xpui/*.scenarios)
XPUI_ONLY = $(if $(APP),--only $(APP)_xpui)
xpui-screens: $(XPUI_APPS:%=build/%_xpui.bin) build/dessen
	@python3 tools/screen_test.py $(XPUI_ONLY) $(XPUI_SCENARIOS)
xpui-bless: $(XPUI_APPS:%=build/%_xpui.bin) build/dessen
	@python3 tools/screen_test.py --update $(XPUI_ONLY) $(XPUI_SCENARIOS)
xpui-standards: $(XPUI_APPS:%=build/%_xpui.bin) build/dessen
	@python3 tools/screen_test.py --standards $(XPUI_SCENARIOS)
xpui-unit:
	cd tests/xpui/unit && $(RUST_SDK) test $(XPUI_LOCKED) --target-dir ../../../build/xpui-unit

# The reading-stats checks, for a build of it ($(1)): the cards from a generated
# session log (tools/readstats_fixture.py), the empty state, an import from
# CrossPoint Sync, and launching from the book card and an app icon. Run for
# both the Rust app and its xpui version.
define readstats_check
	@fs=$$(mktemp -d); python3 tools/readstats_fixture.py $$fs; \
	  out=$$(printf nnnnnnnnnnbb | DESSEN_FS_ROOT=$$fs build/dessen $(1)); rc=$$?; rm -rf $$fs; \
	  test $$rc -eq 0 || { echo "$(1): exit $$rc"; exit 1; }; \
	  for want in ' 30m$$' ' 45m$$' ' 2 days$$' ' 5h 45m$$' ' 32$$' ' 5h 30m$$' ' Dune$$' ' The "Left" Hand$$' ' 1\.6$$' ' Continue reading$$' ' Settings$$'; do \
	    echo "$$out" | grep -q -- "$$want" || { echo "$(1): missing '$$want'"; echo "$$out"; exit 1; }; done; \
	  empty=$$(mktemp -d); printf b | DESSEN_FS_ROOT=$$empty build/dessen $(1) | grep -q 'No reading sessions yet'; rc=$$?; \
	  rm -rf $$empty; test $$rc -eq 0 || { echo "$(1): no empty state"; exit 1; }
	@# Import: an empty reader with a sync account downloads its history (fixture server: sync.test).
	@fs=$$(mktemp -d); net=$$(mktemp -d); api=$$net/sync.test/api/v1/stats/sessions; \
	  mkdir -p $$fs/.crosspoint $$api; \
	  printf '{"username":"reader","password_obf":"","serverUrl":"https://sync.test/"}' > $$fs/.crosspoint/koreader.json; \
	  tz=$$(python3 -c 'import time; print(-time.localtime().tm_gmtoff // 60)'); \
	  printf '{"path":"/api/v1/stats/sessions/export?export_token=t","sessions":32}' > "$$api/export-link?tz=$$tz"; \
	  python3 tools/readstats_fixture.py $$net; mv $$net/.crosspoint/reading_sessions.jsonl "$$api/export?export_token=t"; \
	  out=$$(printf 'csecret\nb' | DESSEN_FS_ROOT=$$fs DESSEN_NET_ROOT=$$net build/dessen $(1)); \
	  test -s $$fs/.crosspoint/reading_sessions.jsonl; rc=$$?; rm -rf $$fs $$net; \
	  echo "$$out" | grep -q 'Import from CrossPoint Sync' && echo "$$out" | grep -q -- ' 5h 45m$$' && test $$rc -eq 0 \
	    || { echo "$(1): import failed"; echo "$$out"; exit 1; }
	printf c | build/dessen $(1) | grep -q 'launch reader /books/dune.epub'
	printf nnc | build/dessen $(1) | grep -q 'launch home 0'
endef

# readstats is checked against a generated session log (tools/readstats_fixture.py),
# then staged next to the `make sd` plugins.
rust: rust-sdk build/hello_rs.bin build/readstats.bin build/dessen
	printf 'c' | build/dessen build/hello_rs.bin | grep -q 'Hello from Rust on dessen'
	$(call readstats_check,build/readstats.bin)
	@mkdir -p build/sd/plugins/readstats && cp build/readstats.bin build/sd/plugins/readstats/app.bin \
	  && cp apps/readstats.json build/sd/plugins/readstats/manifest.json
	@echo "rust apps: PASS"

# Same proof in Zig. Not part of `test` (needs the Zig toolchain). Run: make zig
build/hello_zig.bin: apps/hello.zig sdk/dessen.zig sdk/crt0.S sdk/app.ld sdk/dessen_abi.h
	@mkdir -p build
	zig build-obj -target riscv32-freestanding-none -mcpu=generic_rv32+m -OReleaseSmall \
	    -femit-bin=build/hello_zig.o --dep dsn -Mroot=$< -Mdsn=sdk/dessen.zig
	$(RISCV)gcc -march=rv32im -mabi=ilp32 -nostdlib -mno-relax -Isdk -ffunction-sections \
	    -T sdk/app.ld sdk/crt0.S build/hello_zig.o -o build/hello_zig.elf \
	    -Wl,--gc-sections,--no-warn-rwx-segments
	$(RISCV)objcopy -O binary build/hello_zig.elf $@

zig: build/hello_zig.bin build/dessen
	printf 'n' | build/dessen build/hello_zig.bin | grep -q 'Hello from Zig on dessen'
	@echo "zig app: PASS"

clean:
	rm -rf build

.PHONY: all test sd sync sync-accept size rust rust-check rust-sdk xpui xpui-build xpui-sizes xpui-peaks xpui-screens xpui-bless xpui-standards xpui-unit zig clean
