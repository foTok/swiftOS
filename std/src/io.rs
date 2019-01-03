use core::Result;
use core::Option;

/// io Error Kind
pub enum ErrorKind {
    NotFound,
    PermissionDenied,
    ConnectionRefused,
    ConnectionReset,
    ConnectionAborted,
    NotConnected,
    AddrInUse,
    AddrNotAvailable,
    BrokenPipe,
    AlreadyExists,
    WouldBlock,
    InvalidInput,
    InvalidData,
    TimedOut,
    WriteZero,
    Interrupted,
    Other,
    UnexpectedEof,
}


/// Read Trait
pub trait Read {
    fn read_byte(&mut self) -> Result<u8, ErrorKind>;

    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ErrorKind>;
}

/// Write Trait
pub trait Write {
    fn write_byte(&mut self, byte: u8) -> Result<u8, ErrorKind>;

    fn write(&mut self, buf: & [u8]) -> Result<usize, ErrorKind>;
}
