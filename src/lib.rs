//! A lightweight, `#![no_std]` implementation of the Serial Line CAN protocol.

#![doc(html_root_url = "https://docs.rs/mini-slcan/0.1.0")]
// Deny a few warnings in doctests, since rustdoc `allow`s many warnings by default
#![doc(test(attr(deny(unused_imports, unused_must_use))))]
#![warn(missing_debug_implementations, rust_2018_idioms)]
#![cfg_attr(not(test), no_std)]

mod error;
mod identifier;
pub mod read;
mod readme;
pub mod write;

pub use self::error::{Error, ErrorKind};
pub use self::identifier::{ExtIdentifier, Identifier};

use core::ops::{Deref, DerefMut};
use defmt::Format;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Format)]
pub enum Bitrate {
    _10kbit,
    _20kbit,
    _50kbit,
    _100kbit,
    _125kbit,
    _250kbit,
    _500kbit,
    _800kbit,
    _1mbit,
}

impl Bitrate {
    pub fn kbps(&self) -> u16 {
        match self {
            Bitrate::_10kbit => 10,
            Bitrate::_20kbit => 20,
            Bitrate::_50kbit => 50,
            Bitrate::_100kbit => 100,
            Bitrate::_125kbit => 125,
            Bitrate::_250kbit => 250,
            Bitrate::_500kbit => 500,
            Bitrate::_800kbit => 800,
            Bitrate::_1mbit => 1_000,
        }
    }
}

bitflags::bitflags! {
    /// Status flags reported by an SLCAN device.
    #[derive(Format)]
    pub struct Status: u8 {
        const RX_FIFO_FULL = 1 << 0;
        const TX_FIFO_FULL = 1 << 1;
        const ERROR_WARNING = 1 << 2;
        const DATA_OVERRUN = 1 << 3;
        //const UNUSED = 1 << 4;
        const ERROR_PASSIVE = 1 << 5;
        const ARBITRATION_LOST = 1 << 6;
        const BUS_ERROR = 1 << 7;
    }
}

/// 4-byte serial number of an SLCAN device.
#[derive(Copy, Clone, Eq, PartialEq, Debug, Format)]
pub struct SerialNumber([u8; 4]);

impl SerialNumber {
    /// Creates a new `SerialNumber` from 4 raw bytes.
    ///
    /// The bytes must be alphanumeric ASCII characters, or this will return `None`.
    pub fn new(raw: [u8; 4]) -> Option<Self> {
        if raw.iter().all(|b| b.is_ascii_alphanumeric()) {
            Some(Self(raw))
        } else {
            None
        }
    }

    pub const fn new_const(raw: [u8; 4]) -> Self {
        let valid = raw[0].is_ascii_alphanumeric()
            && raw[1].is_ascii_alphanumeric()
            && raw[2].is_ascii_alphanumeric()
            && raw[3].is_ascii_alphanumeric();

        if !valid {
            let serial_number_contain_non_alphanumeric_characters = ();

            #[allow(unconditional_panic)]
            [serial_number_contain_non_alphanumeric_characters][1];
        }

        Self(raw)
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Default)]
pub struct CanFrame {
    data: [u8; Self::MAX_LENGTH],
    len: u8,
}

impl CanFrame {
    pub const MAX_LENGTH: usize = 8;

    #[inline]
    pub const fn new() -> Self {
        Self {
            data: [0; Self::MAX_LENGTH],
            len: 0,
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.len.into()
    }

    #[inline]
    pub fn data(&self) -> &[u8] {
        &self.data[..usize::from(self.len)]
    }

    #[inline]
    pub fn data_mut(&mut self) -> &mut [u8] {
        &mut self.data[..usize::from(self.len)]
    }

    /// Appends a byte to this CAN frame.
    ///
    /// Returns an error when the frame is already full.
    pub fn push(&mut self, byte: u8) -> Result<(), Error> {
        if self.len() == Self::MAX_LENGTH {
            Err(Error::eof())
        } else {
            self.data[self.len()] = byte;
            self.len += 1;
            Ok(())
        }
    }
}

impl Deref for CanFrame {
    type Target = [u8];

    fn deref(&self) -> &[u8] {
        self.data()
    }
}

impl DerefMut for CanFrame {
    fn deref_mut(&mut self) -> &mut [u8] {
        self.data_mut()
    }
}

impl Format for CanFrame {
    fn format(&self, fmt: &mut defmt::Formatter) {
        self.data().format(fmt)
    }
}

macro_rules! impl_from {
    ( $($len:literal),+ ) => {
        $(
            impl From<[u8; $len]> for CanFrame {
                fn from(arr: [u8; $len]) -> Self {
                    let mut data = [0; Self::MAX_LENGTH];
                    data[..$len].copy_from_slice(&arr);
                    Self {
                        data,
                        len: $len,
                    }
                }
            }
        )+
    };
}

impl_from!(0, 1, 2, 3, 4, 5, 6, 7, 8);
