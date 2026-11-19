// Minimal freestanding runtime for dessen apps. Apps build with -nostdlib, but
// the compiler still emits calls to a few libc symbols (struct copies, array
// initialisers, large shifts). Provide just those. -fno-tree-loop-distribute-
// patterns (in APPFLAGS) keeps these loops from being turned back into calls to
// themselves.
#include <stddef.h>

void *memcpy(void *dst, const void *src, size_t n) {
  unsigned char *d = dst;
  const unsigned char *s = src;
  while (n--) *d++ = *s++;
  return dst;
}
void *memset(void *dst, int c, size_t n) {
  unsigned char *d = dst;
  while (n--) *d++ = (unsigned char)c;
  return dst;
}
void *memmove(void *dst, const void *src, size_t n) {
  unsigned char *d = dst;
  const unsigned char *s = src;
  if (d < s) while (n--) *d++ = *s++;
  else { d += n; s += n; while (n--) *--d = *--s; }
  return dst;
}
