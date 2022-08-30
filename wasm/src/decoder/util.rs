use super::common;
use crate::error::Error;
use crate::error::Result;
use crate::varint::decode_u32;
pub(crate) fn decode_vec<T, R, F>(reader: &mut R, func: F) -> Result<Vec<T>>
where
    R: std::io::Read,
    F: Fn(&mut R) -> Result<T>,
{
    // This is fine. It's already range checked by `decode_u32`
    let length = decode_u32(reader)?.0 as usize;

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
