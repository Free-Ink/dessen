// Exercises the async network calls against desktop fixtures. Exits 0 on pass,
// or the failing check number. Run with DESSEN_NET_ROOT (and DESSEN_FS_ROOT for
// fetch_to_sd). The app is written exactly as it would be on a device: it does
// not know or care whether the host answered immediately or parked it.
#include "dessen.h"

static int eq(const unsigned char *a, const char *b, int n) {
  for (int i = 0; i < n; i++)
    if (a[i] != (unsigned char)b[i]) return 0;
  return 1;
}

#define CHECK(n, cond) \
  if (!(cond)) return n

int main(void) {
  // net_request: resp = u16 status (BE), u16 hdr len (BE), headers, body.
  const char *head = "GET https://example.test/hello\n";
  unsigned char resp[256];
  int hlen = 0;
  while (head[hlen]) hlen++;
  int n = dsn_net_request(head, hlen, "", 0, resp, sizeof resp);
  CHECK(1, n >= 0);
  int status = resp[0] << 8 | resp[1];
  int hdrlen = resp[2] << 8 | resp[3];
  CHECK(2, status == 200);
  const unsigned char *body = resp + 4 + hdrlen;
  CHECK(3, n == 11 && eq(body, "hello world", 11));

  // a 404 fixture still returns the body and the real status
  const char *h2 = "GET https://example.test/missing\n";
  int h2len = 0;
  while (h2[h2len]) h2len++;
  n = dsn_net_request(h2, h2len, "", 0, resp, sizeof resp);
  CHECK(4, n >= 0 && (resp[0] << 8 | resp[1]) == 404);

  // fetch_to_sd streams a URL to a file without it passing through app memory
  int got = dsn_fetch_to_sd("https://example.test/book.bin", 29, "/fetched.bin", 12);
  CHECK(5, got > 0);
  unsigned char back[64];
  CHECK(6, dsn_fs_read("/fetched.bin", 12, back, sizeof back) == got);
  return 0;
}
