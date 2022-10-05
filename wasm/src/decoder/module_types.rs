use crate::error::Error;
use crate::error::Result;

use crate::varint::decode_u32;

use std::io::Read;

#[derive(Debug, PartialEq)]
pub struct ResizableLimits {
    pub min: u32,
    pub max: Option<u32>,
}

impl ResizableLimits {
    pub fn new<R: Read>(data: &mut R) -> Result<Self> {
        let has_max = decode_u32(data)? == 0x01;

        let min_result = decode_u32(data)?;
        let min = min_result;

        let max = if has_max {
            let value = decode_u32(data)?;
            if value < min {
                return Err(Error::MaxBiggerThanMin);
            }

            Some(value)
        } else {
            None
        };

        Ok(Self { min, max })
    }
}

#[repr(u8)]
#[derive(Debug, PartialEq)]
pub enum ExternalKind {
    Function = 0x00,
    Table = 0x01,
    Memory = 0x02,
    Global = 0x03,
}

impl TryFrom<u8> for ExternalKind {
    type Error = Error;
    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            0x00 => Ok(Self::Function),
            0x01 => Ok(Self::Table),
            0x02 => Ok(Self::Memory),
            0x03 => Ok(Self::Global),
            _ => Err(Error::InvalidExternalKind),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum ValueKind {
    // Number Kinds
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,

    // Vector Kinds
    V128 = 0x7B,

    // Reference Kinds
    FuncRef = 0x70,
    ExternRef = 0x6F,
}

impl TryFrom<u8> for ValueKind {
    type Error = Error;

    fn try_from(byte: u8) -> Result<Self> {
        match byte {
            0x7F => Ok(Self::I32),
            0x7E => Ok(Self::I64),
            0x7D => Ok(Self::F32),
            0x7C => Ok(Self::F64),
            0x7B => Ok(Self::V128),
            0x70 => Ok(Self::FuncRef),
            0x6F => Ok(Self::ExternRef),
            _ => Err(Error::InvalidValueKind),
        }
    }
}

#[cfg(test)]
mod test {
    use super::ExternalKind;
    use super::ResizableLimits;
    use crate::error::Error;

    #[test]
    fn decode_resizable_limits_min() {
        let mut bytes: &[u8] = &vec![0x00, 0x01];
        let desc = ResizableLimits::new(&mut bytes).unwrap();
        assert_eq!(desc, { ResizableLimits { min: 1, max: None } })
    }

    #[test]
    fn decode_resizable_limits_minmax() {
        let mut bytes: &[u8] = &vec![0x01, 0x01, 0x01];
        let desc = ResizableLimits::new(&mut bytes).unwrap();
        assert_eq!(desc, {
            ResizableLimits {
                min: 1,
                max: Some(1),
            }
        })
    }

    #[test]
    fn decode_resizable_limits_invalid() {
        let mut bytes: &[u8] = &vec![0x01, 0x02, 0x01];
        let desc = ResizableLimits::new(&mut bytes).unwrap_err();
        assert_eq!(desc, Error::MaxBiggerThanMin);
    }

    #[test]
    fn decode_external_kinds() {
        const EXT_KINDS: [ExternalKind; 4] = [
            ExternalKind::Function,
            ExternalKind::Table,
            ExternalKind::Memory,
            ExternalKind::Global,
        ];

        for x in 0..4 {
            let f: ExternalKind = x.try_into().unwrap();
            assert_eq!(f, EXT_KINDS[x as usize]);
        }
    }

    #[test]
    fn decode_external_kind_invalid() {
        let byte: u8 = 10;
        let err = ExternalKind::try_from(byte).unwrap_err();
        assert_eq!(err, Error::InvalidExternalKind);
    }
}
