// Tries to escape the sandbox. The host must report a memory fault.
#include "dessen.h"

int main(void) {
  *(volatile int *)0x10000000 = 42;
  return 0;
}
