use oxc_span::Atom;

use crate::ecmascript::{
    execution::{Agent, JsResult},
    types::{Reference, Value},
};

use super::{Executable, IndexType, Instr, Instruction, InstructionIter};

#[derive(Debug)]
pub(crate) struct Vm {
    /// Instruction pointer.
    ip: usize,
    stack: Vec<Value>,
    reference_stack: Vec<Option<Reference>>,
    exception_jump_target_stack: Vec<usize>,
    result: Value,
    exception: Option<Value>,
    reference: Option<Reference>,
}

impl Vm {
    fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            exception_jump_target_stack: Vec::new(),
            result: Value::Undefined,
            exception: None,
            reference: None,
        }
    }

    fn fetch_constant(&self, exe: &Executable, index: usize) -> Value {
        exe.constants[index]
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute(agent: &mut Agent, executable: &Executable) -> JsResult<Value> {
        let mut vm = Vm::new();

        let mut iter = InstructionIter::new(&executable.instructions);

        while let Some(instr) = iter.next() {
            match instr.kind {
                Instruction::LoadConstant => {
                    let constant = vm.fetch_constant(executable, instr.args[0].unwrap() as usize);
                    vm.stack.push(constant);
                }
                Instruction::Load => {
                    vm.stack.push(vm.result);
                }
                Instruction::Return => {
                    return Ok(vm.stack.pop().into());
                }
                Instruction::Store => {
                    vm.result = *vm.stack.last().unwrap();
                }
                Instruction::StoreConstant => {
                    let constant = vm.fetch_constant(executable, instr.args[0].unwrap() as usize);
                    vm.result = constant;
                }
                other => todo!("{other:?}"),
            }
        }

        Ok(Value::Undefined)
    }
}
