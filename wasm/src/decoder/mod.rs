mod code_section;
mod common;
mod global_section;
mod import_section;
mod table_section;
mod type_section;
mod util;

use crate::error::Error;
use crate::error::Result;
use crate::varint::decode_u32;

pub enum Section {
    Type(Vec<common::FnType>),
    Import(Vec<common::Import>),
    Function(Vec<u32>),
    Table(Vec<common::Table>),
    Memory(Vec<common::ResizableLimits>),
    Global(Vec<common::GlobalDescriptor>),
    // ExportSection(Vec),
    // StartSection(u32),
    // ElementSection(Vec),
    Code(Vec<common::CodeBlock>),
    // DataSection(Vec),
}

#[derive(Default)]
pub struct Module {
    type_section: Option<Vec<common::FnType>>,
    import_section: Option<Vec<common::Import>>,
    fn_section: Option<Vec<u32>>,
    table_section: Option<Vec<common::Table>>,
    memory_section: Option<Vec<common::ResizableLimits>>,
    global_section: Option<Vec<common::GlobalDescriptor>>,
    code_section: Option<Vec<common::CodeBlock>>,
}

impl Module {
    pub fn new<R: crate::Reader>(reader: &mut R) -> Result<Self> {
        let mut module = Self::default();
        let mut last_section_id: i8 = -1;
        for i in 0..12 {
            if last_section_id >= i {
                return Err(Error::InvalidSectionOrder);
            }
            last_section_id = i;

            let section = decode_any_section(reader)?;
            match section {
                Section::Type(v) => module.type_section = Some(v),
                Section::Import(v) => module.import_section = Some(v),
                Section::Function(v) => module.fn_section = Some(v),
                Section::Table(v) => module.table_section = Some(v),
                Section::Memory(v) => module.memory_section = Some(v),
                Section::Global(v) => module.global_section = Some(v),
                // Section::ExportSection(v) => module.export_section = Some(v),
                // Section::StartSection(v) => module.start_section = Some(v),
                // Section::ElementSection(v) => module.element_section = Some(v),
                Section::Code(v) => module.code_section = Some(v),
                // Section::DataSection(v) => module.data_section = Some(v),
            }
        }

        Ok(module)
    }
}

pub fn decode_any_section<R: crate::Reader>(reader: &mut R) -> Result<Section> {
    let mut section_id_byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut section_id_byte)?;
    // if section_id_byte[0] == 0x08 {
    //     return Ok(Section::Start(leb128::read::unsigned(reader)? as u32));
    // };

    // Consume length. Maybe useful later
    decode_u32(reader)?;
    let section = match section_id_byte[0] {
        0x01 => {
            let vec = util::decode_vec(reader, type_section::decode_fn_type)?;
            Section::Type(vec)
        }
        0x02 => {
            let vec = util::decode_vec(reader, import_section::decode_import)?;
            Section::Import(vec)
        }
        0x03 => {
            let vec = util::decode_vec(reader, |r| Ok(decode_u32(r)?.value))?;
            Section::Function(vec)
        }
        0x04 => {
            let vec = util::decode_vec(reader, table_section::decode_table)?;
            Section::Table(vec)
        }
        0x05 => {
            let vec = util::decode_vec(reader, util::decode_resizable_limits)?;
            Section::Memory(vec)
        }
        0x06 => {
            let vec = util::decode_vec(reader, global_section::decode_global)?;
            Section::Global(vec)
        }
        // 0x07 => Section::Export(section_data),
        // 0x09 => Section::Element(section_data),
        0x0A => {
            let vec = util::decode_vec(reader, code_section::decode_code_block)?;
            Section::Code(vec)
        }
        // 0x0B => Section::Data(section_data),
        _ => {
            if section_id_byte[0] >= 0x02 && section_id_byte[0] <= 0x0B {
                unimplemented!("Section decoder not implemented yet");
            }
            return Err(Error::UnknownSectionID);
        }
    };

    Ok(section)
}
