use super::common::GlobalDescriptor;
use super::common::ValueKind;
use crate::error::Result;

pub fn decode_global<R: std::io::Read>(reader: &mut R) -> Result<GlobalDescriptor> {
    let mut bytes: [u8; 2] = [0; 2];
    reader.read_exact(&mut bytes)?;

    // Missing intializer value.
    Ok(GlobalDescriptor {
        kind: bytes[0].try_into()?,
        mutable: bytes[1] == 1,
    })
}
