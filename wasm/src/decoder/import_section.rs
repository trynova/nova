use super::common::Import;
use crate::error::Result;
use crate::varint::decode_u32;

pub fn decode_import_section<R: std::io::Read>(reader: &mut R) -> Result<Import> {
    let mut module_name = Vec::with_capacity(decode_u32(reader)?.value as usize);
    reader.read_exact(&mut module_name)?;
    let mut name = Vec::with_capacity(decode_u32(reader)?.value as usize);
    reader.read_exact(&mut name)?;
    let mut kind_byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut kind_byte)?;

    Ok(Import {
        module_name: String::from_utf8(module_name)?,
        export_name: String::from_utf8(name)?,
        kind: kind_byte[0].try_into()?,
    })
}
