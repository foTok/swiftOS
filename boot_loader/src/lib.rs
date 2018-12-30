#![feature(asm)]
#![no_builtins]
#![no_std]

use pi::timer::spin_sleep_ms;
use pi::gpio::*;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    let gpio16 = Gpio::<Uninitialized>::new(16);
    let mut gpio16_output = gpio16.into_output();

    loop {
        // on
        gpio16_output.set();
        spin_sleep_ms(1000);
        // off
        gpio16_output.clear();
        spin_sleep_ms(1000);
    }
}
