#![feature(asm)]
#![no_builtins]
#![feature(uniform_paths)]
#![feature(optin_builtin_traits)]
#![no_std]

use core::panic::PanicInfo;
use pi::timer;
use pi::uart;
use pi::gpio;
use std::io::*;

#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    // let mut gpio16 = gpio::Gpio::new(16).into_output();
    // gpio16.set();
    // timer::spin_sleep_ms(100);
    // gpio16.clear();
    // timer::spin_sleep_ms(100);

    let mut shell_uart = uart::MiniUart::new();

    loop {
        shell_uart.wait_for_byte();
        let r = shell_uart.read_byte();
        match r{
            Ok(byte) => { 
                match shell_uart.write_byte(byte){
                    _ => {}
                }
            },
            Err(_) => continue,
        }
    }
}
