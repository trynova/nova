// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod assignment;
mod block_declaration_instantiation;
mod class_definition_evaluation;
mod for_in_of_statement;
mod function_declaration_instantiation;
mod labelled_statement;

use std::{cell::RefCell, rc::Rc};

use super::{
    Executable, ExecutableHeapData, FunctionExpression, Instruction, SendableRef,
    executable::ArrowFunctionExpression,
};
#[cfg(feature = "regexp")]
use crate::ecmascript::builtins::regexp::reg_exp_create_literal;
use crate::{
    ecmascript::{
        execution::Agent,
        syntax_directed_operations::{
            function_definitions::{CompileFunctionBodyData, ContainsExpression},
            scope_analysis::{LexicallyScopedDeclaration, LexicallyScopedDeclarations},
        },
        types::{BUILTIN_STRING_MEMORY, BigInt, IntoValue, Number, PropertyKey, String, Value},
    },
    engine::context::{Bindable, NoGcScope},
    heap::CreateHeapData,
};
use ahash::AHashMap;
use num_traits::Num;
use oxc_ast::ast::{
    self, BindingPattern, BindingRestElement, CallExpression, LabelIdentifier, NewExpression,
    Statement,
};
use oxc_ecmascript::BoundNames;
use oxc_span::Atom;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

pub type IndexType = u16;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NamedEvaluationParameter {
    /// Name is in the result register
    Result,
    /// Name is at the top of the stack
    Stack,
    /// Name is in the reference register
    Reference,
    /// Name is at the top of the reference stack
    ReferenceStack,
}

pub(crate) struct JumpTarget {
    /// Depth of the lexical of the jump target.
    ///
    /// This is used to determine how many ExitDeclarativeEnvironment
    /// instructions are needed before jumping to this target from a continue
    /// or break statement.
    depth: u32,
    /// `continue;` statements that target this jump target.
    pub(crate) continues: Vec<JumpIndex>,
    /// `break;` statements that target this jump target.
    pub(crate) breaks: Vec<JumpIndex>,
}

impl JumpTarget {
    pub(super) fn new(depth: u32) -> Self {
        Self {
            depth,
            continues: vec![],
            breaks: vec![],
        }
    }
}

/// Context for bytecode compilation.
///
/// The lifetimes on this context are:
/// - `'agent`: The lifetime of the Agent, which owns the heap.
/// - `'script`: The lifetime of the oxc Program struct which contains the AST.
/// - `'gc`: The garbage collector marker lifetime, needed for tracking garbage
///   collected data lifetime.
/// - `'scope`: The Javascript scope marker lifetime, only here because `gc`
///   tracks it.
pub(crate) struct CompileContext<'agent, 'script, 'gc, 'scope> {
    pub(crate) agent: &'agent mut Agent,
    pub(crate) gc: NoGcScope<'gc, 'scope>,
    /// Instructions being built
    instructions: Vec<u8>,
    /// Constants being built
    constants: Vec<Value<'gc>>,
    /// Function expressions being built
    function_expressions: Vec<FunctionExpression<'gc>>,
    /// Arrow function expressions being built
    arrow_function_expressions: Vec<ArrowFunctionExpression>,
    class_initializer_bytecodes: Vec<(Option<Executable<'gc>>, bool)>,
    /// NamedEvaluation name parameter
    name_identifier: Option<NamedEvaluationParameter>,
    /// If true, indicates that all bindings being created are lexical.
    ///
    /// Otherwise, all bindings being created are variable scoped.
    lexical_binding_state: bool,
    /// Current depth of the lexical scope stack.
    current_lexical_depth: u32,
    current_jump_target: Option<Rc<RefCell<JumpTarget>>>,
    labelled_statements: Option<Box<AHashMap<Atom<'script>, Rc<RefCell<JumpTarget>>>>>,
    /// `?.` chain jumps that were present in a chain expression.
    optional_chains: Option<Vec<JumpIndex>>,
    /// In a `(a?.b).unbind()?.bind(gc.nogc()).()` chain the evaluation of `(a?.b)` must be considered a
    /// reference.
    is_call_optional_chain_this: bool,
}

impl<'a, 's, 'gc, 'scope> CompileContext<'a, 's, 'gc, 'scope> {
    pub(super) fn new(
        agent: &'a mut Agent,
        gc: NoGcScope<'gc, 'scope>,
    ) -> CompileContext<'a, 's, 'gc, 'scope> {
        CompileContext {
            agent,
            gc,
            instructions: Vec::new(),
            constants: Vec::new(),
            function_expressions: Vec::new(),
            arrow_function_expressions: Vec::new(),
            class_initializer_bytecodes: Vec::new(),
            name_identifier: None,
            lexical_binding_state: false,
            current_lexical_depth: 0,
            optional_chains: None,
            current_jump_target: None,
            labelled_statements: None,
            is_call_optional_chain_this: false,
        }
    }

    pub(super) fn return_jump_target(&mut self, jump_target: Option<Rc<RefCell<JumpTarget>>>) {
        let Some(jump_target) = jump_target else {
            return;
        };
        self.current_jump_target.replace(jump_target);
    }

    pub(super) fn push_new_jump_target(
        &mut self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
    ) -> Option<Rc<RefCell<JumpTarget>>> {
        let depth = self.current_lexical_depth;
        let jump_target = Rc::new(RefCell::new(JumpTarget::new(depth)));
        if let Some(label_set) = label_set {
            for label in label_set {
                let pervious = self
                    .labelled_statements
                    .as_mut()
                    .unwrap()
                    .insert(label.name, jump_target.clone());
                assert!(pervious.is_none());
            }
        }
        self.current_jump_target.replace(jump_target)
    }

    pub(super) fn take_current_jump_target(
        &mut self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
    ) -> JumpTarget {
        if let Some(label_set) = label_set {
            for label in label_set {
                // Note: removing the labeled statement here is important, as
                // without it the Rc::into_inner below will fail.
                self.labelled_statements
                    .as_mut()
                    .unwrap()
                    .remove(&label.name);
            }
        }
        Rc::into_inner(self.current_jump_target.take().unwrap())
            .unwrap()
            .into_inner()
    }

    /// Compile a class static field with an optional initializer into the
    /// current context.
    pub(crate) fn compile_class_static_field(
        &mut self,
        identifier_name: &'s ast::IdentifierName<'s>,
        value: &'s Option<ast::Expression<'s>>,
    ) {
        let identifier = String::from_str(self.agent, identifier_name.name.as_str(), self.gc);
        // Turn the static name to a 'this' property access.
        self.add_instruction(Instruction::ResolveThisBinding);
        self.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            identifier,
        );
        if let Some(value) = value {
            // Minor optimisation: We do not need to push and pop the
            // reference if we know we're not using the reference stack.
            let is_literal = value.is_literal();
            if !is_literal {
                self.add_instruction(Instruction::PushReference);
            }
            value.compile(self);
            if is_reference(value) {
                self.add_instruction(Instruction::GetValue);
            }
            if !is_literal {
                self.add_instruction(Instruction::PopReference);
            }
        } else {
            // Same optimisation is unconditionally valid here.
            self.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        self.add_instruction(Instruction::PutValue);
    }

    /// Compile a class computed field with an optional initializer into the
    /// current context.
    pub(crate) fn compile_class_computed_field(
        &mut self,
        property_key_id: String<'gc>,
        value: &'s Option<ast::Expression<'s>>,
    ) {
        // Resolve 'this' into the stack.
        self.add_instruction(Instruction::ResolveThisBinding);
        self.add_instruction(Instruction::Load);
        // Resolve the static computed key ID to the actual computed key value.
        self.add_instruction_with_identifier(Instruction::ResolveBinding, property_key_id);
        // Store the computed key value as the result.
        self.add_instruction(Instruction::GetValue);
        // Evaluate access to 'this' with the computed key.
        self.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
        if let Some(value) = value {
            // Minor optimisation: We do not need to push and pop the
            // reference if we know we're not using the reference stack.
            let is_literal = value.is_literal();
            if !is_literal {
                self.add_instruction(Instruction::PushReference);
            }
            value.compile(self);
            if is_reference(value) {
                self.add_instruction(Instruction::GetValue);
            }
            if !is_literal {
                self.add_instruction(Instruction::PopReference);
            }
        } else {
            // Same optimisation is unconditionally valid here.
            self.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        self.add_instruction(Instruction::PutValue);
    }

    /// Compile a function body into the current context.
    ///
    /// This is useful when the function body is part of a larger whole, namely
    /// with class constructors.
    pub(crate) fn compile_function_body(&mut self, data: CompileFunctionBodyData<'s>) {
        if self.agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Function ===");
            eprintln!();
        }

        function_declaration_instantiation::instantiation(
            self,
            data.params,
            data.body,
            data.is_strict,
            data.is_lexical,
        );

        // SAFETY: Script referred by the Function uniquely owns the Program
        // and the body buffer does not move under any circumstances during
        // heap operations.
        let body: &[Statement] = unsafe { core::mem::transmute(data.body.statements.as_slice()) };

        self.compile_statements(body);
    }

    pub(super) fn compile_statements(&mut self, body: &'s [Statement<'s>]) {
        let iter = body.iter();

        for stmt in iter {
            stmt.compile(self);
        }
    }

    pub(super) fn do_implicit_return(&mut self) {
        if self.instructions.last() != Some(&Instruction::Return.as_u8()) {
            // If code did not end with a return statement, add it manually
            self.add_instruction(Instruction::Return);
        }
    }

    pub(super) fn finish(self) -> Executable<'gc> {
        self.agent.heap.create(ExecutableHeapData {
            instructions: self.instructions.into_boxed_slice(),
            constants: self.constants.unbind().into_boxed_slice(),
            function_expressions: self.function_expressions.unbind().into_boxed_slice(),
            arrow_function_expressions: self.arrow_function_expressions.into_boxed_slice(),
            class_initializer_bytecodes: self
                .class_initializer_bytecodes
                .into_iter()
                .map(|(exe, b)| (exe.unbind(), b))
                .collect(),
        })
    }

    pub(crate) fn create_identifier(&mut self, atom: &Atom<'_>) -> String<'gc> {
        let existing = self.constants.iter().find_map(|constant| {
            if let Ok(existing_identifier) = String::try_from(*constant) {
                if existing_identifier.as_str(self.agent) == atom.as_str() {
                    Some(existing_identifier)
                } else {
                    None
                }
            } else {
                None
            }
        });
        if let Some(existing) = existing {
            existing
        } else {
            String::from_str(self.agent, atom.as_str(), self.gc)
        }
    }

    fn peek_last_instruction(&self) -> Option<u8> {
        for ele in self.instructions.iter().rev() {
            if *ele == Instruction::ExitDeclarativeEnvironment.as_u8() {
                // Not a "real" instruction
                continue;
            }
            return Some(*ele);
        }
        None
    }

    fn _push_instruction(&mut self, instruction: Instruction) {
        self.instructions
            .push(unsafe { core::mem::transmute::<Instruction, u8>(instruction) });
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
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_jump_slot());
        self._push_instruction(instruction);
        self.add_jump_index()
    }

    fn add_jump_instruction_to_index(&mut self, instruction: Instruction, jump_index: JumpIndex) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_jump_slot());
        self._push_instruction(instruction);
        self.add_double_index(jump_index.index);
    }

    fn get_jump_index_to_here(&self) -> JumpIndex {
        JumpIndex {
            index: self.instructions.len(),
        }
    }

    fn add_constant(&mut self, constant: Value<'gc>) -> usize {
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

    fn add_identifier(&mut self, identifier: String<'gc>) -> usize {
        let duplicate = self
            .constants
            .iter()
            .enumerate()
            .find(|item| String::try_from(*item.1) == Ok(identifier))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.constants.len();
            self.constants.push(identifier.into_value());
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
        constant: impl Into<Value<'gc>>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_constant_index());
        self._push_instruction(instruction);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    fn add_instruction_with_identifier(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_identifier_index());
        self._push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
    }

    fn add_instruction_with_identifier_and_constant(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
        constant: impl Into<Value<'gc>>,
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
        let index = IndexType::try_from(index).expect("Immediate value is too large");
        let bytes: [u8; 2] = index.to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_double_index(&mut self, index: usize) {
        let index = u32::try_from(index).expect("Immediate value is too large");
        let bytes: [u8; 4] = index.to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_instruction_with_function_expression(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.function_expressions.push(function_expression);
        let index = self.function_expressions.len() - 1;
        self.add_index(index);
    }

    /// Add an Instruction that takes a function expression and an immediate
    /// as its bytecode parameters.
    ///
    /// Returns the function expression's index.
    fn add_instruction_with_function_expression_and_immediate(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
        immediate: usize,
    ) -> IndexType {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.function_expressions.push(function_expression);
        let index = self.function_expressions.len() - 1;
        self.add_index(index);
        self.add_index(immediate);
        // Note: add_index would have panicked if this was not a lossless
        // conversion.
        index as IndexType
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
        self.add_double_index(0);
        JumpIndex {
            index: self.instructions.len() - core::mem::size_of::<u32>(),
        }
    }

    fn set_jump_target(&mut self, source: JumpIndex, target: JumpIndex) {
        assert!(target.index < u32::MAX as usize);
        let bytes: [u8; 4] = (target.index as u32).to_ne_bytes();
        self.instructions[source.index..source.index + 4].copy_from_slice(&bytes);
    }

    fn set_jump_target_here(&mut self, jump: JumpIndex) {
        self.set_jump_target(
            jump,
            JumpIndex {
                index: self.instructions.len(),
            },
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct JumpIndex {
    pub(crate) index: usize,
}

pub(crate) trait CompileEvaluation<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>);
}

pub(crate) trait CompileLabelledEvaluation<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    );
}

impl<'a, T: CompileEvaluation<'a>> CompileLabelledEvaluation<'a> for T {
    fn compile_labelled(
        &'a self,
        _label_set: Option<&mut Vec<&'a LabelIdentifier<'a>>>,
        ctx: &mut CompileContext<'_, 'a, '_, '_>,
    ) {
        self.compile(ctx);
    }
}

pub(crate) fn is_reference(expression: &ast::Expression) -> bool {
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

fn is_chain_expression(expression: &ast::Expression) -> bool {
    match expression {
        ast::Expression::ChainExpression(_) => true,
        ast::Expression::ParenthesizedExpression(parenthesized) => {
            is_chain_expression(&parenthesized.expression)
        }
        _ => false,
    }
}

impl<'s> CompileEvaluation<'s> for ast::NumericLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let constant = ctx.agent.heap.create(self.value);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl<'s> CompileEvaluation<'s> for ast::BooleanLiteral {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        ctx.add_instruction_with_constant(Instruction::StoreConstant, self.value);
    }
}

impl<'s> CompileEvaluation<'s> for ast::BigIntLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // Drop out the trailing 'n' from BigInt literals.
        let last_index = self.raw.len() - 1;
        let (big_int_str, radix) = match self.base {
            oxc_syntax::number::BigintBase::Decimal => (&self.raw.as_str()[..last_index], 10),
            oxc_syntax::number::BigintBase::Binary => (&self.raw.as_str()[2..last_index], 2),
            oxc_syntax::number::BigintBase::Octal => (&self.raw.as_str()[2..last_index], 8),
            oxc_syntax::number::BigintBase::Hex => (&self.raw.as_str()[2..last_index], 16),
        };
        let constant = BigInt::from_num_bigint(
            ctx.agent,
            num_bigint::BigInt::from_str_radix(big_int_str, radix).unwrap(),
        );
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl<'s> CompileEvaluation<'s> for ast::NullLiteral {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Null);
    }
}

impl<'s> CompileEvaluation<'s> for ast::StringLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let constant = String::from_str(ctx.agent, self.value.as_str(), ctx.gc);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl<'s> CompileEvaluation<'s> for ast::IdentifierReference<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let identifier = String::from_str(ctx.agent, self.name.as_str(), ctx.gc);
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier);
    }
}

impl<'s> CompileEvaluation<'s> for ast::BindingIdentifier<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let identifier = String::from_str(ctx.agent, self.name.as_str(), ctx.gc);
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier);
    }
}

impl<'s> CompileEvaluation<'s> for ast::UnaryExpression<'s> {
    /// ### ['a 13.5 Unary Operators](https://tc39.es/ecma262/#sec-unary-operators)
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match self.operator {
            // 13.5.5 Unary - Operator
            // https://tc39.es/ecma262/#sec-unary-minus-operator-runtime-semantics-evaluation
            // UnaryExpression : - UnaryExpression
            UnaryOperator::UnaryNegation => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);

                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                if is_reference(&self.argument) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::ToNumeric);

                // 3. If oldValue is a Number, then
                //    a. Return Number::unaryMinus(oldValue).
                // 4. Else,
                //    a. Assert: oldValue is a BigInt.
                //    b. Return BigInt::unaryMinus(oldValue).
                ctx.add_instruction(Instruction::UnaryMinus);
            }
            // 13.5.4 Unary + Operator
            // https://tc39.es/ecma262/#sec-unary-plus-operator
            // UnaryExpression : + UnaryExpression
            UnaryOperator::UnaryPlus => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);

                // 2. Return ? ToNumber(? GetValue(expr)).
                if is_reference(&self.argument) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::ToNumber);
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
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::LogicalNot);
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
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::ToNumeric);
                ctx.add_instruction(Instruction::BitwiseNot);
            }
            // 13.5.3 The typeof Operator
            // UnaryExpression : typeof UnaryExpression
            UnaryOperator::Typeof => {
                // 1. Let val be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);
                // 3. Set val to ? GetValue(val).
                ctx.add_instruction(Instruction::Typeof);
            }
            // 13.5.2 The void operator
            // UnaryExpression : void UnaryExpression
            UnaryOperator::Void => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);
                // NOTE: GetValue must be called even though its value is not used because it may have observable side-effects.
                // 2. Perform ? GetValue(expr).
                if is_reference(&self.argument) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                // 3. Return undefined.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            }
            // 13.5.1 The delete operator
            // https://tc39.es/ecma262/#sec-delete-operator-runtime-semantics-evaluation
            // UnaryExpression : delete UnaryExpression
            UnaryOperator::Delete => {
                // Let ref be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx);
                // 2. If ref is not a Reference Record, return true.
                if !is_reference(&self.argument) {
                    ctx.add_instruction_with_constant(Instruction::StoreConstant, true);
                    return;
                }
                ctx.add_instruction(Instruction::Delete);
            }
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::BinaryExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let lref be ? Evaluation of leftOperand.
        self.left.compile(ctx);

        // 2. Let lval be ? GetValue(lref).
        if is_reference(&self.left) {
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::Load);

        // 3. Let rref be ? Evaluation of rightOperand.
        self.right.compile(ctx);

        // 4. Let rval be ? GetValue(rref).
        if is_reference(&self.right) {
            ctx.add_instruction(Instruction::GetValue);
        }

        match self.operator {
            BinaryOperator::LessThan => {
                ctx.add_instruction(Instruction::LessThan);
            }
            BinaryOperator::LessEqualThan => {
                ctx.add_instruction(Instruction::LessThanEquals);
            }
            BinaryOperator::GreaterThan => {
                ctx.add_instruction(Instruction::GreaterThan);
            }
            BinaryOperator::GreaterEqualThan => {
                ctx.add_instruction(Instruction::GreaterThanEquals);
            }
            BinaryOperator::StrictEquality => {
                ctx.add_instruction(Instruction::IsStrictlyEqual);
            }
            BinaryOperator::StrictInequality => {
                ctx.add_instruction(Instruction::IsStrictlyEqual);
                ctx.add_instruction(Instruction::LogicalNot);
            }
            BinaryOperator::Equality => {
                ctx.add_instruction(Instruction::IsLooselyEqual);
            }
            BinaryOperator::Inequality => {
                ctx.add_instruction(Instruction::IsLooselyEqual);
                ctx.add_instruction(Instruction::LogicalNot);
            }
            BinaryOperator::In => {
                ctx.add_instruction(Instruction::HasProperty);
            }
            BinaryOperator::Instanceof => {
                ctx.add_instruction(Instruction::InstanceofOperator);
            }
            _ => {
                // 5. Return ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
                ctx.add_instruction(Instruction::ApplyStringOrNumericBinaryOperator(
                    self.operator,
                ));
            }
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::LogicalExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        self.left.compile(ctx);
        if is_reference(&self.left) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // We store the left value on the stack, because we'll need to restore
        // it later.
        ctx.add_instruction(Instruction::LoadCopy);

        match self.operator {
            oxc_syntax::operator::LogicalOperator::Or => {
                ctx.add_instruction(Instruction::LogicalNot);
            }
            oxc_syntax::operator::LogicalOperator::And => {}
            oxc_syntax::operator::LogicalOperator::Coalesce => {
                ctx.add_instruction(Instruction::IsNullOrUndefined);
            }
        }
        let jump_to_return_left = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);

        // We're returning the right expression, so we discard the left value
        // at the top of the stack.
        ctx.add_instruction(Instruction::Store);

        self.right.compile(ctx);
        if is_reference(&self.right) {
            ctx.add_instruction(Instruction::GetValue);
        }
        let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::Jump);

        ctx.set_jump_target_here(jump_to_return_left);
        // Return the result of the left expression.
        ctx.add_instruction(Instruction::Store);
        ctx.set_jump_target_here(jump_to_end);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ParenthesizedExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        self.expression.compile(ctx);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ArrowFunctionExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // CompileContext holds a name identifier for us if this is NamedEvaluation.
        let identifier = ctx.name_identifier.take();
        ctx.add_arrow_function_expression(ArrowFunctionExpression {
            expression: SendableRef::new(unsafe {
                core::mem::transmute::<
                    &ast::ArrowFunctionExpression<'_>,
                    &'static ast::ArrowFunctionExpression<'static>,
                >(self)
            }),
            identifier,
        });
    }
}

impl<'s> CompileEvaluation<'s> for ast::Function<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // CompileContext holds a name identifier for us if this is NamedEvaluation.
        let identifier = ctx.name_identifier.take();
        ctx.add_instruction_with_function_expression(
            Instruction::InstantiateOrdinaryFunctionExpression,
            FunctionExpression {
                expression: SendableRef::new(unsafe {
                    core::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                        self,
                    )
                }),
                identifier,
                compiled_bytecode: None,
            },
        );
    }
}

impl<'s> CompileEvaluation<'s> for ast::ObjectExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // TODO: Consider preparing the properties onto the stack and creating
        // the object with a known size.
        ctx.add_instruction(Instruction::ObjectCreate);
        for property in self.properties.iter() {
            match property {
                ast::ObjectPropertyKind::ObjectProperty(prop) => {
                    let mut is_proto_setter = false;
                    match &prop.key {
                        ast::PropertyKey::ArrayExpression(init) => init.compile(ctx),
                        ast::PropertyKey::ArrowFunctionExpression(init) => init.compile(ctx),
                        ast::PropertyKey::AssignmentExpression(init) => init.compile(ctx),
                        ast::PropertyKey::AwaitExpression(init) => init.compile(ctx),
                        ast::PropertyKey::BigIntLiteral(init) => init.compile(ctx),
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
                        #[cfg(feature = "regexp")]
                        ast::PropertyKey::RegExpLiteral(init) => init.compile(ctx),
                        #[cfg(not(feature = "regexp"))]
                        ast::PropertyKey::RegExpLiteral(_) => todo!(),
                        ast::PropertyKey::SequenceExpression(init) => init.compile(ctx),
                        ast::PropertyKey::StaticIdentifier(id) => {
                            if id.name == "__proto__" {
                                if prop.kind == ast::PropertyKind::Init && !prop.shorthand {
                                    // If property key is "__proto__" then we
                                    // should dispatch a SetPrototype instruction.
                                    is_proto_setter = true;
                                } else {
                                    ctx.add_instruction_with_constant(
                                        Instruction::StoreConstant,
                                        BUILTIN_STRING_MEMORY.__proto__,
                                    );
                                }
                            } else {
                                let identifier = PropertyKey::from_str(ctx.agent, &id.name, ctx.gc);
                                ctx.add_instruction_with_constant(
                                    Instruction::StoreConstant,
                                    identifier,
                                );
                            }
                        }
                        ast::PropertyKey::StaticMemberExpression(init) => init.compile(ctx),
                        ast::PropertyKey::StringLiteral(init) => {
                            let identifier = PropertyKey::from_str(ctx.agent, &init.value, ctx.gc);
                            ctx.add_instruction_with_constant(
                                Instruction::StoreConstant,
                                identifier,
                            );
                        }
                        ast::PropertyKey::Super(_) => unreachable!(),
                        ast::PropertyKey::TaggedTemplateExpression(init) => init.compile(ctx),
                        ast::PropertyKey::TemplateLiteral(init) => init.compile(ctx),
                        ast::PropertyKey::ThisExpression(init) => init.compile(ctx),
                        ast::PropertyKey::UnaryExpression(init) => init.compile(ctx),
                        ast::PropertyKey::UpdateExpression(init) => init.compile(ctx),
                        ast::PropertyKey::YieldExpression(init) => init.compile(ctx),
                        // TODO: Implement this expression.
                        ast::PropertyKey::V8IntrinsicExpression(_) => todo!(),
                        ast::PropertyKey::JSXElement(_)
                        | ast::PropertyKey::JSXFragment(_)
                        | ast::PropertyKey::TSAsExpression(_)
                        | ast::PropertyKey::TSSatisfiesExpression(_)
                        | ast::PropertyKey::TSTypeAssertion(_)
                        | ast::PropertyKey::TSNonNullExpression(_)
                        | ast::PropertyKey::TSInstantiationExpression(_) => unreachable!(),
                    }
                    if let Some(prop_key_expression) = prop.key.as_expression() {
                        if is_reference(prop_key_expression) {
                            assert!(!is_proto_setter);
                            ctx.add_instruction(Instruction::GetValue);
                        }
                    }
                    if !is_proto_setter {
                        // Prototype setter doesn't need the key.
                        ctx.add_instruction(Instruction::Load);
                    }
                    match prop.kind {
                        ast::PropertyKind::Init => {
                            if !is_proto_setter && is_anonymous_function_definition(&prop.value) {
                                ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
                            }
                            prop.value.compile(ctx);
                            if is_reference(&prop.value) {
                                ctx.add_instruction(Instruction::GetValue);
                            }
                            // 7. If isProtoSetter is true, then
                            if is_proto_setter {
                                // a. If propValue is an Object or propValue is null, then
                                //     i. Perform ! object.[[SetPrototypeOf]](propValue).
                                // b. Return unused.
                                ctx.add_instruction(Instruction::ObjectSetPrototype);
                            } else {
                                ctx.add_instruction(Instruction::ObjectDefineProperty);
                            }
                        }
                        ast::PropertyKind::Get | ast::PropertyKind::Set => {
                            let is_get = prop.kind == ast::PropertyKind::Get;
                            let ast::Expression::FunctionExpression(function_expression) =
                                &prop.value
                            else {
                                unreachable!()
                            };
                            ctx.add_instruction_with_function_expression_and_immediate(
                                if is_get {
                                    Instruction::ObjectDefineGetter
                                } else {
                                    Instruction::ObjectDefineSetter
                                },
                                FunctionExpression {
                                    expression: SendableRef::new(unsafe {
                                        core::mem::transmute::<
                                            &ast::Function<'_>,
                                            &'static ast::Function<'static>,
                                        >(
                                            function_expression
                                        )
                                    }),
                                    identifier: None,
                                    compiled_bytecode: None,
                                },
                                // enumerable: true,
                                true.into(),
                            );
                        }
                    }
                }
                ast::ObjectPropertyKind::SpreadProperty(spread) => {
                    spread.argument.compile(ctx);
                    if is_reference(&spread.argument) {
                        ctx.add_instruction(Instruction::GetValue);
                    }
                    ctx.add_instruction(Instruction::CopyDataProperties);
                }
            }
        }
        // 3. Return obj
        ctx.add_instruction(Instruction::Store);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ArrayExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let elements_min_count = self.elements.len();
        ctx.add_instruction_with_immediate(Instruction::ArrayCreate, elements_min_count);
        for ele in &self.elements {
            match ele {
                ast::ArrayExpressionElement::SpreadElement(spread) => {
                    spread.argument.compile(ctx);
                    if is_reference(&spread.argument) {
                        ctx.add_instruction(Instruction::GetValue);
                    }
                    ctx.add_instruction(Instruction::GetIteratorSync);

                    let iteration_start = ctx.get_jump_index_to_here();
                    let iteration_end =
                        ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue);
                    ctx.add_instruction(Instruction::ArrayPush);
                    ctx.add_jump_instruction_to_index(Instruction::Jump, iteration_start);
                    ctx.set_jump_target_here(iteration_end);
                }
                ast::ArrayExpressionElement::Elision(_) => {
                    ctx.add_instruction(Instruction::ArrayElision);
                }
                _ => {
                    let expression = ele.to_expression();
                    expression.compile(ctx);
                    if is_reference(expression) {
                        ctx.add_instruction(Instruction::GetValue);
                    }
                    ctx.add_instruction(Instruction::ArrayPush);
                }
            }
        }
        ctx.add_instruction(Instruction::Store);
    }
}

fn compile_arguments<'s>(
    arguments: &'s [ast::Argument<'s>],
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) -> usize {
    // If the arguments don't contain the spread operator, then we can know the
    // number of arguments at compile-time and we can pass it as an argument to
    // the call instruction.
    // Otherwise, the first time we find a spread operator, we need to start
    // tracking the number of arguments in the compiled bytecode. We store this
    // number in the result value, and we pass u16::MAX to the call instruction.
    let mut known_num_arguments = Some(0 as IndexType);

    for argument in arguments {
        // If known_num_arguments is None, the stack contains the number of
        // arguments, followed by the arguments.
        if let ast::Argument::SpreadElement(spread) = argument {
            if let Some(num_arguments) = known_num_arguments.take() {
                ctx.add_instruction_with_constant(Instruction::LoadConstant, num_arguments);
            }

            spread.argument.compile(ctx);
            if is_reference(&spread.argument) {
                ctx.add_instruction(Instruction::GetValue);
            }
            ctx.add_instruction(Instruction::GetIteratorSync);

            let iteration_start = ctx.get_jump_index_to_here();
            let iteration_end = ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue);
            // result: value; stack: [num, ...args]
            ctx.add_instruction(Instruction::LoadStoreSwap);
            // result: num; stack: [value, ...args]
            ctx.add_instruction(Instruction::Increment);
            // result: num + 1; stack: [value, ...args]
            ctx.add_instruction(Instruction::Load);
            // stack: [num + 1, value, ...args]
            ctx.add_jump_instruction_to_index(Instruction::Jump, iteration_start);
            ctx.set_jump_target_here(iteration_end);
        } else {
            let expression = argument.to_expression();
            expression.compile(ctx);
            if is_reference(expression) {
                ctx.add_instruction(Instruction::GetValue);
            }
            if let Some(num_arguments) = known_num_arguments.as_mut() {
                ctx.add_instruction(Instruction::Load);
                // stack: [value, ...args]

                if *num_arguments < IndexType::MAX - 1 {
                    *num_arguments += 1;
                } else {
                    // If we overflow, we switch to tracking the number on the
                    // result value.
                    debug_assert_eq!(*num_arguments, IndexType::MAX - 1);
                    known_num_arguments = None;
                    ctx.add_instruction_with_constant(
                        Instruction::LoadConstant,
                        Value::from(IndexType::MAX),
                    );
                    // stack: [num + 1, value, ...args]
                }
            } else {
                // result: value; stack: [num, ...args]
                ctx.add_instruction(Instruction::LoadStoreSwap);
                // result: num; stack: [value, ...args]
                ctx.add_instruction(Instruction::Increment);
                // result: num + 1; stack: [value, ...args]
                ctx.add_instruction(Instruction::Load);
                // stack: [num + 1, value, ...args]
            }
        }
    }

    if let Some(num_arguments) = known_num_arguments {
        assert_ne!(num_arguments, IndexType::MAX);
        num_arguments as usize
    } else {
        // stack: [num, ...args]
        ctx.add_instruction(Instruction::Store);
        // result: num; stack: [...args]
        IndexType::MAX as usize
    }
}

impl<'s> CompileEvaluation<'s> for CallExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // Direct eval
        if !self.optional {
            if let ast::Expression::Identifier(ident) = &self.callee {
                if ident.name == "eval" {
                    let num_arguments = compile_arguments(&self.arguments, ctx);
                    ctx.add_instruction_with_immediate(Instruction::DirectEvalCall, num_arguments);
                    return;
                }
            }
        }

        // 1. Let ref be ? Evaluation of CallExpression.
        ctx.is_call_optional_chain_this = is_chain_expression(&self.callee);
        let is_super_call = matches!(self.callee, ast::Expression::Super(_));
        let need_pop_reference = if is_super_call {
            // Note: There is nothing to do with super calls here.
            false
        } else {
            self.callee.compile(ctx);
            if is_reference(&self.callee) {
                // 2. Let func be ? GetValue(ref).
                ctx.add_instruction(Instruction::GetValueKeepReference);
                // Optimization: If we know arguments is empty, we don't need to
                // worry about arguments evaluation clobbering our function's this
                // reference.
                if !self.arguments.is_empty() {
                    ctx.add_instruction(Instruction::PushReference);
                    true
                } else {
                    false
                }
            } else {
                false
            }
        };

        if self.optional {
            // Optional Chains

            // Load copy of func to stack.
            ctx.add_instruction(Instruction::LoadCopy);
            // 3. If func is either undefined or null, then
            ctx.add_instruction(Instruction::IsNullOrUndefined);
            // a. Return undefined

            // To return undefined we jump over the rest of the call handling.
            let jump_over_call = if need_pop_reference {
                // If we need to pop the reference stack, then we must do it
                // here before we go to the nullish case handling.
                // Note the inverted jump condition here!
                let jump_to_call = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                // Now we're in our local nullish case handling.
                // First we pop our reference.
                ctx.add_instruction(Instruction::PopReference);
                // And now we're ready to jump over the call.
                let jump_over_call = ctx.add_instruction_with_jump_slot(Instruction::Jump);
                // But if we're jumping to call then we need to land here.
                ctx.set_jump_target_here(jump_to_call);
                jump_over_call
            } else {
                ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue)
            };
            // Register our jump slot to the chain nullish case handling.
            ctx.optional_chains.as_mut().unwrap().push(jump_over_call);
        } else if !is_super_call {
            ctx.add_instruction(Instruction::Load);
        }
        // If we're in an optional chain, we need to pluck it out while we're
        // compiling the parameters: They do not join our chain.
        let optional_chain = ctx.optional_chains.take();
        let num_arguments = compile_arguments(&self.arguments, ctx);
        // After we're done with compiling parameters we go back into the chain.
        if let Some(optional_chain) = optional_chain {
            ctx.optional_chains.replace(optional_chain);
        }

        if is_super_call {
            ctx.add_instruction_with_immediate(Instruction::EvaluateSuper, num_arguments);
        } else {
            if need_pop_reference {
                ctx.add_instruction(Instruction::PopReference);
            }
            ctx.add_instruction_with_immediate(Instruction::EvaluateCall, num_arguments);
        }
    }
}

impl<'s> CompileEvaluation<'s> for NewExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        self.callee.compile(ctx);
        if is_reference(&self.callee) {
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::Load);

        let num_arguments = compile_arguments(&self.arguments, ctx);
        ctx.add_instruction_with_immediate(Instruction::EvaluateNew, num_arguments);
    }
}

impl<'s> CompileEvaluation<'s> for ast::MemberExpression<'s> {
    /// ### ['a 13.3.2 Property Accessors](https://tc39.es/ecma262/#sec-property-accessors)
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match self {
            ast::MemberExpression::ComputedMemberExpression(x) => x.compile(ctx),
            ast::MemberExpression::StaticMemberExpression(x) => x.compile(ctx),
            ast::MemberExpression::PrivateFieldExpression(x) => x.compile(ctx),
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::ComputedMemberExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let baseReference be ? Evaluation of MemberExpression.
        self.object.compile(ctx);

        // 2. Let baseValue be ? GetValue(baseReference).
        if is_reference(&self.object) {
            ctx.add_instruction(Instruction::GetValue);
        }

        if self.optional {
            // Optional Chains

            // Load copy of baseValue to stack.
            ctx.add_instruction(Instruction::LoadCopy);
            // 3. If baseValue is either undefined or null, then
            ctx.add_instruction(Instruction::IsNullOrUndefined);
            // a. Return undefined

            // To return undefined we jump over the property access.
            let jump_over_property_access =
                ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

            // Register our jump slot to the chain nullish case handling.
            ctx.optional_chains
                .as_mut()
                .unwrap()
                .push(jump_over_property_access);
        } else {
            ctx.add_instruction(Instruction::Load);
        }

        // If we're in an optional chain, we need to pluck it out while we're
        // compiling the member expression: They do not join our chain.
        let optional_chain = ctx.optional_chains.take();
        // 4. Return ? EvaluatePropertyAccessWithExpressionKey(baseValue, Expression, strict).
        self.expression.compile(ctx);
        if is_reference(&self.expression) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // After we're done with compiling the member expression we go back
        // into the chain.
        if let Some(optional_chain) = optional_chain {
            ctx.optional_chains.replace(optional_chain);
        }

        ctx.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
    }
}

impl<'s> CompileEvaluation<'s> for ast::StaticMemberExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let baseReference be ? Evaluation of MemberExpression.
        self.object.compile(ctx);

        // 2. Let baseValue be ? GetValue(baseReference).
        if is_reference(&self.object) {
            ctx.add_instruction(Instruction::GetValue);
        }

        if self.optional {
            // Optional Chains

            // Load copy of baseValue to stack.
            ctx.add_instruction(Instruction::LoadCopy);
            // 3. If baseValue is either undefined or null, then
            ctx.add_instruction(Instruction::IsNullOrUndefined);
            // a. Return undefined

            // To return undefined we jump over the property access.
            let jump_over_property_access =
                ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

            // Register our jump slot to the chain nullish case handling.
            ctx.optional_chains
                .as_mut()
                .unwrap()
                .push(jump_over_property_access);

            // Return copy of baseValue from stack if it is not.
            ctx.add_instruction(Instruction::Store);
        }

        // 4. Return EvaluatePropertyAccessWithIdentifierKey(baseValue, IdentifierName, strict).
        let identifier = String::from_str(ctx.agent, self.property.name.as_str(), ctx.gc);
        ctx.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            identifier,
        );
    }
}

impl<'s> CompileEvaluation<'s> for ast::PrivateFieldExpression<'s> {
    fn compile(&'s self, _ctx: &mut CompileContext<'_, 's, '_, '_>) {
        todo!()
    }
}

impl<'s> CompileEvaluation<'s> for ast::AwaitExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let exprRef be ? Evaluation of UnaryExpression.
        self.argument.compile(ctx);
        // 2. Let value be ? GetValue(exprRef).
        if is_reference(&self.argument) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // 3. Return ? Await(value).
        ctx.add_instruction(Instruction::Await);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ChainExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // It's possible that we're compiling a ChainExpression inside a call
        // that is itself in a ChainExpression. We will drop into the previous
        // chain in this case.
        let installed_own_chains = if ctx.optional_chains.is_none() {
            // We prepare for at least two chains to exist. One chain is often
            // enough but two is a bit safer. Three is rare.
            ctx.optional_chains.replace(Vec::with_capacity(2));
            true
        } else {
            false
        };
        let need_get_value = match self.expression {
            ast::ChainElement::CallExpression(ref call) => {
                call.compile(ctx);
                false
            }
            ast::ChainElement::ComputedMemberExpression(ref call) => {
                call.compile(ctx);
                true
            }
            ast::ChainElement::StaticMemberExpression(ref call) => {
                call.compile(ctx);
                true
            }
            ast::ChainElement::PrivateFieldExpression(ref call) => {
                call.compile(ctx);
                true
            }
            ast::ChainElement::TSNonNullExpression(ref _call) => false,
        };
        // If chain succeeded, we come here and should jump over the nullish
        // case handling.
        if need_get_value {
            // If we handled a member or field expression, we need to get its
            // value. However, there's a chance that we cannot just throw away
            // the reference. If the result of the chain expression is going to
            // be used in a (potentially optional) call expression then we need
            // both its value and its reference.
            if ctx.is_call_optional_chain_this {
                ctx.is_call_optional_chain_this = false;
                ctx.add_instruction(Instruction::GetValueKeepReference);
            } else {
                ctx.add_instruction(Instruction::GetValue);
            }
        }
        if installed_own_chains {
            let jump_over_return_undefined = ctx.add_instruction_with_jump_slot(Instruction::Jump);
            let own_chains = ctx.optional_chains.take().unwrap();
            for jump_to_return_undefined in own_chains {
                ctx.set_jump_target_here(jump_to_return_undefined);
            }
            // All optional chains come here with a copy of their null or
            // undefined baseValue on the stack. Pop it off.
            ctx.add_instruction(Instruction::Store);
            // Replace any possible null with undefined.
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            ctx.set_jump_target_here(jump_over_return_undefined);
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::ConditionalExpression<'s> {
    /// ## ['a 13.14 Conditional Operator ( ? : )](https://tc39.es/ecma262/#sec-conditional-operator)
    /// ### [13.14.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-conditional-operator-runtime-semantics-evaluation)
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let lref be ? Evaluation of ShortCircuitExpression.
        self.test.compile(ctx);
        // 2. Let lval be ToBoolean(? GetValue(lref)).
        if is_reference(&self.test) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // Jump over first AssignmentExpression (consequent) if test fails.
        // Note: JumpIfNot performs ToBoolean from above step.
        let jump_to_second = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
        // 3. If lval is true, then
        // a. Let trueRef be ? Evaluation of the first AssignmentExpression.
        self.consequent.compile(ctx);
        // b. Return ? GetValue(trueRef).
        if is_reference(&self.consequent) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // Jump over second AssignmentExpression (alternate).
        let jump_over_second = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        // 4. Else,
        ctx.set_jump_target_here(jump_to_second);
        // a. Let falseRef be ? Evaluation of the second AssignmentExpression.
        self.alternate.compile(ctx);
        // b. Return ? GetValue(falseRef).
        if is_reference(&self.alternate) {
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.set_jump_target_here(jump_over_second);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ImportExpression<'s> {
    fn compile(&'s self, _ctx: &mut CompileContext<'_, 's, '_, '_>) {
        todo!()
    }
}

impl<'s> CompileEvaluation<'s> for ast::MetaProperty<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.meta.name == "new" && self.property.name == "target" {
            ctx.add_instruction(Instruction::GetNewTarget);
        } else {
            todo!();
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::PrivateInExpression<'s> {
    fn compile(&'s self, _ctx: &mut CompileContext<'_, 's, '_, '_>) {
        todo!()
    }
}
#[cfg(feature = "regexp")]
impl<'s> CompileEvaluation<'s> for ast::RegExpLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let pattern = match self.regex.pattern {
            ast::RegExpPattern::Raw(pattern) => pattern,
            ast::RegExpPattern::Invalid(pattern) => pattern,
            // We probably shouldn't be getting parsed RegExps?
            ast::RegExpPattern::Pattern(_) => unreachable!(),
        };
        let pattern = String::from_str(ctx.agent, pattern, ctx.gc);
        let regexp = reg_exp_create_literal(ctx.agent, pattern, Some(self.regex.flags), ctx.gc);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, regexp);
    }
}

impl<'s> CompileEvaluation<'s> for ast::SequenceExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        for expr in &self.expressions {
            expr.compile(ctx);
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::Super {
    fn compile(&'s self, _ctx: &mut CompileContext<'_, 's, '_, '_>) {
        todo!()
    }
}

impl<'s> CompileEvaluation<'s> for ast::TaggedTemplateExpression<'s> {
    fn compile(&'s self, _ctx: &mut CompileContext<'_, 's, '_, '_>) {
        todo!()
    }
}

impl<'s> CompileEvaluation<'s> for ast::TemplateLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.is_no_substitution_template() {
            let constant = String::from_str(
                ctx.agent,
                self.quasi()
                    .as_ref()
                    .expect("Invalid escape sequence in template literal")
                    .as_str(),
                ctx.gc,
            );
            ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
        } else {
            let mut count = 0;
            let mut quasis = self.quasis.as_slice();
            let mut expressions = self.expressions.as_slice();
            while let Some((head, rest)) = quasis.split_first() {
                quasis = rest;
                // 1. Let head be the TV of TemplateHead as defined in 12.9.6.
                let head = String::from_str(
                    ctx.agent,
                    head.value.cooked.as_ref().unwrap().as_str(),
                    ctx.gc,
                );
                ctx.add_instruction_with_constant(Instruction::LoadConstant, head);
                count += 1;
                if let Some((expression, rest)) = expressions.split_first() {
                    expressions = rest;
                    // 2. Let subRef be ? Evaluation of Expression.
                    expression.compile(ctx);
                    if is_reference(expression) {
                        // 3. Let sub be ? GetValue(subRef).
                        ctx.add_instruction(Instruction::GetValue);
                    }
                    // 4. Let middle be ? ToString(sub).
                    // Note: This is done by StringConcat.
                    ctx.add_instruction(Instruction::Load);
                    count += 1;
                }
                // 5. Let tail be ? Evaluation of TemplateSpans.
            }
            // 6. Return the string-concatenation of head, middle, and tail.
            ctx.add_instruction_with_immediate(Instruction::StringConcat, count);
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::ThisExpression {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        ctx.add_instruction(Instruction::ResolveThisBinding);
    }
}

impl<'s> CompileEvaluation<'s> for ast::YieldExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.delegate {
            todo!("`yield*` is not yet supported");
        }
        if let Some(arg) = &self.argument {
            // YieldExpression : yield AssignmentExpression
            // 1. Let exprRef be ? Evaluation of AssignmentExpression.
            arg.compile(ctx);
            // 2. Let value be ? GetValue(exprRef).
            if is_reference(arg) {
                ctx.add_instruction(Instruction::GetValue);
            }
        } else {
            // YieldExpression : yield
            // 1. Return ? Yield(undefined).
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        // 3. Return ? Yield(value).
        ctx.add_instruction(Instruction::Yield);
    }
}

impl<'s> CompileEvaluation<'s> for ast::Expression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match self {
            ast::Expression::ArrayExpression(x) => x.compile(ctx),
            ast::Expression::ArrowFunctionExpression(x) => x.compile(ctx),
            ast::Expression::AssignmentExpression(x) => x.compile(ctx),
            ast::Expression::AwaitExpression(x) => x.compile(ctx),
            ast::Expression::BigIntLiteral(x) => x.compile(ctx),
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
            #[cfg(feature = "regexp")]
            ast::Expression::RegExpLiteral(x) => x.compile(ctx),
            #[cfg(not(feature = "regexp"))]
            ast::Expression::RegExpLiteral(_) => unreachable!(),
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
            // TODO: Implement this expression.
            ast::Expression::V8IntrinsicExpression(_) => todo!(),
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

impl<'s> CompileEvaluation<'s> for ast::UpdateExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match &self.argument {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::ComputedMemberExpression(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::PrivateFieldExpression(_) => todo!(),
            ast::SimpleAssignmentTarget::StaticMemberExpression(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::TSAsExpression(_)
            | ast::SimpleAssignmentTarget::TSNonNullExpression(_)
            | ast::SimpleAssignmentTarget::TSSatisfiesExpression(_)
            | ast::SimpleAssignmentTarget::TSTypeAssertion(_) => unreachable!(),
        }
        ctx.add_instruction(Instruction::GetValueKeepReference);
        ctx.add_instruction(Instruction::ToNumeric);
        if !self.prefix {
            // The return value of postfix increment/decrement is the value
            // after ToNumeric.
            ctx.add_instruction(Instruction::LoadCopy);
        }
        match self.operator {
            oxc_syntax::operator::UpdateOperator::Increment => {
                ctx.add_instruction(Instruction::Increment);
            }
            oxc_syntax::operator::UpdateOperator::Decrement => {
                ctx.add_instruction(Instruction::Decrement);
            }
        }
        if self.prefix {
            ctx.add_instruction(Instruction::LoadCopy);
        }
        ctx.add_instruction(Instruction::PutValue);
        ctx.add_instruction(Instruction::Store);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ExpressionStatement<'s> {
    /// ### ['a 14.5.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-expression-statement-runtime-semantics-evaluation)
    /// `ExpressionStatement : Expression ;`
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let exprRef be ? Evaluation of Expression.
        self.expression.compile(ctx);
        if is_reference(&self.expression) {
            // 2. Return ? GetValue(exprRef).
            ctx.add_instruction(Instruction::GetValue);
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::ReturnStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if let Some(expr) = &self.argument {
            expr.compile(ctx);
            if is_reference(expr) {
                ctx.add_instruction(Instruction::GetValue);
            }
        } else {
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        ctx.add_instruction(Instruction::Return);
    }
}

impl<'s> CompileEvaluation<'s> for ast::IfStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // if (test) consequent
        self.test.compile(ctx);
        if is_reference(&self.test) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // jump over consequent if test fails
        let jump_to_else = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
        self.consequent.compile(ctx);
        let mut jump_over_else: Option<JumpIndex> = None;
        if let Some(alternate) = &self.alternate {
            // Optimisation: If the an else branch exists, the consequent
            // branch needs to end in a jump over it. But if the consequent
            // branch ends in a return statement that jump becomes unnecessary.
            if ctx.peek_last_instruction() != Some(Instruction::Return.as_u8()) {
                jump_over_else = Some(ctx.add_instruction_with_jump_slot(Instruction::Jump));
            }

            // Jump to else-branch when if test fails.
            ctx.set_jump_target_here(jump_to_else);
            alternate.compile(ctx);
        } else {
            // Jump over if-branch when if test fails.
            ctx.set_jump_target_here(jump_to_else);
        }

        // Jump over else-branch at the end of if-branch if necessary.
        // (See optimisation above for when it is not needed.)
        if let Some(jump_over_else) = jump_over_else {
            ctx.set_jump_target_here(jump_over_else);
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::ArrayPattern<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.elements.is_empty() && self.rest.is_none() {
            return;
        }

        ctx.add_instruction(Instruction::Store);
        ctx.add_instruction(Instruction::GetIteratorSync);

        if !self.contains_expression() {
            simple_array_pattern(
                ctx,
                self.elements.iter().map(Option::as_ref),
                self.rest.as_deref(),
                self.elements.len(),
                ctx.lexical_binding_state,
            );
        } else {
            complex_array_pattern(
                ctx,
                self.elements.iter().map(Option::as_ref),
                self.rest.as_deref(),
                ctx.lexical_binding_state,
            );
        }
    }
}

fn simple_array_pattern<'s, I>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    elements: I,
    rest: Option<&'s BindingRestElement<'s>>,
    num_elements: usize,
    has_environment: bool,
) where
    I: Iterator<Item = Option<&'s BindingPattern<'s>>>,
{
    ctx.lexical_binding_state = has_environment;
    ctx.add_instruction_with_immediate_and_immediate(
        Instruction::BeginSimpleArrayBindingPattern,
        num_elements,
        has_environment.into(),
    );

    for ele in elements {
        let Some(ele) = ele else {
            ctx.add_instruction(Instruction::BindingPatternSkip);
            continue;
        };
        match &ele.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(
                    Instruction::BindingPatternBind,
                    identifier_string,
                )
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetValue);
                simple_object_pattern(pattern, ctx, has_environment);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetValue);
                simple_array_pattern(
                    ctx,
                    pattern.elements.iter().map(Option::as_ref),
                    pattern.rest.as_deref(),
                    pattern.elements.len(),
                    has_environment,
                );
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    }

    if let Some(rest) = rest {
        match &rest.argument.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(
                    Instruction::BindingPatternBindRest,
                    identifier_string,
                );
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetRestValue);
                simple_object_pattern(pattern, ctx, has_environment);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetRestValue);
                simple_array_pattern(
                    ctx,
                    pattern.elements.iter().map(Option::as_ref),
                    pattern.rest.as_deref(),
                    pattern.elements.len(),
                    has_environment,
                );
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    } else {
        ctx.add_instruction(Instruction::FinishBindingPattern);
    }
}

fn complex_array_pattern<'s, I>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    elements: I,
    rest: Option<&'s BindingRestElement<'s>>,
    has_environment: bool,
) where
    I: Iterator<Item = Option<&'s BindingPattern<'s>>>,
{
    ctx.lexical_binding_state = has_environment;
    for ele in elements {
        ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);

        let Some(ele) = ele else {
            continue;
        };

        let binding_pattern = match &ele.kind {
            ast::BindingPatternKind::AssignmentPattern(pattern) => {
                // Run the initializer if the result value is undefined.
                ctx.add_instruction(Instruction::LoadCopy);
                ctx.add_instruction(Instruction::IsUndefined);
                let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                ctx.add_instruction(Instruction::Store);
                if is_anonymous_function_definition(&pattern.right) {
                    if let ast::BindingPatternKind::BindingIdentifier(identifier) =
                        &pattern.left.kind
                    {
                        let identifier_string = ctx.create_identifier(&identifier.name);
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            identifier_string,
                        );
                        ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                    }
                }
                pattern.right.compile(ctx);
                ctx.name_identifier = None;
                if is_reference(&pattern.right) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::Load);
                ctx.set_jump_target_here(jump_slot);
                ctx.add_instruction(Instruction::Store);

                &pattern.left.kind
            }
            _ => &ele.kind,
        };

        match binding_pattern {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier_string);
                if !has_environment {
                    ctx.add_instruction(Instruction::PutValue);
                } else {
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                }
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::Load);
                pattern.compile(ctx);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::Load);
                pattern.compile(ctx);
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    }

    if let Some(rest) = rest {
        ctx.add_instruction(Instruction::IteratorRestIntoArray);
        match &rest.argument.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier_string);
                if !has_environment {
                    ctx.add_instruction(Instruction::PutValue);
                } else {
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                }
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::Load);
                pattern.compile(ctx);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::Load);
                pattern.compile(ctx);
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    } else {
        ctx.add_instruction(Instruction::IteratorClose);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ObjectPattern<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if !self.contains_expression() {
            simple_object_pattern(self, ctx, ctx.lexical_binding_state);
        } else {
            complex_object_pattern(self, ctx, ctx.lexical_binding_state);
        }
    }
}

fn simple_object_pattern<'s>(
    pattern: &'s ast::ObjectPattern<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    has_environment: bool,
) {
    ctx.lexical_binding_state = has_environment;
    ctx.add_instruction_with_immediate(
        Instruction::BeginSimpleObjectBindingPattern,
        has_environment.into(),
    );

    for ele in &pattern.properties {
        if ele.shorthand {
            let ast::PropertyKey::StaticIdentifier(identifier) = &ele.key else {
                unreachable!()
            };
            assert!(matches!(
                &ele.value.kind,
                ast::BindingPatternKind::BindingIdentifier(_)
            ));
            let identifier_string = ctx.create_identifier(&identifier.name);
            ctx.add_instruction_with_identifier(Instruction::BindingPatternBind, identifier_string);
        } else {
            let key_string = match &ele.key {
                ast::PropertyKey::StaticIdentifier(identifier) => {
                    // SAFETY: We'll use this value as a PropertyKey directly later.
                    unsafe {
                        PropertyKey::from_str(ctx.agent, &identifier.name, ctx.gc)
                            .into_value_unchecked()
                    }
                }
                ast::PropertyKey::NumericLiteral(literal) => {
                    let numeric_value = Number::from_f64(ctx.agent, literal.value, ctx.gc);
                    if let Number::Integer(_) = numeric_value {
                        numeric_value.into_value()
                    } else {
                        Number::to_string_radix_10(ctx.agent, numeric_value, ctx.gc).into_value()
                    }
                }
                ast::PropertyKey::StringLiteral(literal) => {
                    // SAFETY: We'll use this value as a PropertyKey directly later.
                    unsafe {
                        PropertyKey::from_str(ctx.agent, &literal.value, ctx.gc)
                            .into_value_unchecked()
                    }
                }
                _ => unreachable!(),
            };

            match &ele.value.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let value_identifier_string = ctx.create_identifier(&identifier.name);
                    ctx.add_instruction_with_identifier_and_constant(
                        Instruction::BindingPatternBindNamed,
                        value_identifier_string,
                        key_string,
                    )
                }
                ast::BindingPatternKind::ObjectPattern(pattern) => {
                    ctx.add_instruction_with_constant(
                        Instruction::BindingPatternGetValueNamed,
                        key_string,
                    );
                    simple_object_pattern(pattern, ctx, has_environment);
                }
                ast::BindingPatternKind::ArrayPattern(pattern) => {
                    ctx.add_instruction_with_constant(
                        Instruction::BindingPatternGetValueNamed,
                        key_string,
                    );
                    simple_array_pattern(
                        ctx,
                        pattern.elements.iter().map(Option::as_ref),
                        pattern.rest.as_deref(),
                        pattern.elements.len(),
                        has_environment,
                    );
                }
                ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
            }
        }
    }

    if let Some(rest) = &pattern.rest {
        match &rest.argument.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(
                    Instruction::BindingPatternBindRest,
                    identifier_string,
                );
            }
            _ => unreachable!(),
        }
    } else {
        ctx.add_instruction(Instruction::FinishBindingPattern);
    }
}

fn complex_object_pattern<'s>(
    object_pattern: &'s ast::ObjectPattern<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    has_environment: bool,
) {
    ctx.lexical_binding_state = has_environment;
    // 8.6.2 Runtime Semantics: BindingInitialization
    // BindingPattern : ObjectBindingPattern
    // 1. Perform ? RequireObjectCoercible(value).
    // NOTE: RequireObjectCoercible throws in the same cases as ToObject, and other operations
    // later on (such as GetV) also perform ToObject, so we convert to an object early.
    ctx.add_instruction(Instruction::Store);
    ctx.add_instruction(Instruction::ToObject);
    ctx.add_instruction(Instruction::Load);

    for property in &object_pattern.properties {
        match &property.key {
            ast::PropertyKey::StaticIdentifier(identifier) => {
                ctx.add_instruction(Instruction::Store);
                ctx.add_instruction(Instruction::LoadCopy);
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(
                    Instruction::EvaluatePropertyAccessWithIdentifierKey,
                    identifier_string,
                );
            }
            ast::PropertyKey::PrivateIdentifier(_) => todo!(),
            _ => {
                property.key.to_expression().compile(ctx);
                ctx.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
            }
        }
        if object_pattern.rest.is_some() {
            ctx.add_instruction(Instruction::GetValueKeepReference);
            ctx.add_instruction(Instruction::PushReference);
        } else {
            ctx.add_instruction(Instruction::GetValue);
        }

        let binding_pattern = match &property.value.kind {
            ast::BindingPatternKind::AssignmentPattern(pattern) => {
                // Run the initializer if the result value is undefined.
                ctx.add_instruction(Instruction::LoadCopy);
                ctx.add_instruction(Instruction::IsUndefined);
                let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                ctx.add_instruction(Instruction::Store);
                if is_anonymous_function_definition(&pattern.right) {
                    if let ast::BindingPatternKind::BindingIdentifier(identifier) =
                        &pattern.left.kind
                    {
                        let identifier_string = ctx.create_identifier(&identifier.name);
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            identifier_string,
                        );
                        ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                    }
                }
                pattern.right.compile(ctx);
                ctx.name_identifier = None;
                if is_reference(&pattern.right) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::Load);
                ctx.set_jump_target_here(jump_slot);
                ctx.add_instruction(Instruction::Store);

                &pattern.left.kind
            }
            _ => &property.value.kind,
        };

        match binding_pattern {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_identifier(&identifier.name);
                ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier_string);
                if !has_environment {
                    ctx.add_instruction(Instruction::PutValue);
                } else {
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                }
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::Load);
                pattern.compile(ctx);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::Load);
                pattern.compile(ctx);
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    }

    if let Some(rest) = &object_pattern.rest {
        let ast::BindingPatternKind::BindingIdentifier(identifier) = &rest.argument.kind else {
            unreachable!()
        };

        // We have kept the references for all of the properties read in the reference stack, so we
        // can now use them to exclude those properties from the rest object.
        ctx.add_instruction_with_immediate(
            Instruction::CopyDataPropertiesIntoObject,
            object_pattern.properties.len(),
        );

        let identifier_string = ctx.create_identifier(&identifier.name);
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier_string);
        if !has_environment {
            ctx.add_instruction(Instruction::PutValue);
        } else {
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }
    } else {
        // Don't keep the object on the stack.
        ctx.add_instruction(Instruction::Store);
    }
}

impl<'s> CompileEvaluation<'s> for ast::VariableDeclaration<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match self.kind {
            // VariableStatement : var VariableDeclarationList ;
            ast::VariableDeclarationKind::Var => {
                for decl in &self.declarations {
                    // VariableDeclaration : BindingIdentifier
                    let Some(init) = &decl.init else {
                        // 1. Return EMPTY.
                        continue;
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
                            ctx.add_instruction(Instruction::GetValue);
                        }
                        ctx.add_instruction(Instruction::Load);
                        // 3. Return ? BindingInitialization of BidingPattern with arguments rval and undefined.
                        match &decl.id.kind {
                            ast::BindingPatternKind::BindingIdentifier(_) => unreachable!(),
                            ast::BindingPatternKind::ObjectPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::ArrayPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
                        }
                        continue;
                    };

                    // 1. Let bindingId be StringValue of BindingIdentifier.
                    // 2. Let lhs be ? ResolveBinding(bindingId).
                    let identifier_string =
                        String::from_str(ctx.agent, identifier.name.as_str(), ctx.gc);
                    let identifier = ctx.add_identifier(identifier_string);
                    ctx.add_instruction_with_immediate(Instruction::ResolveBinding, identifier);
                    ctx.add_instruction(Instruction::PushReference);

                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    if is_anonymous_function_definition(init) {
                        ctx.add_instruction_with_immediate(Instruction::LoadConstant, identifier);
                        // a. Let value be ? NamedEvaluation of Initializer with argument StackgId.
                        ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
                        init.compile(ctx);
                    } else {
                        // 4. Else,
                        // a. Let rhs be ? Evaluation of Initializer.
                        init.compile(ctx);
                        // b. Let value be ? GetValue(rhs).
                        if is_reference(init) {
                            ctx.add_instruction(Instruction::GetValue);
                        }
                    }
                    // 5. Perform ? PutValue(lhs, value).
                    ctx.add_instruction(Instruction::PopReference);
                    ctx.add_instruction(Instruction::PutValue);

                    // 6. Return EMPTY.
                    // Store Undefined as the result value.
                    ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
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
                            ctx.add_instruction(Instruction::GetValue);
                        }
                        // 3. Let env be the running execution context's LexicalEnvironment.
                        // 4. Return ? BindingInitialization of BindingPattern with arguments value and env.
                        ctx.add_instruction(Instruction::Load);
                        match &decl.id.kind {
                            ast::BindingPatternKind::BindingIdentifier(_) => unreachable!(),
                            ast::BindingPatternKind::ObjectPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::ArrayPattern(pattern) => pattern.compile(ctx),
                            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
                        }
                        return;
                    };

                    // 1. Let lhs be ! ResolveBinding(StringValue of BindingIdentifier).
                    let identifier_string =
                        String::from_str(ctx.agent, identifier.name.as_str(), ctx.gc);
                    let identifier = ctx.add_identifier(identifier_string);
                    ctx.add_instruction_with_immediate(Instruction::ResolveBinding, identifier);

                    let Some(init) = &decl.init else {
                        // LexicalBinding : BindingIdentifier
                        // 2. Perform ! InitializeReferencedBinding(lhs, undefined).
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            Value::Undefined,
                        );
                        ctx.add_instruction(Instruction::InitializeReferencedBinding);
                        // 3. Return empty.
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            Value::Undefined,
                        );
                        continue;
                    };

                    let do_push_reference = !init.is_literal();
                    //  LexicalBinding : BindingIdentifier Initializer
                    if do_push_reference {
                        ctx.add_instruction(Instruction::PushReference);
                    }
                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    if is_anonymous_function_definition(init) {
                        // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                        ctx.add_instruction_with_immediate(Instruction::LoadConstant, identifier);
                        ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
                        init.compile(ctx);
                    } else {
                        // 4. Else,
                        // a. Let rhs be ? Evaluation of Initializer.
                        init.compile(ctx);
                        // b. Let value be ? GetValue(rhs).
                        if is_reference(init) {
                            ctx.add_instruction(Instruction::GetValue);
                        }
                    }

                    // 5. Perform ! InitializeReferencedBinding(lhs, value).
                    if do_push_reference {
                        ctx.add_instruction(Instruction::PopReference);
                    }
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                    // 6. Return empty.
                    ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                }
            }
            ast::VariableDeclarationKind::Using => todo!(),
            ast::VariableDeclarationKind::AwaitUsing => todo!(),
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::BlockStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.body.is_empty() {
            // Block : {}
            // 1. Return EMPTY.
            return;
        }
        let did_enter_declarative_environment =
            block_declaration_instantiation::instantiation(ctx, self);
        for ele in &self.body {
            ele.compile(ctx);
        }
        if ctx.peek_last_instruction() != Some(Instruction::Return.as_u8()) {
            // Block did not end in a return so we overwrite the result with undefined.
            // TODO: This should be removed; block doesn't reset the value.
            // ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        if did_enter_declarative_environment {
            ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
            ctx.current_lexical_depth -= 1;
        }
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::ForStatement<'s> {
    fn compile_labelled<'gc>(
        &'s self,
        mut label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    ) {
        let previous_jump_target = ctx.push_new_jump_target(label_set.as_deref_mut());

        let mut per_iteration_lets: Vec<String<'_>> = vec![];
        let mut is_lexical = false;

        if let Some(init) = &self.init {
            match init {
                ast::ForStatementInit::ArrayExpression(init) => init.compile(ctx),
                ast::ForStatementInit::ArrowFunctionExpression(init) => init.compile(ctx),
                ast::ForStatementInit::AssignmentExpression(init) => init.compile(ctx),
                ast::ForStatementInit::AwaitExpression(init) => init.compile(ctx),
                ast::ForStatementInit::BigIntLiteral(init) => init.compile(ctx),
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
                #[cfg(feature = "regexp")]
                ast::ForStatementInit::RegExpLiteral(init) => init.compile(ctx),
                #[cfg(not(feature = "regexp"))]
                ast::ForStatementInit::RegExpLiteral(_) => unreachable!(),
                ast::ForStatementInit::SequenceExpression(init) => init.compile(ctx),
                ast::ForStatementInit::StaticMemberExpression(init) => init.compile(ctx),
                ast::ForStatementInit::StringLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::Super(init) => init.compile(ctx),
                ast::ForStatementInit::TaggedTemplateExpression(init) => init.compile(ctx),
                ast::ForStatementInit::TemplateLiteral(init) => init.compile(ctx),
                ast::ForStatementInit::ThisExpression(init) => init.compile(ctx),
                ast::ForStatementInit::UnaryExpression(init) => init.compile(ctx),
                ast::ForStatementInit::UpdateExpression(init) => init.compile(ctx),
                ast::ForStatementInit::VariableDeclaration(init) => {
                    is_lexical = init.kind.is_lexical();
                    if is_lexical {
                        // 1. Let oldEnv be the running execution context's LexicalEnvironment.
                        // 2. Let loopEnv be NewDeclarativeEnvironment(oldEnv).
                        // Note: This declaration environment is not something
                        // that continue/break statements should care about. We
                        // take care of tearing this one down.
                        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
                        // 3. Let isConst be IsConstantDeclaration of LexicalDeclaration.
                        let is_const = init.kind.is_const();
                        // 4. Let boundNames be the BoundNames of LexicalDeclaration.
                        // 5. For each element dn of boundNames, do
                        // a. If isConst is true, then
                        if is_const {
                            init.bound_names(&mut |dn| {
                                // i. Perform ! loopEnv.CreateImmutableBinding(dn, true).
                                let identifier =
                                    String::from_str(ctx.agent, dn.name.as_str(), ctx.gc);
                                ctx.add_instruction_with_identifier(
                                    Instruction::CreateImmutableBinding,
                                    identifier,
                                )
                            });
                        } else {
                            // b. Else,
                            // i. Perform ! loopEnv.CreateMutableBinding(dn, false).
                            init.bound_names(&mut |dn| {
                                let identifier =
                                    String::from_str(ctx.agent, dn.name.as_str(), ctx.gc);
                                // 9. If isConst is false, let perIterationLets
                                // be boundNames; otherwise let perIterationLets
                                // be a new empty List.
                                per_iteration_lets.push(identifier);
                                ctx.add_instruction_with_identifier(
                                    Instruction::CreateMutableBinding,
                                    identifier,
                                )
                            });
                        }
                        // 6. Set the running execution context's LexicalEnvironment to loopEnv.
                    }
                    init.compile(ctx);
                }
                ast::ForStatementInit::YieldExpression(init) => init.compile(ctx),
                // TODO: determine how to handle this case
                ast::ForStatementInit::V8IntrinsicExpression(_) => todo!(),
                ast::ForStatementInit::JSXElement(_)
                | ast::ForStatementInit::JSXFragment(_)
                | ast::ForStatementInit::TSAsExpression(_)
                | ast::ForStatementInit::TSSatisfiesExpression(_)
                | ast::ForStatementInit::TSTypeAssertion(_)
                | ast::ForStatementInit::TSNonNullExpression(_)
                | ast::ForStatementInit::TSInstantiationExpression(_) => unreachable!(),
            }
        }
        // 2. Perform ? CreatePerIterationEnvironment(perIterationBindings).
        let create_per_iteration_env = if !per_iteration_lets.is_empty() {
            Some(|ctx: &mut CompileContext<'_, '_, 'gc, '_>| {
                if per_iteration_lets.len() == 1 {
                    // NOTE: Optimization for the usual case of a single let
                    // binding. We do not need to push and pop from the stack
                    // in this case but can use the result register directly.
                    // There are rather easy further optimizations available as
                    // well around creating a sibling environment directly,
                    // creating an initialized mutable binding directly, and
                    // importantly: The whole loop environment is unnecessary
                    // if the loop contains no closures (that capture the
                    // per-iteration lets).

                    let binding = *per_iteration_lets.first().unwrap();
                    // Get value of binding from lastIterationEnv.
                    ctx.add_instruction_with_identifier(Instruction::ResolveBinding, binding);
                    ctx.add_instruction(Instruction::GetValue);
                    // Current declarative environment is now "outer"
                    ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
                    // NewDeclarativeEnvironment(outer)
                    ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
                    ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, binding);
                    ctx.add_instruction_with_identifier(Instruction::ResolveBinding, binding);
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                } else {
                    for bn in &per_iteration_lets {
                        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, *bn);
                        ctx.add_instruction(Instruction::GetValue);
                        ctx.add_instruction(Instruction::Load);
                    }
                    ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
                    ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
                    for bn in per_iteration_lets.iter().rev() {
                        ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, *bn);
                        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, *bn);
                        ctx.add_instruction(Instruction::Store);
                        ctx.add_instruction(Instruction::InitializeReferencedBinding);
                    }
                }
            })
        } else {
            None
        };

        if let Some(create_per_iteration_env) = create_per_iteration_env {
            create_per_iteration_env(ctx);
        }

        let loop_jump = ctx.get_jump_index_to_here();
        if let Some(test) = &self.test {
            test.compile(ctx);
            if is_reference(test) {
                ctx.add_instruction(Instruction::GetValue);
            }
        } else {
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Boolean(true));
        }
        // jump over consequent if test fails
        let end_jump = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);

        self.body.compile(ctx);

        let jump_target = ctx.take_current_jump_target(label_set);
        for continue_entry in jump_target.continues {
            ctx.set_jump_target_here(continue_entry);
        }

        if let Some(create_per_iteration_env) = create_per_iteration_env {
            create_per_iteration_env(ctx);
        }

        if let Some(update) = &self.update {
            update.compile(ctx);
        }
        ctx.add_jump_instruction_to_index(Instruction::Jump, loop_jump);
        ctx.set_jump_target_here(end_jump);

        for break_entry in jump_target.breaks {
            ctx.set_jump_target_here(break_entry);
        }
        if is_lexical {
            // Lexical binding loops have an extra declarative environment that
            // we need to exit from once we exit the loop.
            ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        }
        ctx.return_jump_target(previous_jump_target);
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::SwitchStatement<'s> {
    fn compile_labelled(
        &'s self,
        mut label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        let previous_jump_target = ctx.push_new_jump_target(label_set.as_deref_mut());
        // 1. Let exprRef be ? Evaluation of Expression.
        self.discriminant.compile(ctx);
        if is_reference(&self.discriminant) {
            // 2. Let switchValue be ? GetValue(exprRef).
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::Load);
        // 3. Let oldEnv be the running execution context's LexicalEnvironment.
        // 4. Let blockEnv be NewDeclarativeEnvironment(oldEnv).
        // 6. Set the running execution context's LexicalEnvironment to blockEnv.
        // 5. Perform BlockDeclarationInstantiation(CaseBlock, blockEnv).
        let did_enter_declarative_environment =
            block_declaration_instantiation::instantiation(ctx, self);

        // 7. Let R be Completion(CaseBlockEvaluation of CaseBlock with argument switchValue).
        let mut has_default = false;
        let mut jump_indexes = Vec::with_capacity(self.cases.len());
        for case in &self.cases {
            let Some(test) = &case.test else {
                // Default case test does not care about the write order: After
                // all other cases have been tested, default will be entered if
                // no other was entered previously. The placement of the
                // default case only matters for fall-through behaviour.
                has_default = true;
                continue;
            };
            // Duplicate the switchValue on the stack. One will remain, one is
            // used by the IsStrictlyEqual
            ctx.add_instruction(Instruction::Store);
            ctx.add_instruction(Instruction::LoadCopy);
            ctx.add_instruction(Instruction::Load);
            // 2. Let exprRef be ? Evaluation of the Expression of C.
            test.compile(ctx);
            // 3. Let clauseSelector be ? GetValue(exprRef).
            if is_reference(test) {
                ctx.add_instruction(Instruction::GetValue);
            }
            // 4. Return IsStrictlyEqual(input, clauseSelector).
            ctx.add_instruction(Instruction::IsStrictlyEqual);
            // b. If found is true then [evaluate case]
            jump_indexes.push(ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue));
        }

        if has_default {
            // 10. If foundInB is true, return V.
            // 11. Let defaultR be Completion(Evaluation of DefaultClause).
            jump_indexes.push(ctx.add_instruction_with_jump_slot(Instruction::Jump));
        }

        let mut index = 0;
        for (i, case) in self.cases.iter().enumerate() {
            let fallthrough_jump = if i != 0 {
                Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
            } else {
                None
            };
            // Jump from IsStrictlyEqual comparison to here.
            let jump_index = if case.test.is_some() {
                let jump_index = jump_indexes.get(index).unwrap();
                index += 1;
                jump_index
            } else {
                // Default case! The jump index is last in the Vec.
                jump_indexes.last().unwrap()
            };
            ctx.set_jump_target_here(jump_index.clone());

            // Pop the switchValue from the stack.
            ctx.add_instruction(Instruction::Store);
            // And override it with undefined
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);

            if let Some(fallthrough_jump) = fallthrough_jump {
                ctx.set_jump_target_here(fallthrough_jump);
            }

            for ele in &case.consequent {
                ele.compile(ctx);
            }
        }

        let jump_target = ctx.take_current_jump_target(label_set);
        for break_entry in jump_target.breaks {
            ctx.set_jump_target_here(break_entry);
        }
        if !jump_target.continues.is_empty() {
            // Some called continue inside a switch statement; these presumably
            // belong to a loop outside.
            let mut previous_jump_target = previous_jump_target.as_ref().unwrap().borrow_mut();
            previous_jump_target
                .continues
                .extend_from_slice(&jump_target.continues);
            // It's unlikely that any duplicates would exist but it is
            // technically possible. We'll try avoid those.
            previous_jump_target.continues.sort();
            previous_jump_target.continues.dedup();
        }
        ctx.return_jump_target(previous_jump_target);

        // 8. Set the running execution context's LexicalEnvironment to oldEnv.
        if did_enter_declarative_environment {
            ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
            ctx.current_lexical_depth -= 1;
        }
        // 9. Return R.
    }
}

impl<'s> CompileEvaluation<'s> for ast::ThrowStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        self.argument.compile(ctx);
        if is_reference(&self.argument) {
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::Throw)
    }
}

impl<'s> CompileEvaluation<'s> for ast::TryStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.finalizer.is_some() {
            todo!();
        }

        let jump_to_catch =
            ctx.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget);
        self.block.compile(ctx);
        ctx.add_instruction(Instruction::PopExceptionJumpTarget);
        let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::Jump);

        let catch_clause = self.handler.as_ref().unwrap();
        ctx.set_jump_target_here(jump_to_catch);

        if let Some(exception_param) = &catch_clause.param {
            // 1. Let oldEnv be the running execution context's LexicalEnvironment.
            // 2. Let catchEnv be NewDeclarativeEnvironment(oldEnv).
            // 4. Set the running execution context's LexicalEnvironment to catchEnv.
            // Note: We skip the declarative environment if there is no catch
            // param as it's not observable.
            ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
            ctx.current_lexical_depth += 1;

            // 3. For each element argName of the BoundNames of CatchParameter, do
            // a. Perform ! catchEnv.CreateMutableBinding(argName, false).
            exception_param.pattern.bound_names(&mut |arg_name| {
                let arg_name = String::from_str(ctx.agent, arg_name.name.as_str(), ctx.gc);
                ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, arg_name);
            });
            // 5. Let status be Completion(BindingInitialization of CatchParameter with arguments thrownValue and catchEnv).
            // 6. If status is an abrupt completion, then
            // a. Set the running execution context's LexicalEnvironment to oldEnv.
            // b. Return ? status.
            match &exception_param.pattern.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let identifier_string = ctx.create_identifier(&identifier.name);
                    ctx.add_instruction_with_identifier(
                        Instruction::ResolveBinding,
                        identifier_string,
                    );
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                }
                ast::BindingPatternKind::ObjectPattern(pattern) => {
                    ctx.add_instruction(Instruction::Load);
                    ctx.lexical_binding_state = true;
                    pattern.compile(ctx);
                }
                ast::BindingPatternKind::ArrayPattern(pattern) => {
                    ctx.add_instruction(Instruction::Load);
                    ctx.lexical_binding_state = true;
                    pattern.compile(ctx);
                }
                ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
            }
        }
        // 7. Let B be Completion(Evaluation of Block).
        catch_clause.body.compile(ctx);
        // 8. Set the running execution context's LexicalEnvironment to oldEnv.
        if catch_clause.param.is_some() {
            ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
            ctx.current_lexical_depth -= 1;
        }
        // 9. Return ? B.
        ctx.set_jump_target_here(jump_to_end);
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::WhileStatement<'s> {
    fn compile_labelled(
        &'s self,
        mut label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        let previous_jump_target = ctx.push_new_jump_target(label_set.as_deref_mut());

        // 2. Repeat
        let start_jump = ctx.get_jump_index_to_here();

        // a. Let exprRef be ? Evaluation of Expression.

        self.test.compile(ctx);
        if is_reference(&self.test) {
            // b. Let exprValue be ? GetValue(exprRef).
            ctx.add_instruction(Instruction::GetValue);
        }

        // c. If ToBoolean(exprValue) is false, return V.
        // jump over loop jump if test fails
        let end_jump = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
        // d. Let stmtResult be Completion(Evaluation of Statement).
        self.body.compile(ctx);

        // e. If LoopContinues(stmtResult, labelSet) is false, return ? UpdateEmpty(stmtResult, V).
        // f. If stmtResult.[[Value]] is not EMPTY, set V to stmtResult.[[Value]].
        ctx.add_jump_instruction_to_index(Instruction::Jump, start_jump.clone());
        let jump_target = ctx.take_current_jump_target(label_set);
        for continue_entry in jump_target.continues {
            ctx.set_jump_target(continue_entry, start_jump.clone());
        }

        ctx.set_jump_target_here(end_jump);

        for break_entry in jump_target.breaks {
            ctx.set_jump_target_here(break_entry);
        }
        ctx.return_jump_target(previous_jump_target);
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::DoWhileStatement<'s> {
    fn compile_labelled(
        &'s self,
        mut label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        let previous_jump_target = ctx.push_new_jump_target(label_set.as_deref_mut());

        let start_jump = ctx.get_jump_index_to_here();
        self.body.compile(ctx);

        let jump_target = ctx.take_current_jump_target(label_set);
        for continue_entry in jump_target.continues {
            ctx.set_jump_target_here(continue_entry);
        }

        self.test.compile(ctx);
        if is_reference(&self.test) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // jump over loop jump if test fails
        let end_jump = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
        ctx.add_jump_instruction_to_index(Instruction::Jump, start_jump);
        ctx.set_jump_target_here(end_jump);

        for break_entry in jump_target.breaks {
            ctx.set_jump_target_here(break_entry);
        }
        ctx.return_jump_target(previous_jump_target);
    }
}

impl<'s> CompileEvaluation<'s> for ast::BreakStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let depth = ctx.current_lexical_depth;
        let jump_target = if let Some(label) = &self.label {
            ctx.labelled_statements
                .as_ref()
                .unwrap()
                .get(&label.name)
                .unwrap()
                .clone()
        } else {
            ctx.current_jump_target.as_ref().unwrap().clone()
        };
        let mut jump_target = jump_target.borrow_mut();
        let jump_depth = jump_target.depth;
        assert!(depth >= jump_depth);
        for _ in jump_depth..depth {
            // We have to exit the declarative environments we've entered.
            ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        }
        let break_jump = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        jump_target.breaks.push(break_jump);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ContinueStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let depth = ctx.current_lexical_depth;
        let jump_target = if let Some(label) = &self.label {
            ctx.labelled_statements
                .as_ref()
                .unwrap()
                .get(&label.name)
                .unwrap()
                .clone()
        } else {
            ctx.current_jump_target.as_ref().unwrap().clone()
        };
        let mut jump_target = jump_target.borrow_mut();
        let jump_depth = jump_target.depth;
        assert!(depth >= jump_depth);
        for _ in jump_depth..depth {
            // We have to exit the declarative environments we've entered.
            ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        }
        let continue_jump = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        jump_target.continues.push(continue_jump);
        if let Some(label) = &self.label {
            let label = label.name.as_str();
            todo!("continue {};", label);
        }
    }
}

impl<'s> CompileEvaluation<'s> for ast::Statement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match self {
            ast::Statement::ExpressionStatement(x) => x.compile(ctx),
            ast::Statement::ReturnStatement(x) => x.compile(ctx),
            ast::Statement::IfStatement(x) => x.compile(ctx),
            ast::Statement::VariableDeclaration(x) => x.compile(ctx),
            ast::Statement::FunctionDeclaration(_) => {
                // Note: Function declaration statements are always hoisted.
                // There is no work left to do here.
            }
            ast::Statement::BlockStatement(x) => x.compile(ctx),
            ast::Statement::EmptyStatement(_) => {}
            ast::Statement::ForStatement(x) => x.compile_labelled(None, ctx),
            ast::Statement::ThrowStatement(x) => x.compile(ctx),
            ast::Statement::TryStatement(x) => x.compile(ctx),
            Statement::BreakStatement(statement) => statement.compile(ctx),
            Statement::ContinueStatement(statement) => statement.compile(ctx),
            Statement::DebuggerStatement(_) => todo!(),
            Statement::DoWhileStatement(statement) => statement.compile_labelled(None, ctx),
            Statement::ForInStatement(statement) => statement.compile_labelled(None, ctx),
            Statement::ForOfStatement(statement) => statement.compile_labelled(None, ctx),
            Statement::LabeledStatement(statement) => statement.compile_labelled(None, ctx),
            Statement::SwitchStatement(statement) => statement.compile_labelled(None, ctx),
            Statement::WhileStatement(statement) => statement.compile_labelled(None, ctx),
            Statement::WithStatement(_) => todo!(),
            Statement::ClassDeclaration(x) => x.compile(ctx),
            Statement::ImportDeclaration(_) => todo!(),
            Statement::ExportAllDeclaration(_) => todo!(),
            Statement::ExportDefaultDeclaration(_) => todo!(),
            Statement::ExportNamedDeclaration(_) => todo!(),
            #[cfg(feature = "typescript")]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {}
            #[cfg(not(feature = "typescript"))]
            Statement::TSTypeAliasDeclaration(_) | Statement::TSInterfaceDeclaration(_) => {
                unreachable!()
            }
            Statement::TSEnumDeclaration(_)
            | Statement::TSExportAssignment(_)
            | Statement::TSImportEqualsDeclaration(_)
            | Statement::TSModuleDeclaration(_)
            | Statement::TSNamespaceExportDeclaration(_) => unreachable!(),
        }
    }
}

fn is_anonymous_function_definition(expression: &ast::Expression) -> bool {
    match expression {
        ast::Expression::ArrowFunctionExpression(_) => true,
        ast::Expression::FunctionExpression(f) => f.id.is_none(),
        ast::Expression::ClassExpression(f) => f.id.is_none(),
        ast::Expression::ParenthesizedExpression(x) => {
            is_anonymous_function_definition(&x.expression)
        }
        _ => false,
    }
}
