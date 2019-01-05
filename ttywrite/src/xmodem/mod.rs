use std::io;

mod read_ext;
mod progress;

pub use progress::{Progress, ProgressFn};
use read_ext::ReadExt;

const SOH: u8 = 0x01;
const EOT: u8 = 0x04;
const ACK: u8 = 0x06;
const NAK: u8 = 0x15;
const CAN: u8 = 0x18;

/// Implementation of the XMODEM protocol.
pub struct Xmodem<R> {
    packet: u8,
    inner: R,
    started: bool,
    progress: ProgressFn
}

impl Xmodem<()> {
    /// Transmits `data` to the receiver `to` using the XMODEM protocol. If the
    /// length of the total data yielded by `data` is not a multiple of 128
    /// bytes, the data is padded with zeroes and sent to the receiver.
    ///
    /// Returns the number of bytes written to `to`, excluding padding zeroes.
    #[inline]
    pub fn transmit<R, W>(data: R, to: W) -> io::Result<usize>
        where W: io::Read + io::Write, R: io::Read
    {
        Xmodem::transmit_with_progress(data, to, progress::noop)
    }

    /// Transmits `data` to the receiver `to` using the XMODEM protocol. If the
    /// length of the total data yielded by `data` is not a multiple of 128
    /// bytes, the data is padded with zeroes and sent to the receiver.
    ///
    /// The function `f` is used as a callback to indicate progress throughout
    /// the transmission. See the [`Progress`] enum for more information.
    ///
    /// Returns the number of bytes written to `to`, excluding padding zeroes.
    pub fn transmit_with_progress<R, W>(mut data: R, to: W, f: ProgressFn) -> io::Result<usize>
        where W: io::Read + io::Write, R: io::Read
    {
        let mut transmitter = Xmodem::new_with_progress(to, f);
        let mut packet = [0u8; 128];
        let mut written = 0;
        'next_packet: loop {
            let n = data.read_max(&mut packet)?;
            packet[n..].iter_mut().for_each(|b| *b = 0);

            if n == 0 {
                transmitter.write_packet(&[])?;
                return Ok(written);
            }

            for _ in 0..10 {
                match transmitter.write_packet(&packet) {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                    Ok(_) => {
                        written += n;
                        continue 'next_packet;
                    }
                }
            }

            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "bad transmit"));
        }
    }

    /// Receives `data` from `from` using the XMODEM protocol and writes it into
    /// `into`. Returns the number of bytes read from `from`, a multiple of 128.
    #[inline]
    pub fn receive<R, W>(from: R, into: W) -> io::Result<usize>
       where R: io::Read + io::Write, W: io::Write
    {
        Xmodem::receive_with_progress(from, into, progress::noop)
    }

    /// Receives `data` from `from` using the XMODEM protocol and writes it into
    /// `into`. Returns the number of bytes read from `from`, a multiple of 128.
    ///
    /// The function `f` is used as a callback to indicate progress throughout
    /// the reception. See the [`Progress`] enum for more information.
    pub fn receive_with_progress<R, W>(from: R, mut into: W, f: ProgressFn) -> io::Result<usize>
       where R: io::Read + io::Write, W: io::Write
    {
        let mut receiver = Xmodem::new_with_progress(from, f);
        let mut packet = [0u8; 128];
        let mut received = 0;
        'next_packet: loop {
            for _ in 0..10 {
                match receiver.read_packet(&mut packet) {
                    Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e),
                    Ok(0) => break 'next_packet,
                    Ok(n) => {
                        received += n;
                        into.write_all(&packet)?;
                        continue 'next_packet;
                    }
                }
            }

            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "bad receive"));
        }

        Ok(received)
    }
}

impl<T: io::Read + io::Write> Xmodem<T> {
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

    /// Reads a single byte from the inner I/O stream. If `abort_on_can` is
    /// `true`, an error of `ConnectionAborted` is returned if the read byte is
    /// `CAN`.
    ///
    /// # Errors
    ///
    /// Returns an error if reading from the inner stream fails or if
    /// `abort_on_can` is `true` and the read byte is `CAN`.
    fn read_byte(&mut self, abort_on_can: bool) -> io::Result<u8> {
        let mut buf = [0u8; 1];
        self.inner.read_exact(&mut buf)?;

        let byte = buf[0];
        if abort_on_can && byte == CAN {
            return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "received CAN"));
        }

        Ok(byte)
    }

    /// Writes a single byte to the inner I/O stream.
    ///
    /// # Errors
    ///
    /// Returns an error if writing to the inner stream fails.
    fn write_byte(&mut self, byte: u8) -> io::Result<()> {
        self.inner.write_all(&[byte])
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
    fn expect_byte_or_cancel(&mut self, byte: u8, msg: &'static str) -> io::Result<u8> {
        let byte_read = self.read_byte(false)?;
        if byte_read==byte {
            return Ok(byte);
        }
        else {
            self.write_byte(CAN)?;
            if byte_read==CAN {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Recieved CAN"));
            }
            else{
                return Err(io::Error::new(io::ErrorKind::InvalidData, msg));
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
    fn expect_byte(&mut self, byte: u8, expected: &'static str) -> io::Result<u8> {
        let byte_read = self.read_byte(false)?;
        if byte_read==byte {
            return Ok(byte);
        }
        else {
            if byte_read==CAN {
                return Err(io::Error::new(io::ErrorKind::ConnectionAborted, "Recieved CAN"));
            }
            else{
                return Err(io::Error::new(io::ErrorKind::InvalidData, expected));
            }
        }
    }

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
    pub fn read_packet(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // check buf
        if buf.len() < 128 {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "error: buf len is less than 128"));
        }
        // Start
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
            self.expect_byte(EOT, "Expect EOT")?;
            self.write_byte(ACK)?;
            self.started = false;
            return Ok(0);
        }
        else if read_byte_1!=SOH{
            // ?? self.write_byte(CAN)?;
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Expect EOT or SOH"));
        } // else, recieved SOH, do nothing.
        // 2. Read packet number
        self.expect_byte_or_cancel(self.packet, "Packet Num")?;
        // 3. Read 255-packet number
        self.expect_byte_or_cancel(!self.packet, "255 - Packet Num")?;
        // 4. Read a packet (128) from the sender
        let mut checksum: u8 = 0;
        let buf_len = buf.len();
        for byte in buf{
            *byte = self.read_byte(false)?;
            checksum = checksum.wrapping_add(*byte);
        }
        // 5. Checksum
        let read_check_sum = self.read_byte(false)?;
        if read_check_sum!=checksum{
            self.write_byte(NAK)?;
            return Err(io::Error::new(io::ErrorKind::Interrupted, "Checksum Fail"));
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
    pub fn write_packet(&mut self, buf: &[u8]) -> io::Result<usize> {
        // Check buf
        if (buf.len()<128) & !buf.is_empty() {
            return Err(io::Error::new(io::ErrorKind::UnexpectedEof, "Unexpected EOF"));
        }
        // Wait NAK to start
        if !self.started{
            (self.progress)(Progress::Waiting);
            self.expect_byte(NAK, "Expect NAK")?;
            self.started = true;
            (self.progress)(Progress::Started);
        }
        // Check End
        if buf.is_empty(){
            self.write_byte(EOT)?;
            self.expect_byte(NAK, "Expect NAK")?;
            self.write_byte(EOT)?;
            self.expect_byte(ACK, "Expeck ACK")?;
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
        let read_ack = self.read_byte(true)?; //??
        if read_ack==ACK{
            (self.progress)(Progress::Packet(self.packet));
            self.packet = self.packet.wrapping_add(1);
            return Ok(buf.len());
        }
        else if read_ack==NAK{
            return Err(io::Error::new(io::ErrorKind::Interrupted, "data corrupted from receiver"));
        }
        else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Expected NAK or ACK"));
        }
    }

    /// Flush this output stream, ensuring that all intermediately buffered
    /// contents reach their destination.
    ///
    /// # Errors
    ///
    /// It is considered an error if not all bytes could be written due to I/O
    /// errors or EOF being reached.
    pub fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}


#[cfg(test)]
mod xmodem_test {
    use super::*;
    use std::sync::mpsc::{Receiver, Sender, channel};
    use std::io::Cursor;

    struct Pipe(Sender<u8>, Receiver<u8>, Vec<u8>);

    fn pipe() -> (Pipe, Pipe) {
        let ((tx1, rx1), (tx2, rx2)) = (channel(), channel());
        (Pipe(tx1, rx2, vec![]), Pipe(tx2, rx1, vec![]))
    }

    impl io::Read for Pipe {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            for i in 0..buf.len() {
                match self.1.recv() {
                    Ok(byte) => buf[i] = byte,
                    Err(_) => return Ok(i)
                }
            }

            Ok(buf.len())
        }
    }

    impl io::Write for Pipe {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            buf.iter().for_each(|b| self.2.push(*b));
            for (i, byte) in buf.iter().cloned().enumerate() {
                if let Err(e) = self.0.send(byte) {
                    eprintln!("Write error: {}", e);
                    return Ok(i);
                }
            }

            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_loop() {
        let mut input = [0u8; 384];
        for (i, chunk) in input.chunks_mut(128).enumerate() {
            chunk.iter_mut().for_each(|b| *b = i as u8);
        }

        let (tx, rx) = pipe();
        let tx_thread = std::thread::spawn(move || Xmodem::transmit(&input[..], rx));
        let rx_thread = std::thread::spawn(move || {
            let mut output = [0u8; 384];
            Xmodem::receive(tx, &mut output[..]).map(|_| output)
        });

        assert_eq!(tx_thread.join().expect("tx join okay").expect("tx okay"), 384);
        let output = rx_thread.join().expect("rx join okay").expect("rx okay");
        assert_eq!(&input[..], &output[..]);
    }

    #[test]
    fn read_byte() {
        let byte = Xmodem::new(Cursor::new(vec![CAN]))
            .read_byte(false)
            .expect("read a byte");

        assert_eq!(byte, CAN);

        let e = Xmodem::new(Cursor::new(vec![CAN]))
            .read_byte(true)
            .expect_err("abort on CAN");

        assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);
    }

    #[test]
    fn test_expect_byte() {
        let mut xmodem = Xmodem::new(Cursor::new(vec![1, 1]));
        assert_eq!(xmodem.expect_byte(1, "1").expect("expected"), 1);
        let e = xmodem.expect_byte(2, "1, please").expect_err("expect the unexpected");
        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_expect_byte_or_cancel() {
        let mut buffer = vec![2, 0];
        let b = Xmodem::new(Cursor::new(buffer.as_mut_slice()))
            .expect_byte_or_cancel(2, "it's a 2")
            .expect("got a 2");

        assert_eq!(b, 2);
    }

    #[test]
    fn test_expect_can() {
        let mut xmodem = Xmodem::new(Cursor::new(vec![CAN]));
        assert_eq!(xmodem.expect_byte(CAN, "hi").expect("CAN"), CAN);
    }

    #[test]
    fn test_unexpected_can() {
        let e = Xmodem::new(Cursor::new(vec![CAN]))
            .expect_byte(SOH, "want SOH")
            .expect_err("have CAN");

        assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);
    }

    #[test]
    fn test_cancel_on_unexpected() {
        let mut buffer = vec![CAN, 0];
        let e = Xmodem::new(Cursor::new(buffer.as_mut_slice()))
            .expect_byte_or_cancel(SOH, "want SOH")
            .expect_err("have CAN");

        assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);
        assert_eq!(buffer[1], CAN);

        let mut buffer = vec![0, 0];
        let e = Xmodem::new(Cursor::new(buffer.as_mut_slice()))
            .expect_byte_or_cancel(SOH, "want SOH")
            .expect_err("have 0");

        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
        assert_eq!(buffer[1], CAN);
    }

    #[test]
    fn test_can_in_packet_and_checksum() {
        let mut input = [0u8; 256];
        input[0] = CAN;

        let (tx, rx) = pipe();
        let tx_thread = std::thread::spawn(move || Xmodem::transmit(&input[..], rx));
        let rx_thread = std::thread::spawn(move || {
            let mut output = [0u8; 256];
            Xmodem::receive(tx, &mut output[..]).map(|_| output)
        });

        assert_eq!(tx_thread.join().expect("tx join okay").expect("tx okay"), 256);
        let output = rx_thread.join().expect("rx join okay").expect("rx okay");
        assert_eq!(&input[..], &output[..]);
    }

    #[test]
    fn test_transmit_reported_bytes() {
        let (input, mut output) = ([0u8; 50], [0u8; 128]);
        let (tx, rx) = pipe();
        let tx_thread = std::thread::spawn(move || Xmodem::transmit(&input[..], rx));
        let rx_thread = std::thread::spawn(move || Xmodem::receive(tx, &mut output[..]));
        assert_eq!(tx_thread.join().expect("tx join okay").expect("tx okay"), 50);
        assert_eq!(rx_thread.join().expect("rx join okay").expect("rx okay"), 128);
    }

    #[test]
    fn test_raw_transmission() {
        let mut input = [0u8; 256];
        let mut output = [0u8; 256];
        (0..256usize).into_iter().enumerate().for_each(|(i, b)| input[i] = b as u8);

        let (mut tx, mut rx) = pipe();
        let tx_thread = std::thread::spawn(move || {
            Xmodem::transmit(&input[..], &mut rx).expect("transmit okay");
            rx.2
        });

        let rx_thread = std::thread::spawn(move || {
            Xmodem::receive(&mut tx, &mut output[..]).expect("receive okay");
            tx.2
        });

        let rx_buf = tx_thread.join().expect("tx join okay");
        let tx_buf = rx_thread.join().expect("rx join okay");

        // check packet 1
        assert_eq!(&rx_buf[0..3], &[SOH, 1, 255 - 1]);
        assert_eq!(&rx_buf[3..(3 + 128)], &input[..128]);
        assert_eq!(rx_buf[131], input[..128].iter().fold(0, |a: u8, b| a.wrapping_add(*b)));

        // check packet 2
        assert_eq!(&rx_buf[132..135], &[SOH, 2, 255 - 2]);
        assert_eq!(&rx_buf[135..(135 + 128)], &input[128..]);
        assert_eq!(rx_buf[263], input[128..].iter().fold(0, |a: u8, b| a.wrapping_add(*b)));

        // check EOT
        assert_eq!(&rx_buf[264..], &[EOT, EOT]);

        // check receiver responses
        assert_eq!(&tx_buf, &[NAK, ACK, ACK, NAK, ACK]);
    }

    #[test]
    fn test_small_packet_eof_error() {
        let mut xmodem = Xmodem::new(Cursor::new(vec![NAK, NAK, NAK]));

        let mut buffer = [1, 2, 3];
        let e = xmodem.read_packet(&mut buffer[..]).expect_err("read EOF");
        assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);

        let e = xmodem.write_packet(&buffer).expect_err("write EOF");
        assert_eq!(e.kind(), io::ErrorKind::UnexpectedEof);
    }

    #[test]
    fn test_bad_control() {
        let mut packet = [0; 128];
        let e = Xmodem::new(Cursor::new(vec![0, CAN]))
            .read_packet(&mut packet[..])
            .expect_err("CAN");

        assert_eq!(e.kind(), io::ErrorKind::ConnectionAborted);

        let e = Xmodem::new(Cursor::new(vec![0, 0xFF]))
            .read_packet(&mut packet[..])
            .expect_err("bad contorl");

        assert_eq!(e.kind(), io::ErrorKind::InvalidData);
    }

    #[test]
    fn test_eot() {
        let mut buffer = vec![NAK, 0, NAK, 0, ACK];
        Xmodem::new(Cursor::new(buffer.as_mut_slice()))
            .write_packet(&[])
            .expect("write empty buf for EOT");

        assert_eq!(&buffer[..], &[NAK, EOT, NAK, EOT, ACK]);
    }
}