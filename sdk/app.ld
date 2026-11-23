/* One flat block: code, data, bss, then stack growing down from the top.
   The app asks for APP_MEM bytes (64K by default). With --defsym=APP_STACK=<bytes>
   it asks only for what it uses, code through bss plus that much stack, and
   APP_MEM becomes the most it may grow to. An app can also define the absolute
   symbol __app_stack itself (Rust: dessen::stack!), which wins over APP_STACK,
   so each app sizes its own stack where it sizes its heap. */
MEMORY { ram (rwx) : ORIGIN = 0, LENGTH = DEFINED(APP_MEM) ? APP_MEM : 64K }
ENTRY(_start)
SECTIONS {
  .text   : { KEEP(*(.header)) *(.text._start) *(.text .text.*) } > ram
  .rodata : { *(.rodata .rodata.* .srodata .srodata.*) } > ram
  .data   : { *(.data .data.* .sdata .sdata.*) } > ram
  .bss    : { *(.bss .bss.* .sbss .sbss.* COMMON) } > ram
  __mem_size = DEFINED(__app_stack) ? ALIGN(. + __app_stack, 16)
             : DEFINED(APP_STACK) ? ALIGN(. + APP_STACK, 16) : LENGTH(ram);
  ASSERT(__mem_size <= LENGTH(ram), "the app and its APP_STACK need more than APP_MEM")
  /DISCARD/ : { *(.eh_frame) *(.comment) *(.riscv.attributes) *(.note.*) }
}
