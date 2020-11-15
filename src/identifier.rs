//! Defines CAN identifier types.

use core::fmt;
use defmt::Format;

/// Standard 11-bit CAN identifier.
#[derive(Copy, Clone, Eq, PartialEq, Format)]
pub struct Identifier(u16);

impl Identifier {
    pub fn from_raw(raw: u16) -> Option<Self> {
        if raw > 0x7FF {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub fn as_raw(&self) -> u16 {
        self.0
    }
}

impl fmt::Debug for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:03X}", self.0)
    }
}

/// Extended 29-bit identifier.
#[derive(Copy, Clone, Eq, PartialEq, Format)]
pub struct ExtIdentifier(u32);

impl ExtIdentifier {
    pub fn from_raw(raw: u32) -> Option<Self> {
        if raw > 0x1FFFFFFF {
            None
        } else {
            Some(Self(raw))
        }
    }

    pub fn as_raw(&self) -> u32 {
        self.0
    }
}

impl fmt::Debug for ExtIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:08X}", self.0)
    }
}
