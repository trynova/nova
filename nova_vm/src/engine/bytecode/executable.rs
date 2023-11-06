use super::Instruction;
use crate::{
    ecmascript::{execution::ExecutionContext, types::Value},
    heap::CreateHeapData,
    Heap,
};
use oxc_ast::ast;
use oxc_span::Atom;
use std::marker::PhantomData;

pub type IndexType = u16;

pub(crate) struct CompileContext<'a, 'b, 'c> {
    heap: &'a mut Heap<'b, 'c>,
    exe: Executable,
}

#[derive(Debug)]
pub(crate) struct Executable {
    pub(crate) instructions: Vec<u8>,
    pub constants: Vec<Value>,
    pub identifiers: Vec<Atom>,
    // TODO: function_expressions
}

impl Executable {
    pub(crate) fn compile<'ctx, 'host>(
        heap: &mut Heap<'ctx, 'host>,
        body: &[ast::Statement],
    ) -> Executable {
        let exe = Executable {
            instructions: Vec::new(),
            constants: Vec::new(),
            identifiers: Vec::new(),
        };

        let mut ctx = CompileContext { heap, exe };

        let iter = if body.len() != 0 {
            body[..body.len() - 1].iter()
        } else {
            body.iter()
        };

        for stmt in iter {
            stmt.compile(&mut ctx);
        }

        if let Some(last) = body.last() {
            last.compile(&mut ctx);
            ctx.exe.add_instruction(Instruction::Return);
        }

        ctx.exe
    }

    fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction as u8);
    }

    fn add_constant(&mut self, constant: Value) -> usize {
        let index = self.constants.len();
        self.constants.push(constant);
        index
    }

    fn add_identifier(&mut self, identifier: Atom) {
        self.identifiers.push(identifier);
    }

    fn add_instruction_with_constant(
        &mut self,
        instruction: Instruction,
        constant: impl Into<Value>,
    ) {
        debug_assert!(instruction.has_constant_index());
        self.add_instruction(instruction);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    fn add_instruction_with_identifier(&mut self, instruction: Instruction, identifier: Atom) {
        debug_assert!(instruction.has_identifier_index());
        self.add_instruction(instruction);
        self.add_identifier(identifier);
    }

    fn add_index(&mut self, index: usize) {
        assert!(index < IndexType::MAX as usize);
        let bytes: [u8; 2] = (index as IndexType).to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_jump_index(&mut self) -> JumpIndex {
        self.add_index(0);
        JumpIndex {
            index: self.instructions.len() - std::mem::size_of::<IndexType>(),
        }
    }

    fn set_jump_target(&mut self, jump: JumpIndex, index: usize) {
        assert!(index < IndexType::MAX as usize);
        let bytes: [u8; 2] = (index as IndexType).to_ne_bytes();
        self.instructions[jump.index] = unsafe { std::mem::transmute(bytes[0]) };
        self.instructions[jump.index + 1] = unsafe { std::mem::transmute(bytes[0]) };
    }

    fn set_jump_target_here(&mut self, jump: JumpIndex) {
        self.set_jump_target(jump, self.instructions.len());
    }
}

#[derive(Debug)]
pub(crate) struct JumpIndex {
    pub(crate) index: usize,
}

pub(crate) trait Compile {
    fn compile(&self, ctx: &mut CompileContext);
}

impl Compile for ast::NumberLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = ctx.heap.create(self.value);
        ctx.exe
            .add_instruction_with_constant(Instruction::LoadConstant, constant);
    }
}

impl Compile for ast::BooleanLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_constant(Instruction::LoadConstant, self.value);
    }
}

impl Compile for ast::Expression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Expression::NumberLiteral(x) => x.compile(ctx),
            ast::Expression::BooleanLiteral(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}

impl Compile for ast::ExpressionStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.expression.compile(ctx);
    }
}

impl Compile for ast::ReturnStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if let Some(expr) = &self.argument {
            expr.compile(ctx);
            ctx.exe.add_instruction(Instruction::Store);
        } else {
            ctx.exe
                .add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        ctx.exe.add_instruction(Instruction::Return);
    }
}

impl Compile for ast::IfStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.test.compile(ctx);
        let jump = ctx.exe.add_jump_index();
        self.consequent.compile(ctx);
        ctx.exe.set_jump_target_here(jump);
    }
}

impl Compile for ast::Statement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Statement::ExpressionStatement(x) => x.compile(ctx),
            ast::Statement::ReturnStatement(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}
