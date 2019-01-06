#![allow(clippy::all)]
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
use std::xmodem::Xmodem;

mod mem;

#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}

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
    gpio16.set();
    timer::spin_sleep_ms(10_000);
    gpio16.clear();

    loop {
        // open a uart to recieve new data
        let mini_uart = uart::MiniUart::new();
        // mem write
        let mem_write = mem::MemWrite::new(BINARY_START_ADDR, BOOTLOADER_START_ADDR);
        // xmodem
        mini_uart.wait_for_byte();
        match Xmodem::receive(mini_uart, mem_write){
            Ok(_) => jump_to(BINARY_START_ADDR as *mut u8),
            Err(_) => {}
        }
    }
}
