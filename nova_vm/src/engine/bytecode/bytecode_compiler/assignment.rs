// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::{self, AssignmentOperator, LogicalOperator};

#[cfg(feature = "typescript")]
use crate::engine::bytecode::bytecode_compiler::PlaceOrValue;
use crate::engine::{
    Instruction,
    bytecode::bytecode_compiler::{
        CompileContext, CompileEvaluation, ExpressionError, NamedEvaluationParameter, Place,
        ValueOutput, is_anonymous_function_definition,
    },
};

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::AssignmentExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let mut do_named_evaluation = false;
        // 1. Let lref be ? Evaluation of LeftHandSideExpression.
        let lref = match &self.left {
            ast::AssignmentTarget::AssignmentTargetIdentifier(identifier) => {
                let place = identifier.compile(ctx);
                // NOTE: If the left hand side does not constitute the start of
                // the assignment expression span, then it means that the left
                // side is inside parentheses and NamedEvaluation should not
                // happen.
                do_named_evaluation = self.span.start == identifier.span.start;
                place
            }
            ast::AssignmentTarget::ComputedMemberExpression(expression) => {
                // 1. If LeftHandSideExpression is neither an ObjectLiteral nor an ArrayLiteral, then
                expression.compile(ctx)?
            }
            ast::AssignmentTarget::ArrayAssignmentTarget(_)
            | ast::AssignmentTarget::ObjectAssignmentTarget(_) => {
                assert_eq!(
                    self.operator,
                    AssignmentOperator::Assign,
                    "SyntaxError: Invalid left-hand side in assignment expression"
                );
                // 2. Let assignmentPattern be the AssignmentPattern that is covered by LeftHandSideExpression.
                // 3. Let rRef be ? Evaluation of AssignmentExpression.
                // 4. Let rVal be ? GetValue(rRef).
                let rval = self.right.compile(ctx)?.get_value(ctx)?;
                // 5. Perform ? DestructuringAssignmentEvaluation of assignmentPattern with argument rVal.
                ctx.add_instruction(Instruction::LoadCopy);
                self.left.to_assignment_target_pattern().compile(ctx)?;
                // 6. Return rVal.
                ctx.add_instruction(Instruction::Store);
                return Ok(rval);
            }
            ast::AssignmentTarget::PrivateFieldExpression(expression) => {
                // 1. If LeftHandSideExpression is neither an ObjectLiteral nor an ArrayLiteral, then
                expression.compile(ctx)?
            }
            ast::AssignmentTarget::StaticMemberExpression(expression) => {
                // 1. If LeftHandSideExpression is neither an ObjectLiteral nor an ArrayLiteral, then
                expression.compile(ctx)?
            }
            #[cfg(feature = "typescript")]
            ast::AssignmentTarget::TSNonNullExpression(x) => {
                let PlaceOrValue::Place(place) = x.expression.compile(ctx)? else {
                    unreachable!()
                };
                do_named_evaluation = self.span.start == x.span.start;
                place
            }
            #[cfg(feature = "typescript")]
            ast::AssignmentTarget::TSAsExpression(x) => {
                let PlaceOrValue::Place(place) = x.expression.compile(ctx)? else {
                    unreachable!()
                };
                do_named_evaluation = self.span.start == x.span.start;
                place
            }
            #[cfg(feature = "typescript")]
            ast::AssignmentTarget::TSSatisfiesExpression(x) => {
                let PlaceOrValue::Place(place) = x.expression.compile(ctx)? else {
                    unreachable!()
                };
                do_named_evaluation = self.span.start == x.span.start;
                place
            }
            #[cfg(not(feature = "typescript"))]
            ast::AssignmentTarget::TSAsExpression(_)
            | ast::AssignmentTarget::TSNonNullExpression(_)
            | ast::AssignmentTarget::TSSatisfiesExpression(_) => unreachable!(),
            ast::AssignmentTarget::TSTypeAssertion(_) => unreachable!(),
        };

        if self.operator.is_assign() {
            let push_reference = lref.has_reference() && !self.right.is_literal();

            if push_reference {
                ctx.add_instruction(Instruction::PushReference);
            }

            if do_named_evaluation && is_anonymous_function_definition(&self.right) {
                ctx.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    lref.identifier().unwrap(),
                );
                ctx.name_identifier = Some(NamedEvaluationParameter::Result);
            }

            let rval = self.right.compile(ctx)?.get_value(ctx)?;

            ctx.add_instruction(Instruction::LoadCopy);

            if push_reference {
                ctx.add_instruction(Instruction::PopReference);
            }

            lref.put_value(ctx, rval)?;
            // ... Return rval.
            ctx.add_instruction(Instruction::Store);
            Ok(rval)
        } else if let Some(operator) = self.operator.to_logical_operator() {
            // 2. Let lval be ? GetValue(lref).
            lref.get_value_keep_reference(ctx)?;
            let push_reference = lref.has_reference() && !self.right.is_literal();
            if push_reference {
                ctx.add_instruction(Instruction::PushReference);
            }
            // We store the left value on the stack, because we'll need to
            // restore it later.
            ctx.add_instruction(Instruction::LoadCopy);

            match operator {
                LogicalOperator::And => {
                    // 3. Let lbool be ToBoolean(lval).
                    // Note: We do not directly call ToBoolean: JumpIfNot does.
                    // 4. If lbool is false, return lval.
                }
                LogicalOperator::Or => {
                    // 3. Let lbool be ToBoolean(lval).
                    // Note: We do not directly call ToBoolean: JumpIfNot does.
                    // 4. If lbool is true, return lval.
                    ctx.add_instruction(Instruction::LogicalNot);
                }
                LogicalOperator::Coalesce => {
                    // 3. If lval is neither undefined nor null, return lval.
                    ctx.add_instruction(Instruction::IsNullOrUndefined);
                }
            };

            let jump_to_else = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);

            // We're returning the right expression, so we discard the left
            // value at the top of the stack.
            ctx.add_instruction(Instruction::Store);

            // 5. If IsAnonymousFunctionDefinition(AssignmentExpression)
            // is true and IsIdentifierRef of LeftHandSideExpression is true,
            // then
            if do_named_evaluation && is_anonymous_function_definition(&self.right) {
                // a. Let lhs be the StringValue of LeftHandSideExpression.
                ctx.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    lref.identifier().unwrap(),
                );
                // b. Let rval be ? NamedEvaluation of AssignmentExpression with argument lhs.
                ctx.name_identifier = Some(NamedEvaluationParameter::Result);
            }
            // a. Let rref be ? Evaluation of AssignmentExpression.
            // b. Let rval be ? GetValue(rref).
            let jump_over_else = 'rref: {
                // Note: no early exits because this path is not unconditional.
                let Ok(rval) = self.right.compile(ctx).and_then(|r| r.get_value(ctx)) else {
                    break 'rref None;
                };
                ctx.add_instruction(Instruction::LoadCopy);
                if push_reference {
                    ctx.add_instruction(Instruction::PopReference);
                }

                // 7. Perform ? PutValue(lref, rval).
                let Ok(_) = lref.put_value(ctx, rval) else {
                    break 'rref None;
                };
                if push_reference {
                    ctx.add_instruction(Instruction::Store);
                    Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
                } else {
                    None
                }
            };

            // 4. ... return lval.
            ctx.set_jump_target_here(jump_to_else);
            ctx.add_instruction(Instruction::Store);
            if push_reference {
                ctx.add_instruction(Instruction::PopReference);
            }
            if let Some(jump_over_else) = jump_over_else {
                ctx.set_jump_target_here(jump_over_else);
            }
            Ok(ValueOutput::Value)
        } else {
            // 2. let lval be ? GetValue(lref).
            lref.get_value_keep_reference(ctx)?;
            ctx.add_instruction(Instruction::Load);
            let do_push_reference = lref.has_reference() && !self.right.is_literal();
            if do_push_reference {
                ctx.add_instruction(Instruction::PushReference);
            }
            // 3. Let rref be ? Evaluation of AssignmentExpression.
            // 4. Let rval be ? GetValue(rref).
            let _rval = self.right.compile(ctx)?.get_value(ctx)?;

            // 5. Let assignmentOpText be the source text matched by AssignmentOperator.
            // 6. Let opText be the sequence of Unicode code points associated with assignmentOpText in the following table:
            let op_text = self.operator.to_binary_operator().unwrap();
            // 7. Let r be ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
            let op_text = match op_text {
                ast::BinaryOperator::Addition => Instruction::ApplyAdditionBinaryOperator,
                ast::BinaryOperator::Subtraction => Instruction::ApplySubtractionBinaryOperator,
                ast::BinaryOperator::Multiplication => {
                    Instruction::ApplyMultiplicationBinaryOperator
                }
                ast::BinaryOperator::Division => Instruction::ApplyDivisionBinaryOperator,
                ast::BinaryOperator::Remainder => Instruction::ApplyRemainderBinaryOperator,
                ast::BinaryOperator::Exponential => Instruction::ApplyExponentialBinaryOperator,
                ast::BinaryOperator::ShiftLeft => Instruction::ApplyShiftLeftBinaryOperator,
                ast::BinaryOperator::ShiftRight => Instruction::ApplyShiftRightBinaryOperator,
                ast::BinaryOperator::ShiftRightZeroFill => {
                    Instruction::ApplyShiftRightZeroFillBinaryOperator
                }
                ast::BinaryOperator::BitwiseOR => Instruction::ApplyBitwiseORBinaryOperator,
                ast::BinaryOperator::BitwiseXOR => Instruction::ApplyBitwiseXORBinaryOperator,
                ast::BinaryOperator::BitwiseAnd => Instruction::ApplyBitwiseAndBinaryOperator,
                _ => unreachable!(),
            };
            ctx.add_instruction(op_text);
            ctx.add_instruction(Instruction::LoadCopy);
            let r = ValueOutput::Value;
            if do_push_reference {
                ctx.add_instruction(Instruction::PopReference);
            }
            // 8. Perform ? PutValue(lref, r).
            lref.put_value(ctx, r)?;
            // 9. Return r.
            ctx.add_instruction(Instruction::Store);
            Ok(r)
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::AssignmentTarget<'s> {
    type Output = Result<(), ExpressionError>;
    /// ## Register states
    ///
    /// ### Entry condition
    /// ```text
    /// result: value
    /// stack: []
    /// reference: None
    /// reference stack: []
    /// ```
    ///
    /// ### Exit condition
    /// ```text
    /// result: None
    /// stack: []
    /// reference: None
    /// reference stack: []
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, 'gc, '_>) -> Self::Output {
        // result: value
        // stack: []
        if let Some(target) = self.as_simple_assignment_target() {
            let needs_load_store = target.is_member_expression();
            if needs_load_store {
                ctx.add_instruction(Instruction::Load);
                // result: None
                // stack: [value]
            }
            let place = target.compile(ctx)?;
            if needs_load_store {
                // result: None
                // stack: [value]
                // reference: &target
                ctx.add_instruction(Instruction::Store);
            }
            // result: value
            // stack: []
            // reference: &target
            place.put_value(ctx, ValueOutput::Value)?;
            // result: None
            // stack: []
            // reference: None
        } else {
            self.to_assignment_target_pattern().compile(ctx)?;
        }
        Ok(())
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::AssignmentTargetPattern<'s>
{
    type Output = Result<(), ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        match self {
            ast::AssignmentTargetPattern::ArrayAssignmentTarget(t) => t.compile(ctx),
            ast::AssignmentTargetPattern::ObjectAssignmentTarget(t) => t.compile(ctx),
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::SimpleAssignmentTarget<'s>
{
    type Output = Result<Place<'s, 'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        match self {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(t) => Ok(t.compile(ctx)),
            ast::SimpleAssignmentTarget::ComputedMemberExpression(t) => t.compile(ctx),
            ast::SimpleAssignmentTarget::StaticMemberExpression(t) => t.compile(ctx),
            ast::SimpleAssignmentTarget::PrivateFieldExpression(t) => t.compile(ctx),
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSNonNullExpression(t) => {
                t.expression.compile(ctx).map(|r| match r {
                    PlaceOrValue::Place(p) => p,
                    _ => unreachable!(),
                })
            }
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSAsExpression(t) => {
                t.expression.compile(ctx).map(|r| match r {
                    PlaceOrValue::Place(p) => p,
                    _ => unreachable!(),
                })
            }
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSSatisfiesExpression(t) => {
                t.expression.compile(ctx).map(|r| match r {
                    PlaceOrValue::Place(p) => p,
                    _ => unreachable!(),
                })
            }
            #[cfg(not(feature = "typescript"))]
            ast::SimpleAssignmentTarget::TSAsExpression(_)
            | ast::SimpleAssignmentTarget::TSNonNullExpression(_)
            | ast::SimpleAssignmentTarget::TSSatisfiesExpression(_) => unreachable!(),
            ast::SimpleAssignmentTarget::TSTypeAssertion(_) => unreachable!(),
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ArrayAssignmentTarget<'s>
{
    type Output = Result<(), ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let sync_iterator = ctx.push_sync_iterator();
        let array_destructuring = ctx.enter_array_destructuring();
        let mut result = Ok(());
        'args: {
            for element in &self.elements {
                if let Some(element) = element {
                    // AssignmentElement : DestructuringAssignmentTarget Initializer (opt)

                    // 1. If DestructuringAssignmentTarget is neither an
                    //    ObjectLiteral nor an ArrayLiteral, then
                    if let ast::AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(element) =
                        element
                    {
                        // a. Let lRef be ? Evaluation of DestructuringAssignmentTarget.
                        if let Some(binding) = element.binding.as_simple_assignment_target() {
                            let lref = match binding.compile(ctx) {
                                Ok(l) => l,
                                Err(e) => {
                                    result = Err(e);
                                    break 'args;
                                }
                            };
                            let needs_push_reference =
                                lref.has_reference() && !element.init.is_literal();
                            if needs_push_reference {
                                ctx.add_instruction(Instruction::PushReference);
                            }
                            // 2. Let value be undefined.
                            // 3. If iteratorRecord.[[Done]] is false, then
                            // a. Let next be ? IteratorStepValue(iteratorRecord).
                            ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);
                            // b. If next is not done, then
                            // i. Set value to next.
                            // 4. If Initializer is present and value is undefined, then
                            //    ...
                            compile_initializer(element, ctx);
                            if needs_push_reference {
                                ctx.add_instruction(Instruction::PopReference);
                            }
                            // 7. Return ? PutValue(lRef, v).
                            if let Err(e) = lref.put_value(ctx, ValueOutput::Value) {
                                result = Err(e);
                                break 'args;
                            }
                        } else {
                            // 2. Let value be undefined.
                            // 3. If iteratorRecord.[[Done]] is false, then
                            // a. Let next be ? IteratorStepValue(iteratorRecord).
                            ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);
                            // b. If next is not done, then
                            // i. Set value to next.
                            // 4. If Initializer is present and value is undefined, then
                            //    ...
                            compile_initializer(element, ctx);
                            // 5. Else,
                            // a. Let v be value.
                            // 6. If DestructuringAssignmentTarget is either an
                            //    ObjectLiteral or an ArrayLiteral, then
                            // a. Let nestedAssignmentPattern be the
                            //    AssignmentPattern that is covered by
                            //    DestructuringAssignmentTarget.
                            let nested_assignment_pattern =
                                element.binding.to_assignment_target_pattern();
                            // b. Return ? DestructuringAssignmentEvaluation of
                            //    nestedAssignmentPattern with argument v.
                            if let Err(e) = nested_assignment_pattern.compile(ctx) {
                                result = Err(e);
                                break 'args;
                            }
                        }
                    } else if let Some(element) = element.as_simple_assignment_target() {
                        // a. Let lRef be ? Evaluation of DestructuringAssignmentTarget.
                        let lref = match element.compile(ctx) {
                            Ok(l) => l,
                            Err(e) => {
                                result = Err(e);
                                break 'args;
                            }
                        };
                        // 2. Let value be undefined.
                        // 3. If iteratorRecord.[[Done]] is false, then
                        // a. Let next be ? IteratorStepValue(iteratorRecord).
                        ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);
                        // b. If next is not done, then
                        // i. Set value to next.
                        // 4. If Initializer is present and value is undefined, then
                        //    ...
                        // 5. Else,
                        // a. Let v be value.
                        // 7. Return ? PutValue(lRef, v).
                        if let Err(e) = lref.put_value(ctx, ValueOutput::Value) {
                            result = Err(e);
                            break 'args;
                        }
                    } else {
                        // 2. Let value be undefined.
                        // 3. If iteratorRecord.[[Done]] is false, then
                        // a. Let next be ? IteratorStepValue(iteratorRecord).
                        ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);
                        // b. If next is not done, then
                        // i. Set value to next.
                        // 4. If Initializer is present and value is undefined, then
                        //    ...
                        // 5. Else,
                        // a. Let v be value.
                        // 6. If DestructuringAssignmentTarget is either an ObjectLiteral or an ArrayLiteral, then
                        // a. Let nestedAssignmentPattern be the AssignmentPattern that is covered by DestructuringAssignmentTarget.
                        // b. Return ? DestructuringAssignmentEvaluation of nestedAssignmentPattern with argument v.
                        let nested_assignment_pattern = element.to_assignment_target_pattern();
                        if let Err(e) = nested_assignment_pattern.compile(ctx) {
                            result = Err(e);
                            break 'args;
                        }
                    }
                } else {
                    // Elision : ,
                    // 1. If iteratorRecord.[[Done]] is false, then
                    // a. Perform ? IteratorStep(iteratorRecord).
                    ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);
                    // 2. Return unused.
                }
            }
            if let Some(rest) = &self.rest {
                if let Some(target) = rest.target.as_simple_assignment_target() {
                    // 1. If DestructuringAssignmentTarget is neither an
                    //    ObjectLiteral nor an ArrayLiteral, then
                    // a. Let lRef be ? Evaluation of
                    //    DestructuringAssignmentTarget.
                    let lref = match target.compile(ctx) {
                        Ok(l) => l,
                        Err(e) => {
                            result = Err(e);
                            break 'args;
                        }
                    };
                    ctx.add_instruction(Instruction::IteratorRestIntoArray);
                    // a. Return ? PutValue(lRef, A).
                    if let Err(e) = lref.put_value(ctx, ValueOutput::Value) {
                        result = Err(e);
                        break 'args;
                    }
                } else {
                    // 5. If DestructuringAssignmentTarget is neither an
                    //    ObjectLiteral nor an ArrayLiteral, then
                    ctx.add_instruction(Instruction::IteratorRestIntoArray);
                    // 6. Let nestedAssignmentPattern be the AssignmentPattern that
                    //    is covered by DestructuringAssignmentTarget.
                    let nested_assignment_pattern = rest.target.to_assignment_target_pattern();
                    // 7. Return ? DestructuringAssignmentEvaluation of
                    //    nestedAssignmentPattern with argument A.
                    if let Err(e) = nested_assignment_pattern.compile(ctx) {
                        result = Err(e);
                        break 'args;
                    }
                }
            }
        }
        // Note: An error during IteratorClose should not jump into
        // IteratorCloseWithError, hence we pop exception jump target here.
        let jump_to_iterator_close_handler = array_destructuring.exit(ctx);
        let jump_to_iterator_pop = sync_iterator.exit(ctx);
        let jump_over_catch = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        // 3. If status is an abrupt completion, then
        {
            ctx.set_jump_target_here(jump_to_iterator_close_handler);
            // a. If iteratorRecord.[[Done]] is false, return
            //    ? IteratorClose(iteratorRecord, status).
            // Note: removing jump_to_iterator_pop catch handler.
            ctx.add_instruction(Instruction::PopExceptionJumpTarget);
            ctx.add_instruction(Instruction::IteratorCloseWithError);
            ctx.set_jump_target_here(jump_to_iterator_pop);
            ctx.add_instruction(Instruction::IteratorPop);
            ctx.add_instruction(Instruction::Throw);
        }
        ctx.set_jump_target_here(jump_over_catch);
        result
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ObjectAssignmentTarget<'s>
{
    type Output = Result<(), ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // result: source
        // stack: []

        ctx.add_instruction(Instruction::ToObject);
        // result: source (converted to object)
        // stack: []

        // Each property and the rest binding require access to the source as
        // object: thus, we effectively need to create a copy of it on the
        // stack before each property call _except_ for the very last one
        // (including the rest binding; before rest we never need a copy as it
        // is always the last one).
        let has_rest = self.rest.is_some();
        let store_copy_cutoff = if has_rest {
            if self.properties.is_empty() {
                // Only rest: we don't need to bother with properties or anything.
                return compile_assignment_target_rest(self.rest.as_ref().unwrap(), ctx, 0);
            } else {
                // Properties and rest: we need to create a copy of source on
                // the stack, and our cutoff happens on the last property:
                // before it we need to StoreCopy, at cutoff we use Store.
                ctx.add_instruction(Instruction::LoadCopy);
                self.properties.len() - 1
            }
        } else {
            if self.properties.is_empty() {
                // No rest and no properties: we've done all we need to do.
                return Ok(());
            }
            if self.properties.len() == 1 {
                // Only one property: we can just compile the property directly.
                return compile_assignment_target_property(
                    self.properties.first().unwrap(),
                    ctx,
                    false,
                );
            } else {
                // At least two properties: we need to create a copy of source
                // on the stack, and our cutoff happens on the second to last
                // property: before it we need to StoreCopy, at cutoff we use
                // Store, and the last property does nothing.
                ctx.add_instruction(Instruction::LoadCopy);
                self.properties.len() - 2
            }
        };
        // result: source
        // stack: [source?]
        for (index, property) in self.properties.iter().enumerate() {
            // result: source
            // stack: [source?]
            compile_assignment_target_property(property, ctx, has_rest)?;
            // result: None
            // stack: [source?]
            match index.cmp(&store_copy_cutoff) {
                std::cmp::Ordering::Less => {
                    // If index is less than the cutoff, there are still more
                    // properties coming after this that need the source. Thus
                    // we must perform a StoreCopy to get source back into
                    // result without removing it from stack.
                    ctx.add_instruction(Instruction::StoreCopy);
                    // result: source
                    // stack: [source]
                }
                std::cmp::Ordering::Equal => {
                    // If index is equal to cutoff, it means that the next
                    // property is the last one that needs the source. Thus we
                    // perform a Store to get the source back into result while
                    // removing it from stack.
                    ctx.add_instruction(Instruction::Store);
                    // result: source
                    // stack: []
                }
                std::cmp::Ordering::Greater => {
                    // If index is greater than cutoff, it means that this is
                    // the last property, no rest property exists after this,
                    // and the stack is empty. We need do nothing here.
                    // result: None
                    // stack: []
                }
            }
        }
        if let Some(rest) = &self.rest {
            // result: source
            // stack: []
            compile_assignment_target_rest(rest, ctx, self.properties.len())?;
        }
        // result: None
        // stack: []
        // reference: None
        // reference stack: []
        Ok(())
    }
}

fn compile_assignment_target_property<'s>(
    property: &'s ast::AssignmentTargetProperty<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    has_rest: bool,
) -> Result<(), ExpressionError> {
    match property {
        ast::AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(identifier) => {
            // result: source
            // stack: [source?]
            let key = ctx.create_string(identifier.binding.name.as_str());
            ctx.add_instruction_with_identifier(
                Instruction::EvaluatePropertyAccessWithIdentifierKey,
                key.to_property_key(),
            );
            let place: Place = key.to_property_key().into();
            // result: None
            // stack: [source?]
            // reference: &source.identifier
            place.get_value_maybe_keep_reference(ctx, has_rest)?;
            if has_rest {
                debug_assert!(place.has_reference());
                ctx.add_instruction(Instruction::PushReference);
            }
            // result: source.identifier
            // stack: [source?]
            // reference: None
            // reference stack: [&source.identifier?]
            identifier.compile(ctx)?;
            // result: None
            // stack: [source?]
            // reference: None
            // reference stack: [&source.identifier?]
        }
        ast::AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
            // result: source
            // stack: [source?]
            let place = property.name.compile(ctx)?;
            // result: None
            // stack: [source?]
            // reference: &source.property
            place.get_value_maybe_keep_reference(ctx, has_rest)?;
            if has_rest {
                debug_assert!(place.has_reference());
                ctx.add_instruction(Instruction::PushReference);
            }
            // result: source.property
            // stack: [source?]
            // reference: None
            // reference stack: [&source.property?]
            property.binding.compile(ctx)?;
            // result: None
            // stack: [source?]
            // reference: None
            // reference stack: [&source.property?]
        }
    }
    // result: None
    // stack: [source?]
    // reference: None
    // reference stack: [&source.property?]
    Ok(())
}

fn compile_assignment_target_rest<'s>(
    rest: &'s ast::AssignmentTargetRest<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    property_count: usize,
) -> Result<(), ExpressionError> {
    // result: source
    // stack: []
    // reference: None
    // reference stack: [...source.properties]
    ctx.add_instruction_with_immediate(Instruction::CopyDataPropertiesIntoObject, property_count);
    // result: object copy
    // stack: []
    // reference: None
    // reference stack: []
    rest.target.compile(ctx)
    // result: None
    // stack: []
    // reference: None
    // reference stack: []
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::AssignmentTargetPropertyIdentifier<'s>
{
    type Output = Result<(), ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // result: binding
        // stack: []

        // Note: the caller is expected to handle the self.binding side of
        // this! When we enter here, self.binding property access result should
        // be in the result register.
        if let Some(init) = &self.init {
            ctx.add_instruction(Instruction::LoadCopy);
            // result: binding
            // stack: [binding]
            ctx.add_instruction(Instruction::IsUndefined);
            // result: binding === undefined
            // stack: [binding]
            let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
            ctx.add_instruction(Instruction::Store);
            // result: binding
            // stack: []
            if is_anonymous_function_definition(init) {
                let identifier_string = ctx.create_string(self.binding.name.as_str());
                ctx.add_instruction_with_constant(Instruction::StoreConstant, identifier_string);
                ctx.name_identifier = Some(NamedEvaluationParameter::Result);
            }
            init.compile(ctx)?.get_value(ctx)?;
            ctx.name_identifier = None;
            // result: init
            // stack: []
            ctx.add_instruction(Instruction::Load);
            // result: None
            // stack: [init]
            ctx.set_jump_target_here(jump_slot);
            // result: None
            // stack: [binding / init]
            ctx.add_instruction(Instruction::Store);
            // result: binding / init
            // stack: []
        }
        let place = self.binding.compile(ctx);
        // result: binding / init
        // stack: []
        // reference: &binding
        place.put_value(ctx, ValueOutput::Value)?;
        // result: None
        // stack: []
        // reference: None
        Ok(())
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::PropertyKey<'s> {
    type Output = Result<Place<'s, 'gc>, ExpressionError>;
    /// ## Register states
    ///
    /// ### Entry condition
    /// ```text
    /// result: source
    /// stack: []
    /// reference: None
    /// reference stack: []
    /// ```
    ///
    /// ### Exit condition
    /// ```text
    /// result: None
    /// stack: []
    /// reference: &source.property
    /// reference stack: []
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // result: source
        // stack: []
        match self {
            ast::PropertyKey::StaticIdentifier(identifier) => {
                Ok(identifier.compile(ctx))
                // result: None
                // stack: []
                // reference: &source.identifier
            }
            // Note: Private names are not allowed in this position.
            ast::PropertyKey::PrivateIdentifier(_) => unreachable!(),
            _ => {
                ctx.add_instruction(Instruction::Load);
                // result: None
                // stack: [source]
                let expr = self.to_expression();
                let source = expr.compile(ctx)?.get_value(ctx)?;

                // result: expr
                // stack: [source]
                ctx.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                // result: None
                // stack: []
                // reference: &source[expr]
                Ok(source.to_expression_key())
            }
        }
    }
}

fn compile_initializer<'s>(
    target: &'s ast::AssignmentTargetWithDefault<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // result: value
    // stack: []
    ctx.add_instruction(Instruction::LoadCopy);
    ctx.add_instruction(Instruction::IsUndefined);
    // result: value === undefined
    // stack: [value]
    let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
    // result: None
    // stack: [value]
    ctx.add_instruction(Instruction::Store);
    // result: value
    // stack: []
    if is_anonymous_function_definition(&target.init)
        && let ast::AssignmentTarget::AssignmentTargetIdentifier(identifier) = &target.binding
    {
        let identifier_string = ctx.create_string(identifier.name.as_str());
        ctx.add_instruction_with_constant(Instruction::StoreConstant, identifier_string);
        ctx.name_identifier = Some(NamedEvaluationParameter::Result);
    }
    // Note: ignore errors; this is not an unconditional path.
    let _ = target
        .init
        .compile(ctx)
        .and_then(|init| init.get_value(ctx));
    ctx.name_identifier = None;
    // result: init
    // stack: []
    ctx.add_instruction(Instruction::Load);
    // result: None
    // stack: [init]
    ctx.set_jump_target_here(jump_slot);
    // result: None
    // stack: [value / init]
    ctx.add_instruction(Instruction::Store);
    // result: value / init
    // stack: []
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::AssignmentTargetMaybeDefault<'s>
{
    type Output = Result<(), ExpressionError>;
    /// ## Register states
    ///
    /// ### Entry condition
    /// ```text
    /// result: value
    /// stack: []
    /// reference: None
    /// reference stack: []
    /// ```
    ///
    /// ### Exit condition
    /// ```text
    /// result: None
    /// stack: []
    /// reference: &source.property
    /// reference stack: []
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) -> Self::Output {
        match self {
            ast::AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(target) => {
                // result: value
                // stack: []
                compile_initializer(target, ctx);
                // result: value / init
                // stack: []
                target.binding.compile(ctx)?;
                // result: None
                // stack: []
            }
            _ => {
                // result: value
                // stack: []
                self.to_assignment_target().compile(ctx)?;
                // result: None
                // stack: []
            }
        }
        Ok(())
    }
}
