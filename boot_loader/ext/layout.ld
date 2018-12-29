SECTIONS {
  .text : {
    KEEP (*(.head))
    . = 0x80000;   /* Space for command line.  */
    __start = .;
    *(.text .text.* .gnu.linkonce.t*)
  }

  .rodata : {
    *(.rodata .rodata.* .gnu.linkonce.r*)
  }

  .data : {
    *(.data .data.* .gnu.linkonce.d*)
  }

  .bss (NOLOAD) : {
    __bss_start = ALIGN(0x10);
    *(.bss .bss.*)
    *(COMMON)

    __bss_end = ALIGN(0x10);

    . = ALIGN(0x10);
    . += 0x1000;
    __cpu0_stack_end = .;

    _end = .;
  }

   __bss_dwords = (__bss_end - __bss_start) >> 3;

  /DISCARD/ : { *(.note.GNU-stack) *(.gnu_debuglink) *(.gnu.lto_*) }
}
