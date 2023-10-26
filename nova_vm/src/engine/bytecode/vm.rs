use oxc_span::Atom;

use crate::ecmascript::{
    execution::Agent,
    types::{Reference, Value},
};

use super::{Executable, IndexType, Instruction};

#[derive(Debug)]
pub struct Vm<'ctx, 'host> {
    agent: &'ctx mut Agent<'ctx, 'host>,
    ip: usize,
    stack: Vec<Value>,
    reference_stack: Vec<Option<Reference>>,
    exception_jump_target_stack: Vec<usize>,
    result: Option<Value>,
    exception: Option<Value>,
    reference: Option<Reference>,
}

impl<'ctx, 'host> Vm<'ctx, 'host> {
    pub fn new(agent: &'ctx mut Agent<'ctx, 'host>) -> Self {
        Self {
            agent,
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            exception_jump_target_stack: Vec::new(),
            result: None,
            exception: None,
            reference: None,
        }
    }

    fn fetch_instruction(&mut self, executable: &Executable) -> Option<Instruction> {
        executable.instructions.get(self.ip).map(|kind| {
            self.ip += 1;
            *kind
        })
    }

    fn fetch_constant(&mut self, executable: &Executable) -> Value {
        let index = self.fetch_index(executable);
        executable.constants[index as usize]
    }

    fn fetch_identifier(&mut self, executable: &Executable) -> Atom {
        let index = self.fetch_index(executable);
        executable.identifiers[index as usize].clone()
    }

    fn fetch_index(&mut self, executable: &Executable) -> IndexType {
        let bytes = IndexType::from_ne_bytes([
            self.fetch_instruction(executable).unwrap() as u8,
            self.fetch_instruction(executable).unwrap() as u8,
        ]);
        bytes as IndexType
    }
}
