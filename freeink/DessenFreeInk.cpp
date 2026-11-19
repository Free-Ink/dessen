#include "DessenFreeInk.h"

#include "../sdk/dessen_abi.h"

namespace dessen {
namespace {

using namespace freeink::ui;

enum { kExit = 1, kWait, kYield }; // positive stop codes for dvm_run
constexpr uint32_t kStrMax = 200;  // longest text an app can pass in one call

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
  st.font = static_cast<FontId>(font <= DSN_FONT_TITLE ? font : DSN_FONT_SMALL);
  st.align = align == DSN_ALIGN_CENTER ? TextAlign::Center : align == DSN_ALIGN_RIGHT ? TextAlign::Right : TextAlign::Left;
  st.maxLines = static_cast<uint8_t>(maxLines < 1 ? 1 : maxLines > 16 ? 16 : maxLines);
  st.bold = flags & DSN_TEXT_BOLD;
  st.inverted = flags & DSN_TEXT_INVERTED;
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

State FreeInkHost::step(DrawTarget& target, const DeviceContext& device, const InputSnapshot& input, uint32_t budget) {
  if (state_ == State::Exited || state_ == State::Faulted) return state_;
  if (state_ == State::Waiting) {
    const uint32_t ev = eventOf(input, viewOf(device));
    if (!ev) return state_;
    vm_.x[10] = ev; // wait_event's return value
  }
  target_ = &target;
  device_ = device;
  const int rc = dvm_run(&vm_, budget, onEcall, this);
  target_ = nullptr;
  state_ = rc == kExit ? State::Exited : rc == kWait ? State::Waiting : rc < 0 ? State::Faulted : State::Running;
  return state_;
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
