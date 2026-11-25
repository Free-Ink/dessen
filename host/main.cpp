// Desktop runner for dessen apps: `dessen app.bin`. It uses the same FreeInk
// binding as the firmware, with a draw target that prints each call. Keys on
// stdin stand in for buttons: c confirm, b back, p prev, n next. Exits with
// the app's exit code, or 101 on a memory fault, 102 on an illegal instruction
// and 103 when the board cannot drive the app (its manifest's "requires").
//
// The board has buttons and no touch, unless DESSEN_BOARD says touch, buttons
// or both; DESSEN_SCREEN=<w>x<h> sets the panel (800x480). `t<x>,<y>;` on
// stdin taps there, `T<x>,<y>;` long-presses there, and ^ v < > swipe up,
// down, left and right.
//
// Measuring: DESSEN_COVERS=1 makes full-size covers, and DESSEN_PAINT="<heap
// start> <heap length> <stack start>" fills the heap arena and the stack area
// with a pattern before the run and prints their high-water marks after it
// (tools/peaks.sh reads the addresses from the ELF). The manifest is DESSEN_MANIFEST, else a manifest.json beside the
// image (the SD-card layout); without one, the app runs on any board.
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <ctime>
#include <fstream>
#include <memory>
#include <sstream>
#include <string>

#include "DesktopServices.h"
#include "DessenFreeInk.h"
#include "Manifest.h"
#include "Snapshot.h"
#include "TraceTarget.h"

int main(int argc, char** argv) {
  if (argc != 2) return fprintf(stderr, "usage: %s app.bin\n", argv[0]), 2;
  FILE* f = fopen(argv[1], "rb");
  if (!f) return perror(argv[1]), 2;
  static uint8_t mem[1u << 20];
  const size_t len = fread(mem, 1, sizeof mem, f);
  if (!feof(f)) return fprintf(stderr, "image larger than %zu bytes\n", sizeof mem), 2;
  fclose(f);

  freeink::ui::DeviceContext screen;
  screen.width = 800, screen.height = 480;
  if (const char* size = getenv("DESSEN_SCREEN"))
    if (sscanf(size, "%hdx%hd", &screen.width, &screen.height) != 2 || screen.width <= 0 || screen.height <= 0)
      return fprintf(stderr, "DESSEN_SCREEN is <width>x<height>\n"), 2;
  if (const char* board = getenv("DESSEN_BOARD")) {
    if (strcmp(board, "touch") && strcmp(board, "buttons") && strcmp(board, "both"))
      return fprintf(stderr, "DESSEN_BOARD is touch, buttons or both\n"), 2;
    screen.hasTouch = strcmp(board, "buttons") != 0;
    screen.hasButtons = strcmp(board, "touch") != 0;
  }
  std::string manifest = getenv("DESSEN_MANIFEST") ? getenv("DESSEN_MANIFEST") : "";
  if (manifest.empty()) {
    const std::string image = argv[1];
    const size_t slash = image.rfind('/');
    manifest = (slash == std::string::npos ? std::string() : image.substr(0, slash + 1)) + "manifest.json";
    if (!std::ifstream(manifest)) manifest.clear();
  }
  if (!manifest.empty()) {
    std::ifstream in(manifest);
    if (!in) return perror(manifest.c_str()), 2;
    std::stringstream text;
    text << in.rdbuf();
    unsigned needs = 0;
    if (!dessen::parseRequires(text.str(), needs))
      return fprintf(stderr, "%s: \"requires\" is not a list of words\n", manifest.c_str()), 2;
    const bool hasPsram = getenv("DESSEN_PSRAM") != nullptr;
    if (!dessen::meets(needs, screen.hasTouch, screen.hasButtons, hasPsram))
      return fprintf(stderr, "%s: this board cannot drive the app\n", manifest.c_str()), 103;
  }

  dessen::FreeInkHost host;
  if (!host.load(mem, static_cast<uint32_t>(len), sizeof mem)) return fprintf(stderr, "not a valid dessen app\n"), 2;
  // DESSEN_SEED=<n>: the binding's own xorshift from seed n, so a run is the
  // same every time (tools/screen_test.py); otherwise the OS's randomness.
  static uint32_t seed = 0;
  if (const char* s = getenv("DESSEN_SEED")) seed = static_cast<uint32_t>(strtoul(s, nullptr, 0)) | 1;
  host.setRandom(seed ? +[]() -> uint32_t { return seed ^= seed << 13, seed ^= seed >> 17, seed ^= seed << 5; }
                      : +[]() -> uint32_t { return arc4random(); });
  // Capability services: storage enabled by DESSEN_FS_ROOT, network
  // from DESSEN_NET_ROOT fixtures when set, crypto via libcrypto.
  const char* fsRoot = getenv("DESSEN_FS_ROOT");
  const char* netRoot = getenv("DESSEN_NET_ROOT");
  dessen::DesktopServices services(fsRoot ? fsRoot : "", "/app", netRoot ? netRoot : "");
  host.setServices(&services);
  // DESSEN_HINTS=1: draw button hints, as a buttons-only board does.
  if (getenv("DESSEN_HINTS")) services.hintHeight = 40;
  services.fakeCovers = getenv("DESSEN_COVERS") != nullptr;

  uint32_t heapStart = 0, heapLen = 0, stackStart = 0;
  const bool paint = getenv("DESSEN_PAINT") && sscanf(getenv("DESSEN_PAINT"), "%u %u %u", &heapStart, &heapLen, &stackStart) == 3;
  const uint32_t top = host.vm().mem_size;
  if (paint && (heapStart > top || heapLen > top - heapStart || stackStart > top))
    return fprintf(stderr, "DESSEN_PAINT is outside the app's memory\n"), 2;
  if (paint) memset(mem + heapStart, 0xa5, heapLen), memset(mem + stackStart, 0xa5, top - stackStart);

  TraceTarget target;
  target.echo = true;
  // DESSEN_TRACE=detail: every argument of every draw call (tools/screen_test.py).
  target.detail = getenv("DESSEN_TRACE") && !strcmp(getenv("DESSEN_TRACE"), "detail");
  // DESSEN_SNAPSHOT=<file.png>: draw through FreeInk's framebuffer target as
  // well, measure text with its fonts, and write the last refreshed frame to
  // the file (host/Snapshot.h). In detail, the fonts' line heights come first.
  const char* snapshotPath = getenv("DESSEN_SNAPSHOT");
  std::unique_ptr<Snapshot> snapshot;
  if (snapshotPath) {
    snapshot = std::make_unique<Snapshot>(screen.width, screen.height);
    target.also = &snapshot->target;
    if (target.detail)
      printf("# line heights %d %d %d\n", target.lineHeight(0), target.lineHeight(1), target.lineHeight(2));
  }
  freeink::ui::InputSnapshot input;
  dessen::FreeInkHost::Swipe swipe = dessen::FreeInkHost::SwipeNone;
  const clock_t t0 = clock();
  for (;;) {
    const dessen::State state = host.step(target, screen, input, swipe, 1000000);
    input = {}, swipe = dessen::FreeInkHost::SwipeNone;
    if (host.takeRefreshRequest()) {
      puts("refresh");
      if (snapshot) snapshot->refreshed();
    }
    if (state == dessen::State::Launch) {
      if (host.launch() == dessen::Launch::Home) {
        printf("[launch home %d]\n", host.launchHomeDest());
      } else {
        const char* k = host.launch() == dessen::Launch::Reader     ? "reader"
                        : host.launch() == dessen::Launch::Settings ? "settings"
                        : host.launch() == dessen::Launch::App      ? "app"
                                                                    : "browser";
        printf("[launch %s %s]\n", k, host.launchPath());
      }
      break;
    }
    if (state == dessen::State::Exited && target.detail) printf("[exit %d]\n", host.exitCode());
    if (state == dessen::State::Exited || state == dessen::State::Faulted) break;
    if (state != dessen::State::Waiting) continue;
    const int c = getchar();
    if (c == EOF) break;
    input.confirm = c == 'c', input.back = c == 'b', input.prev = c == 'p', input.next = c == 'n';
    if ((c == 't' || c == 'T') && scanf("%hd,%hd;", &input.touchX, &input.touchY) == 2)
      input.touchReleased = true, input.longPress = c == 'T';
    if (c == '^') swipe = dessen::FreeInkHost::SwipeUp;
    if (c == 'v') swipe = dessen::FreeInkHost::SwipeDown;
    if (c == '<') swipe = dessen::FreeInkHost::SwipeLeft;
    if (c == '>') swipe = dessen::FreeInkHost::SwipeRight;
  }
  const double secs = static_cast<double>(clock() - t0) / CLOCKS_PER_SEC;
  fflush(stdout);
  if (snapshot && !snapshot->write(snapshotPath)) fprintf(stderr, "no snapshot written to %s\n", snapshotPath);
  fprintf(stderr, "%llu instructions, %.3fs\n", static_cast<unsigned long long>(host.vm().icount), secs);

  if (paint) {
    uint32_t heapTop = heapStart + heapLen;
    while (heapTop > heapStart && mem[heapTop - 1] == 0xa5) --heapTop;
    uint32_t stackLow = stackStart;
    while (stackLow < top && mem[stackLow] == 0xa5) ++stackLow;
    fprintf(stderr, "MEASURE heap %u stack %u free %u\n", heapTop - heapStart, top - stackLow, stackLow - stackStart);
  }
  if (host.state() == dessen::State::Faulted) {
    const bool mem_fault = host.vm().fault_addr != host.vm().pc;
    fprintf(stderr, "FAULT at 0x%08x (pc 0x%08x)\n", host.vm().fault_addr, host.vm().pc);
    return mem_fault ? 101 : 102;
  }
  return host.state() == dessen::State::Exited ? host.exitCode() & 0xff : 0;
}
