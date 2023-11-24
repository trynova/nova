use super::Instruction;
use crate::{
    ecmascript::types::{BigIntHeapData, Reference, Value},
    heap::CreateHeapData,
    Heap,
};
use oxc_ast::ast;
use oxc_span::Atom;
use oxc_syntax::{
    identifier,
    operator::{BinaryOperator, UnaryOperator},
};

pub type IndexType = u16;

pub(crate) struct CompileContext<'a, 'b, 'c> {
    heap: &'a mut Heap<'b, 'c>,
    exe: Executable,
}

#[derive(Debug)]
pub(crate) struct Executable {
    pub(crate) instructions: Vec<u8>,
    pub(crate) constants: Vec<Value>,
    pub(crate) identifiers: Vec<Atom>,
    pub(crate) references: Vec<Reference>,
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
            references: Vec::new(),
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

        // TODO: Remove this and find another way to test.
        if let Some(last) = body.last() {
            last.compile(&mut ctx);
            ctx.exe.add_instruction(Instruction::Return);
        }

        ctx.exe
    }

    fn add_instruction(&mut self, instruction: Instruction) {
        self.instructions
            .push(unsafe { std::mem::transmute(instruction) });
    }

    fn add_constant(&mut self, constant: Value) -> usize {
        let index = self.constants.len();
        self.constants.push(constant);
        index
    }

    fn add_identifier(&mut self, identifier: Atom) -> usize {
        let index = self.identifiers.len();
        self.identifiers.push(identifier);
        index
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
        self.add_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
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

fn is_reference(expression: &ast::Expression) -> bool {
    match expression {
        ast::Expression::Identifier(_)
        | ast::Expression::MemberExpression(_)
        | ast::Expression::Super(_) => true,
        ast::Expression::ParenthesizedExpression(parenthesized) => {
            is_reference(&parenthesized.expression)
        }
        _ => false,
    }
}

impl Compile for ast::NumberLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = ctx.heap.create(self.value);
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl Compile for ast::BooleanLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, self.value);
    }
}

impl Compile for ast::BigintLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = ctx.heap.create(BigIntHeapData {
            data: self.value.clone(),
        });
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl Compile for ast::IdentifierReference {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_identifier(Instruction::ResolveBinding, self.name.clone());
    }
}

impl Compile for ast::BindingIdentifier {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_identifier(Instruction::ResolveBinding, self.name.clone());
    }
}

impl Compile for ast::UnaryExpression<'_> {
    /// ## [13.5 Unary Operators](https://tc39.es/ecma262/#sec-unary-operators)
    fn compile(&self, ctx: &mut CompileContext) {
        match self.operator {
            // 13.5.5 Unary - Operator
            // https://tc39.es/ecma262/#sec-unary-minus-operator-runtime-semantics-evaluation
            // UnaryExpression : - UnaryExpression
            UnaryOperator::UnaryNegation => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);

                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                if is_reference(&self.argument) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ctx.exe.add_instruction(Instruction::ToNumeric);

                // 3. If oldValue is a Number, then
                //    a. Return Number::unaryMinus(oldValue).
                // 4. Else,
                //    a. Assert: oldValue is a BigInt.
                //    b. Return BigInt::unaryMinus(oldValue).
                ctx.exe.add_instruction(Instruction::UnaryMinus);
            }
            UnaryOperator::UnaryPlus => todo!(),
            UnaryOperator::LogicalNot => todo!(),
            UnaryOperator::BitwiseNot => todo!(),
            // 13.5.3 The typeof Operator
            // UnaryExpression : typeof UnaryExpression
            UnaryOperator::Typeof => {
                // 1. Let val be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);
                // 2. If val is a Reference Record, then
                if is_reference(&self.argument) {
                    // a. If IsUnresolvableReference(val) is true, return "undefined".
                    // if is_unresolvable_reference(&self.argument) {  }
                }
                // 3. Set val to ? GetValue(val).
                ctx.exe.add_instruction(Instruction::GetValue);
                ctx.exe.add_instruction(Instruction::Typeof);
            }
            // 13.5.2 The void operator
            // UnaryExpression : void UnaryExpression
            UnaryOperator::Void => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);
                // NOTE: GetValue must be called even though its value is not used because it may have observable side-effects.
                // 2. Perform ? GetValue(expr).
                if is_reference(&self.argument) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                // 3. Return undefined.
                ctx.exe
                    .add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            }
            // 13.5.1 The delete operator
            // https://tc39.es/ecma262/#sec-delete-operator-runtime-semantics-evaluation
            // UnaryExpression : delete UnaryExpression
            UnaryOperator::Delete => todo!(),
        }
    }
}

impl Compile for ast::BinaryExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self.operator {
            BinaryOperator::LessThan => {
                // 13.10.1 Runtime Semantics: Evaluation
                // RelationalExpression : RelationalExpression < ShiftExpression

                // 1. Let lref be ? Evaluation of RelationalExpression.
                self.left.compile(ctx);

                // 2. Let lval be ? GetValue(lref).
                if is_reference(&self.left) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }

                // 3. Let rref be ? Evaluation of ShiftExpression.
                self.right.compile(ctx);

                // 4. Let rval be ? GetValue(rref).
                if is_reference(&self.left) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }

                // 5. Let r be ? IsLessThan(lval, rval, true).
                // 6. If r is undefined, return false. Otherwise, return r.
                ctx.exe.add_instruction(Instruction::LessThan);
            }
            _ => {}
        }

        // 1. Let lref be ? Evaluation of leftOperand.
        self.left.compile(ctx);

        // 2. Let lval be ? GetValue(lref).
        if is_reference(&self.left) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.add_instruction(Instruction::Load);

        // 3. Let rref be ? Evaluation of rightOperand.
        self.right.compile(ctx);

        // 4. Let rval be ? GetValue(rref).
        if is_reference(&self.right) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }

        ctx.exe.add_instruction(Instruction::Load);

        // 5. Return ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
        ctx.exe
            .add_instruction(Instruction::ApplyStringOrNumericBinaryOperator(
                self.operator,
            ));
    }
}

impl Compile for ast::AssignmentExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let ast::AssignmentTarget::SimpleAssignmentTarget(target) = &self.left else {
            todo!("{:?}", self.left);
        };

        let ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(identifier) = &target else {
            todo!("{target:?}");
        };

        identifier.compile(ctx);
        ctx.exe.add_instruction(Instruction::PushReference);

        self.right.compile(ctx);

        if is_reference(&self.right) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }

        ctx.exe
            .add_instruction_with_identifier(Instruction::PutValue, identifier.name.clone());
        ctx.exe.add_instruction(Instruction::PopReference);
    }
}

impl Compile for ast::ParenthesizedExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.expression.compile(ctx);
    }
}

impl Compile for ast::Expression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Expression::NumberLiteral(x) => x.compile(ctx),
            ast::Expression::BooleanLiteral(x) => x.compile(ctx),
            ast::Expression::Identifier(x) => x.compile(ctx),
            ast::Expression::BigintLiteral(x) => x.compile(ctx),
            ast::Expression::UnaryExpression(x) => x.compile(ctx),
            ast::Expression::BinaryExpression(x) => x.compile(ctx),
            ast::Expression::AssignmentExpression(x) => x.compile(ctx),
            ast::Expression::ParenthesizedExpression(x) => x.compile(ctx),
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

impl Compile for ast::VariableDeclaration<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        for decl in &self.declarations {
            match self.kind {
                ast::VariableDeclarationKind::Var => {
                    let ast::BindingPatternKind::BindingIdentifier(identifier) = &decl.id.kind
                    else {
                        todo!("{:?}", decl.id.kind);
                    };

                    if let Some(init) = &decl.init {
                        ctx.exe.add_instruction(Instruction::Load);

                        ctx.exe.add_instruction_with_identifier(
                            Instruction::ResolveBinding,
                            identifier.name.clone(),
                        );

                        ctx.exe.add_instruction(Instruction::PushReference);

                        init.compile(ctx);

                        if is_reference(init) {
                            ctx.exe.add_instruction(Instruction::GetValue);
                        }

                        ctx.exe.add_instruction(Instruction::PutValue);
                        ctx.exe.add_instruction(Instruction::PopReference);

                        ctx.exe.add_instruction(Instruction::Store);
                    }
                }
                other => todo!("{other:?}"),
            }
        }
    }
}

impl Compile for ast::Declaration<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Declaration::VariableDeclaration(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}

impl Compile for ast::Statement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Statement::ExpressionStatement(x) => x.compile(ctx),
            ast::Statement::ReturnStatement(x) => x.compile(ctx),
            ast::Statement::IfStatement(x) => x.compile(ctx),
            ast::Statement::Declaration(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}
