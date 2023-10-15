use super::Instruction;
use crate::{types::Value, Heap};
use oxc_span::Atom;
use std::marker::PhantomData;

pub type IndexType = u16;

#[derive(Debug)]
pub struct Executable<'ctx> {
    pub heap: PhantomData<&'ctx mut Heap>,
    pub instructions: Vec<Instruction>,
    pub constants: Vec<Value>,
    pub identifiers: Vec<Atom>,
    // TODO: function_expressions
}

impl<'ctx> Executable<'ctx> {
    pub fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    pub fn add_constant(&mut self, constant: Value) {
        self.constants.push(constant);
    }

    pub fn add_identifier(&mut self, identifier: Atom) {
        self.identifiers.push(identifier);
    }

    pub fn add_instruction_with_constant(&mut self, instruction: Instruction, constant: Value) {
        debug_assert!(instruction.has_constant_index());
        self.add_instruction(instruction);
        self.add_constant(constant);
    }

    pub fn add_instruction_with_identifier(&mut self, instruction: Instruction, identifier: Atom) {
        debug_assert!(instruction.has_identifier_index());
        self.add_instruction(instruction);
        self.add_identifier(identifier);
    }

    pub fn add_index(&mut self, index: usize) {
        assert!(index < IndexType::MAX as usize);
        let bytes: [u8; 2] = (index as IndexType).to_ne_bytes();
        self.instructions[index] = unsafe { std::mem::transmute(bytes[0]) };
        self.instructions[index + 1] = unsafe { std::mem::transmute(bytes[0]) };
    }

    pub fn add_jump_index(&mut self) -> JumpIndex {
        self.add_index(0);
        JumpIndex {
            index: self.instructions.len() - std::mem::size_of::<IndexType>(),
        }
    }

    pub fn set_jump_target(&mut self, jump: JumpIndex, index: usize) {
        assert!(index < IndexType::MAX as usize);
        let bytes: [u8; 2] = (index as IndexType).to_ne_bytes();
        self.instructions[jump.index] = unsafe { std::mem::transmute(bytes[0]) };
        self.instructions[jump.index + 1] = unsafe { std::mem::transmute(bytes[0]) };
    }

    pub fn set_jump_target_here(&mut self, jump: JumpIndex) {
        self.set_jump_target(jump, self.instructions.len());
    }
}

#[derive(Debug)]
pub struct JumpIndex {
    pub index: usize,
}
