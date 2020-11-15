use defmt::Format;

#[derive(Debug, Format)]
pub struct Error {
    kind: ErrorKind,
}

impl Error {
    pub(crate) fn decode() -> Self {
        Self {
            kind: ErrorKind::Decode,
        }
    }

    pub(crate) fn eof() -> Self {
        Self {
            kind: ErrorKind::Eof,
        }
    }

    #[inline]
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }
}

#[derive(Eq, PartialEq, Copy, Clone, Debug, Format)]
#[non_exhaustive]
pub enum ErrorKind {
    /// Input is malformed and does not adhere to the SLCAN specification.
    Decode,

    /// More data is required to decode the input.
    Eof,
}

// TODO: impl Display+Debug, #[cfg] Error
