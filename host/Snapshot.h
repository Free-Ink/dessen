// The desktop runner's picture of the screen: FreeInk's own framebuffer
// target (DisplayTarget: real glyphs, dithered grays, bitmaps), and the last
// frame the app refreshed written as a 1-bit PNG. DESSEN_SNAPSHOT=<file.png>
// turns it on; tools/screen_test.py compares the pictures.
#pragma once
#include <FreeInkUIDisplayTarget.h>
#include <zlib.h>

#include <cstdint>
#include <cstdio>
#include <vector>

// The fonts the pictures are drawn with: FreeInk's bundled Noto Sans for body
// text (34 px lines, the size of a device's body font) and its gallery fonts
// for the small (18 px) and title (43 px) slots. Not the firmware's own fonts,
// but real glyph metrics: text that stops fitting shows.
#include <gallery_font.h>          // $(FREEINK_SDK)/libs/ui/FreeInkUI/tools
#include <keyboard_gallery_font.h>

struct Snapshot {
  int16_t width, height, rowBytes;
  std::vector<uint8_t> fb, shown;
  freeink::ui::DisplayTarget target;

  // A white page the size of the screen, drawn as is (no rotation).
  Snapshot(int16_t w, int16_t h)
      : width(w), height(h), rowBytes(static_cast<int16_t>((w + 7) / 8)), fb(static_cast<size_t>(rowBytes) * h, 0xFF),
        target(fb.data(), w, h, rowBytes, freeink::ui::Orientation::LandscapeCounterClockwise) {
    target.setFont(0, freeink::ui::kNotoSansSmallFont);
    target.setFont(1, freeink::ui::kNotoSansFont);
    target.setFont(2, freeink::ui::kKeyboardGalleryFont);
  }

  void refreshed() { shown = fb; }

  // The last refreshed frame as a PNG: 1-bit grayscale, a set bit white, as in
  // the framebuffer. False when nothing was refreshed or the file fails.
  bool write(const char* path) const {
    if (shown.empty()) return false;
    std::vector<uint8_t> raw;
    raw.reserve(shown.size() + height);
    for (int16_t y = 0; y < height; ++y) {
      raw.push_back(0);  // filter: none
      raw.insert(raw.end(), shown.begin() + y * rowBytes, shown.begin() + (y + 1) * rowBytes);
    }
    uLongf packedLen = compressBound(raw.size());
    std::vector<uint8_t> packed(packedLen);
    if (compress2(packed.data(), &packedLen, raw.data(), raw.size(), 9) != Z_OK) return false;
    packed.resize(packedLen);
    FILE* f = fopen(path, "wb");
    if (!f) return false;
    static const uint8_t magic[] = {0x89, 'P', 'N', 'G', '\r', '\n', 0x1a, '\n'};
    fwrite(magic, 1, sizeof magic, f);
    const uint8_t header[] = {be(width, 3),  be(width, 2),  be(width, 1),  be(width, 0), be(height, 3), be(height, 2),
                              be(height, 1), be(height, 0), 1 /* depth */, 0 /* gray */, 0,          0,
                              0};
    chunk(f, "IHDR", header, sizeof header);
    chunk(f, "IDAT", packed.data(), packed.size());
    chunk(f, "IEND", nullptr, 0);
    return fclose(f) == 0;
  }

 private:
  static uint8_t be(uint32_t v, int byte) { return static_cast<uint8_t>(v >> (8 * byte)); }
  static void chunk(FILE* f, const char* type, const uint8_t* data, size_t len) {
    const uint8_t size[] = {be(len, 3), be(len, 2), be(len, 1), be(len, 0)};
    fwrite(size, 1, 4, f);
    fwrite(type, 1, 4, f);
    if (len) fwrite(data, 1, len, f);
    uLong crc = crc32(0, reinterpret_cast<const Bytef*>(type), 4);
    if (len) crc = crc32(crc, data, static_cast<uInt>(len));
    const uint8_t sum[] = {be(crc, 3), be(crc, 2), be(crc, 1), be(crc, 0)};
    fwrite(sum, 1, 4, f);
  }
};
