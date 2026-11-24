#!/bin/sh
# Heap and stack high-water marks of the xpui apps, on the desktop runner.
#
#   tools/peaks.sh            every app, at 480x800 and 1072x1448
#
# Each app runs a scripted session (keys, covers at full size, readstats both
# browsing and importing); the runner paints the heap arena and the stack with
# a pattern first and reports how much of each was touched. These are the
# peaks docs/writing-apps.md sizes `heap!` and `stack!` from.
set -e
cd "$(dirname "$0")/.."
ELF=build/xpui/riscv32im-unknown-none-elf/release
BIN=$HOME/.platformio/tools/toolchain-riscv32-esp/bin/riscv32-esp-elf
export DESSEN_COVERS=1 DESSEN_BOARD=both

run() { # run <app> <keys>
  elf=$ELF/$1_xpui
  set -- "$1" "$2" $("$BIN-nm" -S "$elf" | awk '$4 ~ /HEAP$/ {print $1, $2}')
  heap=$((0x$3)); len=$((0x$4 - 16))
  bss_end=$("$BIN-size" -A "$elf" | awk '$1==".bss" {print $2+$3}')
  printf '%-10s' "$1"
  printf "$2" | DESSEN_PAINT="$heap $len $bss_end" build/dessen "build/$1_xpui.bin" 2>&1 >/dev/null | grep -E 'MEASURE|FAULT' | sed 's/MEASURE //'
}

fs=$(mktemp -d); python3 tools/readstats_fixture.py "$fs"
for screen in 480x800 1072x1448; do
  export DESSEN_SCREEN=$screen; echo "== $screen"
  for app in hello dice lightsout minimal home; do run $app nnnccnnppb; done
  run sudoku ncnncnnb
  # Plants, the help page and back, then New and its question, at either screen's buttons.
  run nook 't300,300;t300,400;t200,500;t150,350;t448,32;t30,32;t1040,32;t30,32;t392,32;t984,32;b'
  DESSEN_FS_ROOT=$fs run readstats nnnnnnnnnnbb
  f=$(mktemp -d); net=$(mktemp -d); api=$net/sync.test/api/v1/stats/sessions; mkdir -p "$f/.crosspoint" "$api"
  printf '{"username":"reader","password_obf":"","serverUrl":"https://sync.test/"}' > "$f/.crosspoint/koreader.json"
  tz=$(python3 -c 'import time; print(-time.localtime().tm_gmtoff // 60)')
  printf '{"path":"/api/v1/stats/sessions/export?export_token=t","sessions":32}' > "$api/export-link?tz=$tz"
  python3 tools/readstats_fixture.py "$net"; mv "$net/.crosspoint/reading_sessions.jsonl" "$api/export?export_token=t"
  printf '%-10s' "(import)"; DESSEN_FS_ROOT=$f DESSEN_NET_ROOT=$net run readstats 'csecret\nnnnnnnb' | sed 's/^readstats //'
  rm -rf "$f" "$net"
done
rm -rf "$fs"
