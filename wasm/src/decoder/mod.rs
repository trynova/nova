mod fn_section;
mod instructions;
mod module_types;
mod type_section;
mod util;

use crate::error::Error;
use crate::error::Result;
use crate::varint::decode_u32;
use fn_section::Func;
use std::collections::HashMap;
use std::io::Read;
use type_section::TypeDescriptor;
use util::decode_vec;

#[derive(Default)]
pub struct DecodedModule {
    /// Consumes section 1
    fn_types: Vec<TypeDescriptor>,
    // /// Consumes Section 2
    // imports: Vec<ImportDescriptor>,
    // /// Consumes Section 3 and 10
    funcs: Vec<Func>,
    // /// Consumes Section 4 and 9
    // tables: Vec<TableDescriptor>,
    // /// Consumes Section 5 and 11
    // memory: Vec<MemoryDescriptor>,
    // /// Consumes Section 6
    // globals: Vec<GlobalDescriptor>,
    // /// Consumes Section 7
    // exports: Vec<ExportDescriptor>,
    // /// Consumes Section 8
    // start_fn: Option<u32>,
}

impl DecodedModule {
    pub fn new<R: Read + std::io::Seek>(reader: &mut R) -> Result<Self> {
        let mut begin_bytes: [u8; 8] = [0; 8];
        reader.read_exact(&mut begin_bytes)?;
        let cookie = &begin_bytes[0..4];

        if cookie != "\0asm".as_bytes() {
            return Err(Error::InvalidSignature);
        }

        let version = u32::from_le_bytes(begin_bytes[4..8].try_into().unwrap());

        if version != 1 {
            return Err(Error::IncompatibleVersion);
        }

        let mut section_bytes: HashMap<u8, Vec<u8>> = HashMap::new();

        let mut last_section_id: u8 = 0;
        for _ in 0..12 {
            let mut indicator: [u8; 1] = [0; 1];
            if reader.read(&mut indicator)? == 0 {
                break;
            };

            if last_section_id >= indicator[0] {
                return Err(Error::InvalidSectionOrder);
            }

            let length = decode_u32(reader)?;
            let mut buffer = vec![0; length as usize];

            reader.read_exact(&mut buffer).unwrap();
            section_bytes.insert(indicator[0], buffer);

            last_section_id = indicator[0];
        }

        let mut module = Self::default();

        // Decode type section
        if let Some(bytes) = section_bytes.get(&1).as_mut() {
            module.fn_types = decode_vec(&mut bytes.as_slice(), TypeDescriptor::new)?;
        }

        // Decode Functions
        match (
            section_bytes.contains_key(&3),
            section_bytes.contains_key(&10),
        ) {
            (true, true) => {
                //
            }
            (false, false) => {}
            (true, false) => return Err(Error::MissingSection("Code Section")),
            (false, true) => return Err(Error::MissingSection("Function Section")),
        }

        Ok(module)
    }
}

#[cfg(test)]
mod test {
    use super::DecodedModule;
    #[test]
    pub fn test() {
        let mut bytes = std::fs::File::open("./src/decoder/mod.wasm").unwrap();
        DecodedModule::new(&mut bytes).unwrap();
    }
}
