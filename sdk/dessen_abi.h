// dessen app ABI constants, shared by apps and hosts. The call list itself is
// generated from abi/freeink.api (see dessen_sysnums.h).
#pragma once
#include "dessen_sysnums.h"

#define DSN_MAGIC 0x314e5344u // "DSN1", little-endian
#define DSN_API_VERSION 1

// App image header, at guest address 0. The image is loaded at 0 and the rest
// of memory is zeroed, so .bss needs no startup code.
//   u32 magic, u32 mem_size (bytes requested), u32 entry, u32 api_version
#define DSN_HEADER_SIZE 16

// Calls: number in a7, arguments in a0..a6, result in a0.

#define DSN_WHITE 0
#define DSN_BLACK 1
#define DSN_LIGHT_GRAY 2
#define DSN_DARK_GRAY 3

// Fonts are the firmware's own, so text matches the rest of the device.
#define DSN_FONT_SMALL 0
#define DSN_FONT_BODY 1
#define DSN_FONT_TITLE 2

#define DSN_ALIGN_LEFT 0
#define DSN_ALIGN_CENTER 1
#define DSN_ALIGN_RIGHT 2

#define DSN_TEXT_BOLD 1
#define DSN_TEXT_INVERTED 2

#define DSN_CORNERS_ALL 15

// 1-bit bitmaps, row-major, most significant bit first.
#define DSN_BITMAP_MASK 0 // set bits are drawn in the given colour
#define DSN_BITMAP_BW 1   // set bits black, clear bits white
#define DSN_BITMAP_CENTER 0
#define DSN_BITMAP_STRETCH 1

#ifndef __ASSEMBLER__
#include <stdint.h>
// Filled in by dsn_device(). Apps draw inside the safe area, so width/height
// here (and from dsn_screen) already exclude bezels and rounded corners.
typedef struct {
  uint16_t width, height;           // drawable area
  uint16_t panel_width, panel_height;
  uint16_t inset_top, inset_right, inset_bottom, inset_left; // what was excluded
  uint16_t min_touch;               // smallest comfortable tap target, in pixels
  uint8_t has_touch, has_buttons;
  uint8_t orientation;              // 0 portrait, 1 landscape cw, 2 inverted, 3 landscape ccw
  uint8_t reserved;
} dsn_device_info;
#endif

// Events: type in the low 8 bits. A tap also carries x in bits 8-19 and y in
// bits 20-31, in the same safe-area coordinates used for drawing. Apps get semantic actions, never raw device buttons, so one app
// works on button-only and touch boards.
#define DSN_EV_CONFIRM 1
#define DSN_EV_BACK 2
#define DSN_EV_PREV 3
#define DSN_EV_NEXT 4
#define DSN_EV_TAP 5
#define DSN_EV_TYPE(e) ((e) & 0xff)
#define DSN_EV_X(e) (((e) >> 8) & 0xfff)
#define DSN_EV_Y(e) (((e) >> 20) & 0xfff)
