# This file is the init code to boot Raspberrypi 3b+.
# I refered to git clone https://web.stanford.edu/class/cs140e/assignments/0-blinky/skeleton.git 0-blinky
# In fact, almost copy but read many other materials to understand it and add comments.
# Dec 27 2018 by Wenfeng.

# This section is allocatable and executable.
.section .head, "ax"
# The entry. Only knows by the linker.
.global __start
__start:
    b   __start_ram

.text
.type   __start_ram, %function
__start_ram:
    # msr: general register <== system register.
    # mpidr_el1: offer addtional core identification mechnism.
    mrs	x7, mpidr_el1
    # Check if the first two bits are 0.
    and x7, x7, #3
    # If x7==0, jump to start master core.
    cbz x7, __start_master
    # If x7!=0, it is a slave core and jump to close it.
    b   __hang

__start_master:
    # __cpu0_stack_end is defined in link script.
    #  adrp set x2 as the base adress of __cpu0_stack_end
    adrp    x2, __cpu0_stack_end
    # lo12 is the offset. Help to relocation.
    add x2, x2, #:lo12:__cpu0_stack_end
    # Set stack point.
    mov sp, x2

__clear_bss:
    # There are 31 general registers in ARMv8. x0~x30 for 64bit. w0~w30 for 32bit.
    #  Set w0=__bss_start.
    ldr	w0, __bss_segment + 0
    #  Set w1=__bss_dwords. Then length of BSS segment.
    ldr	w1, __bss_segment + 4

__clear:
    # If x1(w1)==0, clear BSS OK and jump to main function.
    cbz	x1, __go_main
    # xzr: the zero register. Clear the 64 bits start with [x0] and increase the address.
    str	xzr, [x0], #8
    # x1=x1-1. When x1==0, all BSS cleared.
    sub	x1, x1, #1
    # If x1!=0, loop to clear.
    cbnz    x1, __clear
__go_main:
    bl kmain

__hang:
    # Wait for event.
    wfe
    b   __hang

# Store informaton to init BSS segment.
__bss_segment:
    # BSS start address.               
    .word	__bss_start
    # BSS size or length (Byte).   
    .word	__bss_dwords
