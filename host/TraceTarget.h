// A DrawTarget that records every drawing call as a line of text. The desktop
// runner prints them; the tests assert on them. With `also` set (the runner's
// DESSEN_SNAPSHOT), every call is drawn there too and text is measured with
// its fonts, so the trace and the picture agree.
#pragma once
#include <FreeInkUICore.h>

#include <algorithm>
#include <cstdio>
#include <string>
#include <vector>

struct TraceTarget final : freeink::ui::DrawTarget {
  using Rect = freeink::ui::Rect;
  using Paint = freeink::ui::Paint;
  using Point = freeink::ui::Point;

  std::vector<std::string> ops;
  int lines = 0;
  bool echo = false;

  void add(const std::string& op) {
    if (echo) puts(op.c_str());
    else ops.push_back(op);
  }
  static std::string at(Rect r) {
    return std::to_string(r.x) + "," + std::to_string(r.y) + " " + std::to_string(r.width) + "x" + std::to_string(r.height);
  }

  freeink::ui::DrawTarget* also = nullptr;

  freeink::ui::Size measureText(freeink::ui::FontId font, const char* text, freeink::ui::TextStyle style) const override {
    if (also) return also->measureText(font, text, style);
    return freeink::ui::Size{static_cast<int16_t>(8 * strlen(text)), 16};
  }
  int16_t lineHeight(freeink::ui::FontId font) const override { return also ? also->lineHeight(font) : 16; }
  Rect inkBounds(freeink::ui::FontId font, const char* text, freeink::ui::TextStyle style) const override {
    return also ? also->inkBounds(font, text, style) : DrawTarget::inkBounds(font, text, style);
  }
  Rect clip_{0, 0, 32767, 32767};
  Rect clipRect() const override { return clip_; }
  bool setClipRect(Rect r) override {
    clip_ = r;
    if (also) also->setClipRect(r);
    add("clip " + at(r));
    return true;
  }
  // detail: print every argument, in a fixed order, for the screen tests
  // (tools/screen_test.py, DESSEN_TRACE=detail). Off, the short format the
  // tests in freeink/test_host.cpp and the Makefile greps read.
  bool detail = false;
  static std::string n(int v) { return std::to_string(v); }
  static std::string pt(Point p) { return n(p.x) + "," + n(p.y); }
  static std::string ink(Paint p) {
    return "c" + n(static_cast<int>(p.color)) + (p.kind == freeink::ui::PaintKind::Dither ? " d" : "");
  }
  static std::string round(uint8_t radius, uint8_t corners) {
    return radius ? " r" + n(radius) + "/" + n(corners) : "";
  }

  void fill(Rect r, Paint p, uint8_t radius, uint8_t corners) override {
    if (also) also->fill(r, p, radius, corners);
    add("fill " + at(r) + " " + ink(p) + (detail ? round(radius, corners) : ""));
  }
  void stroke(Rect r, Paint p, uint8_t width, uint8_t radius, uint8_t corners) override {
    if (also) also->stroke(r, p, width, radius, corners);
    add("stroke " + at(r) + " w" + n(width) + (detail ? " " + ink(p) + round(radius, corners) : ""));
  }
  void line(Point from, Point to, uint8_t width, Paint p) override {
    lines++;
    if (also) also->line(from, to, width, p);
    if (detail) add("line " + pt(from) + " - " + pt(to) + " w" + n(width) + " " + ink(p));
    else if (echo) printf("line %d,%d - %d,%d\n", from.x, from.y, to.x, to.y);
  }
  void triangle(Point a, Point b, Point c, Paint p) override {
    if (also) also->triangle(a, b, c, p);
    add(detail ? "triangle " + pt(a) + " " + pt(b) + " " + pt(c) + " " + ink(p) : "triangle");
  }
  void text(Rect r, const char* text, freeink::ui::TextStyle s) override {
    if (also) also->text(r, text, s);
    if (!detail) return add("text " + n(r.x) + "," + n(r.y) + " " + text);
    static const char* const align[] = {"left", "center", "right"};
    const unsigned a = static_cast<unsigned>(s.align);
    add("text " + at(r) + " f" + n(s.font) + " " + (a < 3 ? align[a] : "?") + " c" + n(static_cast<int>(s.color)) +
        " m" + n(s.maxLines) + (s.bold ? " bold" : "") + (s.inverted ? " inv" : "") +
        (s.rotation != freeink::ui::Rotation::None ? " rot" + n(static_cast<int>(s.rotation)) : "") +
        (also ? " ink " + at(inkOf(r, text, s)) : "") + " | " + text);
  }
  // Where the lines of a text call land, as `also` lays them out (the same
  // layoutText its text() uses): the place a reader sees, for the snapshot
  // tests' element check. Empty when nothing fits.
  Rect inkOf(Rect r, const char* text, freeink::ui::TextStyle s) const {
    int x0 = 32767, y0 = 32767, x1 = -32768, y1 = -32768;
    freeink::ui::layoutText(*also, r, text, s, [&](const char*, Rect box) {
      x0 = std::min<int>(x0, box.x), x1 = std::max<int>(x1, box.x + box.width);
      y0 = std::min<int>(y0, box.y), y1 = std::max<int>(y1, box.y + box.height);
    });
    if (x1 < x0) return Rect{r.x, r.y, 0, 0};
    return Rect{static_cast<int16_t>(x0), static_cast<int16_t>(y0), static_cast<int16_t>(x1 - x0),
                static_cast<int16_t>(y1 - y0)};
  }
  void bitmap(Rect r, freeink::ui::BitmapRef b, freeink::ui::BitmapMode mode, Paint p, freeink::ui::Rotation rot) override {
    if (also) also->bitmap(r, b, mode, p, rot);
    add("bitmap " + at(r) + " src " + n(b.width) + "x" + n(b.height) +
        (detail ? " m" + n(static_cast<int>(mode)) + " " + ink(p) +
                      (rot != freeink::ui::Rotation::None ? " rot" + n(static_cast<int>(rot)) : "")
                : ""));
  }
};
