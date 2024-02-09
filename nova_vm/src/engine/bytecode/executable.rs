use super::{instructions::Instr, Instruction};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_property_key,
        execution::Agent,
        scripts_and_modules::script::ScriptIdentifier,
        syntax_directed_operations::scope_analysis::{
            LexicallyScopedDeclaration, LexicallyScopedDeclarations,
        },
        types::{BigIntHeapData, Reference, String, Value},
    },
    heap::CreateHeapData,
};
use oxc_ast::{
    ast::{self, CallExpression, FunctionBody, Statement},
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
}

#[derive(Debug)]
pub(crate) struct FunctionExpression {
    pub(crate) expression: &'static ast::Function<'static>,
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
    pub(crate) identifiers: Vec<Atom>,
    pub(crate) references: Vec<Reference>,
    pub(crate) function_expressions: Vec<FunctionExpression>,
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
        eprintln!();
        eprintln!("=== Compiling Script ===");
        eprintln!();
        // SAFETY: Script uniquely owns the Program and the body buffer does
        // not move under any circumstances during heap operations.
        let body: &[Statement] = unsafe {
            std::mem::transmute(
                agent
                    .heap
                    .get_script(script)
                    .ecmascript_code
                    .body
                    .as_slice(),
            )
        };

        Self::_compile_statements(agent, body)
    }

    pub(crate) fn compile_function_body(agent: &mut Agent, body: &FunctionBody<'_>) -> Executable {
        eprintln!();
        eprintln!("=== Compiling Function ===");
        eprintln!();
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
            },
            name_identifier: None,
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
            .push(unsafe { std::mem::transmute(instruction) });
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

    fn add_identifier(&mut self, identifier: &Atom) -> usize {
        let duplicate = self
            .identifiers
            .iter()
            .enumerate()
            .find(|item| item.1 == identifier)
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.identifiers.len();
            self.identifiers.push(identifier.clone());
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

    fn add_instruction_with_identifier(&mut self, instruction: Instruction, identifier: &Atom) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_identifier_index());
        self._push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
    }

    fn add_instruction_with_identifier_and_constant(
        &mut self,
        instruction: Instruction,
        identifier: &Atom,
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

#[derive(Debug)]
pub(crate) struct JumpIndex {
    pub(crate) index: usize,
}

pub(crate) trait CompileEvaluation {
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

impl CompileEvaluation for ast::NumberLiteral<'_> {
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

impl CompileEvaluation for ast::BigintLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = ctx.agent.heap.create(BigIntHeapData {
            data: self.value.clone(),
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

impl CompileEvaluation for ast::StringLiteral {
    fn compile(&self, ctx: &mut CompileContext) {
        let constant = Value::from_str(&mut ctx.agent.heap, self.value.as_str());
        ctx.exe
            .add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl CompileEvaluation for ast::IdentifierReference {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_identifier(Instruction::ResolveBinding, &self.name);
    }
}

impl CompileEvaluation for ast::BindingIdentifier {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.exe
            .add_instruction_with_identifier(Instruction::ResolveBinding, &self.name);
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
            UnaryOperator::UnaryPlus => todo!(),
            UnaryOperator::LogicalNot => todo!(),
            UnaryOperator::BitwiseNot => todo!(),
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
                ctx.exe.add_instruction(Instruction::Load);

                // 3. Let rref be ? Evaluation of ShiftExpression.
                self.right.compile(ctx);

                // 4. Let rval be ? GetValue(rref).
                if is_reference(&self.right) {
                    ctx.exe.add_instruction(Instruction::GetValue);
                }

                // 5. Let r be ? IsLessThan(lval, rval, true).
                // 6. If r is undefined, return false. Otherwise, return r.
                ctx.exe.add_instruction(Instruction::LessThan);
                return;
            }
            _ => {
                // TODO(@carter): Figure out if this fallthrough is correct?
            }
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

impl CompileEvaluation for ast::AssignmentExpression<'_> {
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

        ctx.exe.add_instruction(Instruction::PopReference);
        ctx.exe.add_instruction(Instruction::PutValue);
    }
}

impl CompileEvaluation for ast::ParenthesizedExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.expression.compile(ctx);
    }
}

impl CompileEvaluation for ast::ArrowExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
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
                        ast::PropertyKey::Identifier(id) => {
                            // TODO: If property key is __proto__ and it is not a shorthand ({ __proto__ })
                            // then we should dispatch a SetPrototype instruction.
                            let property_key = String::from_str(ctx.agent, id.name.as_str());
                            let property_key =
                                to_property_key(ctx.agent, property_key.into()).unwrap();
                            ctx.exe.add_instruction_with_constant(
                                Instruction::LoadConstant,
                                property_key,
                            );
                        }
                        ast::PropertyKey::PrivateIdentifier(_) => todo!(),
                        ast::PropertyKey::Expression(_) => todo!(),
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

impl CompileEvaluation for CallExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        self.callee.compile(ctx);
        if is_reference(&self.callee) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.add_instruction(Instruction::Load);
        for ele in &self.arguments {
            match ele {
                ast::Argument::SpreadElement(_) => {
                    panic!("Cannot support SpreadElements currently")
                }
                ast::Argument::Expression(expr) => {
                    expr.compile(ctx);
                    ctx.exe.add_instruction(Instruction::Load);
                }
            }
        }

        ctx.exe
            .add_instruction_with_immediate(Instruction::EvaluateCall, self.arguments.len());
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
        ctx.exe.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            &self.property.name,
        );
    }
}

impl CompileEvaluation for ast::PrivateFieldExpression<'_> {
    fn compile(&self, _ctx: &mut CompileContext) {
        todo!()
    }
}

impl CompileEvaluation for ast::Expression<'_> {
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
            ast::Expression::NullLiteral(x) => x.compile(ctx),
            ast::Expression::StringLiteral(x) => x.compile(ctx),
            ast::Expression::FunctionExpression(x) => x.compile(ctx),
            ast::Expression::ObjectExpression(x) => x.compile(ctx),
            ast::Expression::CallExpression(x) => x.compile(ctx),
            ast::Expression::MemberExpression(x) => x.compile(ctx),
            ast::Expression::UpdateExpression(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}

impl CompileEvaluation for ast::UpdateExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match &self.argument {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::MemberAssignmentTarget(_) => todo!(),
            ast::SimpleAssignmentTarget::TSAsExpression(_)
            | ast::SimpleAssignmentTarget::TSSatisfiesExpression(_)
            | ast::SimpleAssignmentTarget::TSNonNullExpression(_)
            | ast::SimpleAssignmentTarget::TSTypeAssertion(_) => unreachable!(),
        }
        ctx.exe.add_instruction(Instruction::GetValueKeepReference);
        ctx.exe.add_instruction(Instruction::Increment);
        ctx.exe.add_instruction(Instruction::PutValue);
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

impl CompileEvaluation for ast::VariableDeclaration<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self.kind {
            // VariableStatement : var VariableDeclarationList ;
            ast::VariableDeclarationKind::Var => {
                for decl in &self.declarations {
                    // VariableDeclaration : BindingIdentifier
                    if decl.init.is_none() {
                        // 1. Return EMPTY.
                        return;
                    }
                    let ast::BindingPatternKind::BindingIdentifier(identifier) = &decl.id.kind
                    else {
                        todo!("{:?}", decl.id.kind);
                    };

                    // VariableDeclaration : BindingIdentifier Initializer
                    let init = decl.init.as_ref().unwrap();

                    // 1. Let bindingId be StringValue of BindingIdentifier.
                    // 2. Let lhs be ? ResolveBinding(bindingId).
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::ResolveBinding,
                        &identifier.name,
                    );
                    ctx.exe.add_instruction(Instruction::PushReference);

                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    match &init {
                        ast::Expression::ArrowExpression(expr) => {
                            // Always anonymous
                            // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                            let name_identifier = ctx.exe.add_identifier(&identifier.name);
                            ctx.name_identifier = Some(name_identifier);
                            expr.compile(ctx);
                        }
                        ast::Expression::FunctionExpression(expr) => {
                            if expr.id.is_none() {
                                // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                                let name_identifier = ctx.exe.add_identifier(&identifier.name);
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
                        //  LexicalBinding : BindingPattern Initializer
                        todo!("{:?}", decl.id.kind);
                    };

                    // 1. Let lhs be ! ResolveBinding(StringValue of BindingIdentifier).
                    ctx.exe.add_instruction_with_identifier(
                        Instruction::ResolveBinding,
                        &identifier.name,
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
                        ast::Expression::ArrowExpression(expr) => {
                            // Always anonymous
                            // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                            let name_identifier = ctx.exe.add_identifier(&identifier.name);
                            ctx.name_identifier = Some(name_identifier);
                            expr.compile(ctx);
                        }
                        ast::Expression::FunctionExpression(expr) => {
                            if expr.id.is_none() {
                                // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                                let name_identifier = ctx.exe.add_identifier(&identifier.name);
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
                            ctx.exe.add_instruction_with_identifier(
                                Instruction::CreateImmutableBinding,
                                &name.name,
                            );
                        });
                    } else {
                        decl.id.bound_names(&mut |name| {
                            ctx.exe.add_instruction_with_identifier(
                                Instruction::CreateMutableBinding,
                                &name.name,
                            );
                        });
                    }
                }
                LexicallyScopedDeclaration::Function(decl) => {
                    // TODO: InstantiateFunctionObject and InitializeBinding
                    decl.bound_names(&mut |name| {
                        ctx.exe.add_instruction_with_identifier(
                            Instruction::CreateMutableBinding,
                            &name.name,
                        );
                    });
                }
                LexicallyScopedDeclaration::Class(decl) => {
                    decl.bound_names(&mut |name| {
                        ctx.exe.add_instruction_with_identifier(
                            Instruction::CreateMutableBinding,
                            &name.name,
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
        if let Some(init) = &self.init {
            if init.is_lexical_declaration() {
                todo!();
            }
            match init {
                ast::ForStatementInit::VariableDeclaration(init) => init.compile(ctx),
                ast::ForStatementInit::Expression(init) => init.compile(ctx),
                ast::ForStatementInit::UsingDeclaration(_) => todo!(),
            }
        }
        let loop_jump = ctx.exe.get_jump_index_to_here();
        if let Some(test) = &self.test {
            test.compile(ctx);
        }
        // jump over consequent if test fails
        let end_jump = ctx
            .exe
            .add_instruction_with_jump_slot(Instruction::JumpIfNot);
        self.body.compile(ctx);
        let _continue_jump = ctx.exe.get_jump_index_to_here();
        if let Some(update) = &self.update {
            update.compile(ctx);
        }
        ctx.exe
            .add_jump_instruction_to_index(Instruction::Jump, loop_jump);
        ctx.exe.set_jump_target_here(end_jump);
    }
}

impl CompileEvaluation for ast::Statement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::Statement::ExpressionStatement(x) => x.compile(ctx),
            ast::Statement::ReturnStatement(x) => x.compile(ctx),
            ast::Statement::IfStatement(x) => x.compile(ctx),
            ast::Statement::Declaration(x) => x.compile(ctx),
            ast::Statement::BlockStatement(x) => x.compile(ctx),
            ast::Statement::EmptyStatement(_) => {}
            ast::Statement::ForStatement(x) => x.compile(ctx),
            other => todo!("{other:?}"),
        }
    }
}
