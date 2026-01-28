// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::ControlFlow;

use super::{CompileContext, CompileEvaluation, CompileLabelledEvaluation, Instruction, JumpIndex};
use crate::{
    ecmascript::{Primitive, String, Value},
    engine::bytecode::bytecode_compiler::{
        ExpressionError, StatementContinue, StatementResult, ValueOutput,
        compile_context::{BlockEnvPrep, IteratorStackEntry},
        variable_escapes_scope,
    },
};
use oxc_ast::ast;
use oxc_ecmascript::BoundNames;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum IterationKind {
    Enumerate,
    Iterate,
    AsyncIterate,
}

#[derive(Debug, PartialEq, Eq)]
enum LeftHandSideKind {
    Assignment,
    VarBinding,
    LexicalBinding,
}

fn for_in_of_head_evaluation<'s, 'gc>(
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    uninitialized_bound_names: Vec<String<'gc>>,
    expr: &'s ast::Expression<'s>,
    iteration_kind: IterationKind,
) -> Result<(IteratorStackEntry, Option<JumpIndex>), ExpressionError> {
    // 1. Let oldEnv be the running execution context's LexicalEnvironment.
    // 2. If uninitializedBoundNames is not empty, then
    let new_env = if !uninitialized_bound_names.is_empty() {
        // a. Assert: uninitializedBoundNames has no duplicate entries.
        // b. Let newEnv be NewDeclarativeEnvironment(oldEnv).
        let new_env = ctx.enter_lexical_scope();

        // c. For each String name of uninitializedBoundNames, do
        for name in uninitialized_bound_names.iter() {
            // i. Perform ! newEnv.CreateMutableBinding(name, false).
            ctx.add_instruction_with_identifier(
                Instruction::CreateMutableBinding,
                name.to_property_key(),
            );
        }
        // d. Set the running execution context's LexicalEnvironment to newEnv.
        Some(new_env)
    } else {
        None
    };
    // 3. Let exprRef be Completion(Evaluation of expr).
    let expr_ref = expr.compile(ctx);
    // 4. Set the running execution context's LexicalEnvironment to oldEnv.
    if let Some(new_env) = new_env {
        new_env.exit(ctx);
    }
    // 5. Let exprValue be ? GetValue(? exprRef).
    let expr_value = expr_ref?.get_value(ctx)?;
    // 6. If iterationKind is ENUMERATE, then
    match iteration_kind {
        IterationKind::Enumerate => {
            if matches!(
                expr_value,
                ValueOutput::Literal(Primitive::Undefined) | ValueOutput::Literal(Primitive::Null)
            ) {
                // a. If exprValue is either undefined or null, then
                // i. Return Completion Record { [[Type]]: BREAK, [[Value]]: EMPTY, [[Target]]: EMPTY }.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                let return_break_completion_record =
                    ctx.add_instruction_with_jump_slot(Instruction::Jump);
                Ok((ctx.push_enumerator(), Some(return_break_completion_record)))
            } else {
                // a. If exprValue is either undefined or null, then
                // Add a copy to stack.
                let expr_value_copy = ctx.load_copy_to_stack();
                ctx.add_instruction(Instruction::IsNullOrUndefined);
                let jump_over_undefined_or_null =
                    ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                // i. Return Completion Record { [[Type]]: BREAK, [[Value]]: EMPTY, [[Target]]: EMPTY }.
                // Remove the copy added above.
                expr_value_copy.store(ctx);
                // And override with undefined.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                let return_break_completion_record =
                    ctx.add_instruction_with_jump_slot(Instruction::Jump);
                ctx.set_jump_target_here(jump_over_undefined_or_null);
                // Load back the copy from above.
                let expr_value_copy = ctx.mark_stack_value();
                expr_value_copy.store(ctx);
                // b. Let obj be ! ToObject(exprValue).
                // c. Let iterator be EnumerateObjectProperties(obj).
                // d. Let nextMethod be ! GetV(iterator, "next").
                // e. Return the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
                // Note: iteratorKind is SYNC
                Ok((ctx.push_enumerator(), Some(return_break_completion_record)))
            }
        }
        // 7. Else,
        // a. Assert: iterationKind is either ITERATE or ASYNC-ITERATE.
        IterationKind::AsyncIterate => {
            // b. If iterationKind is ASYNC-ITERATE, let iteratorKind be ASYNC.
            // d. Return ? GetIterator(exprValue, iteratorKind).
            Ok((ctx.push_async_iterator(), None))
        }
        IterationKind::Iterate => {
            // c. Else, let iteratorKind be SYNC.
            // d. Return ? GetIterator(exprValue, iteratorKind).
            Ok((ctx.push_sync_iterator(), None))
        }
    }
}

fn for_in_of_body_evaluation<'s, 'gc>(
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    lhs: &'s ast::ForStatementLeft<'s>,
    stmt: &'s ast::Statement<'s>,
    iteration_kind: IterationKind,
    lhs_kind: LeftHandSideKind,
    label_set: Option<&Vec<&'s ast::LabelIdentifier<'s>>>,
    jump_to_iterator_pop_on_error: JumpIndex,
) -> StatementResult<'gc> {
    // 1. If iteratorKind is not present, set iteratorKind to SYNC.
    // 2. Let oldEnv be the running execution context's LexicalEnvironment.
    // 3. Let V be undefined.
    let v = ctx.push_stack_result_value(Some(Value::Undefined));
    // 4. Let destructuring be IsDestructuring of lhs.
    let destructuring = if let ast::ForStatementLeft::VariableDeclaration(lhs) = lhs {
        assert_eq!(lhs.declarations.len(), 1);
        lhs.declarations[0].id.kind.is_destructuring_pattern()
    } else {
        lhs.is_assignment_target_pattern()
    };
    // 5. If destructuring is true and lhsKind is ASSIGNMENT, then
    let assignment_pattern = if destructuring && lhs_kind == LeftHandSideKind::Assignment {
        // a. Assert: lhs is a LeftHandSideExpression.
        // b. Let assignmentPattern be the AssignmentPattern that is covered by lhs.
        Some(lhs.to_assignment_target_pattern())
    } else {
        None
    };

    // 6. Repeat,
    let loop_start = ctx.get_jump_index_to_here();
    let jump_to_done = if iteration_kind == IterationKind::AsyncIterate {
        // a. Let nextResult be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
        ctx.add_instruction(Instruction::IteratorCallNextMethod);
        // b. If iteratorKind is ASYNC, set nextResult to ? Await(nextResult).
        ctx.add_instruction(Instruction::Await);
        // c. If nextResult is not an Object, throw a TypeError exception.
        // d. Let done be ? IteratorComplete(nextResult).
        let jump_to_done = ctx.add_instruction_with_jump_slot(Instruction::IteratorComplete);
        // e. If done is true, return V.
        // f. Let nextValue be ? IteratorValue(nextResult).
        ctx.add_instruction(Instruction::IteratorValue);
        jump_to_done
    } else {
        // Note: IteratorStepValue performs all of the following steps without
        // necessarily creating the nextResult object unnecessarily:
        // a. Let nextResult be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
        // b. If iteratorKind is ASYNC, set nextResult to ? Await(nextResult).
        // c. If nextResult is not an Object, throw a TypeError exception.
        // d. Let done be ? IteratorComplete(nextResult).
        // e. If done is true, return V.
        // f. Let nextValue be ? IteratorValue(nextResult).
        ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue)
    };
    // Note: stepping the iterator happens "outside" the loop in a sense;
    // errors thrown above do not close the iterator; the iterator must still
    // be popped!

    let r#loop = match iteration_kind {
        IterationKind::Enumerate => ctx.enter_loop(label_set.cloned()),
        IterationKind::Iterate => ctx.enter_iterator(label_set.cloned()),
        IterationKind::AsyncIterate => ctx.enter_async_iterator(label_set.cloned()),
    };

    let mut block_prep: Vec<BlockEnvPrep> = vec![];
    // g. If lhsKind is either ASSIGNMENT or VAR-BINDING, then
    let status = match lhs_kind {
        LeftHandSideKind::Assignment | LeftHandSideKind::VarBinding => {
            // i. If destructuring is true, then
            if destructuring {
                // 1. If lhsKind is ASSIGNMENT, then
                if lhs_kind == LeftHandSideKind::Assignment {
                    // a. Let status be Completion(DestructuringAssignmentEvaluation of assignmentPattern with argument nextValue).
                    assignment_pattern.unwrap().compile(ctx)
                } else {
                    // 2. Else,
                    // a. Assert: lhsKind is VAR-BINDING.
                    assert_eq!(lhs_kind, LeftHandSideKind::VarBinding);
                    // b. Assert: lhs is a ForBinding.
                    // c. Let status be Completion(BindingInitialization of lhs with arguments nextValue and undefined).
                    match lhs {
                        ast::ForStatementLeft::VariableDeclaration(decl) => {
                            assert_eq!(decl.declarations.len(), 1);
                            let declaration = decl.declarations.first().unwrap();
                            declaration.id.compile(ctx)
                        }
                        _ => lhs.as_assignment_target().unwrap().compile(ctx),
                    }
                }
            } else {
                // ii. Else,
                // 1. Let lhsRef be Completion(Evaluation of lhs). (It may be evaluated repeatedly.)
                match lhs {
                    ast::ForStatementLeft::VariableDeclaration(decl) => {
                        assert_eq!(decl.declarations.len(), 1);
                        let declaration = decl.declarations.first().unwrap();
                        let ast::BindingPatternKind::BindingIdentifier(binding_identifier) =
                            &declaration.id.kind
                        else {
                            unreachable!()
                        };
                        let lhs_ref = binding_identifier.compile(ctx);
                        // 2. If lhsRef is an abrupt completion, then
                        // a. Let status be lhsRef.
                        // 3. Else,
                        // a. Let status be Completion(PutValue(lhsRef.[[Value]], nextValue)).
                        lhs_ref.put_value(ctx, ValueOutput::Value)
                    }
                    _ => lhs.to_assignment_target().compile(ctx),
                }
            }
        }
        LeftHandSideKind::LexicalBinding => {
            // h. Else,
            // i. Assert: lhsKind is LEXICAL-BINDING.
            // ii. Assert: lhs is a ForDeclaration.
            let ast::ForStatementLeft::VariableDeclaration(lhs) = lhs else {
                unreachable!()
            };
            assert!(lhs.kind.is_lexical());
            // vi. If destructuring is true, then
            if destructuring {
                // iii. Let iterationEnv be NewDeclarativeEnvironment(oldEnv).
                // iv. Perform ForDeclarationBindingInstantiation of lhs with argument iterationEnv.
                // v. Set the running execution context's LexicalEnvironment to iterationEnv.
                lhs.bound_names(&mut |binding_identifier| {
                    if variable_escapes_scope(ctx, binding_identifier) {
                        if !block_prep.iter().any(|p| p.is_env()) {
                            // Optimization: Only enter declarative environment if
                            // bound names exist.
                            block_prep.push(BlockEnvPrep::Env(ctx.enter_lexical_scope()));
                        }
                        let identifier = ctx.create_string(binding_identifier.name.as_str());
                        ctx.add_instruction_with_identifier(
                            if lhs.kind.is_const() {
                                Instruction::CreateImmutableBinding
                            } else {
                                Instruction::CreateMutableBinding
                            },
                            identifier.to_property_key(),
                        );
                    } else {
                        block_prep.push(BlockEnvPrep::Var(
                            ctx.push_stack_variable(binding_identifier.symbol_id(), false),
                        ));
                    }
                });
                // 1. Let status be
                //    Completion(ForDeclarationBindingInitialization of lhs
                //    with arguments nextValue and iterationEnv).
                let lexical_binding_state = ctx.lexical_binding_state;
                ctx.lexical_binding_state = true;
                // ## 14.7.5.3 Runtime Semantics: ForDeclarationBindingInitialization
                // ### ForDeclaration : LetOrConst ForBinding
                // 1. Return ? BindingInitialization of ForBinding with
                //    arguments value and environment.
                assert_eq!(lhs.declarations.len(), 1);
                let lhs = lhs.declarations.first().unwrap();
                assert!(lhs.init.is_none());
                let status = lhs.id.compile(ctx);
                ctx.lexical_binding_state = lexical_binding_state;
                status
            } else {
                // vii. Else,
                lhs.bound_names(&mut |binding_identifier| {
                    // iii. Let iterationEnv be NewDeclarativeEnvironment(oldEnv).
                    // iv. Perform ForDeclarationBindingInstantiation of lhs with argument iterationEnv.
                    // v. Set the running execution context's LexicalEnvironment to iterationEnv.
                    // 1. Assert: lhs binds a single name.
                    debug_assert!(block_prep.is_empty());

                    if variable_escapes_scope(ctx, binding_identifier) {
                        block_prep.push(BlockEnvPrep::Env(ctx.enter_lexical_scope()));
                        // 2. Let lhsName be the sole element of the BoundNames of lhs.
                        let lhs_name = ctx.create_string(binding_identifier.name.as_str());
                        ctx.add_instruction_with_identifier(
                            if lhs.kind.is_const() {
                                Instruction::CreateImmutableBinding
                            } else {
                                Instruction::CreateMutableBinding
                            },
                            lhs_name.to_property_key(),
                        );
                        // 3. Let lhsRef be ! ResolveBinding(lhsName).
                        ctx.add_instruction_with_identifier(
                            Instruction::ResolveBinding,
                            lhs_name.to_property_key(),
                        );
                        // 4. Let status be Completion(InitializeReferencedBinding(lhsRef, nextValue)).
                        ctx.add_instruction(Instruction::InitializeReferencedBinding)
                    } else {
                        block_prep.push(BlockEnvPrep::Var(
                            ctx.push_stack_variable(binding_identifier.symbol_id(), true),
                        ));
                    }
                });
                Ok(())
            }
        }
    };
    // i. If status is an abrupt completion, then ...
    // Note: we move the abrupt completion handling after the loop to improve
    // performance.

    // j. Let result be Completion(Evaluation of stmt).
    let result = if let Err(err) = status {
        ControlFlow::Break(err.into())
    } else {
        let _result = stmt.compile(ctx);
        ControlFlow::Continue(StatementContinue::Value)
    };

    // k. Set the running execution context's LexicalEnvironment to oldEnv.
    for block_prep in block_prep.into_iter().rev() {
        block_prep.exit(ctx);
    }

    let continue_target = ctx.get_jump_index_to_here();

    // Note: This is a loop-internal temporary exit.
    ctx.add_instruction(Instruction::PopExceptionJumpTarget);

    // l. Corollary: If LoopContinues(result, labelSet) is true, then
    //    jump to loop start.
    // m. If result.[[Value]] is not empty, set V to result.[[Value]].
    ctx.add_instruction(Instruction::LoadReplace);
    ctx.add_jump_instruction_to_index(Instruction::Jump, loop_start);
    // Note: this block is here for handling of exceptions iterator loops;
    // these need to perform (Async)IteratorClose. ENUMERATE iteration does not
    // need this as its handling would just rethrow immediately.
    {
        // ## Catch block
        ctx.set_jump_target_here(r#loop.on_abrupt_exit());
        // 2. Set status to Completion(UpdateEmpty(result, V)).
        // Note: according to the specification, UpdateEmpty should be
        // performed only when an abrupt completion (throw here) happens in the
        // stmt evaluation. But! UpdateEmpty is effectively only a stack pop
        // when result value exists, and in catch handling we _always_ have a
        // result value. Thus, the UpdateEmpty here has zero effect except that
        // it takes care of removing V from the stack, which would otherwise be
        // leaked here.
        ctx.add_instruction(Instruction::UpdateEmpty);
        // i. Set the running execution context's LexicalEnvironment to oldEnv.
        // Note: the jump target has already returned to the old environment.
        match iteration_kind {
            // ii. If iteratorKind is ASYNC,
            IterationKind::AsyncIterate => {
                // return ? AsyncIteratorClose(iteratorRecord, status).
                ctx.add_instruction(Instruction::AsyncIteratorCloseWithError);
                // If AsyncIteratorCloseWithError ends up performing an Await then
                // it will have added the thrown error into the stack: we need to
                // rethrow it manually.
                ctx.add_instruction(Instruction::PopExceptionJumpTarget);
                ctx.add_instruction(Instruction::Store);
            }
            // iii. If iterationKind is ENUMERATE, then
            IterationKind::Iterate => {
                // 1. Assert: iterationKind is ITERATE.
                // 2. Return ? IteratorClose(iteratorRecord, status).
                ctx.add_instruction(Instruction::IteratorCloseWithError);
            }
            // iv. Else,
            IterationKind::Enumerate => {
                // 1. Return ? status.
            }
        }
        // Note: we pop the jump_to_iterator_pop_on_error catch handler here.
        ctx.add_instruction(Instruction::PopExceptionJumpTarget);
        ctx.set_jump_target_here(jump_to_iterator_pop_on_error);
        ctx.add_instruction(Instruction::IteratorPop);
        ctx.add_instruction(Instruction::Throw);
        // Note: the catch handling is a dead-end; control flow will never
        // continue past this line.
    }

    // l. If LoopContinues(result, labelSet) is false, then
    // i. If iterationKind is ENUMERATE, then
    // 1. Return ? UpdateEmpty(result, V).
    // ii. Else,
    // 1. Assert: iterationKind is ITERATE.
    // 2. Set status to Completion(UpdateEmpty(result, V)).
    // 4. Return ? IteratorClose(iteratorRecord, status).
    // 3. If iteratorKind is ASYNC, return ? AsyncIteratorClose(iteratorRecord, status).
    r#loop.exit(ctx, continue_target);
    v.forget(ctx);

    // On break
    let jump_over_return_v = if ctx.is_unreachable() {
        None
    } else {
        Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
    };
    // ### See above: this is the "done is true" path.
    // d. Let done be ? IteratorComplete(nextResult).
    // e. If done is true, return V.
    ctx.set_jump_target_here(jump_to_done);
    ctx.add_instruction(Instruction::Store);
    if let Some(jump_over_return_v) = jump_over_return_v {
        ctx.set_jump_target_here(jump_over_return_v);
    }
    result
}

fn get_for_statement_left_hand_side_kind<'gc>(
    left: &ast::ForStatementLeft,
    uninitialized_bound_names: &mut Vec<String<'gc>>,
    ctx: &mut CompileContext<'_, '_, 'gc, '_>,
) -> LeftHandSideKind {
    match left {
        ast::ForStatementLeft::VariableDeclaration(var_decl) => {
            if var_decl.kind.is_lexical() {
                var_decl.bound_names(&mut |binding_identifier| {
                    if variable_escapes_scope(ctx, binding_identifier) {
                        uninitialized_bound_names
                            .push(ctx.create_string(binding_identifier.name.as_str()));
                    }
                });
                LeftHandSideKind::LexicalBinding
            } else {
                LeftHandSideKind::VarBinding
            }
        }
        ast::ForStatementLeft::ArrayAssignmentTarget(_)
        | ast::ForStatementLeft::AssignmentTargetIdentifier(_)
        | ast::ForStatementLeft::ComputedMemberExpression(_)
        | ast::ForStatementLeft::ObjectAssignmentTarget(_)
        | ast::ForStatementLeft::PrivateFieldExpression(_)
        | ast::ForStatementLeft::StaticMemberExpression(_) => LeftHandSideKind::Assignment,
        #[cfg(feature = "typescript")]
        ast::ForStatementLeft::TSNonNullExpression(_)
        | ast::ForStatementLeft::TSAsExpression(_)
        | ast::ForStatementLeft::TSSatisfiesExpression(_) => LeftHandSideKind::Assignment,
        #[cfg(not(feature = "typescript"))]
        ast::ForStatementLeft::TSAsExpression(_)
        | ast::ForStatementLeft::TSNonNullExpression(_)
        | ast::ForStatementLeft::TSSatisfiesExpression(_) => unreachable!(),
        ast::ForStatementLeft::TSTypeAssertion(_) => unreachable!(),
    }
}

impl<'a, 's, 'gc, 'scope> CompileLabelledEvaluation<'a, 's, 'gc, 'scope>
    for ast::ForInStatement<'s>
{
    type Output = StatementResult<'gc>;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    ) -> Self::Output {
        let mut uninitialized_bound_names = vec![];

        let lhs_kind =
            get_for_statement_left_hand_side_kind(&self.left, &mut uninitialized_bound_names, ctx);

        // for-in loops have a path to  skip the entire ForIn/OfBodyEvaluation
        // and just return an empty Break result (which will break the closest
        // labelled statement and turn into undefined).
        let (iterator, key_result) = match for_in_of_head_evaluation(
            ctx,
            uninitialized_bound_names,
            &self.right,
            IterationKind::Enumerate,
        ) {
            Ok(v) => v,
            Err(e) => return ControlFlow::Break(e.into()),
        };
        let _result = for_in_of_body_evaluation(
            ctx,
            &self.left,
            &self.body,
            IterationKind::Enumerate,
            lhs_kind,
            label_set.as_deref(),
            iterator.on_abrupt_exit(),
        );
        iterator.exit(ctx);
        ctx.set_jump_target_here(key_result.unwrap());
        ControlFlow::Continue(StatementContinue::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileLabelledEvaluation<'a, 's, 'gc, 'scope>
    for ast::ForOfStatement<'s>
{
    type Output = StatementResult<'gc>;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    ) -> Self::Output {
        let mut uninitialized_bound_names = vec![];

        let lhs_kind =
            get_for_statement_left_hand_side_kind(&self.left, &mut uninitialized_bound_names, ctx);

        let iteration_kind = if self.r#await {
            IterationKind::AsyncIterate
        } else {
            IterationKind::Iterate
        };

        let (iterator, key_result) = match for_in_of_head_evaluation(
            ctx,
            uninitialized_bound_names,
            &self.right,
            iteration_kind,
        ) {
            Ok(v) => v,
            Err(e) => {
                return ControlFlow::Break(e.into());
            }
        };
        // ForIn/OfHeadEvaluation should never return a jump for ITERATE or
        // ASYNC-ITERATE.
        debug_assert!(key_result.is_none());
        let result = for_in_of_body_evaluation(
            ctx,
            &self.left,
            &self.body,
            iteration_kind,
            lhs_kind,
            label_set.as_deref(),
            iterator.on_abrupt_exit(),
        );
        iterator.exit(ctx);
        result
    }
}
