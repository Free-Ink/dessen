// Drives one async call. The app cannot tell whether the host answered at once
// or parked it for several ticks; it just reads the result. The host test backs
// this with a service that deliberately parks, exercising the kBusy path.
#include "dessen.h"

int main(void) {
  unsigned char resp[64];
  int n = dsn_net_request("GET x\n", 6, "", 0, resp, sizeof resp);
  if (n != 5) return 1;
  if ((resp[0] << 8 | resp[1]) != 200) return 2;            // status
  const unsigned char *body = resp + 4;                     // hdrlen 0
  if (body[0] != 'r' || body[4] != 'y') return 3;           // "ready"
  return 0;
}
