#!/bin/sh
# What each xpui app costs: code, read-only data and bss (the heap is in it)
# from the ELF, and the memory it asks the VM for, from its image header.
# Flags any app that asks for more than 64 KB.
cd "$(dirname "$0")/.."
ELF=build/xpui/riscv32im-unknown-none-elf/release
SIZE=$HOME/.platformio/tools/toolchain-riscv32-esp/bin/riscv32-esp-elf-size
printf '%-10s %7s %7s %7s %7s %8s\n' app text rodata bss image asks
for app in ${*:-hello dice lightsout minimal sudoku nook home readstats}; do
  elf=$ELF/${app}_xpui bin=build/${app}_xpui.bin
  [ -f "$bin" ] || { echo "$app: build/${app}_xpui.bin missing (make xpui)"; continue; }
  set -- $("$SIZE" -A "$elf" | awk '$1==".text"{t=$2} $1==".rodata"{r=$2} $1==".bss"{b=$2} END{print t, r, b}')
  asks=$(od -An -t u4 -j 4 -N 4 "$bin" | tr -d ' ')
  flag=; [ "$asks" -gt 65536 ] && flag="  over 64 KB by $((asks - 65536))"
  printf '%-10s %7s %7s %7s %7s %8s%s\n' "$app" "$1" "$2" "$3" "$(wc -c < "$bin" | tr -d ' ')" "$asks" "$flag"
done
