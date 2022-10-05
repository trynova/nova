use super::module_types::ValueKind;
use super::util::decode_vec;
use crate::error::Error;
use crate::error::Result;

#[derive(Debug, Default, PartialEq)]
pub struct TypeDescriptor {
    params: Vec<ValueKind>,
    results: Vec<ValueKind>,
}

impl TypeDescriptor {
    pub fn new<R: std::io::Read>(bytes: &mut R) -> Result<Self> {
        let mut buff: [u8; 1] = [0; 1];
        bytes.read_exact(&mut buff)?;

        if buff[0] != 0x60 {
            return Err(Error::InvalidSignature);
        }

        let params = decode_vec(bytes, |x| {
            let mut byte_buff: [u8; 1] = [0; 1];
            x.read_exact(&mut byte_buff)?;

            byte_buff[0].try_into()
        })?;

        let results = decode_vec(bytes, |x| {
            let mut byte_buff: [u8; 1] = [0; 1];
            x.read_exact(&mut byte_buff)?;

            byte_buff[0].try_into()
        })?;

        Ok(Self { params, results })
    }

    #[cfg(test)]
    fn from(params: Vec<ValueKind>, results: Vec<ValueKind>) -> Self {
        Self { params, results }
    }
}

#[cfg(test)]
mod test {
    use super::super::module_types::ValueKind;
    use super::TypeDescriptor;
    use crate::error::Error;

    #[test]
    fn empty_descriptor() {
        let mut bytes: &[u8] = &vec![0x60, 0x00, 0x00];
        let result = TypeDescriptor::new(&mut bytes).unwrap();
        let reference = TypeDescriptor::from(vec![], vec![]);
        assert_eq!(reference, result);
    }

    #[test]
    fn params_i32_i32_results_i32_i32() {
        let mut bytes: &[u8] = &vec![0x60, 0x02, 0x7F, 0x7F, 0x02, 0x7F, 0x7F];
        let result = TypeDescriptor::new(&mut bytes).unwrap();
        let reference = TypeDescriptor::from(
            vec![ValueKind::I32, ValueKind::I32],
            vec![ValueKind::I32, ValueKind::I32],
        );

        assert_eq!(reference, result);
    }

    #[test]
    fn invalid_signature() {
        let mut bytes: &[u8] = &vec![0x61];
        let error = TypeDescriptor::new(&mut bytes).unwrap_err();
        assert_eq!(Error::InvalidSignature, error);
    }

    #[test]
    fn invalid_value_kind() {
        let mut bytes: &[u8] = &vec![0x60, 0x01, 0x71];
        let error = TypeDescriptor::new(&mut bytes).unwrap_err();
        assert_eq!(Error::InvalidValueKind, error);
    }
}
