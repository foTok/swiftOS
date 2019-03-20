#![feature(asm)]
#![no_builtins]
#![feature(optin_builtin_traits)]
#![no_std]

use pi::timer;
use pi::gpio;

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    // Turn on the light 3 seconds to show that the Pi is ready.
    // Then turn off the light 4 seconds.
    let mut gpio16 = gpio::Gpio::new(16).into_output();

    loop {
        gpio16.set();
        timer::spin_sleep_ms(3000);
        gpio16.clear();
        timer::spin_sleep_ms(4000);
    }
}
