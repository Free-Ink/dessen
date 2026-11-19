// CPU-bound loop for measuring interpreter speed: count primes below 30000.
#include "dessen.h"

int main(void) {
  int count = 0;
  for (int n = 2; n < 30000; n++) {
    int prime = 1;
    for (int d = 2; d * d <= n; d++)
      if (n % d == 0) {
        prime = 0;
        break;
      }
    count += prime;
  }
  return count == 3245 ? 0 : 1;
}
