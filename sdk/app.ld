/* One flat block: code, data, bss, then stack growing down from the top. */
MEMORY { ram (rwx) : ORIGIN = 0, LENGTH = 64K }
ENTRY(_start)
SECTIONS {
  .text   : { KEEP(*(.header)) *(.text._start) *(.text .text.*) } > ram
  .rodata : { *(.rodata .rodata.* .srodata .srodata.*) } > ram
  .data   : { *(.data .data.* .sdata .sdata.*) } > ram
  .bss    : { *(.bss .bss.* .sbss .sbss.* COMMON) } > ram
  __mem_size = LENGTH(ram);
  /DISCARD/ : { *(.eh_frame) *(.comment) *(.riscv.attributes) *(.note.*) }
}
