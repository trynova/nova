use crate::error::Error;
use crate::error::Result;

use crate::varint::decode_u32;

pub(crate) fn decode_vec<T, R, F>(reader: &mut R, func: F) -> Result<Vec<T>>
where
    R: std::io::Read,
    F: Fn(&mut R) -> Result<T>,
{
    // This is fine. It's already range checked by `decode_u32`
    let length = decode_u32(reader)? as usize;

    let mut v = Vec::with_capacity(length);

    for _ in 0..length {
        v.push(func(reader)?);
    }

    Ok(v)
}
