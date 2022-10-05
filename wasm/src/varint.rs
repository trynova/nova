use crate::error::Error;

const SEGMENT_BITS: u8 = 0x7F;
const CONTINUE_BIT: u8 = 0x80;

pub fn decode_u32<R: std::io::Read>(reader: &mut R) -> Result<u32, Error> {
    let mut length = 0;
    let mut value = 0;

    loop {
        let mut bytes: [u8; 1] = [0; 1];
        reader.read_exact(&mut bytes)?;
        value |= ((bytes[0] & SEGMENT_BITS) as u32) << length;

        length += 7;
        if bytes[0] & CONTINUE_BIT == 0 {
            break;
        }

        if length >= 32 {
            return Err(Error::TooManyBytes {
                expected: 5,
                found: length / 7 + 1,
            });
        }
    }

    length += length / 7;

    Ok(value)
}

pub fn decode_u64<R: std::io::Read>(reader: &mut R) -> Result<u64, Error> {
    let mut length = 0;
    let mut value = 0;

    loop {
        let mut bytes: [u8; 1] = [0; 1];
        reader.read_exact(&mut bytes)?;
        value |= ((bytes[0] & SEGMENT_BITS) as u64) << length;

        length += 7;
        if bytes[0] & CONTINUE_BIT == 0 {
            break;
        }

        if length >= 64 {
            return Err(Error::TooManyBytes {
                expected: 5,
                found: length / 8,
            });
        }
    }

    Ok(value)
}

#[cfg(test)]
mod test {
    use crate::error::Error;
    #[test]
    fn decode_u32() {
        let mut bytes: &[u8] = &[0x00];
        let mut res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(res, 0);

        bytes = &[0x01];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(res, 1);

        bytes = &[0x80, 0x01];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(res, 128);

        bytes = &[0xDD, 0xC7, 0x01];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(res, 25565);

        bytes = &[0xFF, 0xFF, 0xFF, 0xFF, 0x07];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(res, i32::MAX as u32);

        bytes = &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F];
        res = super::decode_u32(&mut bytes).unwrap();
        assert_eq!(res, u32::MAX);
    }

    #[test]
    fn decode_32_over_u32_max() {
        let mut bytes: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x01];
        let res = super::decode_u32(&mut bytes).unwrap_err();
        match res {
            Error::TooManyBytes { expected, found } => {
                assert_eq!(expected, 5);
                assert_eq!(found, 6);
            }
            _ => unreachable!(),
        }
    }
}
