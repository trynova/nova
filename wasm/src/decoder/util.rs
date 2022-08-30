use super::common;
use crate::error::Error;
use crate::error::Result;
use leb128::read;

pub(crate) fn decode_vec<T, R, F>(reader: &mut R, func: F) -> Result<Vec<T>>
where
    R: std::io::Read,
    F: Fn(&mut R) -> Result<T>,
{
    let length = read::unsigned(reader)? as usize;
    if length > u32::MAX as usize {
        return Err(Error::ArrayTooLarge);
    }

    let mut v = Vec::with_capacity(length);

    for _ in 0..length {
        v.push(func(reader)?);
    }

    Ok(v)
}

pub fn decode_kind<R: std::io::Read>(reader: &mut R) -> Result<common::ValueKind> {
    let mut byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut byte)?;

    match byte[0].try_into() {
        Ok(v) => Ok(v),
        Err(_) => Err(Error::InvalidValueKind),
    }
}

pub fn decode_u32<R: crate::Reader>(reader: &mut R) -> Result<u32> {
    let s = reader.seek(std::io::SeekFrom::Current(0))?;
    let length = read::unsigned(reader)?;
    // This is so wacky to do. Replacing it with a better system should be done at some point
    let end = reader.seek(std::io::SeekFrom::Current(0))?;

    if length > u32::MAX as u64 {
        // The `as u8` is fine here because the max that `read::unsigned` reads is 10 bytes.
        return Err(Error::TooManyBytes {
            expected: 5,
            found: (end - s) as u8,
        });
    }

    Ok(length as u32)
}
