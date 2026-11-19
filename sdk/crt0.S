#include "dessen_abi.h"

    .section .header, "a"
    .word DSN_MAGIC
    .word __mem_size
    .word _start
    .word DSN_API_VERSION

    .section .text._start, "ax"
    .globl _start
_start:                 /* host sets sp to the top of guest memory */
    call main
    li a7, DSN_SYS_EXIT /* exit(main's return value) */
    ecall
