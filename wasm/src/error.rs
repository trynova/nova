pub type Result<T> = std::result::Result<T, crate::error::Error>;

#[repr(u8)]
#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    ReadZeroBytes,
    TooManyBytes { expected: u8, found: u8 },
    InvalidSectionOrder,
    UnknownSectionID,
    InvalidValueKind,
    InvalidExternalKind,
    InvalidSignatureType,
    ArrayTooLarge,
    FromUtf8Error(std::str::Utf8Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
            Self::FromUtf8Error(v) => write!(f, "From utf-8 Error: {v}"),
        }?;

        Ok(())
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(v: std::io::Error) -> Self {
        Self::IoError(v)
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(v: std::string::FromUtf8Error) -> Self {
        v.utf8_error().into()
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(v: std::str::Utf8Error) -> Self {
        Self::FromUtf8Error(v)
    }
}
