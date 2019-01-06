/// The module to write byte into memory.
use std::io::*;
use core::result::Result;
use core::result::Result::{Ok, Err};

pub struct MemWrite{
    i: usize, // the index now
    end: usize, //end address
}

impl MemWrite{
    pub fn new(start:usize, end:usize)->MemWrite{
        MemWrite{
            i: start,
            end: end,
        }
    }
}

impl Write for MemWrite{
    fn write_byte(&mut self, byte: u8) -> Result<u8, ErrorKind>{
        if self.i==self.end {
            return Err(ErrorKind::UnexpectedEof);
        }
        unsafe {
            let address: *mut u8 = self.i as *mut u8;
            ::core::ptr::write_volatile(address, byte);
        }
        self.i += 1;
        Ok(byte)
    }
}
