// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod assignment;
mod block_declaration_instantiation;
mod class_definition_evaluation;
mod compile_context;
mod executable_context;
mod finaliser_stack;
mod for_in_of_statement;
mod function_declaration_instantiation;
mod labelled_statement;

use super::{FunctionExpression, Instruction, SendableRef, executable::ArrowFunctionExpression};
#[cfg(feature = "regexp")]
use crate::ecmascript::{
    syntax_directed_operations::{
        function_definitions::ContainsExpression,
        scope_analysis::{LexicallyScopedDeclaration, LexicallyScopedDeclarations},
    },
    types::{BUILTIN_STRING_MEMORY, IntoValue, Number, String, Value},
};
pub(crate) use compile_context::{
    CompileContext, CompileEvaluation, CompileLabelledEvaluation, IndexType, JumpIndex,
    NamedEvaluationParameter,
};
use num_traits::Num;
use oxc_ast::ast::{
    self, BindingPattern, BindingRestElement, CallExpression, LabelIdentifier, NewExpression,
    Statement,
};
use oxc_ecmascript::BoundNames;
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};

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
    matches!(
        expression.get_inner_expression(),
        ast::Expression::Identifier(_)
            | ast::Expression::ComputedMemberExpression(_)
            | ast::Expression::StaticMemberExpression(_)
            | ast::Expression::PrivateFieldExpression(_)
            | ast::Expression::Super(_)
    )
}

pub(crate) fn is_boolean_literal_true(expression: &ast::Expression) -> bool {
    matches!(expression.get_inner_expression(), ast::Expression::BooleanLiteral(lit) if lit.value)
}

pub(crate) fn is_boolean_literal_false(expression: &ast::Expression) -> bool {
    matches!(expression.get_inner_expression(), ast::Expression::BooleanLiteral(lit) if !lit.value)
}

fn is_chain_expression(expression: &ast::Expression) -> bool {
    matches!(
        expression.get_inner_expression(),
        ast::Expression::ChainExpression(_)
    )
}

impl<'s> CompileEvaluation<'s> for ast::NumericLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let constant = ctx.create_number(self.value);
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
        let (literal, radix) = match self.base {
            oxc_syntax::number::BigintBase::Decimal => (&self.raw.as_str()[..last_index], 10),
            oxc_syntax::number::BigintBase::Binary => (&self.raw.as_str()[2..last_index], 2),
            oxc_syntax::number::BigintBase::Octal => (&self.raw.as_str()[2..last_index], 8),
            oxc_syntax::number::BigintBase::Hex => (&self.raw.as_str()[2..last_index], 16),
        };
        let constant = ctx.create_bigint(literal, radix);
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
        let constant = ctx.create_string(self.value.as_str());
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
    }
}

impl<'s> CompileEvaluation<'s> for ast::IdentifierReference<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let identifier = ctx.create_string(self.name.as_str());
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, identifier);
    }
}

impl<'s> CompileEvaluation<'s> for ast::BindingIdentifier<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let identifier = ctx.create_string(self.name.as_str());
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
                        // It shouldn't be possible for objects to be created
                        // with private identifiers as keys.
                        ast::PropertyKey::PrivateIdentifier(_) => unreachable!(),
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
                                let identifier = ctx.create_property_key(&id.name);
                                ctx.add_instruction_with_constant(
                                    Instruction::StoreConstant,
                                    identifier,
                                );
                            }
                        }
                        _ => {
                            let prop_key = prop.key.as_expression().unwrap();
                            prop_key.compile(ctx);
                            if is_reference(prop_key) {
                                assert!(!is_proto_setter);
                                ctx.add_instruction(Instruction::GetValue);
                            }
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
        if self.elements.is_empty() {
            return;
        }
        ctx.add_instruction(Instruction::Load);
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

/// Compile the baseReference part of a member expression with possible
/// optional chaining.
///
/// ```text
/// 1. Let baseReference be ? Evaluation of MemberExpression.
/// 2. Let baseValue be ? GetValue(baseReference).
/// 3. If baseValue is either undefined or null, then
///     a. Return undefined.
/// 4. Return ? ChainEvaluation of OptionalChain with arguments baseValue and baseReference.
/// ```
///
/// After this call, if optional chaining isn't present then the base value is
/// in the result register. If optional chaining is present, then the base
/// value is at the top of the stack.
fn compile_optional_base_reference<'s>(
    object: &'s ast::Expression<'s>,
    is_optional: bool,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // 1. Let baseReference be ? Evaluation of MemberExpression.
    object.compile(ctx);

    // 2. Let baseValue be ? GetValue(baseReference).
    if is_reference(object) {
        ctx.add_instruction(Instruction::GetValue);
    }

    if is_optional {
        // Optional Chains

        // Load copy of baseValue to stack.
        ctx.add_instruction(Instruction::LoadCopy);
        // 3. If baseValue is either undefined or null, then
        ctx.add_instruction(Instruction::IsNullOrUndefined);
        // a. Return undefined

        // To return undefined we jump over the property access.
        let jump_over_property_access = ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

        // Register our jump slot to the chain nullish case handling.
        ctx.optional_chains
            .as_mut()
            .unwrap()
            .push(jump_over_property_access);
    }
}

impl<'s> CompileEvaluation<'s> for ast::ComputedMemberExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        compile_optional_base_reference(&self.object, self.optional, ctx);
        // If we do not have optional chaining present it means that base value
        // is currently in the result slot. We need to store it on the stack.
        if !self.optional {
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
        compile_optional_base_reference(&self.object, self.optional, ctx);
        // If we are in an optional chain then result will be on the top of the
        // stack. We need to pop it into the register slot in that case.
        if self.optional {
            ctx.add_instruction(Instruction::Store);
        }

        // 4. Return EvaluatePropertyAccessWithIdentifierKey(baseValue, IdentifierName, strict).
        let identifier = ctx.create_string(self.property.name.as_str());
        ctx.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            identifier,
        );
    }
}

impl<'s> CompileEvaluation<'s> for ast::PrivateFieldExpression<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        compile_optional_base_reference(&self.object, self.optional, ctx);
        // If we are in an optional chain then result will be on the top of the
        // stack. We need to pop it into the register slot in that case.
        if self.optional {
            ctx.add_instruction(Instruction::Store);
        }

        //  MemberExpression : MemberExpression . PrivateIdentifier
        // 3. Let fieldNameString be the StringValue of PrivateIdentifier.
        // 4. Return MakePrivateReference(baseValue, fieldNameString).

        // 4. Return EvaluatePropertyAccessWithIdentifierKey(baseValue, IdentifierName, strict).
        let identifier = ctx.create_string(&self.field.name);
        ctx.add_instruction_with_identifier(Instruction::MakePrivateReference, identifier);
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
    /// ## [13.10.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-relational-operators-runtime-semantics-evaluation)
    /// ###  RelationalExpression : PrivateIdentifier in ShiftExpression
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let privateIdentifier be the StringValue of PrivateIdentifier.
        let private_identifier = ctx.create_string(&self.left.name);
        // 2. Let rRef be ? Evaluation of ShiftExpression.
        self.right.compile(ctx);
        // 3. Let rVal be ? GetValue(rRef).
        if is_reference(&self.right) {
            ctx.add_instruction(Instruction::GetValue);
        }
        // 4. If rVal is not an Object, throw a TypeError exception.
        // 5. Let privateEnv be the running execution context's PrivateEnvironment.
        // 6. Assert: privateEnv is not null.
        // 7. Let privateName be ResolvePrivateIdentifier(privateEnv, privateIdentifier).
        ctx.add_instruction_with_identifier(Instruction::MakePrivateReference, private_identifier);
        // 8. If PrivateElementFind(rVal, privateName) is not empty, return true.
        // 9. Return false.
        ctx.add_instruction(Instruction::HasPrivateElement);
    }
}
#[cfg(feature = "regexp")]
impl<'s> CompileEvaluation<'s> for ast::RegExpLiteral<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        let pattern = self.regex.pattern.text.as_str();
        let regexp = ctx.create_regexp(pattern, self.regex.flags);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, regexp);
    }
}

impl<'s> CompileEvaluation<'s> for ast::SequenceExpression<'s> {
    /// ### [13.16.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-comma-operator-runtime-semantics-evaluation)
    ///
    /// ```text
    /// Expression : Expression , AssignmentExpression
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 1. Let lRef be ? Evaluation of Expression.
        // 2. Perform ? GetValue(lRef).
        // 3. Let rRef be ? Evaluation of AssignmentExpression.
        // 4. Return ? GetValue(rRef).

        // Note
        // GetValue must be called even though its value is not used because it
        // may have observable side-effects.

        let (last, rest) = self.expressions.split_last().unwrap();
        for expr in rest {
            if expr.is_literal() {
                // Literals do not have observable side-effects when compiled,
                // we can skip these when they're not the last expression.
                continue;
            }
            expr.compile(ctx);
            if is_reference(expr) {
                // Note: GetValue must be called as mentioned above.
                ctx.add_instruction(Instruction::GetValue);
            }
        }
        last.compile(ctx);
        if is_reference(last) {
            ctx.add_instruction(Instruction::GetValue);
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
            let constant = ctx.create_string(
                self.quasi()
                    .as_ref()
                    .expect("Invalid escape sequence in template literal")
                    .as_str(),
            );
            ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
        } else {
            let mut count = 0;
            let mut quasis = self.quasis.as_slice();
            let mut expressions = self.expressions.as_slice();
            while let Some((head, rest)) = quasis.split_first() {
                quasis = rest;
                // 1. Let head be the TV of TemplateHead as defined in 12.9.6.
                let head = ctx.create_string(head.value.cooked.as_ref().unwrap().as_str());
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
        // Note: generators can be resumed with a Return instruction. For those
        // cases we need to generate Return handling here.
        let jump_over_return = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        ctx.compile_return();
        ctx.set_jump_target_here(jump_over_return);
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
            ast::SimpleAssignmentTarget::PrivateFieldExpression(x) => x.compile(ctx),
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
        ctx.compile_return();
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
        let mut jump_over_else = None;
        if let Some(alternate) = &self.alternate {
            // Optimisation: If the an else branch exists, the consequent
            // branch needs to end in a jump over it. But if the consequent
            // branch ends in a terminal statement that jump becomes unnecessary.
            if !ctx.is_unreachable() {
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
    /// ## [8.6.2 Runtime Semantics: BindingInitialization](https://tc39.es/ecma262/#sec-runtime-semantics-bindinginitialization)
    /// ### BindingPattern : ArrayBindingPattern
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if self.elements.is_empty() && self.rest.is_none() {
            // ArrayAssignmentPattern : [ ]
            // 1. Let iteratorRecord be ? GetIterator(value, sync).
            // 2. Return ? IteratorClose(iteratorRecord, NormalCompletion(unused)).
            ctx.add_instruction(Instruction::GetIteratorSync);
            ctx.add_instruction(Instruction::IteratorClose);
            return;
        }

        // 1. Let iteratorRecord be ? GetIterator(value, sync).
        ctx.add_instruction(Instruction::GetIteratorSync);
        let jump_to_catch = ctx.enter_iterator(None);
        // 2. Let result be Completion(IteratorBindingInitialization of
        //    ArrayBindingPattern with arguments iteratorRecord and
        //    environment).
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
        // 3. If iteratorRecord.[[Done]] is false, return
        //    ? IteratorClose(iteratorRecord, result).
        // Note: simple array binding handles IteratorClose at runtime, while
        // complex array binding injects it on its own. We don't need to do
        // anything special here.
        // 4. Return ? result.
        ctx.add_instruction(Instruction::PopExceptionJumpTarget);
        let jump_over_catch = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        {
            // catch handling, we have to call IteratorClose with the error.
            ctx.set_jump_target_here(jump_to_catch);
            ctx.add_instruction(Instruction::IteratorCloseWithError);
        }
        ctx.set_jump_target_here(jump_over_catch);
        ctx.exit_iterator(None);
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
    let lexical_binding_state = ctx.lexical_binding_state;
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
                let identifier_string = ctx.create_string(identifier.name.as_str());
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
                let identifier_string = ctx.create_string(identifier.name.as_str());
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
    ctx.lexical_binding_state = lexical_binding_state;
}

fn check_result_is_undefined(ctx: &mut CompileContext) -> JumpIndex {
    // Run the initializer if the result value is undefined.
    ctx.add_instruction(Instruction::LoadCopy);
    ctx.add_instruction(Instruction::IsUndefined);
    let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
    // Drop the undefined result value and run initializer.
    ctx.add_instruction(Instruction::Store);
    jump_slot
}

/// ### [8.6.3 Runtime Semantics: IteratorBindingInitialization](https://tc39.es/ecma262/#sec-runtime-semantics-iteratorbindinginitialization)
fn complex_array_pattern<'s, I>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    elements: I,
    rest: Option<&'s BindingRestElement<'s>>,
    has_environment: bool,
) where
    I: Iterator<Item = Option<&'s BindingPattern<'s>>>,
{
    let lexical_binding_state = ctx.lexical_binding_state;
    ctx.lexical_binding_state = has_environment;
    for ele in elements {
        ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);

        let Some(ele) = ele else {
            continue;
        };

        ele.compile(ctx);
    }

    if let Some(rest) = rest {
        ctx.add_instruction(Instruction::IteratorRestIntoArray);
        rest.argument.compile(ctx);
    } else {
        ctx.add_instruction(Instruction::IteratorClose);
    }
    ctx.lexical_binding_state = lexical_binding_state;
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
    let lexical_binding_state = ctx.lexical_binding_state;
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
            let identifier_string = ctx.create_string(identifier.name.as_str());
            ctx.add_instruction_with_identifier(Instruction::BindingPatternBind, identifier_string);
        } else {
            let key_string = match &ele.key {
                ast::PropertyKey::StaticIdentifier(identifier) => {
                    // SAFETY: We'll use this value as a PropertyKey directly later.
                    unsafe {
                        ctx.create_property_key(&identifier.name)
                            .into_value_unchecked()
                    }
                }
                ast::PropertyKey::NumericLiteral(literal) => {
                    if let Ok(Number::Integer(integer)) = Number::try_from(literal.value) {
                        // Literal is an integer, just drop it in as a
                        // PropertyKey integer directly.
                        Value::Integer(integer)
                    } else {
                        // Literal is a float: it needs to be converted into a
                        // String.
                        let mut buffer = ryu_js::Buffer::new();
                        ctx.create_string(buffer.format(literal.value)).into_value()
                    }
                }
                ast::PropertyKey::StringLiteral(literal) => {
                    // SAFETY: We'll use this value as a PropertyKey directly later.
                    unsafe {
                        ctx.create_property_key(&literal.value)
                            .into_value_unchecked()
                    }
                }
                ast::PropertyKey::BigIntLiteral(lit) => {
                    // Drop out the trailing 'n' from BigInt literals.
                    let last_index = lit.raw.len() - 1;
                    let (literal, radix) = match lit.base {
                        oxc_syntax::number::BigintBase::Decimal => {
                            (&lit.raw.as_str()[..last_index], 10)
                        }
                        oxc_syntax::number::BigintBase::Binary => {
                            (&lit.raw.as_str()[2..last_index], 2)
                        }
                        oxc_syntax::number::BigintBase::Octal => {
                            (&lit.raw.as_str()[2..last_index], 8)
                        }
                        oxc_syntax::number::BigintBase::Hex => {
                            (&lit.raw.as_str()[2..last_index], 16)
                        }
                    };
                    if let Ok(result) = i64::from_str_radix(literal, radix) {
                        if let Ok(number) = Number::try_from(result) {
                            number.into_value()
                        } else {
                            ctx.create_string_from_owned(result.to_string())
                                .into_value()
                        }
                    } else {
                        let string = num_bigint::BigInt::from_str_radix(literal, radix)
                            .unwrap()
                            .to_string();
                        ctx.create_string_from_owned(string).into_value()
                    }
                }
                _ => unreachable!(),
            };

            match &ele.value.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let value_identifier_string = ctx.create_string(identifier.name.as_str());
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
                let identifier_string = ctx.create_string(identifier.name.as_str());
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
    ctx.lexical_binding_state = lexical_binding_state;
}

fn complex_object_pattern<'s>(
    object_pattern: &'s ast::ObjectPattern<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    has_environment: bool,
) {
    let lexical_binding_state = ctx.lexical_binding_state;
    ctx.lexical_binding_state = has_environment;
    // 8.6.2 Runtime Semantics: BindingInitialization
    // BindingPattern : ObjectBindingPattern
    // 1. Perform ? RequireObjectCoercible(value).
    // NOTE: RequireObjectCoercible throws in the same cases as ToObject, and other operations
    // later on (such as GetV) also perform ToObject, so we convert to an object early.
    ctx.add_instruction(Instruction::ToObject);
    ctx.add_instruction(Instruction::Load);

    for property in &object_pattern.properties {
        match &property.key {
            ast::PropertyKey::StaticIdentifier(identifier) => {
                ctx.add_instruction(Instruction::StoreCopy);
                let identifier_string = ctx.create_string(identifier.name.as_str());
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

        property.value.compile(ctx);
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

        let identifier_string = ctx.create_string(identifier.name.as_str());
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
    ctx.lexical_binding_state = lexical_binding_state;
}

impl<'s> CompileEvaluation<'s> for ast::BindingPattern<'s> {
    /// ## [8.6.2 Runtime Semantics: BindingInitialization](https://tc39.es/ecma262/#sec-runtime-semantics-bindinginitialization)
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        match &self.kind {
            // ### BindingIdentifier : Identifier
            // ### BindingIdentifier : yield
            // ### BindingIdentifier : await
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                // 1. Let name be the StringValue of Identifier.
                // 2. Return ? InitializeBoundName(name, value, environment).
                identifier.compile(ctx);

                // ### 8.6.2.1 InitializeBoundName ( name, value, environment )
                // 1. If environment is not undefined, then
                if ctx.lexical_binding_state {
                    // a. Perform ! environment.InitializeBinding(name, value).
                    // b. Return unused.
                    ctx.add_instruction(Instruction::InitializeReferencedBinding);
                } else {
                    // 2. Else,
                    // a. Let lhs be ? ResolveBinding(name).
                    // b. Return ? PutValue(lhs, value).
                    ctx.add_instruction(Instruction::PutValue);
                }
            }
            // ### BindingPattern : ObjectBindingPattern
            ast::BindingPatternKind::ObjectPattern(object_binding_pattern) => {
                object_binding_pattern.compile(ctx);
            }
            // ### BindingPattern : ArrayBindingPattern
            ast::BindingPatternKind::ArrayPattern(array_binding_pattern) => {
                array_binding_pattern.compile(ctx);
            }
            // ### SingleNameBinding : BindingIdentifier Initializer
            // ### BindingElement : BindingPattern Initializer
            ast::BindingPatternKind::AssignmentPattern(pattern) => {
                match &pattern.left.kind {
                    // ### SingleNameBinding : BindingIdentifier Initializer
                    //
                    // * function (a = 1) {}
                    // * [a = 1]
                    ast::BindingPatternKind::BindingIdentifier(binding_identifier) => {
                        // 1. Let bindingId be the StringValue of BindingIdentifier.
                        // 2. Let lhs be ? ResolveBinding(bindingId, environment).
                        let binding_id = ctx.create_string(binding_identifier.name.as_str());
                        ctx.add_instruction_with_identifier(
                            Instruction::ResolveBinding,
                            binding_id,
                        );
                        // Note: v is already in the result register after
                        // IteratorStepValueOrUndefined above.
                        // 3. Let v be undefined.
                        // 4. If iteratorRecord.[[Done]] is false, then
                        //         a. Let next be ? IteratorStepValue(iteratorRecord).
                        //         b. If next is not done, then
                        //                 i. Set v to next.
                        // 5. If Initializer is present and v is undefined, then
                        let jump_over_initializer = check_result_is_undefined(ctx);
                        if is_anonymous_function_definition(&pattern.right) {
                            // a. If IsAnonymousFunctionDefinition(Initializer) is
                            //    true, then
                            // i. Set v to ? NamedEvaluation of Initializer with
                            //    argument bindingId.
                            ctx.add_instruction_with_constant(
                                Instruction::StoreConstant,
                                binding_id,
                            );
                            ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                        }
                        let right_is_literal = pattern.right.is_literal();
                        if !right_is_literal {
                            ctx.add_instruction(Instruction::PushReference);
                        }
                        // b. Else,
                        // i. Let defaultValue be ? Evaluation of Initializer.
                        pattern.right.compile(ctx);
                        // ii. Set v to ? GetValue(defaultValue).
                        if is_reference(&pattern.right) {
                            ctx.add_instruction(Instruction::GetValue);
                        }
                        if !right_is_literal {
                            ctx.add_instruction(Instruction::PopReference);
                        }
                        ctx.name_identifier = None;
                        ctx.add_instruction(Instruction::Load);
                        ctx.set_jump_target_here(jump_over_initializer);
                        ctx.add_instruction(Instruction::Store);
                        // 6. If environment is undefined,
                        if !ctx.lexical_binding_state {
                            // return ? PutValue(lhs, v).
                            ctx.add_instruction(Instruction::PutValue);
                        } else {
                            // 7. Return ? InitializeReferencedBinding(lhs, v).
                            ctx.add_instruction(Instruction::InitializeReferencedBinding);
                        }
                    }
                    // ### BindingElement : BindingPattern Initializer
                    //
                    // * function ({} = 1)
                    // * [{} = 1]
                    // * function ([] = 1)
                    // * [[] = 1]
                    _ => {
                        // Note: v is already in the result register after
                        // IteratorStepValueOrUndefined above.
                        // 1. Let v be undefined.
                        // 2. If iteratorRecord.[[Done]] is false, then
                        //         a. Let next be ? IteratorStepValue(iteratorRecord).
                        //         b. If next is not done, then
                        //                 i. Set v to next.
                        // 3. If Initializer is present and v is undefined, then
                        let jump_over_initializer = check_result_is_undefined(ctx);
                        // a. Let defaultValue be ? Evaluation of Initializer.
                        pattern.right.compile(ctx);
                        // b. Set v to ? GetValue(defaultValue).
                        if is_reference(&pattern.right) {
                            ctx.add_instruction(Instruction::GetValue);
                        }
                        ctx.add_instruction(Instruction::Load);
                        ctx.set_jump_target_here(jump_over_initializer);
                        ctx.add_instruction(Instruction::Store);
                        // 4. Return ? BindingInitialization of BindingPattern with
                        //    arguments v and environment.
                        pattern.left.compile(ctx)
                    }
                }
            }
        }
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
                        // 1. Let rhs be ? Evaluation of Initializer.
                        init.compile(ctx);
                        // 2. Let rval be ? GetValue(rhs).
                        if is_reference(init) {
                            ctx.add_instruction(Instruction::GetValue);
                        }
                        // 3. Return ? BindingInitialization of BidingPattern with arguments rval and undefined.
                        let lexical_binding_state = ctx.lexical_binding_state;
                        ctx.lexical_binding_state = false;
                        decl.id.compile(ctx);
                        ctx.lexical_binding_state = lexical_binding_state;
                        continue;
                    };

                    // 1. Let bindingId be StringValue of BindingIdentifier.
                    // 2. Let lhs be ? ResolveBinding(bindingId).
                    let identifier_string = ctx.create_string(identifier.name.as_str());
                    let identifier = ctx.add_identifier(identifier_string);
                    ctx.add_instruction_with_immediate(Instruction::ResolveBinding, identifier);
                    let is_literal = init.is_literal();
                    if !is_literal {
                        ctx.add_instruction(Instruction::PushReference);
                    }

                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    if is_anonymous_function_definition(init) {
                        ctx.add_instruction_with_immediate(Instruction::LoadConstant, identifier);
                        // a. Let value be ? NamedEvaluation of Initializer with argument StackId.
                        ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
                        init.compile(ctx);
                    } else {
                        // 4. Else,
                        // a. Let rhs be ? Evaluation of Initializer.
                        init.compile(ctx);
                        // b. Let value be ? GetValue(rhs).
                        if is_reference(init) {
                            debug_assert!(!is_literal);
                            ctx.add_instruction(Instruction::GetValue);
                        }
                    }
                    // 5. Perform ? PutValue(lhs, value).
                    if !is_literal {
                        ctx.add_instruction(Instruction::PopReference);
                    }
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
                        let lexical_binding_state = ctx.lexical_binding_state;
                        ctx.lexical_binding_state = true;
                        decl.id.compile(ctx);
                        ctx.lexical_binding_state = lexical_binding_state;
                        return;
                    };

                    // 1. Let lhs be ! ResolveBinding(StringValue of BindingIdentifier).
                    let identifier_string = ctx.create_string(identifier.name.as_str());
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
        if did_enter_declarative_environment {
            ctx.exit_lexical_scope();
        }
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::ForStatement<'s> {
    fn compile_labelled<'gc>(
        &'s self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    ) {
        let mut per_iteration_lets: Vec<String<'_>> = vec![];
        let mut is_lexical = false;

        if let Some(init) = &self.init {
            match init {
                ast::ForStatementInit::VariableDeclaration(init) => {
                    is_lexical = init.kind.is_lexical();
                    if is_lexical {
                        // 1. Let oldEnv be the running execution context's LexicalEnvironment.
                        // 2. Let loopEnv be NewDeclarativeEnvironment(oldEnv).
                        ctx.enter_lexical_scope();
                        // 3. Let isConst be IsConstantDeclaration of LexicalDeclaration.
                        let is_const = init.kind.is_const();
                        // 4. Let boundNames be the BoundNames of LexicalDeclaration.
                        // 5. For each element dn of boundNames, do
                        // a. If isConst is true, then
                        if is_const {
                            init.bound_names(&mut |dn| {
                                // i. Perform ! loopEnv.CreateImmutableBinding(dn, true).
                                let identifier = ctx.create_string(dn.name.as_str());
                                ctx.add_instruction_with_identifier(
                                    Instruction::CreateImmutableBinding,
                                    identifier,
                                )
                            });
                        } else {
                            // b. Else,
                            // i. Perform ! loopEnv.CreateMutableBinding(dn, false).
                            init.bound_names(&mut |dn| {
                                let identifier = ctx.create_string(dn.name.as_str());
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
                _ => init.as_expression().unwrap().compile(ctx),
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
                    // Note: here we do not use exit & enter lexical
                    // environment helpers as we'd just immediately exit again.
                    ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
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
                    // Note: here we do not use exit & enter lexical
                    // environment helpers as we'd just immediately exit again.
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

        // 3. Repeat,
        ctx.enter_loop(label_set.cloned());
        let loop_jump = ctx.get_jump_index_to_here();
        let end_jump = if let Some(test) = &self.test {
            test.compile(ctx);
            if is_reference(test) {
                ctx.add_instruction(Instruction::GetValue);
            }
            // jump over consequent if test fails
            Some(ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot))
        } else {
            None
        };

        self.body.compile(ctx);

        let continue_target = if create_per_iteration_env.is_none() && self.update.is_none() {
            // If there is no work to be done at the end of the loop, continue
            // can just jump straight to the beginning.
            loop_jump.clone()
        } else {
            let continue_target = ctx.get_jump_index_to_here();
            if let Some(create_per_iteration_env) = create_per_iteration_env {
                create_per_iteration_env(ctx);
            }

            if let Some(update) = &self.update {
                update.compile(ctx);
            }

            continue_target
        };

        ctx.add_jump_instruction_to_index(Instruction::Jump, loop_jump);
        if let Some(end_jump) = end_jump {
            ctx.set_jump_target_here(end_jump);
        }

        ctx.exit_loop(continue_target);

        if is_lexical {
            // Lexical binding loops have an extra declarative environment that
            // we need to exit from once we exit the loop.
            ctx.exit_lexical_scope();
        }
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::SwitchStatement<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        // 1. Let exprRef be ? Evaluation of Expression.
        self.discriminant.compile(ctx);
        if is_reference(&self.discriminant) {
            // 2. Let switchValue be ? GetValue(exprRef).
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::Load);
        ctx.enter_switch(label_set.cloned());
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

        if did_enter_declarative_environment {
            ctx.exit_lexical_scope();
        }

        ctx.exit_switch();
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
            ctx.enter_try_finally_block();
        }

        let jump_to_catch = if self.handler.is_some() {
            Some(ctx.enter_try_catch_block())
        } else {
            None
        };
        // 1. Let B be Completion(Evaluation of Block).
        self.block.compile(ctx);
        // 2. If B is a throw completion, let C be
        //    Completion(CatchClauseEvaluation of Catch with argument B.[[Value]]).
        let jump_over_catch_blocks = if let Some(catch_clause) = &self.handler {
            ctx.exit_try_catch_block();
            // OPTIMISATION: If the end of the try-block is unreachable, we
            // don't need a jump over the catch blocks.
            let jump_over_catch_blocks = if !ctx.is_unreachable() {
                Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
            } else {
                None
            };
            ctx.set_jump_target_here(jump_to_catch.unwrap());

            // 14.15.2 Runtime Semantics: CatchClauseEvaluation
            if let Some(exception_param) = &catch_clause.param {
                // 1. Let oldEnv be the running execution context's LexicalEnvironment.
                // 2. Let catchEnv be NewDeclarativeEnvironment(oldEnv).
                // 4. Set the running execution context's LexicalEnvironment to catchEnv.
                // Note: We skip the declarative environment if there is no catch
                // param as it's not observable.
                ctx.enter_lexical_scope();

                // 3. For each element argName of the BoundNames of CatchParameter, do
                // a. Perform ! catchEnv.CreateMutableBinding(argName, false).
                exception_param.pattern.bound_names(&mut |arg_name| {
                    let arg_name = ctx.create_string(arg_name.name.as_str());
                    ctx.add_instruction_with_identifier(
                        Instruction::CreateMutableBinding,
                        arg_name,
                    );
                });
                // 5. Let status be Completion(BindingInitialization of
                //    CatchParameter with arguments thrownValue and catchEnv).
                let lexical_binding_state = ctx.lexical_binding_state;
                ctx.lexical_binding_state = true;
                exception_param.pattern.compile(ctx);
                ctx.lexical_binding_state = lexical_binding_state;
                // 6. If status is an abrupt completion, then
                // a. Set the running execution context's LexicalEnvironment to oldEnv.
                // b. Return ? status.
            }
            // 7. Let B be Completion(Evaluation of Block).
            catch_clause.body.compile(ctx);
            // 8. Set the running execution context's LexicalEnvironment to oldEnv.
            if catch_clause.param.is_some() {
                ctx.exit_lexical_scope();
            }
            // 9. Return ? B.
            jump_over_catch_blocks
        } else {
            assert!(jump_to_catch.is_none());
            None
        };
        if let Some(finalizer) = &self.finalizer {
            ctx.exit_try_finally_block(finalizer, jump_over_catch_blocks);
        } else if let Some(jump_over_catch_blocks) = jump_over_catch_blocks {
            // If we have a catch block following the normal execution but no
            // finally block then we'll have to handle the jump out ourselves.
            ctx.set_jump_target_here(jump_over_catch_blocks);
        }
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::WhileStatement<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        ctx.enter_loop(label_set.cloned());

        // 1. Let V be undefined.
        // 2. Repeat
        let continue_target = ctx.get_jump_index_to_here();

        // a. Let exprRef be ? Evaluation of Expression.
        // OPTIMISATION: while(true) loops are pretty common, skip the test.
        let end_jump = if !is_boolean_literal_true(&self.test) {
            self.test.compile(ctx);
            if is_reference(&self.test) {
                // b. Let exprValue be ? GetValue(exprRef).
                ctx.add_instruction(Instruction::GetValue);
            }

            // c. If ToBoolean(exprValue) is false, return V.
            // jump over loop jump if test fails
            Some(ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot))
        } else {
            None
        };

        // d. Let stmtResult be Completion(Evaluation of Statement).
        self.body.compile(ctx);

        ctx.add_jump_instruction_to_index(Instruction::Jump, continue_target.clone());
        // e. If LoopContinues(stmtResult, labelSet) is false, return ? UpdateEmpty(stmtResult, V).
        // f. If stmtResult.[[Value]] is not EMPTY, set V to stmtResult.[[Value]].
        if let Some(end_jump) = end_jump {
            ctx.set_jump_target_here(end_jump);
        }
        ctx.exit_loop(continue_target);
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::DoWhileStatement<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        ctx.enter_loop(label_set.cloned());
        let start_jump = ctx.get_jump_index_to_here();
        self.body.compile(ctx);

        let continue_target = ctx.get_jump_index_to_here();

        // jump over loop jump if test fails
        // OPTIMISATION: do {} while(true) loops are still somewhat common,
        // skip the test.
        let end_jump = if !is_boolean_literal_true(&self.test) {
            self.test.compile(ctx);
            if is_reference(&self.test) {
                ctx.add_instruction(Instruction::GetValue);
            }

            Some(ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot))
        } else {
            None
        };
        ctx.add_jump_instruction_to_index(Instruction::Jump, start_jump);
        if let Some(end_jump) = end_jump {
            ctx.set_jump_target_here(end_jump);
        }
        ctx.exit_loop(continue_target);
    }
}

impl<'s> CompileEvaluation<'s> for ast::BreakStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        ctx.compile_break(self.label.as_ref());
    }
}

impl<'s> CompileEvaluation<'s> for ast::ContinueStatement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        ctx.compile_continue(self.label.as_ref());
    }
}

impl<'s> CompileEvaluation<'s> for ast::Statement<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        if ctx.is_unreachable() {
            // OPTIMISATION: If the previous statement was terminal, then later
            // statements cannot be executed and do not need to be compiled.
            return;
        }
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
