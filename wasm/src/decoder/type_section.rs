use super::common;
use super::util;
use crate::error::Error;
use crate::error::Result;

const FN_SIGNATURE: u8 = common::ValueKind::Func as u8;

pub fn decode_type_section<R: std::io::Read>(reader: &mut R) -> Result<common::FnType> {
    let mut byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut byte)?;

    if byte[0] != FN_SIGNATURE {
        return Err(Error::InvalidSignatureType);
    }

    let params = util::decode_vec(reader, util::decode_kind)?;
    let result = util::decode_vec(reader, util::decode_kind)?;
    Ok(common::FnType {
        params: params.into_boxed_slice(),
        result: result.into_boxed_slice(),
    })
}
