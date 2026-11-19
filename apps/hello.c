#include "dessen.h"

int main(void) {
  dsn_clear();
  dsn_text(40, 60, "Hello from dessen");
  dsn_text(40, 100, "Press any button");
  dsn_refresh();
  return dsn_wait_event(); // exit code = the button pressed
}
