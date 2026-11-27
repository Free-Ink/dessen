#include "dessen.h"

int main(void) {
  unsigned char buf[DSN_STATS_SUMMARY_BYTES] = {0};
  const unsigned char empty[1] = {0};
  if (dsn_reading_stats_start("/../x", 5, 0) != DSN_ERR_INVALID) return 1;
  if (dsn_reading_stats_start("bad", 3, 0) != DSN_ERR_INVALID) return 2;
  if (dsn_reading_stats_read(99, 0, buf, sizeof buf) != DSN_ERR_INVALID) return 3;
  if (dsn_reading_stats_read(DSN_STATS_DAYS, 1, buf, 17) != DSN_ERR_INVALID) return 4;
  if (dsn_reading_stats_read(DSN_STATS_SUMMARY, 0, buf, 191) != DSN_ERR_TOO_BIG) return 5;
  int version = dsn_reading_stats_read(DSN_STATS_CAPABILITY, 0, buf, 0);
  if (version == DSN_ERR_UNAVAILABLE) return 0;
  if (version != DSN_STATS_VERSION) return 6;
  if (dsn_reading_stats_start(empty, 0, 20000) != 0) return 7;
  if (dsn_reading_stats_read(DSN_STATS_SUMMARY, 0, buf, sizeof buf) != sizeof buf ||
      buf[0] != DSN_STATS_VERSION || buf[48] != 60) return 8;
  if (dsn_reading_stats_read(DSN_STATS_DAYS, 20000, buf, 16) != 16 || buf[0] != 60) return 9;
  if (dsn_reading_stats_read(DSN_STATS_BOOKS, 0, buf, 16) != 16 || buf[0] != 0x42) return 10;
  if (dsn_reading_stats_read(DSN_STATS_BOOK_INFO, 0, buf, 100) != 100 || buf[20] != 'B') return 11;
  if (dsn_reading_stats_read(DSN_STATS_RELEASE, 0, buf, 0) != 0) return 12;
  if (dsn_reading_stats_read(DSN_STATS_SUMMARY, 0, buf, sizeof buf) != DSN_ERR_UNAVAILABLE) return 13;
  if (dsn_reading_stats_start("0123456789abcdef0123456789abcdef", 32, 0) != DSN_STATS_NO_HISTORY) return 14;
  return 0;
}
