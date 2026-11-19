// Host check for the FreeInk binding: runs real app binaries against a
// recording DrawTarget. Usage: test_host hello oob lightsout dice sudoku drawtest asynctest (.bin)
#include <cassert>
#include <cstdio>
#include <string>
#include <vector>

#include <openssl/evp.h>
#include <openssl/pkcs12.h>
#include <openssl/x509.h>

#include "../host/DesktopServices.h"
#include "../host/TraceTarget.h"
#include "../sdk/dessen_abi.h"
#include "DessenFreeInk.h"
#include "DessenServices.h"

using namespace freeink::ui;

static uint8_t mem[64 * 1024];
static uint8_t mem2[64 * 1024]; // a second app for the async-path test

// A network service that always parks (returns kPending) and completes after a
// fixed number of poll()s, writing a canned response. Exercises the kBusy path
// without a real network — the app (asynctest) can't tell it was delayed.
struct ParkingNet : dessen::Services {
  int ticks = 3;
  int countdown = 0;
  uint8_t* resp = nullptr;
  uint32_t cap = 0;
  int polls = 0;
  int netStart(const char*, const uint8_t*, uint32_t, uint8_t* r, uint32_t c) override {
    resp = r;
    cap = c;
    countdown = ticks;
    return dessen::kPending;
  }
  int poll() override {
    polls++;
    if (--countdown > 0) return dessen::kPending;
    static const uint8_t canned[] = {0x00, 0xC8, 0x00, 0x00, 'r', 'e', 'a', 'd', 'y'}; // 200, "ready"
    if (sizeof canned > cap) return DSN_ERR_TOO_BIG;
    memcpy(resp, canned, sizeof canned);
    return 5; // body length
  }
};

static void testAsync(const char* appPath) {
  FILE* f = fopen(appPath, "rb");
  assert(f && "asynctest.bin missing");
  const uint32_t n = static_cast<uint32_t>(fread(mem2, 1, sizeof mem2, f));
  fclose(f);
  ParkingNet net;
  dessen::FreeInkHost host;
  assert(host.load(mem2, n, sizeof mem2));
  host.setServices(&net);
  TraceTarget target;
  DeviceContext dev;
  dev.width = 400;
  dev.height = 300;
  InputSnapshot none;
  int busyTicks = 0;
  dessen::State st;
  for (int i = 0; i < 50; i++) {
    st = host.step(target, dev, none);
    if (st == dessen::State::Busy) busyTicks++;
    if (st == dessen::State::Exited || st == dessen::State::Faulted) break;
  }
  assert(st == dessen::State::Exited);
  assert(host.exitCode() == 0);    // app validated the canned response
  assert(busyTicks >= 2);          // it really parked across ticks
  assert(net.polls >= 3);
}

// Round-trips a PKCS#12 through DesktopServices::pkcs12Parse and checks the blob
// layout (u16 key length, PKCS#8 key DER, X.509 cert DER).
static void testPkcs12() {
  EVP_PKEY* key = EVP_RSA_gen(1024);
  assert(key);
  X509* cert = X509_new();
  ASN1_INTEGER_set(X509_get_serialNumber(cert), 1);
  X509_gmtime_adj(X509_getm_notBefore(cert), 0);
  X509_gmtime_adj(X509_getm_notAfter(cert), 3600);
  X509_set_pubkey(cert, key);
  X509_NAME* name = X509_get_subject_name(cert);
  X509_NAME_add_entry_by_txt(name, "CN", MBSTRING_ASC, reinterpret_cast<const unsigned char*>("t"), -1, -1, 0);
  X509_set_issuer_name(cert, name);
  assert(X509_sign(cert, key, EVP_sha256()));
  PKCS12* p12 = PKCS12_create("pw", "f", key, cert, nullptr, 0, 0, 0, 0, 0);
  assert(p12);
  uint8_t* der = nullptr;
  const int dl = i2d_PKCS12(p12, &der);
  assert(dl > 0);

  dessen::DesktopServices svc(".");
  uint8_t blob[4096];
  int bl = svc.pkcs12Parse(der, dl, "pw", blob, sizeof blob);
  assert(bl > 4);
  const uint32_t keyLen = blob[0] << 8 | blob[1];
  assert(2 + keyLen < static_cast<uint32_t>(bl));
  const uint8_t* kp = blob + 2;
  EVP_PKEY* reKey = d2i_AutoPrivateKey(nullptr, &kp, keyLen);
  assert(reKey); // the key DER re-parses
  const uint8_t* cp = blob + 2 + keyLen;
  X509* reCert = d2i_X509(nullptr, &cp, bl - 2 - keyLen);
  assert(reCert); // the cert DER re-parses
  // wrong password is refused
  assert(svc.pkcs12Parse(der, dl, "wrong", blob, sizeof blob) < 0);

  EVP_PKEY_free(reKey);
  X509_free(reCert);
  OPENSSL_free(der);
  PKCS12_free(p12);
  X509_free(cert);
  EVP_PKEY_free(key);
}

static uint32_t readApp(const char* path) {
  FILE* f = fopen(path, "rb");
  assert(f && "app binary missing");
  const size_t n = fread(mem, 1, sizeof mem, f);
  fclose(f);
  return static_cast<uint32_t>(n);
}

int main(int argc, char** argv) {
  assert(argc == 8);
  DeviceContext screen;
  screen.width = 480, screen.height = 800;
  TraceTarget target;
  dessen::FreeInkHost host;
  InputSnapshot none;

  // hello: draws, asks for a refresh, then blocks until an event arrives.
  assert(host.load(mem, readApp(argv[1]), sizeof mem));
  assert(host.step(target, screen, none) == dessen::State::Running); // yielded at refresh
  assert(host.takeRefreshRequest() && !host.takeRefreshRequest());
  assert(host.step(target, screen, none) == dessen::State::Waiting);
  assert((target.ops == std::vector<std::string>{"fill 0,0 480x800 c1", "text 40,60 Hello from dessen",
                                                 "text 40,100 Press any button"}));
  const uint64_t idle = host.vm().icount;
  assert(host.step(target, screen, none) == dessen::State::Waiting); // no input: nothing runs
  assert(host.vm().icount == idle && target.ops.size() == 3);

  InputSnapshot back;
  back.back = true;
  assert(host.step(target, screen, back) == dessen::State::Exited);
  assert(host.exitCode() == DSN_EV_BACK);

  // oob: a sandbox escape attempt ends as a fault, not a crash.
  assert(host.load(mem, readApp(argv[2]), sizeof mem));
  assert(host.step(target, screen, none) == dessen::State::Faulted);
  assert(host.vm().fault_addr == 0x10000000u);

  // lightsout: a tap inside the grid counts as one move and is redrawn.
  assert(host.load(mem, readApp(argv[3]), sizeof mem));
  while (host.step(target, screen, none) == dessen::State::Running) {}
  target.ops.clear();
  InputSnapshot tap;
  tap.touchReleased = true;
  tap.touchX = 240;
  tap.touchY = 280;
  while (host.step(target, screen, tap) == dessen::State::Running) tap = none;
  assert(host.state() == dessen::State::Waiting && host.takeRefreshRequest());
  bool counted = false;
  for (const auto& op : target.ops) counted |= op.find("Moves 1") != std::string::npos;
  assert(counted);

  // dice: tapping the d4 with the random source pinned to its maximum rolls a 4.
  assert(host.load(mem, readApp(argv[4]), sizeof mem));
  host.setRandom([]() -> uint32_t { return 0xffffffffu; });
  while (host.step(target, screen, none) == dessen::State::Running) {}
  assert(target.lines > 20); // the polyhedral outlines
  target.ops.clear();
  tap.touchReleased = true;
  tap.touchX = 72; // centre of the first die on a 480-wide screen
  tap.touchY = 112;
  while (host.step(target, screen, tap) == dessen::State::Running) tap = none;
  bool rolledFour = false;
  for (const auto& op : target.ops) rolledFour |= op.find("d4=4") != std::string::npos;
  assert(rolledFour);

  // sudoku: generation spans many budget slices, then the puzzle has 30 clues
  // (51 empty), and filling a cell from the number pad leaves 50.
  assert(host.load(mem, readApp(argv[5]), sizeof mem));
  host.setRandom(nullptr);
  target.ops.clear();
  int slices = 0;
  while (host.step(target, screen, none) == dessen::State::Running) slices++;
  assert(host.state() == dessen::State::Waiting && slices > 2);
  const auto shows = [&](const char* text) {
    for (const auto& op : target.ops)
      if (op.find(text) != std::string::npos) return true;
    return false;
  };
  assert(shows("51 cells left"));
  // 480x800: cell 49, grid_x 19, pad_w 44, pad_y 503. Find an empty cell by
  // tapping cells until a pad press changes the count.
  bool filled = false;
  for (int c = 0; c < 81 && !filled; c++) {
    for (int press = 0; press < 2; press++) {
      InputSnapshot t;
      t.touchReleased = true;
      t.touchX = static_cast<int16_t>(press ? 19 + 22 : 19 + (c % 9) * 49 + 24);
      t.touchY = static_cast<int16_t>(press ? 503 + 22 : 48 + (c / 9) * 49 + 24);
      target.ops.clear();
      while (host.step(target, screen, t) == dessen::State::Running) t = none;
    }
    filled = shows("50 cells left");
  }
  assert(filled);

  // drawtest: every drawing call reaches the target with clipped, unpacked
  // arguments, and bad input is refused (the app checks the return values).
  assert(host.load(mem, readApp(argv[6]), sizeof mem));
  target.ops.clear();
  target.lines = 0;
  while (host.step(target, screen, none) == dessen::State::Running) {}
  assert(host.state() == dessen::State::Exited && host.exitCode() == 0);
  assert(shows("fill 10,10 50x30 c4") && shows("stroke 0,0 40x40 w3") && shows("triangle"));
  assert(shows("text 0,100 wrapped and centred") && shows("bitmap 200,10 8x8 src 8x8") && target.lines == 1);

  // Safe area: with bezel insets the app's (0,0) moves inside them, clear
  // still wipes the whole panel, and taps are reported in app coordinates.
  DeviceContext bezel = screen;
  bezel.safeArea = Insets{10, 20, 30, 40}; // top, right, bottom, left
  assert(host.load(mem, readApp(argv[1]), sizeof mem));
  target.ops.clear();
  while (host.step(target, bezel, none) == dessen::State::Running) {}
  assert((target.ops == std::vector<std::string>{"fill 0,0 480x800 c1", "text 80,70 Hello from dessen",
                                                 "text 80,110 Press any button"}));
  InputSnapshot edge;
  edge.touchReleased = true;
  edge.touchX = 5; // on the bezel: not delivered
  edge.touchY = 5;
  assert(host.step(target, bezel, edge) == dessen::State::Waiting);
  edge.touchX = 140; // app (100, 100)
  edge.touchY = 110;
  assert(host.step(target, bezel, edge) == dessen::State::Exited);
  assert(static_cast<uint32_t>(host.exitCode()) == (DSN_EV_TAP | 100u << 8 | 100u << 20));

  // drawtest again inside the bezel: it checks dsn_device() against dsn_screen().
  assert(host.load(mem, readApp(argv[6]), sizeof mem));
  while (host.step(target, bezel, none) == dessen::State::Running) {}
  assert(host.state() == dessen::State::Exited && host.exitCode() == 0);

  // A truncated or garbage image is rejected.
  mem[0] ^= 0xff;
  assert(!host.load(mem, 16, sizeof mem));

  testAsync(argv[7]); // kBusy parking/poll path
  testPkcs12();       // PKCS#12 parse + blob layout

  puts("freeink binding: PASS");
  return 0;
}
