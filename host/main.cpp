// Desktop runner for dessen apps: `dessen app.bin`. It uses the same FreeInk
// binding as the firmware, with a draw target that prints each call. Keys on
// stdin stand in for buttons: c confirm, b back, p prev, n next. Exits with
// the app's exit code, or 101 on a memory fault and 102 on an illegal instruction.
#include <cstdio>
#include <cstdlib>
#include <ctime>

#include "DesktopServices.h"
#include "DessenFreeInk.h"
#include "TraceTarget.h"

int main(int argc, char** argv) {
  if (argc != 2) return fprintf(stderr, "usage: %s app.bin\n", argv[0]), 2;
  FILE* f = fopen(argv[1], "rb");
  if (!f) return perror(argv[1]), 2;
  static uint8_t mem[1u << 20];
  const size_t len = fread(mem, 1, sizeof mem, f);
  if (!feof(f)) return fprintf(stderr, "image larger than %zu bytes\n", sizeof mem), 2;
  fclose(f);

  dessen::FreeInkHost host;
  if (!host.load(mem, static_cast<uint32_t>(len), sizeof mem)) return fprintf(stderr, "not a valid dessen app\n"), 2;
  host.setRandom([]() -> uint32_t { return arc4random(); });
  // Capability services: storage under DESSEN_FS_ROOT (cwd by default), network
  // from DESSEN_NET_ROOT fixtures when set, crypto via libcrypto.
  const char* fsRoot = getenv("DESSEN_FS_ROOT");
  const char* netRoot = getenv("DESSEN_NET_ROOT");
  dessen::DesktopServices services(fsRoot ? fsRoot : ".", "/app", netRoot ? netRoot : "");
  host.setServices(&services);

  TraceTarget target;
  target.echo = true;
  freeink::ui::DeviceContext screen;
  screen.width = 800, screen.height = 480;
  freeink::ui::InputSnapshot input;
  const clock_t t0 = clock();
  for (;;) {
    const dessen::State state = host.step(target, screen, input, 1000000);
    input = {};
    if (host.takeRefreshRequest()) puts("refresh");
    if (state == dessen::State::Exited || state == dessen::State::Faulted) break;
    if (state != dessen::State::Waiting) continue;
    const int c = getchar();
    if (c == EOF) break;
    input.confirm = c == 'c', input.back = c == 'b', input.prev = c == 'p', input.next = c == 'n';
  }
  const double secs = static_cast<double>(clock() - t0) / CLOCKS_PER_SEC;
  fflush(stdout);
  fprintf(stderr, "%llu instructions, %.3fs\n", static_cast<unsigned long long>(host.vm().icount), secs);

  if (host.state() == dessen::State::Faulted) {
    const bool mem_fault = host.vm().fault_addr != host.vm().pc;
    fprintf(stderr, "FAULT at 0x%08x (pc 0x%08x)\n", host.vm().fault_addr, host.vm().pc);
    return mem_fault ? 101 : 102;
  }
  return host.state() == dessen::State::Exited ? host.exitCode() & 0xff : 0;
}
