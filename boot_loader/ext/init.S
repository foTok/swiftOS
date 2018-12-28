; This file is the init code to boot Raspberrypi 3b+.
; I refered to git clone https://web.stanford.edu/class/cs140e/assignments/0-blinky/skeleton.git 0-blinky
; In fact, almost copy but read many other materials to understand it and add comments.
; The way jumping to kmain is to respect Linux 0.11.
; Dec 27 2018 by Wenfeng.
.section .head "ax"             ; This section is allocatable and executable.
.global __start                 ; The entry. Only knows by the linker.
__start:
    b   __start_ram

.text
.type   __start_ram, %function
__start_ram:
    ; msr: general register <== system register.
    ; mpidr_el1: offer addtional core identification mechnism.
    msr x7, mpidr_el1
    and x7, x7, #3              ; Check if the first two bits are 0.
    cbz x7, __start_master      ; If x7==0, jump to start master core.
    b   __hang                  ; If x7!=0, it is a slave core and jump to close it.

__start_master:
    adrp    x2, __cpu0_stack_end    ; __cpu0_stack_end is defined in link script.
                                    ; adrp set x2 as the base adress of __cpu0_stack_end
    add x2, x2, #:lo12:__cpu0_stack_end ; lo12 is the offset. Help to relocation.
    mov sp, x2                  ; Set stack point.

__clear_bss:
    ;  There are 31 general registers in ARMv8. x0~x30 for 64bit. w0~w30 for 32bit.
    ldr	w0, __bss_segment + 0   ; Set w0=__bss_start.
    ldr	w1, __bss_segment + 4   ; Set w1=__bass_dwords. Then length of BSS segment.

__clear:
    cbz	x1, __go_main           ; If x1(w1)==0, clear BSS OK and jump to main function.
    str	xzr, [x0], #8           ; xzr: the zero register. Clear the 64 bits start with [x0] and increase the address.
    sub	x1, x1, #1              ; x1=x1-1. When x1==0, all BSS cleared.
    cbnz    x1, __clear         ; If x1!=0, loop to clear.
__go_main:
    mov lr, kmain               ; Respect Linux 0.11.
    ret

__hang:
    wfe                         ; Wait for event.
    b   __hang

__bss_segment:                  ; Store informaton to init BSS segment.
    .word	__bss_start         ; BSS start address.
    .word	__bss_dwords        ; BSS size or length (Byte).
