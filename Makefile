# Any RV32 GCC works. PlatformIO keeps its copy under tools/ permanently; the
# packages/ link comes and goes depending on which board was built last.
RISCV ?= $(HOME)/.platformio/tools/toolchain-riscv32-esp/bin/riscv32-esp-elf-
# ponytail: no libgcc/libc linked, so 64-bit division and memcpy-style builtins
# will fail to link. Add a small runtime to sdk/ when an app first needs one.
APPFLAGS = -march=rv32im -mabi=ilp32 -Os -ffreestanding -nostdlib -mno-relax \
           -fno-tree-loop-distribute-patterns -Wall -Isdk -T sdk/app.ld -Wl,--no-warn-rwx-segments
APPS = hello selftest oob bench drawtest lightsout dice sudoku
SD_APPS = hello lightsout dice sudoku

# FreeInk SDK checkout, used read-only for its UI headers.
FREEINK_SDK ?= ../freeink-sdk

GEN = sdk/dessen_sysnums.h sdk/dessen_calls.h freeink/dessen_dispatch.inc
HOSTFLAGS = -std=c++17 -O2 -Wall -Wextra -Ivm -Isdk -Ifreeink -Ihost -I$(FREEINK_SDK)/libs/ui/FreeInkUI/include
BINDING = sdk/dessen_abi.h freeink/DessenFreeInk.cpp freeink/DessenFreeInk.h host/TraceTarget.h vm/dessen_vm.h $(GEN)

all: build/dessen build/test_freeink $(APPS:%=build/%.bin)

# The whole call list, both sides, comes from one description file.
sdk/dessen_calls.h: abi/freeink.api tools/gen_api.py
	python3 tools/gen_api.py
sdk/dessen_sysnums.h freeink/dessen_dispatch.inc: sdk/dessen_calls.h

build/dessen_vm.o: vm/dessen_vm.c vm/dessen_vm.h sdk/dessen_abi.h $(GEN)
	@mkdir -p build
	cc -O2 -Wall -Wextra -Ivm -Isdk -c vm/dessen_vm.c -o $@

build/dessen: host/main.cpp build/dessen_vm.o $(BINDING)
	c++ $(HOSTFLAGS) host/main.cpp freeink/DessenFreeInk.cpp build/dessen_vm.o -o $@

build/test_freeink: freeink/test_host.cpp build/dessen_vm.o $(BINDING)
	c++ $(HOSTFLAGS) freeink/test_host.cpp freeink/DessenFreeInk.cpp build/dessen_vm.o -o $@

build/%.bin: apps/%.c sdk/crt0.S sdk/app.ld sdk/dessen.h sdk/dessen_abi.h $(GEN)
	@mkdir -p build
	$(RISCV)gcc $(APPFLAGS) sdk/crt0.S $< -o build/$*.elf
	$(RISCV)objcopy -O binary build/$*.elf $@

# Exit codes: selftest 0 = all checks pass; hello returns the event it got
# (c = confirm = 1); oob must fault (101 = memory fault).
test: all
	build/dessen build/selftest.bin
	printf c | build/dessen build/hello.bin; test $$? -eq 1
	build/dessen build/oob.bin; test $$? -eq 101
	build/dessen build/bench.bin
	printf ncb | build/dessen build/lightsout.bin | grep -q 'Moves 1'
	printf cb | build/dessen build/dice.bin | grep -q 'Last:  d20='
	printf b | build/dessen build/sudoku.bin | grep -q 'cells left'
	build/test_freeink build/hello.bin build/oob.bin build/lightsout.bin build/dice.bin build/sudoku.bin build/drawtest.bin
	@echo PASS

# Plugin folders ready to copy into /plugins on the SD card.
sd: all
	@for a in $(SD_APPS); do mkdir -p build/sd/plugins/$$a && cp build/$$a.bin build/sd/plugins/$$a/app.bin \
	  && cp apps/$$a.json build/sd/plugins/$$a/manifest.json; done; ls build/sd/plugins

# Report FreeInk SDK surface that dessen does not yet expose, and SDK pin drift.
# Does not change code — exposing a method is a judgment call (see AGENTS.md).
sync:
	python3 tools/sync_check.py --freeink-sdk $(FREEINK_SDK)

# Record the current SDK commit as reviewed, after looking at `make sync`.
sync-accept:
	git -C $(FREEINK_SDK) rev-parse HEAD > abi/.freeink-sync
	@echo "pinned $(FREEINK_SDK) @ $$(cat abi/.freeink-sync | cut -c1-10)"

# Flash cost of the interpreter on an ESP32-C3, per crosspoint's size rule.
size:
	@mkdir -p build
	$(RISCV)gcc -march=rv32imc_zicsr_zifencei -mabi=ilp32 -Os -Ivm -Isdk -c vm/dessen_vm.c -o build/dessen_vm.c3.o
	$(RISCV)g++ -std=gnu++2a -march=rv32imc_zicsr_zifencei -mabi=ilp32 -Os -ffunction-sections -fdata-sections -fno-exceptions -fno-rtti \
	    -Ivm -Isdk -Ifreeink -I$(FREEINK_SDK)/libs/ui/FreeInkUI/include -c freeink/DessenFreeInk.cpp -o build/binding.c3.o
	$(RISCV)size build/dessen_vm.c3.o build/binding.c3.o

clean:
	rm -rf build

.PHONY: all test sd sync sync-accept size clean
