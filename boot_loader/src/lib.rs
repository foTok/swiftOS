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

const BEL: u8 = 0x07u8;
const BS: u8 = 0x08u8;
const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const ESC: u8 = 0x1bu8;
const DEL: u8 = 0x7fu8;


#[no_mangle]
pub unsafe extern "C" fn kmain() {
    let mut gpio16_output = Gpio::new(16).into_output();
    let mut the_uart = MiniUart::new();

    loop {
        //on
        gpio16_output.set();
        spin_sleep_ms(100);
        // off
        gpio16_output.clear();
        spin_sleep_ms(100);

        //uart
        let byte = the_uart.read_byte();
        if (byte>=32 as u8) && (byte<=126 ){
            the_uart.write_byte(byte);
        }
        else if byte==DEL{
            the_uart.write_byte('d' as u8);
            the_uart.write_byte('e' as u8);
            the_uart.write_byte('l' as u8);
            the_uart.write_byte(DEL);
        }
        
        else if byte==BS {
            the_uart.write_byte('b' as u8);
            the_uart.write_byte('s' as u8);
            the_uart.write_byte(BS);
        }
        else{
            the_uart.write_byte(BEL);
        }
    }
}
