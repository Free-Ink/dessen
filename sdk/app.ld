/* One flat block: code, data, bss, then stack growing down from the top. */
MEMORY { ram (rwx) : ORIGIN = 0, LENGTH = DEFINED(APP_MEM) ? APP_MEM : 64K }
ENTRY(_start)
SECTIONS {
  .text   : { KEEP(*(.header)) *(.text._start) *(.text .text.*) } > ram
  .rodata : { *(.rodata .rodata.* .srodata .srodata.*) } > ram
  .data   : { *(.data .data.* .sdata .sdata.*) } > ram
  .bss    : { *(.bss .bss.* .sbss .sbss.* COMMON) } > ram
  __mem_size = LENGTH(ram);
  /DISCARD/ : { *(.eh_frame) *(.comment) *(.riscv.attributes) *(.note.*) }
}
