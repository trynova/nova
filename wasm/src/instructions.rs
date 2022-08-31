use crate::decoder::common::RefKind;
use crate::decoder::common::ValueKind;

pub enum Instruction<'a> {
    // Control Instructions
    Unreachable,
    Nop,
    Block(Option<&'a [ValueKind]>),
    Loop(Option<&'a [ValueKind]>),
    If(Option<&'a [ValueKind]>),
    Else,
    Br(u32),
    BrIf(u32),
    /// `BrTable(tableIndices, default)`
    BrTable(&'a [u32], u32),
    Return,
    /// `Call(function_id)`
    Call(u32),
    /// `CallIndirect(signature_id)`
    CallIndirect(u32),
    Drop,
    Select,
    End,

    // Reference Instructions
    RefNull(RefKind),
    RefIsNull,
    RefFunc(u32),
    // Variable Instructions
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),

    // Integer Arithmetic Instructions
    I32Add,
    I64Add,
    I32Sub,
    I64Sub,
    I32Mul,
    I64Mul,
    I32DivS,
    I64DivS,
    I32DivU,
    I64DivU,
    I32RemS,
    I64RemS,
    I32RemU,
    I64RemU,
    I32And,
    I64And,
    I32Or,
    I64Or,
    I32Xor,
    I64Xor,
}
