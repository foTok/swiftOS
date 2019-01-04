mod read_ext;
mod progress;

use crate::io::Read;
use crate::io::Write;
use crate::io::ErrorKind;
use read_ext::ReadExt;
use progress::*;

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;


pub struct Xmodem<R> {
    packet: u8,     // package ID. 0~255. Roll back to 0: 0=>255=>0
    inner: R,       // receiver or transmiter
    started: bool,
    progress: ProgressFn,
}

impl Xmodem<()> {
    /// Read data from *ds* and send the data by *port*.
    /// If transmit successfully, return the byte number.
    /// Else, return Err(())
    #[inline]
    pub fn transmit<R, W>(ds: R, port: W) -> Result<usize, ErrorKind> 
        where R: Read,
              W: Read + Write
    {
        Xmodem::transmit_with_progress(ds, port, progress::noop)
    }

    #[inline]
    pub fn transmit_with_progress<R, W>(mut ds: R, port: W, f: ProgressFn) -> Result<usize, ErrorKind> 
        where R: Read,
              W: Read + Write
    {
        let mut transmitter = Xmodem::new_with_progress(port, f);
        let mut packet = [0u8; 128];
        let mut written = 0;
        'next_packet: loop {
            let n = ds.read_max(&mut packet)?;
            packet[n..].iter_mut().for_each(|b| *b = 0);

            if n == 0 {
                transmitter.write_packet(&[])?;
                return Ok(written);
            }

            for _ in 0..10 {
                match transmitter.write_packet(&packet) {
                    Err(e) => {
                        match e {
                            ErrorKind::Interrupted => continue,
                            _ => return Err(e),
                        }
                    },
                    Ok(_) => {
                        written += n;
                        continue 'next_packet;
                    }
                }
            }

            return Err(ErrorKind::BrokenPipe);
        }
    }

    /// Receives `data` from `from` using the XMODEM protocol and writes it into
    /// `into`. Returns the number of bytes read from `from`, a multiple of 128.
    #[inline]
    pub fn receive<R, W>(port: R, into: W) -> Result<usize, ErrorKind>
       where R: Read + Write,
             W: Write
    {
        Xmodem::receive_with_progress(port, into, progress::noop)
    }

    /// Receives `data` from `from` using the XMODEM protocol and writes it into
    /// `into`. Returns the number of bytes read from `from`, a multiple of 128.
    ///
    /// The function `f` is used as a callback to indicate progress throughout
    /// the reception. See the [`Progress`] enum for more information.
    pub fn receive_with_progress<R, W>(port: R, mut into: W, f: ProgressFn) -> Result<usize, ErrorKind>
       where R: Read + Write, 
             W: Write
    {
        let mut receiver = Xmodem::new_with_progress(port, f);
        let mut packet = [0u8; 128];
        let mut received = 0;
        'next_packet: loop {
            for _ in 0..10 {
                match receiver.read_packet(&mut packet) {
                    Err(e) => {
                        match e {
                            ErrorKind::Interrupted => continue,
                            _ => return Err(e),
                        }
                    },
                    Ok(0) => break 'next_packet,
                    Ok(n) => {
                        received += n;
                        into.write(&packet)?;
                        continue 'next_packet;
                    }
                }
            }

            return Err(ErrorKind::BrokenPipe);
        }
        Ok(received)
    }
}


impl<T:Read + Write> Xmodem<T> {
    /// Returns a new `Xmodem` instance with the internal reader/writer set to
    /// `inner`. The returned instance can be used for both receiving
    /// (downloading) and sending (uploading).
    pub fn new(inner: T) -> Self {
        Xmodem { packet: 1, started: false, inner, progress: progress::noop}
    }

    /// Returns a new `Xmodem` instance with the internal reader/writer set to
    /// `inner`. The returned instance can be used for both receiving
    /// (downloading) and sending (uploading). The function `f` is used as a
    /// callback to indicate progress throughout the transfer. See the
    /// [`Progress`] enum for more information.
    pub fn new_with_progress(inner: T, f: ProgressFn) -> Self {
        Xmodem { packet: 1, started: false, inner, progress: f }
    }

    /// basic data send and receive functions
    /// Read a byte
    fn read_byte(&mut self, abort_on_can: bool) -> Result<u8, ErrorKind> {
        let byte = self.inner.read_byte()?;

        if abort_on_can && byte == CAN {
            return Err(ErrorKind::ConnectionAborted);
        }

        Ok(byte)
    }

    /// Send a byte
    fn write_byte(&mut self, byte: u8) -> Result<u8, ErrorKind> {
        self.inner.write_byte(byte)
    }

    /// Reads a single byte from the inner I/O stream and compares it to `byte`.
    /// If the bytes match, the byte is returned as an `Ok`. If they differ and
    /// the read byte is not `CAN`, an error of `InvalidData` with the message
    /// `expected` is returned. If they differ and the read byte is `CAN`, an
    /// error of `ConnectionAborted` is returned. In either case, if they bytes
    /// differ, a `CAN` byte is written out to the inner stream.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the inner stream fails, if the read
    /// byte was not `byte`, if the read byte was `CAN` and `byte` is not `CAN`,
    /// or if writing the `CAN` byte failed on byte mismatch.
    fn expect_byte_or_cancel(&mut self, byte: u8) -> Result<u8, ErrorKind> {
        let byte_read = self.read_byte(false)?;
        if byte_read==byte {
            return Ok(byte);
        }
        else {
            self.write_byte(CAN)?;
            if byte_read==CAN {
                return Err(ErrorKind::ConnectionAborted);
            }
            else{
                return Err(ErrorKind::InvalidData);
            }
        }
    }

    /// Reads a single byte from the inner I/O stream and compares it to `byte`.
    /// If they differ, an error of `InvalidData` with the message `expected` is
    /// returned. Otherwise the byte is returned. If `byte` is not `CAN` and the
    /// read byte is `CAN`, a `ConnectionAborted` error is returned.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the inner stream fails, or if the read
    /// byte was not `byte`. If the read byte differed and was `CAN`, an error
    /// of `ConnectionAborted` is returned. Otherwise, the error kind is
    /// `InvalidData`.
    fn expect_byte(&mut self, byte: u8) -> Result<u8, ErrorKind> {
        let byte_read = self.read_byte(false)?;
        if byte_read==byte {
            return Ok(byte);
        }
        else {
            if byte_read==CAN {
                return Err(ErrorKind::ConnectionAborted);
            }
            else{
                return Err(ErrorKind::InvalidData);
            }
        }
    }

    /// Transmit package
    /// Reads (downloads) a single packet from the inner stream using the XMODEM
    /// protocol. On success, returns the number of bytes read (always 128).
    ///
    /// The progress callback is called with `Progress::Start` when reception
    /// for the first packet has started and subsequently with
    /// `Progress::Packet` when a packet is received successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or writing to the inner stream fails at any
    /// point. Also returns an error if the XMODEM protocol indicates an error.
    /// In particular, an `InvalidData` error is returned when:
    ///
    ///   * The sender's first byte for a packet isn't `EOT` or `SOH`.
    ///   * The sender doesn't send a second `EOT` after the first.
    ///   * The received packet numbers don't match the expected values.
    ///
    /// An error of kind `Interrupted` is returned if a packet checksum fails.
    ///
    /// An error of kind `ConnectionAborted` is returned if a `CAN` byte is
    /// received when not expected.
    ///
    /// An error of kind `UnexpectedEof` is returned if `buf.len() < 128`.
    pub fn read_packet(&mut self, buf: &mut [u8]) -> Result<usize, ErrorKind> {
        // check buf
        if buf.len() < 128 {
            return Err(ErrorKind::UnexpectedEof);
        }
        // Start, only one time.
        if !self.started{
            self.started = true;
            self.write_byte(NAK)?;
            (self.progress)(Progress::Started);
        }
        // 1. wait for SOH or EOT
        // SOH: OK; EOT: end transimition; Other: cancel
        let read_byte_1 = self.read_byte(true)?;
        if read_byte_1==EOT{
            self.write_byte(NAK)?;
            self.expect_byte(EOT)?;
            self.write_byte(ACK)?;
            self.started = false;
            return Ok(0);
        }
        else if read_byte_1!=SOH{
            self.write_byte(CAN)?;
            return Err(ErrorKind::InvalidData);
        } // else, recieved SOH, do nothing.
        // 2. Read packet number
        self.expect_byte_or_cancel(self.packet)?;
        // 3. Read 255-packet number
        self.expect_byte_or_cancel(!self.packet)?;
        // 4. Read a packet (128) from the sender
        let mut checksum: u8 = 0;
        let buf_len = buf.len();
        for byte in buf{
            *byte = self.read_byte(false)?;
            checksum = checksum.wrapping_add(*byte);
        }
        // 5. Checksum
        let read_check_sum = self.read_byte(false)?;
        // 6. Verify Checksum
        if read_check_sum!=checksum{
            self.write_byte(NAK)?;
            return Err(ErrorKind::Interrupted);
        }
        else {
            self.write_byte(ACK)?;
            (self.progress)(Progress::Packet(self.packet));
            self.packet = self.packet.wrapping_add(1);
            return Ok(buf_len);
        }
    }

    /// Sends (uploads) a single packet to the inner stream using the XMODEM
    /// protocol. If `buf` is empty, end of transmissions is sent. Users of this
    /// interface should ensure that `write_packet(&[])` is called when data
    /// transmission is complete. On success, returns the number of bytes
    /// written.
    ///
    /// The progress callback is called with `Progress::Waiting` before waiting
    /// for the receiver's `NAK`, `Progress::Start` when transmission of the
    /// first packet has started and subsequently with `Progress::Packet` when a
    /// packet is sent successfully.
    ///
    /// # Errors
    ///
    /// Returns an error if reading or writing to the inner stream fails at any
    /// point. Also returns an error if the XMODEM protocol indicates an error.
    /// In particular, an `InvalidData` error is returned when:
    ///
    ///   * The receiver's first byte isn't a `NAK`.
    ///   * The receiver doesn't respond with a `NAK` to the first `EOT`.
    ///   * The receiver doesn't respond with an `ACK` to the second `EOT`.
    ///   * The receiver responds to a complete packet with something besides
    ///     `ACK` or `NAK`.
    ///
    /// An error of kind `UnexpectedEof` is returned if `buf.len() < 128 &&
    /// buf.len() != 0`.
    ///
    /// An error of kind `ConnectionAborted` is returned if a `CAN` byte is
    /// received when not expected.
    ///
    /// An error of kind `Interrupted` is returned if a packet checksum fails.
    pub fn write_packet(&mut self, buf: &[u8]) -> Result<usize, ErrorKind> {
        // Check buf
        if (buf.len()<128) & !buf.is_empty() {
            return Err(ErrorKind::UnexpectedEof);
        }
        // Wait NAK to start
        if !self.started{
            (self.progress)(Progress::Waiting);
            self.expect_byte(NAK)?;
            self.started = true;
            (self.progress)(Progress::Started);
        }
        // Check End
        if buf.is_empty(){
            self.write_byte(EOT)?;
            self.expect_byte(NAK)?;
            self.write_byte(EOT)?;
            self.expect_byte(ACK)?;
            self.started = false;
            return Ok(0);
        }
        // 1. send SOH
        self.write_byte(SOH)?;
        // 2. send packet number
        self.write_byte(self.packet)?;
        // 3. send 255-packet number
        self.write_byte(!self.packet)?;
        // 4. send packet
        let mut checksum: u8 = 0;
        for byte in buf {
            self.write_byte(*byte)?;
            checksum = checksum.wrapping_add(*byte);
        }
        // 5. send check sum
        self.write_byte(checksum)?;
        // 6. read data
        let read_ack = self.read_byte(true)?;
        if read_ack==ACK{
            (self.progress)(Progress::Packet(self.packet));
            self.packet = self.packet.wrapping_add(1);
            return Ok(buf.len());
        }
        else if read_ack==NAK{
            return Err(ErrorKind::Interrupted);
        }
        else {
            return Err(ErrorKind::InvalidData);
        }
    }
}
