#![allow(clippy::all)]
#![feature(asm)]
#![no_builtins]
#![feature(uniform_paths)]
#![feature(optin_builtin_traits)]
#![no_std]

use core::result::Result::{Ok, Err};
use pi::timer;
use pi::uart;
use pi::gpio;
use std::mem;
use std::xmodem::Xmodem;

const BINARY_START_ADDR: usize = 0x80000;
const BOOTLOADER_START_ADDR: usize = 0x4000000;

fn jump_to(addr: *mut u8) -> ! {
    unsafe {
        asm!("br $0" : : "r"(addr as usize));
        loop { asm!("nop" :::: "volatile")  }
    }
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    // Turn on the light 10s to show that the Pi is ready.
    // Then turn off the light.
    let mut gpio16 = gpio::Gpio::new(16).into_output();
    loop {
        gpio16.set();
        // open a uart to recieve new data
        let mut mini_uart = uart::MiniUart::new();
        mini_uart.set_read_timeout(750);
        // mem write
        let mem_write = mem::MemWrite::new(BINARY_START_ADDR, BOOTLOADER_START_ADDR);
        // xmodem
        match Xmodem::receive(mini_uart, mem_write){
            Ok(_) => jump_to(BINARY_START_ADDR as *mut u8),
            Err(_) => {},
        }
        gpio16.clear();
        timer::spin_sleep_ms(1000);
    }
}
