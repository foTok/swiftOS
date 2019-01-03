use core::Result;
use io::ErrorKind;

pub trait ReadExt{
    fn read_max(&mut self, mut buf: &mut [u8]) -> Result<usize,()> {
        let start_len = buf.len();
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                Err(ref e) if e.kind() == ErrorKind::Interrupted => {}
                Err(e) => return Err(e),
            }
        }

        Ok(start_len - buf.len())
    }
}

impl<T> ReadExt for T {  }
