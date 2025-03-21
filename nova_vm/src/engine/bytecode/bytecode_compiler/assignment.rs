// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::{self, AssignmentOperator};

use crate::ecmascript::types::String;
use crate::engine::Instruction;

use super::{
    CompileContext, CompileEvaluation, NamedEvaluationParameter, is_anonymous_function_definition,
    is_reference,
};

impl CompileEvaluation for ast::AssignmentExpression<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        // 1. Let lref be ? Evaluation of LeftHandSideExpression.
        match &self.left {
            ast::AssignmentTarget::AssignmentTargetIdentifier(identifier) => {
                identifier.compile(ctx);
            }
            ast::AssignmentTarget::ComputedMemberExpression(expression) => {
                expression.compile(ctx);
            }
            ast::AssignmentTarget::ArrayAssignmentTarget(_)
            | ast::AssignmentTarget::ObjectAssignmentTarget(_) => {
                assert_eq!(
                    self.operator,
                    AssignmentOperator::Assign,
                    "SyntaxError: Invalid left-hand side in assignment expression"
                );
                self.right.compile(ctx);
                if is_reference(&self.right) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::LoadCopy);
                self.left.compile(ctx);
                ctx.add_instruction(Instruction::Store);
                return;
            }
            ast::AssignmentTarget::PrivateFieldExpression(expression) => expression.compile(ctx),
            ast::AssignmentTarget::StaticMemberExpression(expression) => {
                expression.compile(ctx);
            }
            ast::AssignmentTarget::TSAsExpression(_)
            | ast::AssignmentTarget::TSSatisfiesExpression(_)
            | ast::AssignmentTarget::TSNonNullExpression(_)
            | ast::AssignmentTarget::TSTypeAssertion(_)
            | ast::AssignmentTarget::TSInstantiationExpression(_) => unreachable!(),
        };

        if self.operator == AssignmentOperator::Assign {
            let is_rhs_literal = self.right.is_literal();

            if !is_rhs_literal {
                ctx.add_instruction(Instruction::PushReference);
            }

            self.right.compile(ctx);

            if is_reference(&self.right) {
                ctx.add_instruction(Instruction::GetValue);
            }

            ctx.add_instruction(Instruction::LoadCopy);

            if !is_rhs_literal {
                ctx.add_instruction(Instruction::PopReference);
            }

            ctx.add_instruction(Instruction::PutValue);

            // ... Return rval.
            ctx.add_instruction(Instruction::Store);
        } else if matches!(
            self.operator,
            AssignmentOperator::LogicalAnd
                | AssignmentOperator::LogicalNullish
                | AssignmentOperator::LogicalOr
        ) {
            // 2. Let lval be ? GetValue(lref).
            ctx.add_instruction(Instruction::GetValueKeepReference);
            ctx.add_instruction(Instruction::PushReference);
            // We store the left value on the stack, because we'll need to
            // restore it later.
            ctx.add_instruction(Instruction::LoadCopy);

            match self.operator {
                AssignmentOperator::LogicalAnd => {
                    // 3. Let lbool be ToBoolean(lval).
                    // Note: We do not directly call ToBoolean: JumpIfNot does.
                    // 4. If lbool is false, return lval.
                }
                AssignmentOperator::LogicalOr => {
                    // 3. Let lbool be ToBoolean(lval).
                    // Note: We do not directly call ToBoolean: JumpIfNot does.
                    // 4. If lbool is true, return lval.
                    ctx.add_instruction(Instruction::LogicalNot);
                }
                AssignmentOperator::LogicalNullish => {
                    // 3. If lval is neither undefined nor null, return lval.
                    ctx.add_instruction(Instruction::IsNullOrUndefined);
                }
                _ => unreachable!(),
            };

            let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);

            // We're returning the right expression, so we discard the left
            // value at the top of the stack.
            ctx.add_instruction(Instruction::Store);

            // 5. If IsAnonymousFunctionDefinition(AssignmentExpression)
            // is true and IsIdentifierRef of LeftHandSideExpression is true,
            // then
            if matches!(
                self.left,
                ast::AssignmentTarget::AssignmentTargetIdentifier(_)
            ) && is_anonymous_function_definition(&self.right)
            {
                // a. Let lhs be the StringValue of LeftHandSideExpression.
                // b. Let rval be ? NamedEvaluation of AssignmentExpression with argument lhs.
                ctx.name_identifier = Some(NamedEvaluationParameter::ReferenceStack);
                self.right.compile(ctx);
            } else {
                // 6. Else
                // a. Let rref be ? Evaluation of AssignmentExpression.
                self.right.compile(ctx);
                // b. Let rval be ? GetValue(rref).
                if is_reference(&self.right) {
                    ctx.add_instruction(Instruction::GetValue);
                }
            }

            // 7. Perform ? PutValue(lref, rval).
            ctx.add_instruction(Instruction::LoadCopy);
            ctx.add_instruction(Instruction::PopReference);
            ctx.add_instruction(Instruction::PutValue);

            // 4. ... return lval.
            ctx.set_jump_target_here(jump_to_end);
            ctx.add_instruction(Instruction::Store);
        } else {
            // 2. let lval be ? GetValue(lref).
            ctx.add_instruction(Instruction::GetValueKeepReference);
            ctx.add_instruction(Instruction::Load);
            ctx.add_instruction(Instruction::PushReference);
            // 3. Let rref be ? Evaluation of AssignmentExpression.
            self.right.compile(ctx);

            // 4. Let rval be ? GetValue(rref).
            if is_reference(&self.right) {
                ctx.add_instruction(Instruction::GetValue);
            }

            // 5. Let assignmentOpText be the source text matched by AssignmentOperator.
            // 6. Let opText be the sequence of Unicode code points associated with assignmentOpText in the following table:
            let op_text = self.operator.to_binary_operator().unwrap();
            // 7. Let r be ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
            ctx.add_instruction(Instruction::ApplyStringOrNumericBinaryOperator(op_text));
            ctx.add_instruction(Instruction::LoadCopy);
            // 8. Perform ? PutValue(lref, r).
            ctx.add_instruction(Instruction::PopReference);
            ctx.add_instruction(Instruction::PutValue);
            // 9. Return r.
            ctx.add_instruction(Instruction::Store);
        }
    }
}

impl CompileEvaluation for ast::AssignmentTarget<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::AssignmentTarget::ArrayAssignmentTarget(array) => {
                array.compile(ctx);
            }
            ast::AssignmentTarget::AssignmentTargetIdentifier(identifier) => {
                identifier.compile(ctx);
                ctx.add_instruction(Instruction::PutValue);
            }
            ast::AssignmentTarget::ComputedMemberExpression(expression) => {
                ctx.add_instruction(Instruction::Load);
                expression.compile(ctx);
                ctx.add_instruction(Instruction::Store);
                ctx.add_instruction(Instruction::PutValue);
            }
            ast::AssignmentTarget::ObjectAssignmentTarget(object) => {
                object.compile(ctx);
            }
            ast::AssignmentTarget::PrivateFieldExpression(_) => todo!(),
            ast::AssignmentTarget::StaticMemberExpression(expression) => {
                ctx.add_instruction(Instruction::Load);
                expression.compile(ctx);
                ctx.add_instruction(Instruction::Store);
                ctx.add_instruction(Instruction::PutValue);
            }
            ast::AssignmentTarget::TSAsExpression(_)
            | ast::AssignmentTarget::TSSatisfiesExpression(_)
            | ast::AssignmentTarget::TSNonNullExpression(_)
            | ast::AssignmentTarget::TSTypeAssertion(_)
            | ast::AssignmentTarget::TSInstantiationExpression(_) => unreachable!(),
        }
    }
}

impl CompileEvaluation for ast::ArrayAssignmentTarget<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.add_instruction(Instruction::GetIteratorSync);
        for element in &self.elements {
            ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);
            if let Some(element) = element {
                element.compile(ctx);
            }
        }
        if let Some(rest) = &self.rest {
            ctx.add_instruction(Instruction::IteratorRestIntoArray);
            rest.target.compile(ctx);
        } else {
            ctx.add_instruction(Instruction::IteratorClose);
        }
    }
}

impl CompileEvaluation for ast::ObjectAssignmentTarget<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        ctx.add_instruction(Instruction::ToObject);
        if self.properties.len() > 1 || self.rest.is_some() {
            ctx.add_instruction(Instruction::LoadCopy);
        }
        for (index, property) in self.properties.iter().enumerate() {
            property.compile(ctx);
            let offset = if self.rest.is_some() {
                index + 1
            } else {
                index + 2
            };
            #[allow(clippy::comparison_chain)]
            if offset < self.properties.len() {
                ctx.add_instruction(Instruction::StoreCopy);
            } else if offset == self.properties.len() {
                ctx.add_instruction(Instruction::Store);
            }
        }
        if let Some(_rest) = &self.rest {
            todo!()
        }
    }
}

impl CompileEvaluation for ast::AssignmentTargetProperty<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(identifier) => {
                identifier.compile(ctx);
            }
            ast::AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
                property.compile(ctx);
            }
        }
    }
}

impl CompileEvaluation for ast::AssignmentTargetPropertyIdentifier<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let key = String::from_str(ctx.agent, self.binding.name.as_str(), ctx.gc);
        ctx.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            key,
        );
        ctx.add_instruction(Instruction::GetValue);
        if let Some(init) = &self.init {
            ctx.add_instruction(Instruction::LoadCopy);
            ctx.add_instruction(Instruction::IsUndefined);
            let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
            ctx.add_instruction(Instruction::Store);
            if is_anonymous_function_definition(init) {
                let identifier_string = ctx.create_identifier(&self.binding.name);
                ctx.add_instruction_with_constant(Instruction::StoreConstant, identifier_string);
                ctx.name_identifier = Some(NamedEvaluationParameter::Result);
            }
            init.compile(ctx);
            ctx.name_identifier = None;
            if is_reference(init) {
                ctx.add_instruction(Instruction::GetValue);
            }
            ctx.add_instruction(Instruction::Load);
            ctx.set_jump_target_here(jump_slot);
            ctx.add_instruction(Instruction::Store);
        }
        self.binding.compile(ctx);
        ctx.add_instruction(Instruction::PutValue);
    }
}

impl CompileEvaluation for ast::AssignmentTargetPropertyProperty<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match &self.name {
            ast::PropertyKey::StaticIdentifier(identifier) => {
                let key = String::from_str(ctx.agent, identifier.name.as_str(), ctx.gc);
                ctx.add_instruction_with_identifier(
                    Instruction::EvaluatePropertyAccessWithIdentifierKey,
                    key,
                );
            }
            ast::PropertyKey::PrivateIdentifier(_) => todo!(),
            _ => {
                ctx.add_instruction(Instruction::Load);
                let name = self.name.to_expression();
                name.compile(ctx);
                if is_reference(name) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
            }
        }
        ctx.add_instruction(Instruction::GetValue);
        self.binding.compile(ctx);
    }
}

impl CompileEvaluation for ast::AssignmentTargetMaybeDefault<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        match self {
            ast::AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(target) => {
                ctx.add_instruction(Instruction::LoadCopy);
                ctx.add_instruction(Instruction::IsUndefined);
                let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                ctx.add_instruction(Instruction::Store);
                if is_anonymous_function_definition(&target.init) {
                    if let ast::AssignmentTarget::AssignmentTargetIdentifier(identifier) =
                        &target.binding
                    {
                        let identifier_string = ctx.create_identifier(&identifier.name);
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            identifier_string,
                        );
                        ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                    }
                }
                target.init.compile(ctx);
                ctx.name_identifier = None;
                if is_reference(&target.init) {
                    ctx.add_instruction(Instruction::GetValue);
                }
                ctx.add_instruction(Instruction::Load);
                ctx.set_jump_target_here(jump_slot);
                ctx.add_instruction(Instruction::Store);
                target.binding.compile(ctx);
            }
            _ => {
                self.to_assignment_target().compile(ctx);
            }
        }
    }
}
