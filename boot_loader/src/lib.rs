#![feature(asm)]
#![no_builtins]
#![no_std]

use pi::timer::spin_sleep_ms;
use pi::gpio::*;
use pi::uart::*;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    let mut gpio16_output = Gpio::new(16).into_output();
    let mut the_uart = MiniUart::new();

    loop {
        //on
        gpio16_output.set();
        spin_sleep_ms(1000);
        // off
        gpio16_output.clear();
        spin_sleep_ms(1000);

        //uart
        let byte = the_uart.read_byte();
        the_uart.write_byte(byte);
    }
}
