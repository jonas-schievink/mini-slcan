//! Decoding of SLCAN messages.

#[cfg(test)]
mod tests;

use crate::{Bitrate, CanFrame, Error, ExtIdentifier, Identifier};
use defmt::Format;

/// A command sent from the host to the SLCAN device.
#[derive(Debug, Eq, PartialEq, Format)]
#[non_exhaustive]
pub enum Command {
    SetupWithBitrate {
        bitrate: Bitrate,
    },

    Open,

    Close,

    TxStandard {
        identifier: Identifier,
        frame: CanFrame,
    },

    /// Transmit an extended CAN frame.
    TxExt {
        identifier: ExtIdentifier,
        frame: CanFrame,
    },

    TxStandardRtr {
        identifier: Identifier,
        len: u8,
    },

    TxExtRtr {
        identifier: ExtIdentifier,
        len: u8,
    },

    ReadStatus,
    ReadVersion,
    ReadSerial,
    SetRxTimestamp {
        timestamp: bool,
    },
}

impl Command {
    pub const MAX_ENCODED_LEN: usize = 1 + 8 + 1 + 16 + 1; // Tiiiiiiiildddddddddddddddd\r

    /// Decodes a command from an input string. The input must contain the terminating `CR`
    /// character (ASCII 13).
    pub fn decode(input: &[u8]) -> Result<Self, Error> {
        let mut reader = Reader { input };

        let op = reader.read_byte()?;
        let cmd = match op {
            b'S' => {
                let bitrate = match reader.read_byte()? {
                    b'0' => Bitrate::_10kbit,
                    b'1' => Bitrate::_20kbit,
                    b'2' => Bitrate::_50kbit,
                    b'3' => Bitrate::_100kbit,
                    b'4' => Bitrate::_125kbit,
                    b'5' => Bitrate::_250kbit,
                    b'6' => Bitrate::_500kbit,
                    b'7' => Bitrate::_800kbit,
                    b'8' => Bitrate::_1mbit,
                    _ => return Err(Error::decode()),
                };

                Command::SetupWithBitrate { bitrate }
            }
            b'O' => Command::Open,
            b'C' => Command::Close,
            b't' => {
                let identifier = reader.read_hex_identifier()?;
                let len = reader.read_hex_u4()?;
                if len > 8 {
                    return Err(Error::decode());
                }
                let frame = reader.read_frame(len)?;

                Command::TxStandard { identifier, frame }
            }
            b'T' => {
                let identifier = reader.read_hex_ext_identifier()?;
                let len = reader.read_hex_u4()?;
                if len > 8 {
                    return Err(Error::decode());
                }
                let frame = reader.read_frame(len)?;

                Command::TxExt { identifier, frame }
            }
            b'r' => {
                let identifier = reader.read_hex_identifier()?;
                let len = reader.read_hex_u4()?;
                if len > 8 {
                    return Err(Error::decode());
                }

                Command::TxStandardRtr { identifier, len }
            }
            b'R' => {
                let identifier = reader.read_hex_ext_identifier()?;
                let len = reader.read_hex_u4()?;
                if len > 8 {
                    return Err(Error::decode());
                }

                Command::TxExtRtr { identifier, len }
            }
            b'F' => Command::ReadStatus,
            b'V' => Command::ReadVersion,
            b'N' => Command::ReadSerial,
            b'Z' => {
                let timestamp = match reader.read_byte()? {
                    b'0' => false,
                    b'1' => true,
                    _ => return Err(Error::decode()),
                };

                Command::SetRxTimestamp { timestamp }
            }
            _ => return Err(Error::decode()),
        };

        if reader.read_byte()? != b'\r' {
            return Err(Error::decode());
        }

        // Reject trailing undecoded data.
        if !reader.input.is_empty() {
            return Err(Error::decode());
        }

        Ok(cmd)
    }
}

/// A byte buffer that yields decoded `Command`s.
///
/// This is meant to be used by apps that receive bytewise data and want to decode `Command`s from
/// that.
#[derive(Default, Debug)]
pub struct CommandBuf {
    /// Invariant: `bytes[..used]` Never contains `\r`.
    bytes: [u8; Command::MAX_ENCODED_LEN],
    used: u8,
}

impl CommandBuf {
    /// Creates a new, empty `CommandBuf`.
    pub const fn new() -> Self {
        Self {
            bytes: [0; Command::MAX_ENCODED_LEN],
            used: 0,
        }
    }

    /// Returns the currently unused part of the buffer.
    ///
    /// The caller can copy new input bytes into the returned slice, and call `advance_by` to mark
    /// them as part of the `CommandBuf`.
    pub fn tail_mut(&mut self) -> &mut [u8] {
        &mut self.bytes[usize::from(self.used)..]
    }

    fn is_full(&self) -> bool {
        usize::from(self.used) == Command::MAX_ENCODED_LEN
    }

    fn find_cr(&self, start: usize) -> Option<usize> {
        self.bytes[start..usize::from(self.used)]
            .iter()
            .position(|b| *b == b'\r')
            .map(|pos| pos + start)
    }

    /// Marks `len` more bytes from the buffer's tail as consumed, and returns an iterator over all
    /// `Command`s in the buffer.
    ///
    /// When dropped, the returned iterator will remove the decoded bytes from the `CommandBuf`.
    pub fn advance_by(&mut self, amount: u8) -> impl Iterator<Item = Result<Command, Error>> + '_ {
        self.used += amount;
        assert!(usize::from(self.used) <= Command::MAX_ENCODED_LEN);

        CommandIter { buf: self, pos: 0 }
    }
}

struct CommandIter<'a> {
    buf: &'a mut CommandBuf,
    pos: u8,
}

impl Iterator for CommandIter<'_> {
    type Item = Result<Command, Error>;

    fn next(&mut self) -> Option<Self::Item> {
        let pos = usize::from(self.pos);
        let end = match self.buf.find_cr(pos) {
            Some(pos) => pos,
            None if pos == 0 && self.buf.is_full() => {
                // There is no `\r` in the entire buffer to terminate the received command. That
                // means that the input is invalid, since the buffer can hold the longest command,
                // including the trailing `\r`.
                // Yield an error, and mark the whole buffer as consumed to make space for new data.
                self.pos = Command::MAX_ENCODED_LEN as u8;
                return Some(Err(Error::decode()));
            }
            None => return None,
        };

        let cmd = &self.buf.bytes[pos..end + 1];
        self.pos += cmd.len() as u8;

        Some(Command::decode(cmd))
    }
}

impl Drop for CommandIter<'_> {
    fn drop(&mut self) {
        let decoded_end = usize::from(self.pos);
        self.buf.bytes.copy_within(decoded_end.., 0);
        self.buf.used -= decoded_end as u8;
    }
}

#[derive(Eq, PartialEq)]
struct Reader<'a> {
    input: &'a [u8],
}

impl<'a> Reader<'a> {
    fn read_byte(&mut self) -> Result<u8, Error> {
        match self.input {
            [] => Err(Error::eof()),
            [b, rest @ ..] => {
                self.input = rest;
                Ok(*b)
            }
        }
    }

    fn read_hex_digits(&mut self, digits: u8) -> Result<u32, Error> {
        let mut val = 0;

        for _ in 0..digits {
            val <<= 4;
            val |= unhex(self.read_byte()?)? as u32;
        }

        Ok(val)
    }

    fn read_hex_u4(&mut self) -> Result<u8, Error> {
        Ok(self.read_hex_digits(1)? as u8)
    }

    fn read_hex_u8(&mut self) -> Result<u8, Error> {
        Ok(self.read_hex_digits(2)? as u8)
    }

    fn read_hex_identifier(&mut self) -> Result<Identifier, Error> {
        let raw = self.read_hex_digits(3)? as u16;
        Identifier::from_raw(raw).ok_or(Error::decode())
    }

    fn read_hex_ext_identifier(&mut self) -> Result<ExtIdentifier, Error> {
        let raw = self.read_hex_digits(8)?;
        ExtIdentifier::from_raw(raw).ok_or(Error::decode())
    }

    fn read_frame(&mut self, len: u8) -> Result<CanFrame, Error> {
        assert!(len <= 8);

        let mut frame = CanFrame::new();

        for _ in 0..len {
            let byte = self.read_hex_u8()?;

            // Can never fail, because we limit `len` to 8 or less.
            frame.push(byte).unwrap();
        }

        Ok(frame)
    }
}

fn unhex(digit: u8) -> Result<u8, Error> {
    match digit {
        b'0'..=b'9' => Ok(digit - b'0'),
        b'A'..=b'F' => Ok(digit - b'A' + 10),
        _ => Err(Error::decode()),
    }
}
