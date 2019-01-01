use core::fmt;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile, Reserved};

use crate::timer;
use crate::common::IO_BASE;
use crate::gpio::{Gpio, Function};

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
    MU_STAT_s: ReadVolatile<u16>,
    MU_STAT_rf: ReadVolatile<u8>,
    MU_STAT_tf: ReadVolatile<u8>,
    MU_BAUD: Volatile<u16>,
    _r9: Reserved<u16>
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<u32>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
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

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        loop{
            if self.registers.MU_LSR.has_mask(LsrStatus::TxAvailable as u8){
                break;
            }
        }
        self.registers.MU_IO.write(byte);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        self.registers.MU_LSR.has_mask(LsrStatus::DataReady as u8)
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        match self.timeout {
            None => {
                loop {
                    if self.has_byte(){
                        return Ok(());
                    }
                }
            },
            Some(timeout) => {
                let t0 = timer::current_time();
                loop {
                    if self.has_byte(){
                        return Ok(());
                    }
                    let t1 = timer::current_time();
                    if t1 - t0 > (timeout as u64)*1000 {
                        return Err(());
                    }
                }
            }
        }
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        loop{
            if self.has_byte(){
                return self.registers.MU_IO.read();
            }
        }
    }
}

impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            match byte {
                b'\n' => {
                    self.write_byte(b'\r');
                    self.write_byte(b'\n');
                },
                _ => self.write_byte(byte)
            }
        }
        Ok(())
    }
}

#[cfg(feature = "std")]
mod uart_io {
    use std::io;
    use super::MiniUart;

    impl io::Read for MiniUart {
       fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
           match self.wait_for_byte() {
               Ok(()) => {
                   let mut index = 0;
                   while self.has_byte() && index < buf.len() {
                       buf[index] = self.read_byte();
                       index += 1;
                   }
                   Ok(index)
               },
               Err(()) => {
                   Err(io::Error::new(io::ErrorKind::TimedOut, "reading UART timed out"))
               }
           }
       }
    }

    impl io::Write for MiniUart {
       fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
           for byte in buf {
               self.write_byte(*byte);
           }
           Ok(buf.len())
       }

       fn flush(&mut self) -> io::Result<()> {
           unimplemented!()
       }
    }
}
