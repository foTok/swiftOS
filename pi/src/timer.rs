use crate::common::IO_BASE;
use std::volatile::prelude::*;
use std::volatile::{Volatile, ReadVolatile};

/// The base address for the ARM system timer registers.
const TIMER_REG_BASE: usize = IO_BASE + 0x3000;

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    CS: Volatile<u32>,
    CLO: ReadVolatile<u32>,
    CHI: ReadVolatile<u32>,
    COMPARE: [Volatile<u32>; 4]
}

/// The Raspberry Pi ARM system timer.
pub struct Timer {
    registers: &'static mut Registers
}

impl Timer {
    /// Returns a new instance of `Timer`.
    pub fn new() -> Timer {
        Timer {
            registers: unsafe { &mut *(TIMER_REG_BASE as *mut Registers) },
        }
    }

    /// Reads the system timer's counter and returns the 64-bit counter value.
    /// The returned value is the number of elapsed microseconds.
    pub fn read(&self) -> u64 {
        let clo = self.registers.CLO.read() as u64;
        let chi = self.registers.CHI.read() as u64;
        (chi<<32)+clo
    }
}

/// Returns the current time in microseconds.
pub fn current_time() -> u64 {
    let tm = Timer::new();
    tm.read()
}

/// Spins until `us` microseconds have passed.
pub fn spin_sleep_us(us: u64) {
    let tm = Timer::new();
    let t0 = tm.read();
    loop{
        let t1 = tm.read();
        if (t1 - t0)>=us{
            break;
        }
    }
}

/// Spins until `ms` milliseconds have passed.
pub fn spin_sleep_ms(ms: u64) {
    let tm = Timer::new();
    let t0 = tm.read();
    let us = ms*1000;
    loop{
        let t1 = tm.read();
        if (t1 - t0)>=us{
            break;
        }
    }
}
