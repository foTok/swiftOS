#![feature(asm)]
#![no_builtins]
#![no_std]

use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}


const GPIO_BASE: usize = 0x3F000000 + 0x200000;
const GPIO_FSEL1: *mut u32 = (GPIO_BASE + 0x04) as *mut u32;
const GPIO_SET0: *mut u32 = (GPIO_BASE + 0x1C) as *mut u32;
const GPIO_CLR0: *mut u32 = (GPIO_BASE + 0x28) as *mut u32;

#[inline(never)]
fn spin_sleep_ms(ms: usize) {
    for _ in 0..(ms * 600) {
        unsafe { asm!("nop" :::: "volatile"); }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    // To set GPIO 16 as output, we should set GPIO Alternate function select register 1
    // bit 20-18 as 0b001.
    let fsel1 = GPIO_FSEL1.read_volatile();
    GPIO_FSEL1.write_volatile(fsel1 | 1<<18 as u32);
    // STEP 2: Continuously set and clear GPIO 16.
    loop {
        // on
        let set0 = GPIO_SET0.read_volatile();
        GPIO_SET0.write_volatile(set0 | 1<<16 as u32);
        spin_sleep_ms(1000);
        // off
        let clr0 = GPIO_CLR0.read_volatile();
        GPIO_CLR0.write_volatile(clr0 | 1<<16 as u32);
        spin_sleep_ms(1000);
    }
}
