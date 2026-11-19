// Runs inside the VM; exits 0 on pass, or the number of the first failed check.
// `volatile` stops the compiler from folding the checks away.
#include "dessen.h"

static volatile int vi;
static volatile unsigned vu;
static int bss_word; // must read as zero without startup code
static signed char bytes[4] = {-1, 2, -3, 4};

static int fib(int n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }

#define CHECK(n, cond) \
  if (!(cond)) return n

int main(void) {
  CHECK(1, bss_word == 0);
  CHECK(2, fib(15) == 610); // calls, branches, stack

  vi = -7;
  CHECK(3, vi / 2 == -3 && vi % 2 == -1); // signed div/rem truncate
  CHECK(4, vi >> 1 == -4);                // arithmetic shift
  vu = 0xfffffff9u;
  CHECK(5, vu / 2 == 0x7ffffffcu && vu >> 1 == 0x7ffffffcu);

  vi = 0;
  CHECK(6, 5 / vi == -1 && 5 % vi == 5); // RISC-V div-by-zero results
  vi = -1;
  vu = 0x80000000u;
  CHECK(7, (int)vu / vi == (int)0x80000000u && (int)vu % vi == 0); // overflow

  vi = -3;
  CHECK(8, (long long)vi * 0x40000000 == -0xC0000000LL);         // mulh
  vu = 0xffffffffu;
  CHECK(9, (unsigned long long)vu * vu == 0xfffffffe00000001ULL); // mulhu

  volatile signed char *p = bytes; // sign- and zero-extending loads
  CHECK(10, p[0] == -1 && p[2] == -3 && ((volatile unsigned char *)p)[0] == 255);
  volatile short s = -2;
  CHECK(11, s == -2 && (unsigned short)s == 0xfffe);
  return 0;
}
