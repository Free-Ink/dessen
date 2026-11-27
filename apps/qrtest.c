// Exercise the QR capability boundary with a desktop host or a mock encoder.
#include "dessen.h"

int main(void) {
  unsigned char data[4] = {'a', 0, 'b', 0};
  unsigned char out[68] = {0};
  if (dsn_qr_bitmap(data, 0, out, sizeof out) != DSN_ERR_INVALID) return 1;
  if (dsn_qr_bitmap(data, 3, out, 3) != DSN_ERR_TOO_BIG) return 2;
  if (dsn_qr_bitmap(data, 3, data, sizeof data) != DSN_ERR_INVALID) return 3;
  if (dsn_qr_bitmap((const unsigned char*)0x7fffffffu, 3, out, sizeof out) != DSN_ERR_INVALID) return 4;
  int n = dsn_qr_bitmap(data, 3, out, sizeof out);
  if (n == DSN_ERR_UNAVAILABLE) return 0;
  // Mock encoder: 21 by 21, 3 bytes per row, plus the four-byte header.
  if (n != 67 || out[0] != 21 || out[1] != 0 || out[2] != 21 || out[3] != 0 ||
      out[4] != 0x80 || out[67] != 0) return 5;
  out[0] = 0x55;
  if (dsn_qr_bitmap(data, 3, out, 66) != DSN_ERR_TOO_BIG || out[0] != 0x55) return 6;
  return 0;
}
