use super::common::CodeBlock;
use super::util;
use crate::error::Result;
use crate::varint::decode_u32;

#[allow(clippy::read_zero_byte_vec)]
pub fn decode_code_section<R: std::io::Read>(reader: &mut R) -> Result<CodeBlock> {
    let body_size = decode_u32(reader)?.value;

    let v = util::decode_vec(reader, |x| Ok((decode_u32(x)?, util::decode_kind(x)?)))?;

    let mut locals = Vec::new();
    let mut read = 0;
    for (res, kind) in v {
        for _ in 0..res.value {
            locals.push(kind);
        }
        read += res.bytes_read as u32;
    }

    let instruction_size = body_size - read;

    let mut instructions = Vec::with_capacity(instruction_size as usize);

    reader.read_exact(&mut instructions)?;

    Ok(CodeBlock {
        locals: locals.into_boxed_slice(),
        instructions,
    })
}
