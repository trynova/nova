use leb128::read;

pub type Result<T> = std::result::Result<T, crate::error::Error>;

#[repr(u8)]
#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ReadZeroBytes,
    TooManyBytes(u8),
    InvalidSectionOrder,
    UnknownSectionID,
    InvalidValueKind,
    InvalidSignatureType,
    ArrayTooLarge,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => e.fmt(f)?,
            Self::ReadZeroBytes => f.write_str("Tried to read from reader but got 0 bytes")?,
            Self::TooManyBytes(size) => {
                f.write_str(&format!("Varint is more than {} bytes long", size))?
            }
            Self::InvalidValueKind => f.write_str("Invalid Value Kind")?,
            Self::InvalidSignatureType => f.write_str("Invalid Signature Kind")?,
            Self::InvalidSectionOrder => f.write_str("Invalid Section Order")?,
            Self::UnknownSectionID => f.write_str("Unknown Section ID")?,
            Self::ArrayTooLarge => f.write_str("Array Too Large")?,
        }

        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(v: std::io::Error) -> Self {
        Self::IoError(v)
    }
}

impl From<read::Error> for Error {
    fn from(v: read::Error) -> Self {
        match v {
            read::Error::IoError(e) => Self::IoError(e),
            read::Error::Overflow => Self::TooManyBytes(10),
        }
    }
}
