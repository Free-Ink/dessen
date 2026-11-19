// Minimal Arduino.h for the desktop/host build of the FreeInk UI library.
// The UI sources touch only these few Arduino conveniences; everything else
// comes from the C++ standard library. Device builds use the real header.
#pragma once
#include <algorithm>
#include <cmath>
#include <cstdint>
using std::abs;
using std::max;
using std::min;
inline long map(long x, long inMin, long inMax, long outMin, long outMax) {
  if (inMax == inMin) return outMin;
  return (x - inMin) * (outMax - outMin) / (inMax - inMin) + outMin;
}
