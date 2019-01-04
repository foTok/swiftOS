use crate::io::ErrorKind;
use crate::io::Read;

pub trait ReadExt: Read{
    fn read_max(&mut self, mut buf: &mut [u8]) -> Result<usize,ErrorKind> {
        let start_len = buf.len();
        while !buf.is_empty() {
            match self.read(buf) {
                Ok(0) => break,
                Ok(n) => { let tmp = buf; buf = &mut tmp[n..]; }
                Err(e) => {
                    match e {
                        ErrorKind::Interrupted => {},
                        _ => return Err(e),
                    }
                }
            }
        }

        Ok(start_len - buf.len())
    }
}

impl<T: Read> ReadExt for T {  }
