use super::common;
use super::util;
use crate::error::Error;
use crate::error::Result;

#[cfg(feature = "no_std")]
use core2::io;
#[cfg(not(feature = "no_std"))]
use std::io;

const FN_SIGNATURE: u8 = common::ValueKind::Func as u8;

pub fn decode_type_section<R: io::Read>(reader: &mut R) -> Result<common::FnType> {
    let mut byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut byte)?;

    if byte[0] != FN_SIGNATURE {
        return Err(Error::InvalidSignatureType);
    }

    let params = util::decode_vec(reader, util::decode_kind)?;
    let results = util::decode_vec(reader, util::decode_kind)?;
    Ok(common::FnType {
        params: params.into_boxed_slice(),
        results: results.into_boxed_slice(),
    })
}
