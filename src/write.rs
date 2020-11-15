//! Encoding of SLCAN messages.

#[cfg(test)]
mod tests;

use core::mem;

use crate::{CanFrame, Error, ExtIdentifier, Identifier, SerialNumber, Status};
use defmt::Format;

const MAX_RESPONSE_LEN: usize = 6;
const MAX_NOTIF_LEN: usize = 1 + 8 + 1 + 16 + 4 + 1; // Tiiiiiiiilddddddddddddddddssss\r

/// A byte buffer that can hold any `Response`.
#[derive(Default, Debug)]
pub struct ResponseBuf([u8; MAX_RESPONSE_LEN]);

impl ResponseBuf {
    pub const LEN: usize = MAX_RESPONSE_LEN;

    pub const fn new() -> Self {
        Self([0; MAX_RESPONSE_LEN])
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn as_slice_mut(&mut self) -> &mut [u8] {
        &mut self.0
    }
}

/// A response to a `Command`.
#[derive(Debug, Clone, Eq, PartialEq, Format)]
#[non_exhaustive]
pub enum Response {
    /// General error response (ASCII BELL).
    Error,

    /// Generic acknowledgement of a command (`CR`).
    Ack,

    /// Standard CAN frame enqueued for transmission.
    TxAck,

    /// Extended CAN frame enqueued for transmission.
    ExtTxAck,

    /// Status flags response.
    Status(Status),

    /// Response to the `ReadVersion` command.
    Version {
        hardware_version: u8,
        software_version: u8,
    },

    /// Response to the `ReadSerial` command.
    Serial(SerialNumber),
}

impl Response {
    pub fn encode<'a>(&self, buf: &'a mut ResponseBuf) -> Result<&'a [u8], Error> {
        let mut writer = Writer { buf: &mut buf.0 };
        match self {
            Response::Error => {
                // BELL (ASCII 7) - not followed by CR
                writer.write(7)?;
            }
            Response::Ack => writer.write(b'\r')?,
            Response::TxAck => {
                writer.write(b'z')?;
                writer.write(b'\r')?;
            }
            Response::ExtTxAck => {
                writer.write(b'Z')?;
                writer.write(b'\r')?;
            }
            Response::Status(flags) => {
                writer.write(b'F')?;
                writer.write_hex_u8(flags.bits().into())?;
                writer.write(b'\r')?;
            }
            Response::Version {
                hardware_version,
                software_version,
            } => {
                writer.write(b'V')?;
                writer.write_hex_u8((*hardware_version).into())?;
                writer.write_hex_u8((*software_version).into())?;
                writer.write(b'\r')?;
            }
            Response::Serial(serial) => {
                writer.write(b'N')?;
                for byte in &serial.0 {
                    writer.write(*byte)?;
                }
                writer.write(b'\r')?;
            }
        }

        let remaining = writer.buf.len();
        let used = buf.0.len() - remaining;
        Ok(&buf.0[..used])
    }
}

#[derive(Default, Debug)]
pub struct NotificationBuf([u8; MAX_NOTIF_LEN]);

impl NotificationBuf {
    pub const fn new() -> Self {
        Self([0; MAX_NOTIF_LEN])
    }
}

/// An unprompted message sent by the SLCAN device.
#[derive(Debug, Format)]
pub enum Notification {
    Rx {
        identifier: Identifier,
        frame: CanFrame,
    },

    RxExt {
        identifier: ExtIdentifier,
        frame: CanFrame,
    },

    RxRtr {
        identifier: Identifier,
        /// Must be in range 0..=8.
        len: u8,
    },

    RxExtRtr {
        identifier: ExtIdentifier,
        /// Must be in range 0..=8.
        len: u8,
    },
}

impl Notification {
    pub fn encode<'a>(&self, buf: &'a mut NotificationBuf) -> Result<&'a [u8], Error> {
        let mut writer = Writer { buf: &mut buf.0 };
        match self {
            Notification::Rx { identifier, frame } => {
                writer.write(b't')?;
                writer.write_identifier(*identifier)?;
                writer.write_frame(frame)?;
            }
            Notification::RxExt { identifier, frame } => {
                writer.write(b'T')?;
                writer.write_ext_identifier(*identifier)?;
                writer.write_frame(frame)?;
            }
            Notification::RxRtr { identifier, len } => {
                writer.write(b'r')?;
                writer.write_identifier(*identifier)?;
                writer.write_hex_u4(*len)?;
            }
            Notification::RxExtRtr { identifier, len } => {
                writer.write(b'R')?;
                writer.write_ext_identifier(*identifier)?;
                writer.write_hex_u4(*len)?;
            }
        }

        let remaining = writer.buf.len();
        let used = buf.0.len() - remaining;
        Ok(&buf.0[..used])
    }
}

/// A notification with 16-bit timestamp.
///
/// Timestamps are disabled by default, and are turned on by the host by sending a `SetRxTimestamp`
/// command.
#[derive(Debug)]
pub struct TimestampedNotification {
    notif: Notification,
    timestamp: u16,
}

impl TimestampedNotification {
    /// `timestamp` must be in range `0..=0xEA5F`.
    pub fn new(notif: Notification, timestamp: u16) -> Self {
        Self { notif, timestamp }
    }

    pub fn encode<'a>(&self, buf: &'a mut NotificationBuf) -> Result<&'a [u8], Error> {
        let used = self.notif.encode(buf)?.len();
        let mut writer = Writer {
            buf: &mut buf.0[used..],
        };
        writer.write_hex_u16(self.timestamp)?;

        let remaining = writer.buf.len();
        let used = buf.0.len() - remaining;
        Ok(&buf.0[..used])
    }
}

struct Writer<'a> {
    buf: &'a mut [u8],
}

impl<'a> Writer<'a> {
    fn write(&mut self, byte: u8) -> Result<(), Error> {
        let buf = mem::replace(&mut self.buf, &mut []);
        match buf {
            [] => Err(Error::eof()),
            [b, rest @ ..] => {
                *b = byte;
                self.buf = rest;
                Ok(())
            }
        }
    }

    fn write_hex_u4(&mut self, val: u8) -> Result<(), Error> {
        self.write_hex(val.into(), 1)
    }

    fn write_hex_u8(&mut self, val: u8) -> Result<(), Error> {
        self.write_hex(val.into(), 2)
    }

    fn write_hex_u16(&mut self, val: u16) -> Result<(), Error> {
        self.write_hex(val.into(), 4)
    }

    fn write_identifier(&mut self, id: Identifier) -> Result<(), Error> {
        self.write_hex(id.as_raw().into(), 3)
    }

    fn write_ext_identifier(&mut self, id: ExtIdentifier) -> Result<(), Error> {
        self.write_hex(id.as_raw().into(), 8)
    }

    fn write_frame(&mut self, frame: &CanFrame) -> Result<(), Error> {
        self.write_hex_u4(frame.len() as u8)?;
        for b in frame.data() {
            self.write_hex_u8(*b)?;
        }
        Ok(())
    }

    fn write_hex(&mut self, value: u32, digits: u8) -> Result<(), Error> {
        let mut shift = digits * 4;

        for _ in 0..digits {
            shift -= 4;
            let digit = (value >> shift) & 0xF;
            self.write(hex(digit as u8))?;
        }

        Ok(())
    }
}

fn hex(nibble: u8) -> u8 {
    match nibble {
        0..=9 => b'0' + nibble,
        10..=15 => b'A' + nibble - 10,
        _ => unreachable!(),
    }
}
