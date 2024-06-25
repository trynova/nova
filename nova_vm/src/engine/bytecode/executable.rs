use super::{instructions::Instr, Instruction};
use crate::{
    ecmascript::{
        builtins::regexp::reg_exp_create,
        execution::Agent,
        scripts_and_modules::script::ScriptIdentifier,
        syntax_directed_operations::scope_analysis::{
            LexicallyScopedDeclaration, LexicallyScopedDeclarations,
        },
        types::{BigIntHeapData, Reference, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::CreateHeapData,
};
use num_bigint::BigInt;
use num_traits::Num;
use oxc_ast::{
    ast::{self, CallExpression, FunctionBody, NewExpression, Statement},
    syntax_directed_operations::BoundNames,
};
use oxc_span::Atom;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

pub type IndexType = u16;

pub(crate) struct CompileContext<'agent> {
    agent: &'agent mut Agent,
    exe: Executable,
    /// NamedEvaluation name parameter
    name_identifier: Option<usize>,
    /// If true, indicates that all bindings being created are lexical.
    ///
    /// Otherwise, all bindings being created are variable scoped.
    lexical_binding_state: bool,
    /// `continue;` statement jumps that were present in the current loop.
    current_continue: Option<Vec<JumpIndex>>,
    /// `break;` statement jumps that were present in the current loop.
    current_break: Option<Vec<JumpIndex>>,
}

impl CompileContext<'_> {
    pub(crate) fn create_identifier(&mut self, atom: &Atom<'_>) -> String {
        let existing =
            self.exe.identifiers.iter().find(|existing_identifier| {
                existing_identifier.as_str(self.agent) == atom.as_str()
            });
        if let Some(&existing) = existing {
            existing
        } else {
            String::from_str(self.agent, atom.as_str())
        }
    }
}

#[derive(Debug)]
pub(crate) struct FunctionExpression {
    pub(crate) expression: &'static ast::Function<'static>,
    pub(crate) identifier: Option<usize>,
    pub(crate) home_object: Option<usize>,
}

#[derive(Debug)]
pub(crate) struct ArrowFunctionExpression {
    pub(crate) expression: &'static ast::ArrowFunctionExpression<'static>,
    pub(crate) identifier: Option<usize>,
    pub(crate) home_object: Option<usize>,
}

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug)]
pub(crate) struct Executable {
    pub instructions: Vec<u8>,
    pub(crate) constants: Vec<Value>,
    pub(crate) identifiers: Vec<String>,
    pub(crate) references: Vec<Reference>,
    pub(crate) function_expressions: Vec<FunctionExpression>,
    pub(crate) arrow_function_expressions: Vec<ArrowFunctionExpression>,
}

impl Executable {
    pub(super) fn get_instruction(&self, ip: &mut usize) -> Option<Instr> {
        if *ip >= self.instructions.len() {
            return None;
        }

        let kind: Instruction =
            unsafe { std::mem::transmute::<u8, Instruction>(self.instructions[*ip]) };
        *ip += 1;

        let mut args: [Option<IndexType>; 2] = [None, None];

        for item in args.iter_mut().take(kind.argument_count() as usize) {
            let length = self.instructions[*ip..].len();
            if length >= 2 {
                let bytes = IndexType::from_ne_bytes(unsafe {
                    *std::mem::transmute::<*const u8, *const [u8; 2]>(
                        self.instructions[*ip..].as_ptr(),
                    )
                });
                *ip += 2;
                *item = Some(bytes);
            } else {
                *ip += 1;
                *item = None;
            }
        }

        Some(Instr { kind, args })
    }

    pub(super) fn peek_last_instruction(&self) -> Option<u8> {
        for ele in self.instructions.iter().rev() {
            if *ele == Instruction::ExitDeclarativeEnvironment.as_u8() {
                // Not a "real" instruction
                continue;
            }
            return Some(*ele);
        }
        None
    }

    pub(crate) fn compile_script(agent: &mut Agent, script: ScriptIdentifier) -> Executable {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Script ===");
            eprintln!();
        }
        // SAFETY: Script uniquely owns the Program and the body buffer does
        // not move under any circumstances during heap operations.
        let body: &[Statement] =
            unsafe { std::mem::transmute(agent[script].ecmascript_code.body.as_slice()) };

        Self::_compile_statements(agent, body)
    }

    pub(crate) fn compile_function_body(agent: &mut Agent, body: &FunctionBody<'_>) -> Executable {
        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Function ===");
            eprintln!();
        }
        // SAFETY: Script referred by the Function uniquely owns the Program
        // and the body buffer does not move under any circumstances during
        // heap operations.
        let body: &[Statement] = unsafe { std::mem::transmute(body.statements.as_slice()) };

        Self::_compile_statements(agent, body)
    }

    fn _compile_statements(agent: &mut Agent, body: &[Statement]) -> Executable {
        let mut ctx = CompileContext {
            agent,
            exe: Executable {
                instructions: Vec::new(),
                constants: Vec::new(),
                identifiers: Vec::new(),
                references: Vec::new(),
                function_expressions: Vec::new(),
                arrow_function_expressions: Vec::new(),
            },
            name_identifier: None,
            lexical_binding_state: false,
            current_continue: None,
            current_break: None,
        };

        let iter = body.iter();

        for stmt in iter {
            stmt.compile(&mut ctx);
        }

        if ctx.exe.instructions.last() != Some(&Instruction::Return.as_u8()) {
            // If code did not end with a return statement, add it manually
            ctx.exe.add_instruction(Instruction::Return);
            return ctx.exe;
        }

        ctx.exe
    }

    fn _push_instruction(&mut self, instruction: Instruction) {
        self.instructions
            .push(unsafe { std::mem::transmute::<Instruction, u8>(instruction) });
    }

    fn add_instruction(&mut self, instruction: Instruction) {
        debug_assert_eq!(instruction.argument_count(), 0);
        debug_assert!(
            !instruction.has_constant_index()
                && !instruction.has_function_expression_index()
                && !instruction.has_identifier_index()
        );
        self._push_instruction(instruction);
    }

    fn add_instruction_with_jump_slot(&mut self, instruction: Instruction) -> JumpIndex {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_jump_slot());
        self._push_instruction(instruction);
        self.add_jump_index()
    }

    fn add_jump_instruction_to_index(&mut self, instruction: Instruction, jump_index: JumpIndex) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_jump_slot());
        self._push_instruction(instruction);
        self.add_index(jump_index.index);
    }

    fn get_jump_index_to_here(&self) -> JumpIndex {
        JumpIndex {
            index: self.instructions.len(),
        }
    }

    fn add_constant(&mut self, constant: Value) -> usize {
        let duplicate = self
            .constants
            .iter()
            .enumerate()
            .find(|item| item.1.eq(&constant))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.constants.len();
            self.constants.push(constant);
            index
        })
    }

    fn add_identifier(&mut self, identifier: String) -> usize {
        let duplicate = self
            .identifiers
            .iter()
            .enumerate()
            .find(|item| *item.1 == identifier)
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.identifiers.len();
            self.identifiers.push(identifier);
            index
        })
    }

    fn add_instruction_with_immediate(&mut self, instruction: Instruction, immediate: usize) {
        debug_assert_eq!(instruction.argument_count(), 1);
        self._push_instruction(instruction);
        self.add_index(immediate);
    }

    fn add_instruction_with_constant(
        &mut self,
        instruction: Instruction,
        constant: impl Into<Value>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_constant_index());
        self._push_instruction(instruction);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    fn add_instruction_with_identifier(&mut self, instruction: Instruction, identifier: String) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_identifier_index());
        self._push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
    }

    fn add_instruction_with_identifier_and_constant(
        &mut self,
        instruction: Instruction,
        identifier: String,
        constant: impl Into<Value>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_identifier_index() && instruction.has_constant_index());
        self._push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    fn add_instruction_with_immediate_and_immediate(
        &mut self,
        instruction: Instruction,
        immediate1: usize,
        immediate2: usize,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        self._push_instruction(instruction);
        self.add_index(immediate1);
        self.add_index(immediate2)
    }

    fn add_index(&mut self, index: usize) {
        assert!(index < IndexType::MAX as usize);
        let bytes: [u8; 2] = (index as IndexType).to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_function_expression(&mut self, function_expression: FunctionExpression) {
        let instruction = Instruction::InstantiateOrdinaryFunctionExpression;
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.function_expressions.push(function_expression);
        let index = self.function_expressions.len() - 1;
        self.add_index(index);
    }

    fn add_arrow_function_expression(
        &mut self,
        arrow_function_expression: ArrowFunctionExpression,
    ) {
        let instruction = Instruction::InstantiateArrowFunctionExpression;
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.arrow_function_expressions
            .push(arrow_function_expression);
        let index = self.arrow_function_expressions.len() - 1;
        self.add_index(index);
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
        self.instructions[jump.index] = bytes[0];
        self.instructions[jump.index + 1] = bytes[1];
    }

    fn set_jump_target_here(&mut self, jump: JumpIndex) {
        self.set_jump_target(jump, self.instructions.len());
    }
}

#[derive(Debug, Clone)]
#[repr(transparent)]
pub(crate) struct JumpIndex {
    pub(crate) index: usize,
}

pub(crate) trait CompileEvaluation {
    fn compile(&self, ctx: &mut CompileContext);
}

fn is_reference(expression: &ast::Expression) -> bool {
    match expression {
        ast::Expression::Identifier(_)
        | ast::Expression::ComputedMemberExpression(_)
        | ast::Expression::StaticMemberExpression(_)
        | ast::Expression::PrivateFieldExpression(_)
        | ast::Expression::Super(_) => true,
        ast::Expression::ParenthesizedExpression(parenthesized) => {
            is_reference(&parenthesized.expression)
        }
        _ => false,
    }
}

impl CompileEvaluation for ast::NumericLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = ctx.agent.heap.create(self.value);
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl CompileEvaluation for ast::BooleanLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, self.value);
    }
}

impl CompileEvaluation for ast::BigIntLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let radix = match self.base {
            oxc_syntax::number::BigintBase::Decimal => 10,
            oxc_syntax::number::BigintBase::Binary => 2,
            oxc_syntax::number::BigintBase::Octal => 8,
            oxc_syntax::number::BigintBase::Hex => 16,
        };
        // Drop out the trailing 'n' from BigInt literals.
        let last_index = self.raw.len() - 1;
        let big_int_str = &self.raw.as_str()[..last_index];
        let constant = ctx.agent.heap.create(BigIntHeapData {
            // Drop out the trailing 'n' from BigInt literals.
            data: BigInt::from_str_radix(big_int_str, radix).unwrap(),
        });
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl CompileEvaluation for ast::NullLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, Value::Null);
    }
}

impl CompileEvaluation for ast::StringLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = String::from_str(ctx.agent, self.value.as_str());
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl CompileEvaluation for ast::IdentifierReference<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let identifier = String::from_str(ctx.agent, self.name.as_str());
        ctx.exe
            .add_instruction_with_identifier(Instruction::ResolveBinding, identifier);
    }
}

impl CompileEvaluation for ast::BindingIdentifier<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let identifier = String::from_str(ctx.agent, self.name.as_str());
        ctx.exe
            .add_instruction_with_identifier(Instruction::ResolveBinding, identifier);
    }
}

impl CompileEvaluation for ast::UnaryExpression<'_> {
    /// ### [13.5 Unary Operators](https://tc39.es/ecma262/#sec-unary-operators)
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
            // 13.5.4 Unary + Operator
            // https://tc39.es/ecma262/#sec-unary-plus-operator
            // UnaryExpression : + UnaryExpression
            UnaryOperator::UnaryPlus => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);

                // 2. Return ? ToNumber(? GetValue(expr)).
                if is_reference(&self.argument) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ctx.exe.add_instruction(Instruction::ToNumber);
            }
            // 13.5.6 Unary ! Operator
            // https://tc39.es/ecma262/#sec-logical-not-operator-runtime-semantics-evaluation
            // UnaryExpression : ! UnaryExpression
            UnaryOperator::LogicalNot => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);

                // 2. Let oldValue be ToBoolean(? GetValue(expr)).
                // 3. If oldValue is true, return false.
                // 4. Return true.
                if is_reference(&self.argument) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ctx.exe.add_instruction(Instruction::LogicalNot);
            }
            // 13.5.7 Unary ~ Operator
            // https://tc39.es/ecma262/#sec-bitwise-not-operator-runtime-semantics-evaluation
            // UnaryExpression : ~ UnaryExpression
            UnaryOperator::BitwiseNot => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);

                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                // 3. If oldValue is a Number, then
                //    a. Return Number::bitwiseNOT(oldValue).
                // 4. Else,
                //    a. Assert: oldValue is a BigInt.
                //    b. Return BigInt::bitwiseNOT(oldValue).
                if is_reference(&self.argument) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ctx.exe.add_instruction(Instruction::BitwiseNot);
            }
            // 13.5.3 The typeof Operator
            // UnaryExpression : typeof UnaryExpression
            UnaryOperator::Typeof => {
                // 1. Let val be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);
                // 3. Set val to ? GetValue(val).
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

impl CompileEvaluation for ast::BinaryExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
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

        match self.operator {
            BinaryOperator::LessThan => {
                ctx.exe.add_instruction(Instruction::LessThan);
            }
            BinaryOperator::LessEqualThan => {
                ctx.exe.add_instruction(Instruction::LessThanEquals);
            }
            BinaryOperator::GreaterThan => {
                ctx.exe.add_instruction(Instruction::GreaterThan);
            }
            BinaryOperator::GreaterEqualThan => {
                ctx.exe.add_instruction(Instruction::GreaterThanEquals);
            }
            BinaryOperator::StrictEquality => {
                ctx.exe.add_instruction(Instruction::IsStrictlyEqual);
            }
            BinaryOperator::StrictInequality => {
                ctx.exe.add_instruction(Instruction::IsStrictlyEqual);
                ctx.exe.add_instruction(Instruction::LogicalNot);
            }
            BinaryOperator::Equality => {
                ctx.exe.add_instruction(Instruction::IsLooselyEqual);
            }
            BinaryOperator::Inequality => {
                ctx.exe.add_instruction(Instruction::IsLooselyEqual);
                ctx.exe.add_instruction(Instruction::LogicalNot);
            }
            BinaryOperator::In => {
                ctx.exe.add_instruction(Instruction::HasProperty);
            }
            BinaryOperator::Instanceof => {
                ctx.exe.add_instruction(Instruction::InstanceofOperator);
            }
            _ => {
                // 5. Return ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
                ctx.exe
                    .add_instruction(Instruction::ApplyStringOrNumericBinaryOperator(
                        self.operator,
                    ));
            }
        }
    }
}

impl CompileEvaluation for ast::LogicalExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.left.compile(ctx);
        if is_reference(&self.left) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        // We store the left value on the stack, because we'll need to restore
        // it later.
        ctx.exe.add_instruction(Instruction::LoadCopy);

        match self.operator {
            oxc_syntax::operator::LogicalOperator::Or => {
                ctx.exe.add_instruction(Instruction::LogicalNot);
            }
            oxc_syntax::operator::LogicalOperator::And => {}
            oxc_syntax::operator::LogicalOperator::Coalesce => {
                ctx.exe.add_instruction(Instruction::IsNullOrUndefined);
            }
        }
        let jump_to_return_left = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::JumpIfNot);

        // We're returning the right expression, so we discard the left value
        // at the top of the stack.
        ctx.exe.add_instruction(Instruction::Store);

        self.right.compile(ctx);
        if is_reference(&self.right) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        let jump_to_end = ctx.exe.add_instruction_with_jump_slot(Instruction::Jump);

        ctx.exe.set_jump_target_here(jump_to_return_left);
        // Return the result of the left expression.
        ctx.exe.add_instruction(Instruction::Store);
        ctx.exe.set_jump_target_here(jump_to_end);
    }
}

impl CompileEvaluation for ast::AssignmentExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        // 1. Let lref be ? Evaluation of LeftHandSideExpression.
        let identifier = match &self.left {
            ast::AssignmentTarget::ArrayAssignmentTarget(_) => todo!(),
            ast::AssignmentTarget::AssignmentTargetIdentifier(identifier) => {
                // impl CompileEvaluation for ast::IdentifierReference<'_>
                // is inlined here to reuse the identifier.
                let identifier = String::from_str(ctx.agent, identifier.name.as_str());
                let identifier = ctx.exe.add_identifier(identifier);
                ctx.exe
                    .add_instruction_with_immediate(Instruction::ResolveBinding, identifier);
                Some(identifier)
            }
            ast::AssignmentTarget::ComputedMemberExpression(expression) => {
                expression.compile(ctx);
                None
            }
            ast::AssignmentTarget::ObjectAssignmentTarget(_) => todo!(),
            ast::AssignmentTarget::PrivateFieldExpression(_) => todo!(),
            ast::AssignmentTarget::StaticMemberExpression(expression) => {
                expression.compile(ctx);
                None
            }
            ast::AssignmentTarget::TSAsExpression(_)
            | ast::AssignmentTarget::TSSatisfiesExpression(_)
            | ast::AssignmentTarget::TSNonNullExpression(_)
            | ast::AssignmentTarget::TSTypeAssertion(_)
            | ast::AssignmentTarget::TSInstantiationExpression(_) => unreachable!(),
        };

        if self.operator == oxc_syntax::operator::AssignmentOperator::Assign {
            ctx.exe.add_instruction(Instruction::PushReference);
            self.right.compile(ctx);

            if is_reference(&self.right) {
                ctx.exe.add_instruction(Instruction::GetValue);
            }

            ctx.exe.add_instruction(Instruction::PopReference);
            ctx.exe.add_instruction(Instruction::PutValue);
        } else if matches!(
            self.operator,
            oxc_syntax::operator::AssignmentOperator::LogicalAnd
                | oxc_syntax::operator::AssignmentOperator::LogicalNullish
                | oxc_syntax::operator::AssignmentOperator::LogicalOr
        ) {
            // 2. Let lval be ? GetValue(lref).
            ctx.exe.add_instruction(Instruction::GetValueKeepReference);
            ctx.exe.add_instruction(Instruction::PushReference);
            // We store the left value on the stack, because we'll need to
            // restore it later.
            ctx.exe.add_instruction(Instruction::LoadCopy);

            match self.operator {
                oxc_syntax::operator::AssignmentOperator::LogicalAnd => {
                    // 3. Let lbool be ToBoolean(lval).
                    // Note: We do not directly call ToBoolean: JumpIfNot does.
                    // 4. If lbool is false, return lval.
                }
                oxc_syntax::operator::AssignmentOperator::LogicalOr => {
                    // 3. Let lbool be ToBoolean(lval).
                    // Note: We do not directly call ToBoolean: JumpIfNot does.
                    // 4. If lbool is true, return lval.
                    ctx.exe.add_instruction(Instruction::LogicalNot);
                }
                oxc_syntax::operator::AssignmentOperator::LogicalNullish => {
                    // 3. If lval is neither undefined nor null, return lval.
                    ctx.exe.add_instruction(Instruction::IsNullOrUndefined);
                }
                _ => unreachable!(),
            }

            let jump_to_end = ctx
                .exe
                .add_instruction_with_jump_slot(Instruction::JumpIfNot);

            // We're returning the right expression, so we discard the left
            // value at the top of the stack.
            ctx.exe.add_instruction(Instruction::Store);

            // 5. If IsAnonymousFunctionDefinition(AssignmentExpression)
            // is true and IsIdentifierRef of LeftHandSideExpression is true,
            // then
            if let Some(identifier) = identifier {
                // a. Let lhs be the StringValue of LeftHandSideExpression.
                match &self.right {
                    ast::Expression::ArrowFunctionExpression(expr) => {
                        // Always anonymous
                        // b. Let rval be ? NamedEvaluation of AssignmentExpression with argument lhs.
                        ctx.name_identifier = Some(identifier);
                        expr.compile(ctx);
                    }
                    ast::Expression::FunctionExpression(expr) => {
                        if expr.id.is_none() {
                            ctx.name_identifier = Some(identifier);
                        }
                        // b. Let rval be ? NamedEvaluation of AssignmentExpression with argument lhs.
                        expr.compile(ctx);
                    }
                    _ => {
                        // 6. Else
                        // a. Let rref be ? Evaluation of AssignmentExpression.
                        self.right.compile(ctx);
                        // b. Let rval be ? GetValue(rref).
                        if is_reference(&self.right) {
                            ctx.exe.add_instruction(Instruction::GetValue);
                        }
                    }
                };
            } else {
                // 6. Else
                // a. Let rref be ? Evaluation of AssignmentExpression.
                self.right.compile(ctx);
                if is_reference(&self.right) {
                    // b. Let rval be ? GetValue(rref).
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
            }

            // 7. Perform ? PutValue(lref, rval).
            ctx.exe.add_instruction(Instruction::LoadCopy);
            ctx.exe.add_instruction(Instruction::PopReference);
            ctx.exe.add_instruction(Instruction::PutValue);

            // 4. ... return lval.
            ctx.exe.set_jump_target_here(jump_to_end);
            ctx.exe.add_instruction(Instruction::Store);
        } else {
            // 2. let lval be ? GetValue(lref).
            ctx.exe.add_instruction(Instruction::GetValueKeepReference);
            ctx.exe.add_instruction(Instruction::Load);
            ctx.exe.add_instruction(Instruction::PushReference);
            // 3. Let rref be ? Evaluation of AssignmentExpression.
            self.right.compile(ctx);

            // 4. Let rval be ? GetValue(rref).
            if is_reference(&self.right) {
                ctx.exe.add_instruction(Instruction::GetValue);
            }

            // 5. Let assignmentOpText be the source text matched by AssignmentOperator.
            // 6. Let opText be the sequence of Unicode code points associated with assignmentOpText in the following table:
            let op_text = match self.operator {
                oxc_syntax::operator::AssignmentOperator::Addition => BinaryOperator::Addition,
                oxc_syntax::operator::AssignmentOperator::Subtraction => {
                    BinaryOperator::Subtraction
                }
                oxc_syntax::operator::AssignmentOperator::Multiplication => {
                    BinaryOperator::Multiplication
                }
                oxc_syntax::operator::AssignmentOperator::Division => BinaryOperator::Division,
                oxc_syntax::operator::AssignmentOperator::Remainder => BinaryOperator::Remainder,
                oxc_syntax::operator::AssignmentOperator::ShiftLeft => BinaryOperator::ShiftLeft,
                oxc_syntax::operator::AssignmentOperator::ShiftRight => BinaryOperator::ShiftRight,
                oxc_syntax::operator::AssignmentOperator::ShiftRightZeroFill => {
                    BinaryOperator::ShiftRightZeroFill
                }
                oxc_syntax::operator::AssignmentOperator::BitwiseOR => BinaryOperator::BitwiseOR,
                oxc_syntax::operator::AssignmentOperator::BitwiseXOR => BinaryOperator::BitwiseXOR,
                oxc_syntax::operator::AssignmentOperator::BitwiseAnd => BinaryOperator::BitwiseAnd,
                oxc_syntax::operator::AssignmentOperator::Exponential => {
                    BinaryOperator::Exponential
                }
                _ => unreachable!(),
            };
            // 7. Let r be ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
            ctx.exe
                .add_instruction(Instruction::ApplyStringOrNumericBinaryOperator(op_text));
            ctx.exe.add_instruction(Instruction::LoadCopy);
            // 8. Perform ? PutValue(lref, r).
            ctx.exe.add_instruction(Instruction::PopReference);
            ctx.exe.add_instruction(Instruction::PutValue);
            // 9. Return r.
            ctx.exe.add_instruction(Instruction::Store);
        }
    }
}

impl CompileEvaluation for ast::ParenthesizedExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.expression.compile(ctx);
    }
}

impl CompileEvaluation for ast::ArrowFunctionExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_arrow_function_expression(ArrowFunctionExpression {
                expression: unsafe {
                    std::mem::transmute::<
                        &ast::ArrowFunctionExpression<'_>,
                        &'static ast::ArrowFunctionExpression<'static>,
                    >(self)
                },
                // CompileContext holds a name identifier for us if this is NamedEvaluation.
                identifier: ctx.name_identifier.take(),
                home_object: None,
            });
    }
}

impl CompileEvaluation for ast::Function<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe.add_function_expression(FunctionExpression {
            expression: unsafe {
                std::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(self)
            },
            // CompileContext holds a name identifier for us if this is NamedEvaluation.
            identifier: ctx.name_identifier.take(),
            home_object: None,
        });
    }
}

impl CompileEvaluation for ast::ObjectExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        // TODO: Consider preparing the properties onto the stack and creating
        // the object with a known size.
        ctx.exe.add_instruction(Instruction::ObjectCreate);
        for property in self.properties.iter() {
            match property {
                ast::ObjectPropertyKind::ObjectProperty(prop) => {
                    match &prop.key {
                        ast::PropertyKey::ArrayExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ArrowFunctionExpression(init) => init.compile(ctx),
                        ast::PropertyKey::AssignmentExpression(init) => init.compile(ctx),
                        ast::PropertyKey::AwaitExpression(init) => init.compile(ctx),
                        ast::PropertyKey::BigintLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::BinaryExpression(init) => init.compile(ctx),
                        ast::PropertyKey::BooleanLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::CallExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ChainExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ClassExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ComputedMemberExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ConditionalExpression(init) => init.compile(ctx),
                        ast::PropertyKey::FunctionExpression(init) => init.compile(ctx),
                        ast::PropertyKey::Identifier(init) => init.compile(ctx),
                        ast::PropertyKey::ImportExpression(init) => init.compile(ctx),
                        ast::PropertyKey::LogicalExpression(init) => init.compile(ctx),
                        ast::PropertyKey::MetaProperty(init) => init.compile(ctx),
                        ast::PropertyKey::NewExpression(init) => init.compile(ctx),
                        ast::PropertyKey::NullLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::NumericLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::ObjectExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ParenthesizedExpression(init) => init.compile(ctx),
                        ast::PropertyKey::PrivateFieldExpression(init) => init.compile(ctx),
                        ast::PropertyKey::PrivateIdentifier(_init) => todo!(),
                        ast::PropertyKey::PrivateInExpression(init) => init.compile(ctx),
                        ast::PropertyKey::RegExpLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::SequenceExpression(init) => init.compile(ctx),
                        ast::PropertyKey::StaticIdentifier(id) => {
                            if id.name == "__proto__" {
                                // TODO: If property key is "__proto__" then we
                                // should dispatch a SetPrototype instruction.
                                todo!();
                            } else {
                                let property_key = crate::ecmascript::types::PropertyKey::from_str(
                                    ctx.agent, &id.name,
                                );
                                ctx.exe.add_instruction_with_constant(
                                    Instruction::LoadConstant,
                                    property_key,
                                );
                            }
                        }
                        ast::PropertyKey::StaticMemberExpression(init) => init.compile(ctx),
                        ast::PropertyKey::StringLiteral(init) => {
                            if !prop.computed && init.value == "__proto__" {
                                // TODO: If property key is "__proto__" then we
                                // should dispatch a SetPrototype instruction.
                                todo!();
                            } else {
                                let property_key = crate::ecmascript::types::PropertyKey::from_str(
                                    ctx.agent,
                                    &init.value,
                                );
                                ctx.exe.add_instruction_with_constant(
                                    Instruction::LoadConstant,
                                    property_key,
                                );
                            }
                        }
                        ast::PropertyKey::Super(init) => {
                            init.compile(ctx);
                            ctx.exe.add_instruction(Instruction::GetValue);
                        }
                        ast::PropertyKey::TaggedTemplateExpression(init) => init.compile(ctx),
                        ast::PropertyKey::TemplateLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::ThisExpression(init) => init.compile(ctx),
                        ast::PropertyKey::UnaryExpression(init) => init.compile(ctx),
                        ast::PropertyKey::UpdateExpression(init) => init.compile(ctx),
                        ast::PropertyKey::YieldExpression(init) => init.compile(ctx),
                        ast::PropertyKey::JSXElement(_)
                        | ast::PropertyKey::JSXFragment(_)
                        | ast::PropertyKey::TSAsExpression(_)
                        | ast::PropertyKey::TSSatisfiesExpression(_)
                        | ast::PropertyKey::TSTypeAssertion(_)
                        | ast::PropertyKey::TSNonNullExpression(_)
                        | ast::PropertyKey::TSInstantiationExpression(_) => unreachable!(),
                    }
                    prop.value.compile(ctx);
                    if is_reference(&prop.value) {
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }

                    ctx.exe.add_instruction(Instruction::ObjectSetProperty);
                }
                ast::ObjectPropertyKind::SpreadProperty(_) => {
                    todo!("...spread not yet implemented")
                }
            }
        }
        // 3. Return obj
        ctx.exe.add_instruction(Instruction::Store);
    }
}

impl CompileEvaluation for ast::ArrayExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let elements_min_count = self.elements.len();
        ctx.exe
            .add_instruction_with_immediate(Instruction::ArrayCreate, elements_min_count);
        for ele in &self.elements {
            match ele {
                ast::ArrayExpressionElement::ArrayExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::ArrowFunctionExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::AssignmentExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::AwaitExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::BigintLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::BinaryExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::BooleanLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::CallExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::ChainExpression(init) => {
                    init.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ast::ArrayExpressionElement::ClassExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::ComputedMemberExpression(init) => {
                    init.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ast::ArrayExpressionElement::ConditionalExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::Elision(_) => todo!(),
                ast::ArrayExpressionElement::FunctionExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::Identifier(init) => {
                    init.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ast::ArrayExpressionElement::ImportExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::LogicalExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::MetaProperty(init) => init.compile(ctx),
                ast::ArrayExpressionElement::NewExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::NullLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::NumericLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::ObjectExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::ParenthesizedExpression(init) => {
                    init.compile(ctx);
                    if is_reference(&init.expression) {
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                }
                ast::ArrayExpressionElement::PrivateFieldExpression(init) => {
                    init.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ast::ArrayExpressionElement::PrivateInExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::RegExpLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::SequenceExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::SpreadElement(_) => todo!(),
                ast::ArrayExpressionElement::StaticMemberExpression(init) => {
                    init.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ast::ArrayExpressionElement::StringLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::Super(init) => {
                    init.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                ast::ArrayExpressionElement::TaggedTemplateExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::TemplateLiteral(init) => init.compile(ctx),
                ast::ArrayExpressionElement::ThisExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::UnaryExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::UpdateExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::YieldExpression(init) => init.compile(ctx),
                ast::ArrayExpressionElement::JSXElement(_)
                | ast::ArrayExpressionElement::JSXFragment(_)
                | ast::ArrayExpressionElement::TSAsExpression(_)
                | ast::ArrayExpressionElement::TSSatisfiesExpression(_)
                | ast::ArrayExpressionElement::TSTypeAssertion(_)
                | ast::ArrayExpressionElement::TSNonNullExpression(_)
                | ast::ArrayExpressionElement::TSInstantiationExpression(_) => unreachable!(),
            }
            ctx.exe.add_instruction(Instruction::ArrayPush);
        }
        ctx.exe.add_instruction(Instruction::Store);
    }
}

impl CompileEvaluation for ast::Argument<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Argument::SpreadElement(_) => {
                panic!("Cannot support SpreadElements currently")
            }
            _ => {
                match self {
                    ast::Argument::BooleanLiteral(x) => x.compile(ctx),
                    ast::Argument::NullLiteral(x) => x.compile(ctx),
                    ast::Argument::NumericLiteral(x) => x.compile(ctx),
                    ast::Argument::BigintLiteral(x) => x.compile(ctx),
                    ast::Argument::RegExpLiteral(x) => x.compile(ctx),
                    ast::Argument::StringLiteral(x) => x.compile(ctx),
                    ast::Argument::TemplateLiteral(x) => x.compile(ctx),
                    ast::Argument::MetaProperty(x) => x.compile(ctx),
                    ast::Argument::ArrayExpression(x) => x.compile(ctx),
                    ast::Argument::ArrowFunctionExpression(x) => x.compile(ctx),
                    ast::Argument::AssignmentExpression(x) => x.compile(ctx),
                    ast::Argument::AwaitExpression(x) => x.compile(ctx),
                    ast::Argument::BinaryExpression(x) => x.compile(ctx),
                    ast::Argument::CallExpression(x) => x.compile(ctx),
                    ast::Argument::ChainExpression(x) => x.compile(ctx),
                    ast::Argument::ClassExpression(x) => x.compile(ctx),
                    ast::Argument::ConditionalExpression(x) => x.compile(ctx),
                    ast::Argument::FunctionExpression(x) => x.compile(ctx),
                    ast::Argument::ImportExpression(x) => x.compile(ctx),
                    ast::Argument::LogicalExpression(x) => x.compile(ctx),
                    ast::Argument::NewExpression(x) => x.compile(ctx),
                    ast::Argument::ObjectExpression(x) => x.compile(ctx),
                    ast::Argument::SequenceExpression(x) => x.compile(ctx),
                    ast::Argument::TaggedTemplateExpression(x) => x.compile(ctx),
                    ast::Argument::ThisExpression(x) => x.compile(ctx),
                    ast::Argument::UnaryExpression(x) => x.compile(ctx),
                    ast::Argument::UpdateExpression(x) => x.compile(ctx),
                    ast::Argument::YieldExpression(x) => x.compile(ctx),
                    ast::Argument::PrivateInExpression(x) => x.compile(ctx),
                    ast::Argument::Identifier(x) => {
                        x.compile(ctx);
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                    ast::Argument::Super(x) => {
                        x.compile(ctx);
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                    ast::Argument::ParenthesizedExpression(x) => {
                        x.compile(ctx);
                        if is_reference(&x.expression) {
                            ctx.exe.add_instruction(Instruction::GetValue);
                        }
                    }
                    ast::Argument::ComputedMemberExpression(x) => {
                        x.compile(ctx);
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                    ast::Argument::StaticMemberExpression(x) => {
                        x.compile(ctx);
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                    ast::Argument::PrivateFieldExpression(x) => {
                        x.compile(ctx);
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                    ast::Argument::SpreadElement(_)
                    | ast::Argument::JSXElement(_)
                    | ast::Argument::JSXFragment(_)
                    | ast::Argument::TSAsExpression(_)
                    | ast::Argument::TSSatisfiesExpression(_)
                    | ast::Argument::TSTypeAssertion(_)
                    | ast::Argument::TSNonNullExpression(_)
                    | ast::Argument::TSInstantiationExpression(_) => unreachable!(),
                }
                ctx.exe.add_instruction(Instruction::Load);
            }
        }
    }
}

impl CompileEvaluation for CallExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.callee.compile(ctx);
        let need_pop_reference = if is_reference(&self.callee) {
            ctx.exe.add_instruction(Instruction::GetValueKeepReference);
            if !self.arguments.is_empty() {
                ctx.exe.add_instruction(Instruction::PushReference);
                true
            } else {
                false
            }
        } else {
            false
        };
        ctx.exe.add_instruction(Instruction::Load);
        for ele in &self.arguments {
            ele.compile(ctx);
        }

        if need_pop_reference {
            ctx.exe.add_instruction(Instruction::PopReference);
        }
        ctx.exe
            .add_instruction_with_immediate(Instruction::EvaluateCall, self.arguments.len());
    }
}

impl CompileEvaluation for NewExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.callee.compile(ctx);
        if is_reference(&self.callee) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.add_instruction(Instruction::Load);
        for ele in &self.arguments {
            ele.compile(ctx);
        }

        ctx.exe
            .add_instruction_with_immediate(Instruction::EvaluateNew, self.arguments.len());
    }
}

impl CompileEvaluation for ast::MemberExpression<'_> {
    /// ### [13.3.2 Property Accessors](https://tc39.es/ecma262/#sec-property-accessors)
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::MemberExpression::ComputedMemberExpression(x) => x.compile(ctx),
            ast::MemberExpression::StaticMemberExpression(x) => x.compile(ctx),
            ast::MemberExpression::PrivateFieldExpression(x) => x.compile(ctx),
        }
    }
}

impl CompileEvaluation for ast::ComputedMemberExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        // 1. Let baseReference be ? Evaluation of MemberExpression.
        self.object.compile(ctx);

        // 2. Let baseValue be ? GetValue(baseReference).
        if is_reference(&self.object) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.add_instruction(Instruction::Load);

        // 4. Return ? EvaluatePropertyAccessWithExpressionKey(baseValue, Expression, strict).
        self.expression.compile(ctx);
        if is_reference(&self.expression) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }

        ctx.exe
            .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
    }
}

impl CompileEvaluation for ast::StaticMemberExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        // 1. Let baseReference be ? Evaluation of MemberExpression.
        self.object.compile(ctx);

        // 2. Let baseValue be ? GetValue(baseReference).
        if is_reference(&self.object) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }

        // 4. Return EvaluatePropertyAccessWithIdentifierKey(baseValue, IdentifierName, strict).
        let identifier = String::from_str(ctx.agent, self.property.name.as_str());
        ctx.exe.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            identifier,
        );
    }
}

impl CompileEvaluation for ast::PrivateFieldExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::AwaitExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::ChainExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::Class<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::ConditionalExpression<'_> {
    /// ## [13.14 Conditional Operator ( ? : )](https://tc39.es/ecma262/#sec-conditional-operator)
    /// ### [13.14.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-conditional-operator-runtime-semantics-evaluation)
    fn compile(&self, ctx: &mut CompileContext) {
        // 1. Let lref be ? Evaluation of ShortCircuitExpression.
        self.test.compile(ctx);
        // 2. Let lval be ToBoolean(? GetValue(lref)).
        if is_reference(&self.test) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        // Jump over first AssignmentExpression (consequent) if test fails.
        // Note: JumpIfNot performs ToBoolean from above step.
        let jump_to_second = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::JumpIfNot);
        // 3. If lval is true, then
        // a. Let trueRef be ? Evaluation of the first AssignmentExpression.
        self.consequent.compile(ctx);
        // b. Return ? GetValue(trueRef).
        if is_reference(&self.consequent) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        // Jump over second AssignmentExpression (alternate).
        let jump_over_second = ctx.exe.add_instruction_with_jump_slot(Instruction::Jump);
        // 4. Else,
        ctx.exe.set_jump_target_here(jump_to_second);
        // a. Let falseRef be ? Evaluation of the second AssignmentExpression.
        self.alternate.compile(ctx);
        // b. Return ? GetValue(falseRef).
        if is_reference(&self.alternate) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.set_jump_target_here(jump_over_second);
    }
}

impl CompileEvaluation for ast::ImportExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::MetaProperty<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::PrivateInExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::RegExpLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let pattern = String::from_str(ctx.agent, self.regex.pattern.as_str());
        let regexp =
            reg_exp_create(ctx.agent, pattern.into_value(), Some(self.regex.flags)).unwrap();
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, regexp);
    }
}

impl CompileEvaluation for ast::SequenceExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        for expr in &self.expressions {
            expr.compile(ctx);
        }
    }
}

impl CompileEvaluation for ast::Super {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::TaggedTemplateExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::TemplateLiteral<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if self.is_no_substitution_template() {
            let constant = String::from_str(
                ctx.agent,
                self.quasi()
                    .as_ref()
                    .expect("Invalid escape sequence in template literal")
                    .as_str(),
            );
            ctx.exe
                .add_instruction_with_constant(Instruction::StoreConstant, constant);
        } else {
            let mut count = 0;
            let mut quasis = self.quasis.as_slice();
            let mut expressions = self.expressions.as_slice();
            while let Some((head, rest)) = quasis.split_first() {
                quasis = rest;
                // 1. Let head be the TV of TemplateHead as defined in 12.9.6.
                let head =
                    String::from_str(ctx.agent, head.value.cooked.as_ref().unwrap().as_str());
                ctx.exe
                    .add_instruction_with_constant(Instruction::LoadConstant, head);
                count += 1;
                if let Some((expression, rest)) = expressions.split_first() {
                    expressions = rest;
                    // 2. Let subRef be ? Evaluation of Expression.
                    expression.compile(ctx);
                    if is_reference(expression) {
                        // 3. Let sub be ? GetValue(subRef).
                        ctx.exe.add_instruction(Instruction::GetValue);
                    }
                    // 4. Let middle be ? ToString(sub).
                    // Note: This is done by StringConcat.
                    ctx.exe.add_instruction(Instruction::Load);
                    count += 1;
                }
                // 5. Let tail be ? Evaluation of TemplateSpans.
            }
            // 6. Return the string-concatenation of head, middle, and tail.
            ctx.exe
                .add_instruction_with_immediate(Instruction::StringConcat, count);
        }
    }
}

impl CompileEvaluation for ast::ThisExpression {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe.add_instruction(Instruction::ResolveThisBinding);
    }
}

impl CompileEvaluation for ast::UsingDeclaration<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::YieldExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::Expression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Expression::ArrayExpression(x) => x.compile(ctx),
            ast::Expression::ArrowFunctionExpression(x) => x.compile(ctx),
            ast::Expression::AssignmentExpression(x) => x.compile(ctx),
            ast::Expression::AwaitExpression(x) => x.compile(ctx),
            ast::Expression::BigintLiteral(x) => x.compile(ctx),
            ast::Expression::BinaryExpression(x) => x.compile(ctx),
            ast::Expression::BooleanLiteral(x) => x.compile(ctx),
            ast::Expression::CallExpression(x) => x.compile(ctx),
            ast::Expression::ChainExpression(x) => x.compile(ctx),
            ast::Expression::ClassExpression(x) => x.compile(ctx),
            ast::Expression::ComputedMemberExpression(x) => x.compile(ctx),
            ast::Expression::ConditionalExpression(x) => x.compile(ctx),
            ast::Expression::FunctionExpression(x) => x.compile(ctx),
            ast::Expression::Identifier(x) => x.compile(ctx),
            ast::Expression::ImportExpression(x) => x.compile(ctx),
            ast::Expression::LogicalExpression(x) => x.compile(ctx),
            ast::Expression::MetaProperty(x) => x.compile(ctx),
            ast::Expression::NewExpression(x) => x.compile(ctx),
            ast::Expression::NullLiteral(x) => x.compile(ctx),
            ast::Expression::NumericLiteral(x) => x.compile(ctx),
            ast::Expression::ObjectExpression(x) => x.compile(ctx),
            ast::Expression::ParenthesizedExpression(x) => x.compile(ctx),
            ast::Expression::PrivateFieldExpression(x) => x.compile(ctx),
            ast::Expression::PrivateInExpression(x) => x.compile(ctx),
            ast::Expression::RegExpLiteral(x) => x.compile(ctx),
            ast::Expression::SequenceExpression(x) => x.compile(ctx),
            ast::Expression::StaticMemberExpression(x) => x.compile(ctx),
            ast::Expression::StringLiteral(x) => x.compile(ctx),
            ast::Expression::Super(x) => x.compile(ctx),
            ast::Expression::TaggedTemplateExpression(x) => x.compile(ctx),
            ast::Expression::TemplateLiteral(x) => x.compile(ctx),
            ast::Expression::ThisExpression(x) => x.compile(ctx),
            ast::Expression::UnaryExpression(x) => x.compile(ctx),
            ast::Expression::UpdateExpression(x) => x.compile(ctx),
            ast::Expression::YieldExpression(x) => x.compile(ctx),
            ast::Expression::JSXElement(_)
            | ast::Expression::JSXFragment(_)
            | ast::Expression::TSAsExpression(_)
            | ast::Expression::TSSatisfiesExpression(_)
            | ast::Expression::TSTypeAssertion(_)
            | ast::Expression::TSNonNullExpression(_)
            | ast::Expression::TSInstantiationExpression(_) => unreachable!(),
        }
    }
}

impl CompileEvaluation for ast::UpdateExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match &self.argument {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::ComputedMemberExpression(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::PrivateFieldExpression(_) => todo!(),
            ast::SimpleAssignmentTarget::StaticMemberExpression(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::TSAsExpression(_)
            | ast::SimpleAssignmentTarget::TSInstantiationExpression(_)
            | ast::SimpleAssignmentTarget::TSNonNullExpression(_)
            | ast::SimpleAssignmentTarget::TSSatisfiesExpression(_)
            | ast::SimpleAssignmentTarget::TSTypeAssertion(_) => unreachable!(),
        }
        ctx.exe.add_instruction(Instruction::GetValueKeepReference);
        if self.prefix {
            ctx.exe.add_instruction(Instruction::LoadCopy);
        }
        match self.operator {
            oxc_syntax::operator::UpdateOperator::Increment => {
                ctx.exe.add_instruction(Instruction::Increment);
            }
            oxc_syntax::operator::UpdateOperator::Decrement => {
                ctx.exe.add_instruction(Instruction::Decrement);
            }
        }
        if !self.prefix {
            ctx.exe.add_instruction(Instruction::LoadCopy);
        }
        ctx.exe.add_instruction(Instruction::PutValue);
        ctx.exe.add_instruction(Instruction::Store);
    }
}

impl CompileEvaluation for ast::ExpressionStatement<'_> {
    /// ### [14.5.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-expression-statement-runtime-semantics-evaluation)
    /// `ExpressionStatement : Expression ;`
    fn compile(&self, ctx: &mut CompileContext) {
        // 1. Let exprRef be ? Evaluation of Expression.
        self.expression.compile(ctx);
        if is_reference(&self.expression) {
            // 2. Return ? GetValue(exprRef).
            ctx.exe.add_instruction(Instruction::GetValue);
        }
    }
}

impl CompileEvaluation for ast::ReturnStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if let Some(expr) = &self.argument {
            expr.compile(ctx);
            if is_reference(expr) {
                ctx.exe.add_instruction(Instruction::GetValue);
            }
        } else {
            ctx.exe
                .add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        ctx.exe.add_instruction(Instruction::Return);
    }
}

impl CompileEvaluation for ast::IfStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        // if (test) consequent
        self.test.compile(ctx);
        if is_reference(&self.test) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        // jump over consequent if test fails
        let jump = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::JumpIfNot);
        self.consequent.compile(ctx);
        ctx.exe.set_jump_target_here(jump);
        let mut jump_over_else: Option<JumpIndex> = None;
        if let Some(alternate) = &self.alternate {
            // Optimisation: If the an else branch exists, the consequent
            // branch needs to end in a jump over it. But if the consequent
            // branch ends in a return statement that jump becomes unnecessary.
            if ctx.exe.peek_last_instruction() != Some(Instruction::Return.as_u8()) {
                jump_over_else = Some(ctx.exe.add_instruction_with_jump_slot(Instruction::Jump));
            }
            alternate.compile(ctx);
        }
        if let Some(jump_over_else) = jump_over_else {
            ctx.exe.set_jump_target_here(jump_over_else);
        }
    }
}

impl CompileEvaluation for ast::ArrayPattern<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if self.elements.is_empty() {
            return;
        }

        // TODO: Lift rest parameter restriction; it's eminently possible to
        // handle those as well.
        let simple = if self.rest.is_none()
            && !self.elements.iter().any(|ele| {
                // An array destructuring formed of only skipped values
                !ele.is_none()
                    // and binding identifier
                        && !matches!(
                            ele.as_ref().unwrap().kind,
                            ast::BindingPatternKind::BindingIdentifier(_)
                        )
            }) {
            // can be
            ctx.exe.add_instruction_with_immediate_and_immediate(
                Instruction::BeginSimpleArrayBindingPattern,
                self.elements.len(),
                ctx.lexical_binding_state.into(),
            );
            true
        } else {
            ctx.exe.add_instruction_with_immediate(
                Instruction::BeginArrayBindingPattern,
                ctx.lexical_binding_state.into(),
            );
            false
        };
        for ele in &self.elements {
            let Some(ele) = ele else {
                ctx.exe.add_instruction(Instruction::BindingPatternSkip);
                continue;
            };
            match &ele.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let identifier_string = ctx.create_identifier(&identifier.name);
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::BindingPatternBind,
                        identifier_string,
                    )
                }
                ast::BindingPatternKind::ObjectPattern(pattern) => {
                    ctx.exe.add_instruction(Instruction::BindingPatternGetValue);
                    pattern.compile(ctx);
                }
                ast::BindingPatternKind::ArrayPattern(pattern) => {
                    ctx.exe.add_instruction(Instruction::BindingPatternGetValue);
                    pattern.compile(ctx);
                }
                ast::BindingPatternKind::AssignmentPattern(pattern) => {
                    pattern.compile(ctx);
                }
            }
        }
        if let Some(rest) = &self.rest {
            match &rest.argument.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let identifier_string = ctx.create_identifier(&identifier.name);
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::BindingPatternBindRest,
                        identifier_string,
                    );
                }
                ast::BindingPatternKind::ObjectPattern(pattern) => {
                    ctx.exe
                        .add_instruction(Instruction::BindingPatternGetRestValue);
                    pattern.compile(ctx);
                }
                ast::BindingPatternKind::ArrayPattern(pattern) => {
                    ctx.exe
                        .add_instruction(Instruction::BindingPatternGetRestValue);
                    pattern.compile(ctx);
                }
                ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
            }
        }
        if !simple {
            ctx.exe.add_instruction(Instruction::FinishBindingPattern);
        }
    }
}

impl CompileEvaluation for ast::ObjectPattern<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe.add_instruction_with_immediate(
            Instruction::BeginObjectBindingPattern,
            ctx.lexical_binding_state.into(),
        );

        for ele in &self.properties {
            match &ele.key {
                ast::PropertyKey::StaticIdentifier(identifier) => {
                    let identifier_string = ctx.create_identifier(&identifier.name);
                    if ele.shorthand {
                        if let ast::BindingPatternKind::AssignmentPattern(_) = &ele.value.kind {
                            todo!();
                        } else {
                            ctx.exe.add_instruction_with_identifier(
                                Instruction::BindingPatternBind,
                                identifier_string,
                            );
                        }
                        // Skip the rest of the hard work.
                        continue;
                    }
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::EvaluatePropertyAccessWithIdentifierKey,
                        identifier_string,
                    );
                }
                ast::PropertyKey::PrivateIdentifier(_) => todo!(),
                ast::PropertyKey::BooleanLiteral(boolean) => {
                    // SAFETY: Keys use ToString, which special cases booleans.
                    let identifier_string = if boolean.value {
                        BUILTIN_STRING_MEMORY.r#true
                    } else {
                        BUILTIN_STRING_MEMORY.r#false
                    };
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::EvaluatePropertyAccessWithIdentifierKey,
                        identifier_string,
                    );
                }
                ast::PropertyKey::NullLiteral(_) => {
                    // SAFETY: Keys use ToString, which special cases null.
                    let identifier_string = BUILTIN_STRING_MEMORY.null;
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::BindingPatternBind,
                        identifier_string,
                    );
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::EvaluatePropertyAccessWithIdentifierKey,
                        identifier_string,
                    );
                }
                ast::PropertyKey::NumericLiteral(numeric) => {
                    // SAFETY: Keys use ToString, which special cases numbers
                    // by calling Number::toString(argument, 10). The work is
                    // inlined here.
                    let mut buffer = ryu_js::Buffer::new();
                    let identifier_string =
                        String::from_string(ctx.agent, buffer.format(numeric.value).to_string());
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::EvaluatePropertyAccessWithIdentifierKey,
                        identifier_string,
                    );
                }
                ast::PropertyKey::StringLiteral(literal) => {
                    let identifier_string = ctx.create_identifier(&literal.value);
                    if let ast::BindingPatternKind::BindingIdentifier(binding) = &ele.value.kind {
                        if binding.name == literal.value {
                            // const { "asd": asd } = source;
                            ctx.exe.add_instruction_with_identifier(
                                Instruction::BindingPatternBind,
                                identifier_string,
                            );
                            // Skip the rest of the hard stuff
                            continue;
                        }
                    }
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::EvaluatePropertyAccessWithIdentifierKey,
                        identifier_string,
                    );
                }
                ast::PropertyKey::ArrayExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ArrowFunctionExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::AssignmentExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::AwaitExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::BigintLiteral(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::BinaryExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::CallExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ChainExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ClassExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ConditionalExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::FunctionExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ImportExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::LogicalExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::MetaProperty(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::NewExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ObjectExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::PrivateInExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::RegExpLiteral(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::SequenceExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::TaggedTemplateExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::TemplateLiteral(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ThisExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::UnaryExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::UpdateExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::YieldExpression(expr) => {
                    expr.compile(ctx);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::Identifier(x) => {
                    x.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::Super(x) => {
                    x.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::ParenthesizedExpression(x) => {
                    x.compile(ctx);
                    if is_reference(&x.expression) {
                        ctx.exe.add_instruction(Instruction::GetValue);
                        ctx.exe
                            .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                    }
                }
                ast::PropertyKey::ComputedMemberExpression(x) => {
                    x.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::StaticMemberExpression(x) => {
                    x.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::PrivateFieldExpression(x) => {
                    x.compile(ctx);
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe
                        .add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                }
                ast::PropertyKey::JSXElement(_)
                | ast::PropertyKey::JSXFragment(_)
                | ast::PropertyKey::TSAsExpression(_)
                | ast::PropertyKey::TSSatisfiesExpression(_)
                | ast::PropertyKey::TSTypeAssertion(_)
                | ast::PropertyKey::TSNonNullExpression(_)
                | ast::PropertyKey::TSInstantiationExpression(_) => unreachable!(),
            }
            // We have either evaluated the property access by identifier
            // expression key. Now we need to assign resolve the access and
            // bind the a value.
            match &ele.value.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let identifier_string = ctx.create_identifier(&identifier.name);
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::BindingPatternBind,
                        identifier_string,
                    );
                }
                // const { key: { a }} = value;
                ast::BindingPatternKind::ObjectPattern(pattern) => {
                    ctx.exe.add_instruction(Instruction::BindingPatternGetValue);
                    pattern.compile(ctx);
                }
                // const { key: [a] } = value;
                ast::BindingPatternKind::ArrayPattern(pattern) => {
                    ctx.exe.add_instruction(Instruction::BindingPatternGetValue);
                    pattern.compile(ctx);
                }
                // const { a = 3 } = value;
                ast::BindingPatternKind::AssignmentPattern(_) => todo!(),
            }
        }
        ctx.exe.add_instruction(Instruction::FinishBindingPattern);
    }
}

impl CompileEvaluation for ast::AssignmentPattern<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match &self.left.kind {
            // const { a = 3 } = value;
            // const [a = 3] = value;
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.exe.add_instruction_with_identifier(
                    Instruction::BindingPatternBindWithInitializer,
                    identifier_string,
                );
            }
            // const {{ a } = right} = value;
            ast::BindingPatternKind::ObjectPattern(_) => {
                todo!();
            }
            // const [[ a ] = right] = value;
            ast::BindingPatternKind::ArrayPattern(_) => {
                todo!();
            }
            // Probably unreachable? Assignment into an assignment?
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
        self.right.compile(ctx);
    }
}

impl CompileEvaluation for ast::VariableDeclaration<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self.kind {
            // VariableStatement : var VariableDeclarationList ;
            ast::VariableDeclarationKind::Var => {
                for decl in &self.declarations {
                    // VariableDeclaration : BindingIdentifier
                    let Some(init) = &decl.init else {
                        // 1. Return EMPTY.
                        return;
                    };
                    // VariableDeclaration : BindingIdentifier Initializer

                    let ast::BindingPatternKind::BindingIdentifier(identifier) = &decl.id.kind
                    else {
                        //  VariableDeclaration : BindingPattern Initializer
                        ctx.lexical_binding_state = false;
                        // 1. Let rhs be ? Evaluation of Initializer.
                        init.compile(ctx);
                        // 2. Let rval be ? GetValue(rhs).
                        if is_reference(init) {
                            ctx.exe.add_instruction(Instruction::GetValue);
                        }
                        ctx.exe.add_instruction(Instruction::Load);
                        // 3. Return ? BindingInitialization of BidingPattern with arguments rval and undefined.
                        match &decl.id.kind {
                            ast::BindingPatternKind::BindingIdentifier(_) => unreachable!(),
                            ast::BindingPatternKind::ObjectPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::ArrayPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::AssignmentPattern(pattern) => {
                                pattern.compile(ctx)
                            }
                        }
                        return;
                    };

                    // 1. Let bindingId be StringValue of BindingIdentifier.
                    // 2. Let lhs be ? ResolveBinding(bindingId).
                    let identifier_string = String::from_str(ctx.agent, identifier.name.as_str());
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::ResolveBinding,
                        identifier_string,
                    );
                    ctx.exe.add_instruction(Instruction::PushReference);

                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    match &init {
                        ast::Expression::ArrowFunctionExpression(expr) => {
                            // Always anonymous
                            // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                            let name_identifier = ctx.exe.add_identifier(identifier_string);
                            ctx.name_identifier = Some(name_identifier);
                            expr.compile(ctx);
                        }
                        ast::Expression::FunctionExpression(expr) => {
                            if expr.id.is_none() {
                                // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                                let name_identifier = ctx.exe.add_identifier(identifier_string);
                                ctx.name_identifier = Some(name_identifier);
                            }
                            // 4. Else,
                            // a. Let rhs be ? Evaluation of Initializer.
                            expr.compile(ctx);
                        }
                        _ => {
                            // 4. Else,
                            // a. Let rhs be ? Evaluation of Initializer.
                            init.compile(ctx);
                            // b. Let value be ? GetValue(rhs).
                            if is_reference(init) {
                                ctx.exe.add_instruction(Instruction::GetValue);
                            }
                        }
                    };
                    // 5. Perform ? PutValue(lhs, value).
                    ctx.exe.add_instruction(Instruction::PopReference);
                    ctx.exe.add_instruction(Instruction::PutValue);

                    // 6. Return EMPTY.
                    // Store Undefined as the result value.
                    ctx.exe.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        Value::Undefined,
                    );
                }
            }
            ast::VariableDeclarationKind::Let | ast::VariableDeclarationKind::Const => {
                for decl in &self.declarations {
                    let ast::BindingPatternKind::BindingIdentifier(identifier) = &decl.id.kind
                    else {
                        ctx.lexical_binding_state = true;
                        let init = decl.init.as_ref().unwrap();

                        //  LexicalBinding : BindingPattern Initializer
                        // 1. Let rhs be ? Evaluation of Initializer.
                        init.compile(ctx);
                        // 2. Let value be ? GetValue(rhs).
                        if is_reference(init) {
                            ctx.exe.add_instruction(Instruction::GetValue);
                        }
                        // 3. Let env be the running execution context's LexicalEnvironment.
                        // 4. Return ? BindingInitialization of BindingPattern with arguments value and env.
                        ctx.exe.add_instruction(Instruction::Load);
                        match &decl.id.kind {
                            ast::BindingPatternKind::BindingIdentifier(_) => unreachable!(),
                            ast::BindingPatternKind::ObjectPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::ArrayPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::AssignmentPattern(pattern) => {
                                pattern.compile(ctx)
                            }
                        }
                        return;
                    };

                    // 1. Let lhs be ! ResolveBinding(StringValue of BindingIdentifier).
                    let identifier_string = String::from_str(ctx.agent, identifier.name.as_str());
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::ResolveBinding,
                        identifier_string,
                    );

                    let Some(init) = &decl.init else {
                        // LexicalBinding : BindingIdentifier
                        // 2. Perform ! InitializeReferencedBinding(lhs, undefined).
                        ctx.exe.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            Value::Undefined,
                        );
                        ctx.exe
                            .add_instruction(Instruction::InitializeReferencedBinding);
                        // 3. Return empty.
                        ctx.exe.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            Value::Undefined,
                        );
                        return;
                    };

                    //  LexicalBinding : BindingIdentifier Initializer
                    ctx.exe.add_instruction(Instruction::PushReference);
                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    match &init {
                        ast::Expression::ArrowFunctionExpression(expr) => {
                            // Always anonymous
                            // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                            let name_identifier = ctx.exe.add_identifier(identifier_string);
                            ctx.name_identifier = Some(name_identifier);
                            expr.compile(ctx);
                        }
                        ast::Expression::FunctionExpression(expr) => {
                            if expr.id.is_none() {
                                // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                                let name_identifier = ctx.exe.add_identifier(identifier_string);
                                ctx.name_identifier = Some(name_identifier);
                            }
                            // 4. Else,
                            // a. Let rhs be ? Evaluation of Initializer.
                            expr.compile(ctx);
                        }
                        _ => {
                            // 4. Else,
                            // a. Let rhs be ? Evaluation of Initializer.
                            init.compile(ctx);
                            // b. Let value be ? GetValue(rhs).
                            if is_reference(init) {
                                ctx.exe.add_instruction(Instruction::GetValue);
                            }
                        }
                    };

                    // 5. Perform ! InitializeReferencedBinding(lhs, value).
                    ctx.exe.add_instruction(Instruction::PopReference);
                    ctx.exe
                        .add_instruction(Instruction::InitializeReferencedBinding);
                    // 6. Return empty.
                    ctx.exe.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        Value::Undefined,
                    );
                }
            }
        }
    }
}

impl CompileEvaluation for ast::Declaration<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Declaration::VariableDeclaration(x) => x.compile(ctx),
            ast::Declaration::FunctionDeclaration(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}

impl CompileEvaluation for ast::BlockStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if self.body.is_empty() {
            // Block : {}
            // 1. Return EMPTY.
            return;
        }
        ctx.exe
            .add_instruction(Instruction::EnterDeclarativeEnvironment);
        // SAFETY: Stupid lifetime transmute.
        let body = unsafe {
            std::mem::transmute::<
                &oxc_allocator::Vec<'_, Statement<'_>>,
                &'static oxc_allocator::Vec<'static, Statement<'static>>,
            >(&self.body)
        };
        body.lexically_scoped_declarations(&mut |decl| {
            match decl {
                LexicallyScopedDeclaration::Variable(decl) => {
                    if decl.kind.is_const() {
                        decl.id.bound_names(&mut |name| {
                            let identifier = String::from_str(ctx.agent, name.name.as_str());
                            ctx.exe.add_instruction_with_identifier(
                                Instruction::CreateImmutableBinding,
                                identifier,
                            );
                        });
                    } else if decl.kind.is_lexical() {
                        decl.id.bound_names(&mut |name| {
                            let identifier = String::from_str(ctx.agent, name.name.as_str());
                            ctx.exe.add_instruction_with_identifier(
                                Instruction::CreateMutableBinding,
                                identifier,
                            );
                        });
                    }
                }
                LexicallyScopedDeclaration::Function(decl) => {
                    // TODO: InstantiateFunctionObject and InitializeBinding
                    decl.bound_names(&mut |name| {
                        let identifier = String::from_str(ctx.agent, name.name.as_str());
                        ctx.exe.add_instruction_with_identifier(
                            Instruction::CreateMutableBinding,
                            identifier,
                        );
                    });
                }
                LexicallyScopedDeclaration::Class(decl) => {
                    decl.bound_names(&mut |name| {
                        let identifier = String::from_str(ctx.agent, name.name.as_str());
                        ctx.exe.add_instruction_with_identifier(
                            Instruction::CreateMutableBinding,
                            identifier,
                        );
                    });
                }
                LexicallyScopedDeclaration::DefaultExport => unreachable!(),
            }
        });
        for ele in &self.body {
            ele.compile(ctx);
        }
        if ctx.exe.peek_last_instruction() != Some(Instruction::Return.as_u8()) {
            // Block did not end in a return so we overwrite the result with undefined.
            ctx.exe
                .add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        ctx.exe
            .add_instruction(Instruction::ExitDeclarativeEnvironment);
    }
}

impl CompileEvaluation for ast::ForStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let previous_continue = ctx.current_continue.replace(vec![]);
        let previous_break = ctx.current_break.replace(vec![]);

        if let Some(init) = &self.init {
            if init.is_lexical_declaration() {
                todo!();
            }
            match init {
                ast::ForStatementInit::ArrayExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ArrowFunctionExpression(init) => init.compile(ctx),
                ast::ForStatementInit::AssignmentExpression(init) => init.compile(ctx),
                ast::ForStatementInit::AwaitExpression(init) => init.compile(ctx),
                ast::ForStatementInit::BigintLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::BinaryExpression(init) => init.compile(ctx),
                ast::ForStatementInit::BooleanLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::CallExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ChainExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ClassExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ComputedMemberExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ConditionalExpression(init) => init.compile(ctx),
                ast::ForStatementInit::FunctionExpression(init) => init.compile(ctx),
                ast::ForStatementInit::Identifier(init) => init.compile(ctx),
                ast::ForStatementInit::ImportExpression(init) => init.compile(ctx),
                ast::ForStatementInit::LogicalExpression(init) => init.compile(ctx),
                ast::ForStatementInit::MetaProperty(init) => init.compile(ctx),
                ast::ForStatementInit::NewExpression(init) => init.compile(ctx),
                ast::ForStatementInit::NullLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::NumericLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::ObjectExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ParenthesizedExpression(init) => init.compile(ctx),
                ast::ForStatementInit::PrivateFieldExpression(init) => init.compile(ctx),
                ast::ForStatementInit::PrivateInExpression(init) => init.compile(ctx),
                ast::ForStatementInit::RegExpLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::SequenceExpression(init) => init.compile(ctx),
                ast::ForStatementInit::StaticMemberExpression(init) => init.compile(ctx),
                ast::ForStatementInit::StringLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::Super(init) => init.compile(ctx),
                ast::ForStatementInit::TaggedTemplateExpression(init) => init.compile(ctx),
                ast::ForStatementInit::TemplateLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::ThisExpression(init) => init.compile(ctx),
                ast::ForStatementInit::UnaryExpression(init) => init.compile(ctx),
                ast::ForStatementInit::UpdateExpression(init) => init.compile(ctx),
                ast::ForStatementInit::UsingDeclaration(init) => init.compile(ctx),
                ast::ForStatementInit::VariableDeclaration(init) => init.compile(ctx),
                ast::ForStatementInit::YieldExpression(init) => init.compile(ctx),
                ast::ForStatementInit::JSXElement(_)
                | ast::ForStatementInit::JSXFragment(_)
                | ast::ForStatementInit::TSAsExpression(_)
                | ast::ForStatementInit::TSSatisfiesExpression(_)
                | ast::ForStatementInit::TSTypeAssertion(_)
                | ast::ForStatementInit::TSNonNullExpression(_)
                | ast::ForStatementInit::TSInstantiationExpression(_) => unreachable!(),
            }
        }
        let loop_jump = ctx.exe.get_jump_index_to_here();
        if let Some(test) = &self.test {
            test.compile(ctx);
            if is_reference(test) {
                ctx.exe.add_instruction(Instruction::GetValue);
            }
        }
        // jump over consequent if test fails
        let end_jump = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::JumpIfNot);
        self.body.compile(ctx);

        let own_continues = ctx.current_continue.take().unwrap();
        for continue_entry in own_continues {
            ctx.exe.set_jump_target_here(continue_entry);
        }

        if let Some(update) = &self.update {
            update.compile(ctx);
        }
        ctx.exe
            .add_jump_instruction_to_index(Instruction::Jump, loop_jump);
        ctx.exe.set_jump_target_here(end_jump);

        let own_breaks = ctx.current_break.take().unwrap();
        for break_entry in own_breaks {
            ctx.exe.set_jump_target_here(break_entry);
        }
        ctx.current_break = previous_break;
        ctx.current_continue = previous_continue;
    }
}

impl CompileEvaluation for ast::ThrowStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.argument.compile(ctx);
        if is_reference(&self.argument) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.add_instruction(Instruction::Throw)
    }
}

impl CompileEvaluation for ast::TryStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if self.finalizer.is_some() {
            todo!();
        }

        let jump_to_catch = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget);
        self.block.compile(ctx);
        ctx.exe.add_instruction(Instruction::PopExceptionJumpTarget);
        let jump_to_end = ctx.exe.add_instruction_with_jump_slot(Instruction::Jump);

        let catch_clause = self.handler.as_ref().unwrap();
        ctx.exe.set_jump_target_here(jump_to_catch);
        if let Some(exception_param) = &catch_clause.param {
            let ast::BindingPatternKind::BindingIdentifier(identifier) =
                &exception_param.pattern.kind
            else {
                todo!("{:?}", exception_param.pattern.kind);
            };
            ctx.exe
                .add_instruction(Instruction::EnterDeclarativeEnvironment);
            let identifier_string = String::from_str(ctx.agent, identifier.name.as_str());
            ctx.exe.add_instruction_with_identifier(
                Instruction::CreateCatchBinding,
                identifier_string,
            );
        }
        catch_clause.body.compile(ctx);
        if catch_clause.param.is_some() {
            ctx.exe
                .add_instruction(Instruction::ExitDeclarativeEnvironment);
        }
        ctx.exe.set_jump_target_here(jump_to_end);
    }
}

impl CompileEvaluation for ast::DoWhileStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let previous_continue = ctx.current_continue.replace(vec![]);
        let previous_break = ctx.current_break.replace(vec![]);

        let start_jump = ctx.exe.get_jump_index_to_here();
        self.body.compile(ctx);

        let own_continues = ctx.current_continue.take().unwrap();
        for continue_entry in own_continues {
            ctx.exe.set_jump_target_here(continue_entry);
        }

        self.test.compile(ctx);
        if is_reference(&self.test) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        // jump over loop jump if test fails
        let end_jump = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::JumpIfNot);
        ctx.exe
            .add_jump_instruction_to_index(Instruction::Jump, start_jump);
        ctx.exe.set_jump_target_here(end_jump);

        let own_breaks = ctx.current_break.take().unwrap();
        for break_entry in own_breaks {
            ctx.exe.set_jump_target_here(break_entry);
        }
        ctx.current_break = previous_break;
        ctx.current_continue = previous_continue;
    }
}

impl CompileEvaluation for ast::BreakStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if let Some(label) = &self.label {
            let label = label.name.as_str();
            todo!("break {};", label);
        }
        let break_jump = ctx.exe.add_instruction_with_jump_slot(Instruction::Jump);
        ctx.current_break.as_mut().unwrap().push(break_jump);
    }
}

impl CompileEvaluation for ast::ContinueStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        if let Some(label) = &self.label {
            let label = label.name.as_str();
            todo!("continue {};", label);
        }
        let continue_jump = ctx.exe.add_instruction_with_jump_slot(Instruction::Jump);
        ctx.current_continue.as_mut().unwrap().push(continue_jump);
    }
}

impl CompileEvaluation for ast::Statement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Statement::ExpressionStatement(x) => x.compile(ctx),
            ast::Statement::ReturnStatement(x) => x.compile(ctx),
            ast::Statement::IfStatement(x) => x.compile(ctx),
            ast::Statement::VariableDeclaration(x) => x.compile(ctx),
            ast::Statement::FunctionDeclaration(x) => x.compile(ctx),
            ast::Statement::BlockStatement(x) => x.compile(ctx),
            ast::Statement::EmptyStatement(_) => {}
            ast::Statement::ForStatement(x) => x.compile(ctx),
            ast::Statement::ThrowStatement(x) => x.compile(ctx),
            ast::Statement::TryStatement(x) => x.compile(ctx),
            Statement::BreakStatement(statement) => statement.compile(ctx),
            Statement::ContinueStatement(statement) => statement.compile(ctx),
            Statement::DebuggerStatement(_) => todo!(),
            Statement::DoWhileStatement(statement) => statement.compile(ctx),
            Statement::ForInStatement(_) => todo!(),
            Statement::ForOfStatement(_) => todo!(),
            Statement::LabeledStatement(_) => todo!(),
            Statement::SwitchStatement(_) => todo!(),
            Statement::WhileStatement(_) => todo!(),
            Statement::WithStatement(_) => todo!(),
            Statement::ClassDeclaration(_) => todo!(),
            Statement::UsingDeclaration(_) => todo!(),
            Statement::ImportDeclaration(_) => todo!(),
            Statement::ExportAllDeclaration(_) => todo!(),
            Statement::ExportDefaultDeclaration(_) => todo!(),
            Statement::ExportNamedDeclaration(_) => todo!(),
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSInterfaceDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_)
            | Statement::TSTypeAliasDeclaration(_) => unreachable!(),
        }
    }
}
