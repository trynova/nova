#[cfg(feature = "no_std")]
use core2::io;
#[cfg(not(feature = "no_std"))]
use std::io;

pub type Result<T> = core::result::Result<T, Error>;

#[repr(u8)]
#[derive(Debug)]
pub enum Error {
    IoError(io::Error),
    ReadZeroBytes,
    TooManyBytes { expected: u8, found: u8 },
    InvalidSectionOrder,
    UnknownSectionID,
    InvalidValueKind,
    InvalidExternalKind,
    InvalidSignatureType,
    ArrayTooLarge,
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::IoError(e) => e.fmt(f),
            Self::ReadZeroBytes => f.write_str("Tried to read from reader but got 0 bytes"),
            Self::TooManyBytes { expected, found } => write!(
                f,
                "Varint is too long! Expected {expected} bytes but found {found}"
            ),
            Self::InvalidValueKind => f.write_str("Invalid Value Kind"),
            Self::InvalidSignatureType => f.write_str("Invalid Signature Kind"),
            Self::InvalidSectionOrder => f.write_str("Invalid Section Order"),
            Self::UnknownSectionID => f.write_str("Unknown Section ID"),
            Self::ArrayTooLarge => f.write_str("Array Too Large"),
            Self::InvalidExternalKind => f.write_str("Invalid External Kind"),
        }?;

        Ok(())
    }
}

impl core::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(v: io::Error) -> Self {
        Self::IoError(v)
    }
}
