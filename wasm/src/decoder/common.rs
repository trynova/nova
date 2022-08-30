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
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x00 => Ok(Self::Function),
            0x01 => Ok(Self::Table),
            0x02 => Ok(Self::Memory),
            0x03 => Ok(Self::Global),
            _ => Err(()),
        }
    }
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum ValueKind {
    I32 = 0x7F,
    I64 = 0x7E,
    F32 = 0x7D,
    F64 = 0x7C,
    Funcref = 0x70,
    Func = 0x60,
    Void = 0x40,
}

impl From<ValueKind> for u8 {
    fn from(v: ValueKind) -> Self {
        v as Self
    }
}

impl TryFrom<u8> for ValueKind {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0x7F => Ok(Self::I32),
            0x7E => Ok(Self::I64),
            0x7D => Ok(Self::F32),
            0x7C => Ok(Self::F64),
            0x70 => Ok(Self::Funcref),
            0x60 => Ok(Self::Func),
            0x40 => Ok(Self::Void),
            _ => Err(()),
        }
    }
}

pub struct FnType {
    pub params: Vec<ValueKind>,
    pub result: Vec<ValueKind>,
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
    pub locals: Vec<ValueKind>,
    pub instructions: Vec<u8>,
}
