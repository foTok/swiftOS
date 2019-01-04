use crate::timer;
use crate::common::IO_BASE;
use crate::gpio::{Gpio, Function};
use std::io::*;
use std::volatile::*;

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;
/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    MU_IO: Volatile<u8>,
    _r0: [Reserved<u8>; 3],
    MU_IER: Volatile<u8>,
    _r1: [Reserved<u8>; 3],
    MU_IIR: Volatile<u8>,
    _r2: [Reserved<u8>; 3],
    MU_LCR: Volatile<u8>,
    _r3: [Reserved<u8>; 3],
    MU_MCR: Volatile<u8>,
    _r4: [Reserved<u8>; 3],
    MU_LSR: ReadVolatile<u8>,
    _r5: [Reserved<u8>; 3],
    MU_MSR: ReadVolatile<u8>,
    _r6: [Reserved<u8>; 3],
    MU_SCRATCH: Volatile<u8>,
    _r7: [Reserved<u8>; 3],
    MU_CNTL: Volatile<u8>,
    _r8: [Reserved<u8>; 3],
    MU_STAT: ReadVolatile<u32>,
    MU_BAUD: Volatile<u16>,
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<u32>,
}

impl MiniUart {
    pub fn new() -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*AUX_ENABLES).or_mask(1);
            &mut *(MU_REG_BASE as *mut Registers)
        };
        // 1. Set GPIO 14 as TXD1
        Gpio::new(14).into_alt(Function::Alt5);
        // 2. Set GPIO 15 as RDXD1
        Gpio::new(15).into_alt(Function::Alt5);
        // 3. Set data size as 8 bits
        registers.MU_LCR.or_mask(0b11);
        // 4. Set BAUD rate to ~115200
        registers.MU_BAUD.write(270 as u16);
        // 5. Enable
        registers.MU_CNTL.or_mask(0b11);

        MiniUart {
            registers: registers,
            timeout: None,
        }
    }

    /// Set the read timeout to `milliseconds` milliseconds.
    pub fn set_read_timeout(&mut self, milliseconds: u32) {
        self.timeout = Some(milliseconds);
    }

    /// Returns `true` if there is at least one byte ready to be read.
    pub fn has_byte(&self) -> bool {
        self.registers.MU_LSR.has_mask(LsrStatus::DataReady as u8)
    }

    /// Do nothing. Stop when there is at least one byte to read.
    pub fn wait_for_byte(&self) {
        loop {
            if self.has_byte(){
                break;
            }
        }
    }
}


impl Read for MiniUart {
    fn read_byte(& self) -> Result<u8, ErrorKind>{
        match self.timeout {
            Some(timeout) => {
                let t0 = timer::current_time();
                loop{
                    if self.has_byte(){
                        return Ok(self.registers.MU_IO.read());
                    }
                    let t1 = timer::current_time();
                    if t1 - t0 > (timeout as u64) * 1000{
                        return Err(ErrorKind::TimedOut);
                    }
                }
            },
            None => {
                loop{
                    if self.has_byte(){
                        return Ok(self.registers.MU_IO.read());
                    }
                }
            }
        }
    }
}

impl Write for MiniUart {
    fn write_byte(&mut self, byte: u8) -> Result<u8, ErrorKind>{
        match self.timeout {
            Some(timeout) => {
                let t0 = timer::current_time();
                loop{
                    if self.registers.MU_LSR.has_mask(LsrStatus::TxAvailable as u8){
                        break;
                    }
                    let t1 = timer::current_time();
                    if t1-t0 > (timeout as u64) * 1000{
                        return Err(ErrorKind::TimedOut);
                    }
                }
            },
            None => {
                loop{
                    if self.registers.MU_LSR.has_mask(LsrStatus::TxAvailable as u8){
                        break;
                    }
                }
            }
        }
        self.registers.MU_IO.write(byte);
        Ok(byte)
    }
}
