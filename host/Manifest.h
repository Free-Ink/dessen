// The "requires" field of an app's manifest: what a board must have for the
// app to be usable there, e.g. `"requires": ["touch"]`. Absent or empty means
// the app works with whatever input the board has, the norm for every app that
// offers both a touch path and a button path.
//
// The manifest is read before the app runs, so this is the launcher's
// question ("offer this app here?"). How the app adapts once running (hints,
// navigation) is dsn_device's answer, not this one's.
#pragma once

#include <cstring>
#include <string>

namespace dessen {

enum Need : unsigned {
  NeedTouch = 1u << 0,
  NeedButtons = 1u << 1,
  // A requirement this host does not know: it cannot promise to meet it.
  NeedUnknown = 1u << 31,
};

// Reads `"requires": ["...", ...]` from manifest text into `needs`. Returns
// false when the field is there but malformed (not an array of plain words).
// A plain search for the key is sound: a quote inside any JSON string is
// escaped, so `"requires"` can only be the key itself.
inline bool parseRequires(const std::string& json, unsigned& needs) {
  needs = 0;
  size_t at = json.find("\"requires\"");
  if (at == std::string::npos) return true;
  at += 10;
  auto skip = [&] {
    while (at < json.size() && strchr(" \t\r\n", json[at])) at++;
  };
  auto take = [&](char c) {
    skip();
    if (at >= json.size() || json[at] != c) return false;
    at++;
    return true;
  };
  if (!take(':') || !take('[')) return false;
  if (take(']')) return true;
  do {
    if (!take('"')) return false;
    const size_t end = json.find('"', at);
    if (end == std::string::npos) return false;
    const std::string word = json.substr(at, end - at);
    if (word.empty() || word.find('\\') != std::string::npos) return false;
    needs |= word == "touch" ? NeedTouch : word == "buttons" ? NeedButtons : NeedUnknown;
    at = end + 1;
  } while (take(','));
  return take(']');
}

// Whether a board with this input meets every need.
inline bool meets(unsigned needs, bool hasTouch, bool hasButtons) {
  return !(needs & NeedUnknown) && (!(needs & NeedTouch) || hasTouch) && (!(needs & NeedButtons) || hasButtons);
}

}  // namespace dessen
