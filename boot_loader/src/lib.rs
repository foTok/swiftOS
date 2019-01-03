#![feature(asm)]
#![no_builtins]
#![feature(uniform_paths)]
#![feature(optin_builtin_traits)]
#![no_std]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> !{
    loop {}
}

#[no_mangle]
pub unsafe extern "C" fn kmain() {
    unimplemented!();
}
