use crate::error::Error;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct DecodedResult<T> {
    pub value: T,
    pub bytes_read: u8,
}

pub fn decode_u32<R: std::io::Read>(reader: &mut R) -> Result<DecodedResult<u32>, Error> {
    let mut length = 0;
    let mut value = 0;
    let mut bytes_read: u8 = 0;

    loop {
        let mut bytes: [u8; 1] = [0; 1];
        reader.read_exact(&mut bytes)?;
        bytes_read += 1;
        value |= ((bytes[0] & SEGMENT_BITS) as u32) << length;

        length += 7;
        if bytes[0] & CONTINUE_BIT == 0 {
            break;
        }

        if length >= 32 {
            return Err(Error::TooManyBytes {
                expected: 5,
                found: bytes_read,
            });
        }
    }

    length += length / 7;

    Ok(DecodedResult {
        value,
        bytes_read: length / 8,
    })
}

#[allow(dead_code)]
pub fn decode_u64<R: std::io::Read>(reader: &mut R) -> Result<DecodedResult<u64>, Error> {
    let mut length = 0;
    let mut value = 0;
    let mut bytes_read: u8 = 0;

    loop {
        let mut bytes: [u8; 1] = [0; 1];
        reader.read_exact(&mut bytes)?;
        bytes_read += 1;
        value |= ((bytes[0] & SEGMENT_BITS) as u64) << length;

        length += 7;
        if bytes[0] & CONTINUE_BIT == 0 {
            break;
        }

        if length >= 64 {
            return Err(Error::TooManyBytes {
                expected: 10,
                found: bytes_read,
            });
        }
    }

    Ok(DecodedResult { value, bytes_read })
}

#[cfg(test)]
mod test {

    #[test]
    fn decode_u32() {
        let mut bytes: &[u8] = &[0x00];
        let mut res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: 0,
                bytes_read: 1
            }
        );

        bytes = &[0x01];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: 1,
                bytes_read: 1
            }
        );

        bytes = &[0x80, 0x01];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: 128,
                bytes_read: 2
            }
        );

        bytes = &[0xDD, 0xC7, 0x01];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: 25565,
                bytes_read: 3
            }
        );

        bytes = &[0xFF, 0xFF, 0xFF, 0xFF, 0x07];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: i32::MAX as u32,
                bytes_read: 5
            }
        );

        bytes = &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: u32::MAX,
                bytes_read: 5
            }
        );
    }

    // Phosra: these tests suck. Skye said she'll make not use this stuff on Discord.
    /*#[test]
    fn decode_32_over_u32_max() {
        let mut bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        let res = super::decode_u32(&mut bytes).unwrap_err();
        match res {
            Error::TooManyBytes { expected, found } => {
                assert_eq!(expected, 5);
                assert_eq!(found, 5);
            }
            _ => unreachable!(),
        }
    }*/

    #[test]
    fn decode_u64() {
        let mut bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        let res = super::decode_u64(&mut bytes).unwrap();
        assert_eq!(
            res,
            super::DecodedResult {
                value: u64::MAX,
                bytes_read: 10
            }
        );
    }
}
