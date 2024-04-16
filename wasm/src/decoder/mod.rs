mod code_section;
mod common;
mod type_section;
mod util;
#[cfg(feature = "no_std")]
use alloc::vec::Vec;
#[cfg(feature = "no_std")]
use core2::io;
#[cfg(not(feature = "no_std"))]
use std::io;

use crate::error::Error;
use crate::error::Result;
use crate::varint::decode_u32;

pub enum Section {
    Type(Vec<common::FnType>),
    // ImportSection(Vec<Import>),
    Function(Vec<u32>),
    // TableSection(Vec<ResizableLimits>),
    // MemorySection(Vec<ResizableLimits>),
    // GlobalSection(Vec),
    // ExportSection(Vec),
    // StartSection(u32),
    // ElementSection(Vec),
    Code(Vec<common::CodeBlock>),
    // DataSection(Vec),
}

#[derive(Default)]
pub struct Module {
    type_section: Option<Vec<common::FnType>>,
    fn_section: Option<Vec<u32>>,
    code_section: Option<Vec<common::CodeBlock>>,
}

impl Module {
    pub fn new<R: io::Read>(reader: &mut R) -> Result<Self> {
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
                // Section::ImportSection(v) => module.import_section = Some(v),
                Section::Function(v) => module.fn_section = Some(v),
                // Section::TableSection(v) => module.table_section = Some(v),
                // Section::MemorySection(v) => module.memory_section = Some(v),
                // Section::GlobalSection(v) => module.global_section = Some(v),
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

pub fn decode_any_section<R: io::Read>(reader: &mut R) -> Result<Section> {
    let mut section_id_byte: [u8; 1] = [0; 1];
    reader.read_exact(&mut section_id_byte)?;
    // if section_id_byte[0] == 0x08 {
    //     return Ok(Section::Start(leb128::read::unsigned(reader)? as u32));
    // };

    // Consume length. Maybe useful later
    decode_u32(reader)?;
    let section = match section_id_byte[0] {
        0x01 => {
            let vec = util::decode_vec(reader, type_section::decode_type_section)?;
            Section::Type(vec)
        }
        // 0x02 => {
        //     let vec = util::decode_vec(reader, import_section::decode_import_section)?;
        //     Section::Import(vec)
        // }
        0x03 => {
            let vec = util::decode_vec(reader, |r| Ok(decode_u32(r)?.value))?;
            Section::Function(vec)
        }
        // 0x03 => Section::Function(section_data),
        // 0x04 => Section::Table(section_data),
        // 0x05 => Section::Memory(section_data),
        // 0x06 => Section::Global(section_data),
        // 0x07 => Section::Export(section_data),
        // 0x09 => Section::Element(section_data),
        0x0A => {
            let vec = util::decode_vec(reader, code_section::decode_code_section)?;
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
