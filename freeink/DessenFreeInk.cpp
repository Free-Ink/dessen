#include "DessenFreeInk.h"

#include <cstdio>
#include <cstring>

#include <FreeInkUILayout.h>
#include <components/controls/button.h>
#include <components/controls/checkbox.h>
#include <components/controls/progress-bar.h>
#include <components/controls/toggle.h>
#include <components/lists/setting-row.h>

#include "../sdk/dessen_abi.h"
#include "DessenServices.h"

namespace dessen {
namespace {

using namespace freeink::ui;

enum { kExit = 1, kWait, kYield, kBusy, kLaunch }; // positive stop codes for dvm_run
constexpr uint32_t kStrMax = 200;  // longest text an app can pass in one call
constexpr uint32_t kPathMax = 255; // longest storage path

// The app's view of the screen: the safe area's size, and where its top-left
// corner sits on the panel.
struct View {
  int16_t width, height, originX, originY;
};

View viewOf(const DeviceContext& device) {
  const Rect safe = device.safeRect();
  if (safe.empty() || safe.x < 0 || safe.y < 0 || safe.right() > device.width || safe.bottom() > device.height)
    return View{device.width, device.height, 0, 0}; // nonsense insets: use the whole panel
  return View{safe.width, safe.height, safe.x, safe.y};
}

uint32_t eventOf(const InputSnapshot& in, View view) {
  if (in.confirm) return DSN_EV_CONFIRM;
  if (in.back) return DSN_EV_BACK;
  if (in.prev || in.focusPrev) return DSN_EV_PREV;
  if (in.next || in.focusNext) return DSN_EV_NEXT;
  if (in.touchReleased) {
    const int32_t x = in.touchX - view.originX, y = in.touchY - view.originY;
    if (in.touchX < 0 || in.touchY < 0 || x < 0 || y < 0 || x >= view.width || y >= view.height) return 0;
    return DSN_EV_TAP | (static_cast<uint32_t>(x) & 0xfff) << 8 | (static_cast<uint32_t>(y) & 0xfff) << 20;
  }
  return 0;
}

// --- Sanitizers. Everything an app passes is untrusted; these are the only
// --- way app values reach the draw target. Used by the generated dispatch.

// NUL-terminated host copy of guest text, cut at kStrMax. NULL if out of range.
const char* guestStr(dvm_t* vm, uint32_t addr, uint32_t len, char* buf) {
  if (len > kStrMax) len = kStrMax;
  const void* s = dvm_ptr(vm, addr, len);
  if (!s) return nullptr;
  memcpy(buf, s, len);
  buf[len] = '\0';
  return buf;
}

// App rectangle -> panel rectangle, clipped to the safe area. Empty if nothing is left.
Rect clip(int32_t x, int32_t y, int32_t w, int32_t h, View view) {
  if (w <= 0 || h <= 0 || x >= view.width || y >= view.height) return Rect{};
  if (x < 0) { w += x; x = 0; }
  if (y < 0) { h += y; y = 0; }
  if (w > view.width - x) w = view.width - x;
  if (h > view.height - y) h = view.height - y;
  if (w <= 0 || h <= 0) return Rect{};
  return Rect{static_cast<int16_t>(x + view.originX), static_cast<int16_t>(y + view.originY), static_cast<int16_t>(w),
              static_cast<int16_t>(h)};
}

// App point -> panel point, pinned inside the safe area (callers check it is non-empty).
Point pin(int32_t x, int32_t y, View view) {
  return Point{static_cast<int16_t>((x < 0 ? 0 : x >= view.width ? view.width - 1 : x) + view.originX),
               static_cast<int16_t>((y < 0 ? 0 : y >= view.height ? view.height - 1 : y) + view.originY)};
}

uint32_t writeDeviceInfo(const DeviceContext& device, View view, uint8_t* out, uint32_t len) {
  dsn_device_info info{};
  info.width = static_cast<uint16_t>(view.width);
  info.height = static_cast<uint16_t>(view.height);
  info.panel_width = static_cast<uint16_t>(device.width);
  info.panel_height = static_cast<uint16_t>(device.height);
  info.inset_top = static_cast<uint16_t>(view.originY);
  info.inset_left = static_cast<uint16_t>(view.originX);
  info.inset_right = static_cast<uint16_t>(device.width - view.originX - view.width);
  info.inset_bottom = static_cast<uint16_t>(device.height - view.originY - view.height);
  info.min_touch = static_cast<uint16_t>(device.minTouchSize);
  info.has_touch = device.hasTouch;
  info.has_buttons = device.hasButtons;
  info.orientation = static_cast<uint8_t>(device.orientation);
  const uint32_t n = len < sizeof info ? len : sizeof info; // older apps may ask for less
  memcpy(out, &info, n);
  return n;
}

Paint paint(uint32_t color) {
  switch (color) {
    case DSN_BLACK: return Paint::solid(Color::Black);
    case DSN_LIGHT_GRAY: return Paint::solid(Color::LightGray);
    case DSN_DARK_GRAY: return Paint::solid(Color::DarkGray);
    default: return Paint::solid(Color::White);
  }
}

TextStyle textStyle(uint32_t font, uint32_t align, uint32_t maxLines, uint32_t flags) {
  TextStyle st;
  st.font = static_cast<FontId>(font <= DSN_FONT_CUSTOM ? font : DSN_FONT_SMALL);
  st.align = align == DSN_ALIGN_CENTER ? TextAlign::Center : align == DSN_ALIGN_RIGHT ? TextAlign::Right : TextAlign::Left;
  st.maxLines = static_cast<uint8_t>(maxLines < 1 ? 1 : maxLines > 16 ? 16 : maxLines);
  st.bold = flags & DSN_TEXT_BOLD;
  st.inverted = flags & DSN_TEXT_INVERTED;
  st.rotation = flags & DSN_TEXT_ROT_CW90 ? Rotation::CW90 : flags & DSN_TEXT_ROT_CCW90 ? Rotation::CCW90 : Rotation::None;
  return st;
}

// w <= 0 means "to the right screen edge"; h <= 0 means "as tall as maxLines".
uint32_t drawText(DrawTarget& target, View screen, int32_t x, int32_t y, int32_t w, int32_t h, const char* s,
                  TextStyle st) {
  const int16_t lh = target.lineHeight(st.font);
  if (w <= 0) w = screen.width - x;
  if (h <= 0) h = lh * st.maxLines;
  const Rect r = clip(x, y, w, h, screen);
  if (r.height < lh) return static_cast<uint32_t>(-1); // no room for even one line
  target.text(r, s, st);
  return 0;
}

uint32_t drawBitmap(DrawTarget& target, View screen, int32_t x, int32_t y, int32_t w, int32_t h, const uint8_t* data,
                    uint32_t dataLen, uint32_t bw, uint32_t bh, uint32_t format, uint32_t mode, uint32_t color,
                    uint32_t rotation) {
  const Rect r = clip(x, y, w, h, screen);
  // Reject anything the target would have to guess about: unknown formats, a
  // buffer shorter than the pixels it claims, or a rect that leaves the screen.
  if (format > DSN_BITMAP_BW || mode > DSN_BITMAP_STRETCH || rotation > 3 || bw == 0 || bh == 0 ||
      static_cast<uint64_t>((bw + 7) / 8) * bh > dataLen || r.width != w || r.height != h)
    return static_cast<uint32_t>(-1);
  BitmapRef ref;
  ref.data = data;
  ref.width = static_cast<uint16_t>(bw);
  ref.height = static_cast<uint16_t>(bh);
  ref.format = format == DSN_BITMAP_MASK ? BitmapFormat::Mask1 : BitmapFormat::BW1;
  ref.progmem = false;
  target.bitmap(r, ref, mode == DSN_BITMAP_STRETCH ? BitmapMode::Stretch : BitmapMode::Center, paint(color),
                static_cast<Rotation>(rotation));
  return 0;
}

using Interactions = InteractionBuffer<kUiMaxInteractions>;  // the app's per-pass hit table

// The device's selection-row style (the LightPill theme the firmware ships):
// the selected row is a light-gray dither fill with black ink, a disabled row
// dims its label. Passed to the real fui components so an app's rows match the
// rest of the device. ponytail: this is the one shipping theme; plumb it from
// the firmware if a second ever ships.
StyleSet listRowStyles() {
  StyleSet s;
  s.explicitlySet = true;
  s.normal.background = Paint::solid(Color::White);
  s.normal.foreground = Paint::solid(Color::Black);
  s.selected.background = Paint::dither(Color::LightGray);
  s.selected.foreground = Paint::solid(Color::Black);
  s.active = s.selected;
  s.disabled.background = Paint::solid(Color::White);
  s.disabled.foreground = Paint::dither(Color::LightGray);
  return s;
}

// One menu row, drawn by the real fui setting-row component: the selected row
// gets the device's selection highlight, a disabled row is dimmed and drops its
// chevron, an enabled row can show a trailing ">" marking that it opens another
// screen. Non-interactive (the app owns layout, scrolling and tap math); it
// just draws the row so a menu matches the system UI. (list.h is the
// interactive, scrolling list — add it when an app needs routing + a viewport.)
uint32_t drawListRow(DrawTarget& target, const DeviceContext& device, Interactions& ib, View screen, int32_t x,
                     int32_t y, int32_t w, int32_t h, const char* label, uint32_t flags) {
  const Rect r = clip(x, y, w, h, screen);
  if (r.empty()) return static_cast<uint32_t>(-1);
  InputSnapshot none{};
  Frame<kUiMaxInteractions> frame(target, device, none, ib);
  SettingRowProps props;
  props.label = label;
  props.styles = listRowStyles();
  props.enabled = !(flags & DSN_ROW_DISABLED);
  props.state = (flags & DSN_ROW_SELECTED) ? StateSelected : StateNormal;
  props.drawChevron = (flags & DSN_ROW_CHEVRON) && props.enabled;
  settingRow(frame, r, props);  // action stays NO_ACTION: draws, registers no hit
  return 0;
}

// Both LE, so a straight copy of four int16s matches the guest dsn_rect_t.
void putRect(uint8_t* out, Rect r) {
  const int16_t v[4] = {r.x, r.y, r.width, r.height};
  memcpy(out, v, sizeof v);
}

// One cell of a cols×rows grid inside (x,y,w,h), via the firmware's own layout
// engine (layoutTree Grid). Writes the cell's rect; -1 on a span out of range.
uint32_t layoutGridCell(int32_t x, int32_t y, int32_t w, int32_t h, uint32_t cols, uint32_t rows, uint32_t gap,
                        uint32_t col, uint32_t row, uint32_t colspan, uint32_t rowspan, uint8_t* out, uint32_t outLen) {
  if (outLen < 8) return static_cast<uint32_t>(-1);
  LayoutNode child;
  child.id = "c";
  child.column = static_cast<uint8_t>(col);
  child.row = static_cast<uint8_t>(row);
  child.columnSpan = static_cast<uint8_t>(colspan);
  child.rowSpan = static_cast<uint8_t>(rowspan);
  LayoutNode grid;
  grid.kind = LayoutKind::Grid;
  grid.columns = static_cast<uint8_t>(cols);
  grid.rows = static_cast<uint8_t>(rows);
  grid.gap = static_cast<int16_t>(gap);
  grid.children = &child;
  grid.childCount = 1;
  Rect cell{};
  bool got = false;
  layoutTree(grid, Rect{static_cast<int16_t>(x), static_cast<int16_t>(y), static_cast<int16_t>(w),
                        static_cast<int16_t>(h)},
             [&](const char*, Rect r) { cell = r; got = true; });  // engine rejects a bad span by not emitting
  if (!got) return static_cast<uint32_t>(-1);
  putRect(out, cell);
  return 0;
}

// Split (x,y,w,h) along `axis` into slots sized by the `lengths` blob (4 bytes
// each: u8 kind, i16 value LE, u8 flex), via the engine's layoutLinear. Writes
// one dsn_rect_t per slot; returns the slot count, or -1 on bad/too-small args.
uint32_t layoutSplit(int32_t x, int32_t y, int32_t w, int32_t h, uint32_t axis, uint32_t gap, const uint8_t* lengths,
                     uint32_t lenBytes, uint8_t* out, uint32_t outLen) {
  const uint32_t count = lenBytes / 4;
  if (count == 0 || count > 64 || outLen < count * 8) return static_cast<uint32_t>(-1);
  layoutLinear(
      Rect{static_cast<int16_t>(x), static_cast<int16_t>(y), static_cast<int16_t>(w), static_cast<int16_t>(h)},
      axis ? Axis::Column : Axis::Row, static_cast<int16_t>(gap), static_cast<uint8_t>(count),
      [&](uint8_t i) {
        const uint8_t* p = lengths + i * 4;
        const int16_t value = static_cast<int16_t>(p[1] | p[2] << 8);
        return p[0] == DSN_LEN_FIXED ? LayoutLength::fixed(value) : LayoutLength::flexible(p[3] ? p[3] : 1, value);
      },
      [&](uint8_t i, Rect r) { putRect(out + i * 8, r); });
  return count;
}

// ---- Interactive widgets (the real fui components) --------------------------
// Each draws one component and registers its hit rect in the host's interaction
// buffer, so ui_event() can route a tap or button-nav to the component's action
// the way the firmware's own screens do. The app still lays widgets out and
// calls ui_begin()/ui_event(); it does not do its own hit-testing.
uint32_t drawButton(DrawTarget& target, const DeviceContext& device, Interactions& ib, View screen, int32_t x,
                    int32_t y, int32_t w, int32_t h, const char* label, uint32_t action, uint32_t flags) {
  const Rect r = clip(x, y, w, h, screen);
  if (r.empty()) return static_cast<uint32_t>(-1);
  InputSnapshot none{};  // drawing registers hits + reads focus; routing is separate
  Frame<kUiMaxInteractions> frame(target, device, none, ib);
  ButtonProps props;
  props.label = label;
  props.action = static_cast<ActionId>(action);
  props.enabled = !(flags & DSN_W_DISABLED);
  button(frame, r, props);
  return 0;
}

uint32_t drawCheckbox(DrawTarget& target, const DeviceContext& device, Interactions& ib, View screen, int32_t x,
                      int32_t y, int32_t w, int32_t h, const char* label, uint32_t action, uint32_t flags) {
  const Rect r = clip(x, y, w, h, screen);
  if (r.empty()) return static_cast<uint32_t>(-1);
  InputSnapshot none{};
  Frame<kUiMaxInteractions> frame(target, device, none, ib);
  CheckboxProps props;
  props.label = label;
  props.action = static_cast<ActionId>(action);
  props.checked = flags & DSN_W_CHECKED;
  props.enabled = !(flags & DSN_W_DISABLED);
  checkbox(frame, r, props);
  return 0;
}

uint32_t drawToggle(DrawTarget& target, const DeviceContext& device, Interactions& ib, View screen, int32_t x,
                    int32_t y, int32_t w, int32_t h, uint32_t action, uint32_t flags) {
  const Rect r = clip(x, y, w, h, screen);
  if (r.empty()) return static_cast<uint32_t>(-1);
  InputSnapshot none{};
  Frame<kUiMaxInteractions> frame(target, device, none, ib);
  ToggleProps props;
  props.action = static_cast<ActionId>(action);
  props.checked = flags & DSN_W_CHECKED;  // "on"
  props.enabled = !(flags & DSN_W_DISABLED);
  toggle(frame, r, props);
  return 0;
}

uint32_t drawProgress(DrawTarget& target, const DeviceContext& device, Interactions& ib, View screen, int32_t x,
                      int32_t y, int32_t w, int32_t h, uint32_t permille) {
  const Rect r = clip(x, y, w, h, screen);
  if (r.empty()) return static_cast<uint32_t>(-1);
  InputSnapshot none{};
  Frame<kUiMaxInteractions> frame(target, device, none, ib);
  ProgressBarProps props;
  props.value = permille > 1000 ? 1000 : static_cast<int32_t>(permille);
  props.max = 1000;
  props.border = Paint::solid(Color::Black);
  props.borderWidth = 1;
  progressBar(frame, r, props);
  return 0;
}

// ---- Capability sanitizers. Every app value reaches a Services method only
// ---- through one of these. Each rejects bad input rather than trusting it. ----

Services* svc(FreeInkHost& self) { return self.services(); }

// Copy a guest path (raw bytes) into `buf` with the storage path rules: it must
// start with '/', hold no empty / "." / ".." component and no control bytes,
// and fit kPathMax. Returns false (and leaves buf untouched) on any violation.
bool guestPath(const uint8_t* p, uint32_t len, char* buf) {
  if (!p || len == 0 || len > kPathMax || p[0] != '/') return false;
  for (uint32_t i = 0; i < len; i++)
    if (p[i] < 0x20 || p[i] == 0x7f) return false;
  // No "." or ".." path component (between slashes or at the ends).
  uint32_t start = 1;
  for (uint32_t i = 1; i <= len; i++) {
    if (i == len || p[i] == '/') {
      const uint32_t n = i - start;
      if (n == 0 && i != len) return false;                       // empty component (// )
      if (n == 1 && p[start] == '.') return false;                // .
      if (n == 2 && p[start] == '.' && p[start + 1] == '.') return false; // ..
      start = i + 1;
    }
  }
  memcpy(buf, p, len);
  buf[len] = '\0';
  return true;
}

int svcDeviceId(FreeInkHost& self, uint8_t* out, uint32_t cap) {
  return svc(self) ? svc(self)->deviceId(reinterpret_cast<char*>(out), cap) : DSN_ERR_UNAVAILABLE;
}

// One entry for every storage op; `mode` picks it. `out`/`outLen` is the app's
// output buffer (read buffer for 'w'/'k'); `expires` only used by 'k'.
int svcFs(FreeInkHost& self, char mode, const uint8_t* path, uint32_t pathLen, uint8_t* out, uint32_t outLen,
          uint32_t, uint32_t expires) {
  Services* s = svc(self);
  if (!s) return DSN_ERR_UNAVAILABLE;
  char pbuf[kPathMax + 1];
  if (mode != 'a' && !guestPath(path, pathLen, pbuf)) return DSN_ERR_INVALID;
  switch (mode) {
    case 'r': return s->fsRead(pbuf, out, outLen);
    case 'w': return s->fsWrite(pbuf, out, outLen);
    case 'l': return s->fsList(pbuf, reinterpret_cast<char*>(out), outLen);
    case 'd': return s->fsDelete(pbuf);
    case 'a': return s->fsAppDir(reinterpret_cast<char*>(out), outLen);
    case 'k': return s->keystore(pbuf, out, outLen, expires);
    default: return DSN_ERR_INVALID;
  }
}

int svcHash(FreeInkHost& self, int algo, const uint8_t* data, uint32_t n, uint8_t* out, uint32_t cap) {
  if (algo != DSN_HASH_SHA1 && algo != DSN_HASH_SHA256) return DSN_ERR_INVALID;
  return svc(self) ? svc(self)->hash(algo, data, n, out, cap) : DSN_ERR_UNAVAILABLE;
}

int svcAes(FreeInkHost& self, int flags, const uint8_t* keyiv, uint32_t kl, const uint8_t* data, uint32_t n,
           uint8_t* out, uint32_t cap) {
  if (kl != 32) return DSN_ERR_INVALID;  // key16 || iv16
  // CBC input must be block-aligned except when padding is added on encrypt.
  const bool blockAligned = (n & 15) == 0;
  const bool padEncrypt = (flags & DSN_AES_ENCRYPT) && !(flags & DSN_AES_NOPAD);
  if (!blockAligned && !padEncrypt) return DSN_ERR_INVALID;
  return svc(self) ? svc(self)->aesCbc(flags, keyiv, data, n, out, cap) : DSN_ERR_UNAVAILABLE;
}

// RSA encrypt/decrypt/sign share one entry; `op` picks it.
int svcRsa(FreeInkHost& self, char op, int flags, const uint8_t* key, uint32_t kl, const uint8_t* data, uint32_t n,
           uint8_t* out, uint32_t cap) {
  Services* s = svc(self);
  if (!s) return DSN_ERR_UNAVAILABLE;
  switch (op) {
    case 'e': return s->rsaEncrypt(key, kl, data, n, out, cap);
    case 'd': return s->rsaDecrypt(flags, key, kl, data, n, out, cap);
    case 's': return s->rsaSign(flags, key, kl, data, n, out, cap);
    default: return DSN_ERR_INVALID;
  }
}

int svcPkcs12(FreeInkHost& self, const uint8_t* p12, uint32_t n, const char* pass, uint8_t* out, uint32_t cap) {
  if (!pass) return DSN_ERR_INVALID;
  return svc(self) ? svc(self)->pkcs12Parse(p12, n, pass, out, cap) : DSN_ERR_UNAVAILABLE;
}

int svcSecure(FreeInkHost& self, int wrap, const uint8_t* in, uint32_t n, uint8_t* out, uint32_t cap) {
  Services* s = svc(self);
  if (!s) return DSN_ERR_UNAVAILABLE;
  return wrap ? s->secureWrap(in, n, out, cap) : s->secureUnwrap(in, n, out, cap);
}

int svcClock(FreeInkHost& self, uint8_t* out, uint32_t cap) {
  if (cap < 8) return DSN_ERR_TOO_BIG;
  return svc(self) ? svc(self)->clock(out) : DSN_ERR_UNAVAILABLE;
}

int svcAccel(FreeInkHost& self, uint8_t* out, uint32_t cap) {
  if (cap < 6) return DSN_ERR_TOO_BIG;
  return svc(self) ? svc(self)->accel(out) : DSN_ERR_UNAVAILABLE;
}

int svcFontLoad(FreeInkHost& self, const uint8_t* path, uint32_t pathLen, int size) {
  Services* s = svc(self);
  if (!s) return DSN_ERR_UNAVAILABLE;
  char pbuf[kPathMax + 1];
  if (!guestPath(path, pathLen, pbuf) || size < 4 || size > 400) return DSN_ERR_INVALID;
  return s->fontLoad(pbuf, size);  // DSN_FONT_CUSTOM on success
}

int svcProgress(FreeInkHost& self, const uint8_t* path, uint32_t pathLen) {
  Services* s = svc(self);
  if (!s) return DSN_ERR_UNAVAILABLE;
  char pbuf[kPathMax + 1];
  if (!guestPath(path, pathLen, pbuf)) return DSN_ERR_INVALID;
  return s->bookProgress(pbuf);
}

} // namespace

uint32_t FreeInkHost::nextRandom() {
  if (random_) return random_();
  uint32_t s = fallbackSeed_; // xorshift32
  s ^= s << 13, s ^= s >> 17, s ^= s << 5;
  return fallbackSeed_ = s;
}

bool FreeInkHost::load(uint8_t* mem, uint32_t len, uint32_t memCap) {
  refresh_ = false;
  state_ = dvm_load(&vm_, mem, len, memCap) ? State::Running : State::Faulted;
  return state_ == State::Running;
}

void FreeInkHost::randomBytesInto(uint8_t* out, uint32_t n) {
  if (services_ && services_->randomBytes(out, n)) return;
  for (uint32_t i = 0; i < n; i++) out[i] = static_cast<uint8_t>(nextRandom());
}

void FreeInkHost::finishAsync(int code) {
  vm_.x[10] = static_cast<uint32_t>(code);
  asyncPending_ = false;
}

uint32_t FreeInkHost::routeUi(const freeink::ui::InputSnapshot& input) {
  using namespace freeink::ui;
  // dessen delivers semantic PREV/NEXT; drive widget focus with them (page-turn
  // style prev/next dispatch isn't used here), keep touch/confirm/back as-is.
  InputSnapshot ri = input;
  ri.focusPrev = input.focusPrev || input.prev;
  ri.focusNext = input.focusNext || input.next;
  ri.prev = ri.next = false;
  const int16_t before = interactions_.focusedIndex();
  const ActionEvent e = interactions_.route(ri);
  if (e.action != NO_ACTION)  // a widget fired: action in low 16, value in high 16
    return static_cast<uint32_t>(e.action) | static_cast<uint32_t>(static_cast<uint16_t>(e.value)) << 16;
  if (interactions_.focusedIndex() != before)  // focus moved: ask the app to repaint
    return static_cast<uint32_t>(DSN_UI_REDRAW) << 16;
  if (input.back) return static_cast<uint32_t>(DSN_UI_BACK) << 16;  // back, no widget handler
  return 0;
}

State FreeInkHost::step(DrawTarget& target, const DeviceContext& device, const InputSnapshot& input, uint32_t budget) {
  if (state_ == State::Exited || state_ == State::Faulted) return state_;
  if (state_ == State::Busy) {
    const int code = services_ ? services_->poll() : DSN_ERR_IO;
    if (code == kPending) return state_; // still working; try again next tick
    finishAsync(code);                   // result ready: resume the VM below
  } else if (state_ == State::Waiting) {
    const uint32_t ev = uiWait_ ? routeUi(input) : eventOf(input, viewOf(device));
    if (!ev) return state_;
    vm_.x[10] = ev; // wait_event's / ui_event's return value
    uiWait_ = false;
  }
  target_ = &target;
  device_ = device;
  const int rc = dvm_run(&vm_, budget, onEcall, this);
  target_ = nullptr;
  state_ = rc == kExit     ? State::Exited
           : rc == kWait   ? State::Waiting
           : rc == kBusy   ? State::Busy
           : rc == kLaunch ? State::Launch
           : rc < 0        ? State::Faulted
                           : State::Running;
  return state_;
}

int FreeInkHost::requestLaunch(int kind, const uint8_t* path, uint32_t pathLen) {
  launch_ = kind == 1 ? Launch::Reader : kind == 2 ? Launch::Settings : kind == 3 ? Launch::Browser : Launch::None;
  launchPath_[0] = '\0';
  if (launch_ == Launch::None) {
    vm_.x[10] = static_cast<uint32_t>(DSN_ERR_INVALID);
    return 0;
  }
  if (launch_ == Launch::Reader) {
    if (!path || pathLen == 0 || pathLen >= sizeof launchPath_) {
      launch_ = Launch::None;
      vm_.x[10] = static_cast<uint32_t>(DSN_ERR_INVALID);
      return 0;
    }
    memcpy(launchPath_, path, pathLen);
    launchPath_[pathLen] = '\0';
  }
  return kLaunch;
}

// --- async entry points, called from the generated dispatch. Each either
// --- completes synchronously (sets x[10], returns 0) or parks (returns kBusy).
namespace {
inline uint32_t u(int code) { return static_cast<uint32_t>(code); }
}

int FreeInkHost::requestLaunchHome(int dest) {
  if (dest < 0 || dest > 5) {
    vm_.x[10] = static_cast<uint32_t>(DSN_ERR_INVALID);
    return 0;
  }
  launch_ = Launch::Home;
  launchHomeDest_ = dest;
  return kLaunch;
}

int FreeInkHost::beginNet(const uint8_t* head, uint32_t headLen, const uint8_t* body, uint32_t bodyLen, uint8_t* resp,
                          uint32_t respCap) {
  if (!services_) { vm_.x[10] = u(DSN_ERR_UNAVAILABLE); return 0; }
  char hbuf[1025];
  if (headLen > sizeof hbuf - 1) { vm_.x[10] = u(DSN_ERR_INVALID); return 0; }
  memcpy(hbuf, head, headLen);
  hbuf[headLen] = '\0';
  const int code = services_->netStart(hbuf, body, bodyLen, resp, respCap);
  if (code == kPending) { asyncPending_ = true; return kBusy; }
  vm_.x[10] = u(code);
  return 0;
}

int FreeInkHost::beginFetch(const uint8_t* url, uint32_t urlLen, const uint8_t* dest, uint32_t destLen) {
  if (!services_) { vm_.x[10] = u(DSN_ERR_UNAVAILABLE); return 0; }
  char ubuf[1025], dbuf[kPathMax + 1];
  if (urlLen > sizeof ubuf - 1) { vm_.x[10] = u(DSN_ERR_INVALID); return 0; }
  if (!guestPath(dest, destLen, dbuf)) { vm_.x[10] = u(DSN_ERR_INVALID); return 0; }
  memcpy(ubuf, url, urlLen);
  ubuf[urlLen] = '\0';
  const int code = services_->fetchStart(ubuf, dbuf);
  if (code == kPending) { asyncPending_ = true; return kBusy; }
  vm_.x[10] = u(code);
  return 0;
}

int FreeInkHost::beginTextInput(const char* prompt, uint8_t* out, uint32_t cap, int flags) {
  if (!services_) { vm_.x[10] = u(DSN_ERR_UNAVAILABLE); return 0; }
  const int code = services_->textInputStart(prompt, flags, reinterpret_cast<char*>(out), cap);
  if (code == kPending) { asyncPending_ = true; return kBusy; }
  vm_.x[10] = u(code);
  return 0;
}

int FreeInkHost::beginWifi() {
  if (!services_) { vm_.x[10] = u(DSN_ERR_UNAVAILABLE); return 0; }
  const int code = services_->wifiConnectStart();
  if (code == kPending) { asyncPending_ = true; return kBusy; }
  vm_.x[10] = u(code);
  return 0;
}

int FreeInkHost::beginKeygen(uint32_t bits, uint8_t* pub, uint32_t pubCap, uint8_t* priv, uint32_t privCap) {
  if (!services_) { vm_.x[10] = u(DSN_ERR_UNAVAILABLE); return 0; }
  if (bits != 1024 && bits != 2048) { vm_.x[10] = u(DSN_ERR_INVALID); return 0; }
  const int code = services_->keygenStart(bits, pub, pubCap, priv, privCap);
  if (code == kPending) { asyncPending_ = true; return kBusy; }
  vm_.x[10] = u(code);
  return 0;
}

int FreeInkHost::onEcall(dvm_t* vm, void* selfPtr) {
  auto& self = *static_cast<FreeInkHost*>(selfPtr);
  DrawTarget& target = *self.target_;
  const View screen = viewOf(self.device_);
  uint32_t* a = &vm->x[10];

  switch (vm->x[17]) {
#include "dessen_dispatch.inc"
    default: a[0] = static_cast<uint32_t>(-1); // unknown call
  }
  return 0;
}

} // namespace dessen
