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

    for x in v.iter_mut().take(length) {
        *x = func(reader)?;
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

pub fn decode_u32<R: std::io::Read>(reader: &mut R) -> Result<u32> {
    let length = read::unsigned(reader)?;
    if length > u32::MAX as u64 {
        return Err(Error::TooManyBytes(5));
    }

    Ok(length as u32)
}
