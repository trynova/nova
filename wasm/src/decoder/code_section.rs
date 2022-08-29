use super::common::CodeBlock;
use super::util;
use crate::error::Result;

pub fn decode_code_section<R: crate::Reader>(reader: &mut R) -> Result<CodeBlock> {
    let body_size = util::decode_u32(reader)? as u64;
    let offset = reader.seek(std::io::SeekFrom::Current(0))?;

    let v = util::decode_vec(reader, |x| {
        Ok((util::decode_u32(x)?, util::decode_kind(x)?))
    })?;

    let mut locals = Vec::new();
    for x in v {
        for _ in 0..x.0 {
            locals.push(x.1);
        }
    }

    let read = reader.seek(std::io::SeekFrom::Current(0))?;

    let instruction_size = body_size - (read - offset);

    let mut instructions = Vec::with_capacity(instruction_size as usize);

    reader.read_exact(&mut instructions)?;

    Ok(CodeBlock {
        locals,
        instructions,
    })
}
