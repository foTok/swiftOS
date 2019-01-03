use core::marker::{Copy, Clone}
pub enum Progress {
    /// Waiting for receiver to send NAK.
    Waiting,
    /// Download/upload has started.
    Started,
    /// Packet `.0` was transmitted/received.
    Packet(u8),
}

impl Clone for Progress {
    fn clone(&self) -> Progress{
        match self {
            Progress::Waiting => Progress::Waiting,
            Progress::Started => Progress::Started,
            Progress::Packet(id) => Progress::Packet(id),
        }
    }
}

impl Copy for Progress {}

/// Type for progress callbacks.
pub type ProgressFn = fn(Progress);

/// Noop progress callback.
pub fn noop(_: Progress) {  }
