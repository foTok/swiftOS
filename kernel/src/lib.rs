#![feature(asm)]
#![no_builtins]
#![feature(uniform_paths)]
#![feature(optin_builtin_traits)]
#![no_std]

use core::panic::PanicInfo;
use core::result::Result::{Ok, Err};
use pi::timer;
use pi::uart;
use pi::gpio;

mod mem;

#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}


#[no_mangle]
pub unsafe extern "C" fn kmain() {
    // Turn on the light 10s to show that the Pi is ready.
    // Then turn off the light.
    let mut gpio16 = gpio::Gpio::new(16).into_output();

    loop {
        gpio16.set();
        timer::spin_sleep_ms(3000);
        gpio16.clear();
        timer::spin_sleep_ms(4000);
    }

}
