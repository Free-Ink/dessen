#include "dessen.h"

int main(void) {
  if (dsn_timer_set(999) != DSN_ERR_INVALID || dsn_timer_set(3600001) != DSN_ERR_INVALID) return 1;
  int enabled = dsn_timer_set(1000);
  if (enabled == DSN_ERR_UNAVAILABLE) return 0;
  if (enabled != 0) return 2;
  if (dsn_wait_event() != DSN_EV_CONFIRM) return 3; // input takes priority
  if (dsn_wait_event() != DSN_EV_TIMER) return 4;
  if (dsn_timer_set(5000) != 0) return 5;
  if (dsn_wait_event() != DSN_EV_TIMER) return 6;
  if (dsn_timer_set(0) != 0) return 7;
  if (dsn_wait_event() != DSN_EV_BACK) return 8;
  return 0;
}
