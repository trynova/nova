use super::instructions::Instruction;

pub struct Func {
    type_idx: u32,
    code_body: Vec<Instruction>,
}
