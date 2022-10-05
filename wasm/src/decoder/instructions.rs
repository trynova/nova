use super::module_types::ValueKind;
use super::util::decode_vec;
use crate::error::Result;
use crate::varint::decode_u32;
use std::io::Read;

pub enum BlockType {
    Void,
    Value(ValueKind),
    /// `u32` refers to the result type index in the type section
    Signature(u32),
}

impl BlockType {
    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self> {
        let mut byte: [u8; 1] = [0; 1];
        reader.read_exact(&mut byte)?;
        if byte[0] == 0x40 {
            return Ok(Self::Void);
        } else if let Ok(v) = byte[0].try_into() {
            Ok(Self::Value(v))
        } else {
            Ok(Self::Signature(decode_u32(reader)?))
        }
    }
}

pub enum Instruction {
    Unreachable,
    Nop,
    Block(BlockType),
    Loop(BlockType),
    If(BlockType),
    Else,
    Br(u32),
    BrIf(u32),
    BrTable { branches: Vec<u32>, fallback: u32 },
    Return,
    Call(u32),
    CallIndirect { type_index: u32, table_index: u32 },

    RefNull(ValueKind),
    RefIsNull,
    RefFunc(u32),

    Drop,
    Select(Option<Vec<ValueKind>>),

    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
}

impl Instruction {
    pub fn from_reader<R: Read>(reader: &mut R) -> Result<Self> {
        let mut byte: [u8; 1] = [0; 1];
        reader.read_exact(&mut byte)?;
        let v = match byte[0] {
            0x00 => Self::Unreachable,
            0x01 => Self::Nop,
            // Block
            0x02 => Self::Block(BlockType::from_reader(reader)?),
            // Loop
            0x03 => Self::Loop(BlockType::from_reader(reader)?),
            0x04 => Self::If(BlockType::from_reader(reader)?),
            0x05 => Self::Else,
            0x0C => Self::Br(decode_u32(reader)?),
            0x0D => Self::BrIf(decode_u32(reader)?),
            0x0E => Self::BrTable {
                branches: decode_vec(reader, decode_u32)?,
                fallback: decode_u32(reader)?,
            },
            0x0F => Self::Return,
            0x10 => Self::Call(decode_u32(reader)?),
            0x11 => Self::CallIndirect {
                type_index: decode_u32(reader)?,
                table_index: decode_u32(reader)?,
            },

            0xD0 => {
                let mut val_byte: [u8; 1] = [0; 1];
                reader.read_exact(&mut val_byte)?;

                Self::RefNull(val_byte[0].try_into()?)
            }
            0xD1 => Self::RefIsNull,
            0xD2 => Self::RefFunc(decode_u32(reader)?),

            0x1A => Self::Drop,
            0x1B => Self::Select(None),
            0x1C => Self::Select(Some(decode_vec(reader, |x| {
                let mut b: [u8; 1] = [0; 1];
                x.read_exact(&mut b)?;
                b[0].try_into()
            })?)),

            0x20 => Self::LocalGet(decode_u32(reader)?),
            0x21 => Self::LocalSet(decode_u32(reader)?),
            0x22 => Self::LocalTee(decode_u32(reader)?),
            0x23 => Self::GlobalGet(decode_u32(reader)?),
            0x24 => Self::GlobalSet(decode_u32(reader)?),

            0x25 => Self::TableGet(decode_u32(reader)?),
            0x26 => Self::TableSet(decode_u32(reader)?),
        };

        Ok(v)
    }
}
