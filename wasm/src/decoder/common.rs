use crate::error;

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ExternalKind {
    Function = 0x00,
    Table = 0x01,
    Memory = 0x02,
    Global = 0x03,
}

impl From<ExternalKind> for u8 {
    fn from(v: ExternalKind) -> Self {
        v as Self
    }
}

impl TryFrom<u8> for ExternalKind {
    type Error = error::Error;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::Function),
            0x01 => Ok(Self::Table),
            0x02 => Ok(Self::Memory),
            0x03 => Ok(Self::Global),
            _ => Err(error::Error::InvalidExternalKind),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum NumKind {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum RefKind {
    FuncRef = 0x70,
    ExternalRef = 0x6F,
}

#[repr(u8)]
#[derive(Copy, Clone)]
enum VecKind {
    V128 = 0x7B,
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ValueKind {
    RefKind(RefKind),
    NumKind(NumKind),
    VecKind(VecKind),
}

impl From<ValueKind> for u8 {
    fn from(v: ValueKind) -> Self {
        match v {
            ValueKind::NumKind(v) => v as Self,
            ValueKind::RefKind(v) => v as Self,
            ValueKind::VecKind(v) => v as Self,
        }
    }
}

impl TryFrom<u8> for ValueKind {
    type Error = error::Error;

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x7F => Ok(Self::NumKind(NumKind::I32)),
            0x7E => Ok(Self::NumKind(NumKind::I64)),
            0x7D => Ok(Self::NumKind(NumKind::F32)),
            0x7C => Ok(Self::NumKind(NumKind::F64)),
            0x70 => Ok(Self::RefKind(RefKind::FuncRef)),
            0x6F => Ok(Self::RefKind(RefKind::ExternalRef)),
            0x7B => Ok(Self::VecKind(VecKind::V128)),
            _ => Err(error::Error::InvalidValueKind),
        }
    }
}

pub struct FnType {
    pub params: Box<[ValueKind]>,
    pub results: Box<[ValueKind]>,
}

// pub struct Import {
//     pub module_name: String,
//     pub export_name: String,
//     pub kind: ExternalKind,
// }

// pub struct ResizableLimits {
//     pub min: u32,
//     pub max: Option<u32>,
// }

// pub struct GlobalDescriptor {
//     pub kind: ValueKind,
//     pub mutable: bool,
// }

// pub struct Export {
//     pub name: String,
//     pub kind: ExternalKind,
//     pub index: u32,
// }

pub struct CodeBlock {
    pub locals: Box<[ValueKind]>,
    pub instructions: Vec<u8>,
}
