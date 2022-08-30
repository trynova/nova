use super::common::RefKind;
use super::common::Table;
use super::util;
use crate::error::Error;
use crate::error::Result;

const EXTERN_REF_SIGNATURE: u8 = RefKind::ExternalRef as u8;
const FUNC_REF_SIGNATURE: u8 = RefKind::FuncRef as u8;

pub fn decode_table<R: std::io::Read>(reader: &mut R) -> Result<Table> {
    let mut indicator_byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut indicator_byte)?;

    if indicator_byte[0] != EXTERN_REF_SIGNATURE && indicator_byte[0] != FUNC_REF_SIGNATURE {
        return Err(Error::InvalidSignatureType);
    }

    Ok(Table {
        kind: RefKind::ExternalRef,
        limits: util::decode_resizable_limits(reader)?,
    })
}
