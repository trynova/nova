// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{
    CompileContext, CompileEvaluation, CompileLabelledEvaluation, Instruction, JumpIndex,
    is_reference,
};
use crate::ecmascript::types::{String, Value};
use oxc_ast::ast::{self, BindingPatternKind, ForStatementLeft};
use oxc_ecmascript::BoundNames;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IterationKind {
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

#[derive(Debug, PartialEq, Eq)]
enum IteratorKind {
    Sync,
    Async,
}

fn for_in_of_head_evaluation<'s, 'gc>(
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    uninitialized_bound_names: Vec<String<'gc>>,
    expr: &'s ast::Expression<'s>,
    iteration_kind: IterationKind,
) -> Option<JumpIndex> {
    // 1. Let oldEnv be the running execution context's LexicalEnvironment.
    // 2. If uninitializedBoundNames is not empty, then
    if !uninitialized_bound_names.is_empty() {
        // a. Assert: uninitializedBoundNames has no duplicate entries.
        // b. Let newEnv be NewDeclarativeEnvironment(oldEnv).
        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
        ctx.current_lexical_depth += 1;

        // c. For each String name of uninitializedBoundNames, do
        for name in uninitialized_bound_names.iter() {
            // i. Perform ! newEnv.CreateMutableBinding(name, false).
            ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, *name);
        }
        // d. Set the running execution context's LexicalEnvironment to newEnv.
    }
    // 3. Let exprRef be Completion(Evaluation of expr).
    expr.compile(ctx);
    // 4. Set the running execution context's LexicalEnvironment to oldEnv.
    if !uninitialized_bound_names.is_empty() {
        ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        ctx.current_lexical_depth -= 1;
    }
    // 5. Let exprValue be ? GetValue(? exprRef).
    if is_reference(expr) {
        ctx.add_instruction(Instruction::GetValue);
    }
    // 6. If iterationKind is ENUMERATE, then
    match iteration_kind {
        IterationKind::Enumerate => {
            // a. If exprValue is either undefined or null, then
            // Add a copy to stack.
            ctx.add_instruction(Instruction::LoadCopy);
            ctx.add_instruction(Instruction::IsNullOrUndefined);
            let jump_over_undefined_or_null =
                ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
            // i. Return Completion Record { [[Type]]: BREAK, [[Value]]: EMPTY, [[Target]]: EMPTY }.
            // Remove the copy added above.
            ctx.add_instruction(Instruction::Store);
            // And override with undefined.
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            let return_break_completion_record =
                ctx.add_instruction_with_jump_slot(Instruction::Jump);
            ctx.set_jump_target_here(jump_over_undefined_or_null);
            // Load back the copy from above.
            ctx.add_instruction(Instruction::Store);
            // b. Let obj be ! ToObject(exprValue).
            // c. Let iterator be EnumerateObjectProperties(obj).
            // d. Let nextMethod be ! GetV(iterator, "next").
            // e. Return the Iterator Record { [[Iterator]]: iterator, [[NextMethod]]: nextMethod, [[Done]]: false }.
            ctx.add_instruction(Instruction::EnumerateObjectProperties);
            // Note: iteratorKind is SYNC
            Some(return_break_completion_record)
        }
        // 7. Else,
        // a. Assert: iterationKind is either ITERATE or ASYNC-ITERATE.
        IterationKind::AsyncIterate => {
            // b. If iterationKind is ASYNC-ITERATE, let iteratorKind be ASYNC.
            // d. Return ? GetIterator(exprValue, iteratorKind).
            ctx.add_instruction(Instruction::GetIteratorAsync);
            None
        }
        IterationKind::Iterate => {
            // c. Else, let iteratorKind be SYNC.
            // d. Return ? GetIterator(exprValue, iteratorKind).
            ctx.add_instruction(Instruction::GetIteratorSync);
            None
        }
    }
}

enum AssignmentPattern<'a> {
    ArrayAssignmentTarget(&'a ast::ArrayAssignmentTarget<'a>),
    ObjectAssignmentTarget(&'a ast::ObjectAssignmentTarget<'a>),
}

fn for_in_of_body_evaluation<'s>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    lhs: &'s ast::ForStatementLeft<'s>,
    stmt: &'s ast::Statement<'s>,
    // In the spec, keyResult contains the iteratorRecord.
    // For us that is on the iterator stack but we do have a potential jump to
    // the end from an undefined or null for-in iterator target value.
    key_result: Option<JumpIndex>,
    iteration_kind: IterationKind,
    lhs_kind: LeftHandSideKind,
    mut label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
) {
    // TODO: Any labelled breaks or continues targeting higher-up loops need to
    // jump to (Async)IteratorClose below and then jump onwards towards the
    // original labelled statement. This requires overwriting labelled jump
    // targets in the context for the duration of the loop and then rethreading
    // them to jump on their merry way after the close call, before finally
    // returning the original labelled jump targets into the context.
    let previous_jump_target = ctx.push_new_jump_target(label_set.as_deref_mut());

    // 1. If iteratorKind is not present, set iteratorKind to SYNC.
    let iterator_kind = match iteration_kind {
        IterationKind::Enumerate | IterationKind::Iterate => IteratorKind::Sync,
        IterationKind::AsyncIterate => IteratorKind::Async,
    };
    // 2. Let oldEnv be the running execution context's LexicalEnvironment.
    // 3. Let V be undefined.
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
        Some(match lhs {
            ast::ForStatementLeft::ArrayAssignmentTarget(lhs) => {
                AssignmentPattern::ArrayAssignmentTarget(lhs)
            }
            ast::ForStatementLeft::ObjectAssignmentTarget(lhs) => {
                AssignmentPattern::ObjectAssignmentTarget(lhs)
            }
            _ => unreachable!(),
        })
    } else {
        None
    };

    // 6. Repeat,
    let loop_start = ctx.get_jump_index_to_here();
    // a. Let nextResult be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
    // b. If iteratorKind is ASYNC, set nextResult to ? Await(nextResult).
    // c. If nextResult is not an Object, throw a TypeError exception.
    // d. Let done be ? IteratorComplete(nextResult).
    // e. If done is true, return V.
    // f. Let nextValue be ? IteratorValue(nextResult).
    // Note: stepping the iterator happens "outside" the loop in a sense;
    // errors thrown by the iterator itself do not cause the iterator to be
    // closed so there's no jump to iterator close on error here.
    let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue);
    let jump_to_iterator_close_on_error = if iteration_kind == IterationKind::Enumerate {
        // Enumerate simply rethrows errors
        None
    } else {
        Some(ctx.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget))
    };
    let pushed_exception_jump_target = jump_to_iterator_close_on_error.is_some();
    let mut entered_declarative_environment = false;
    // g. If lhsKind is either ASSIGNMENT or VAR-BINDING, then
    match lhs_kind {
        LeftHandSideKind::Assignment | LeftHandSideKind::VarBinding => {
            // i. If destructuring is true, then
            if destructuring {
                // 1. If lhsKind is ASSIGNMENT, then
                if lhs_kind == LeftHandSideKind::Assignment {
                    // a. Let status be Completion(DestructuringAssignmentEvaluation of assignmentPattern with argument nextValue).
                    match assignment_pattern.unwrap() {
                        AssignmentPattern::ArrayAssignmentTarget(lhs) => {
                            lhs.compile(ctx);
                        }
                        AssignmentPattern::ObjectAssignmentTarget(lhs) => {
                            lhs.compile(ctx);
                        }
                    }
                } else {
                    // 2. Else,
                    // a. Assert: lhsKind is VAR-BINDING.
                    assert_eq!(lhs_kind, LeftHandSideKind::VarBinding);
                    // b. Assert: lhs is a ForBinding.
                    // c. Let status be Completion(BindingInitialization of lhs with arguments nextValue and undefined).
                    let ForStatementLeft::VariableDeclaration(decl) = &lhs else {
                        unreachable!()
                    };
                    assert_eq!(decl.declarations.len(), 1);
                    let declaration = decl.declarations.first().unwrap();
                    ctx.add_instruction(Instruction::Load);
                    match &declaration.id.kind {
                        BindingPatternKind::ObjectPattern(pattern) => pattern.compile(ctx),
                        BindingPatternKind::ArrayPattern(pattern) => pattern.compile(ctx),
                        BindingPatternKind::BindingIdentifier(_)
                        | BindingPatternKind::AssignmentPattern(_) => unreachable!(),
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
                        let identifier =
                            String::from_str(ctx.agent, binding_identifier.name.as_str(), ctx.gc);
                        ctx.add_instruction_with_identifier(
                            Instruction::ResolveBinding,
                            identifier,
                        );
                    }
                    ast::ForStatementLeft::AssignmentTargetIdentifier(identifier_reference) => {
                        let identifier =
                            String::from_str(ctx.agent, identifier_reference.name.as_str(), ctx.gc);
                        ctx.add_instruction_with_identifier(
                            Instruction::ResolveBinding,
                            identifier,
                        );
                    }
                    ForStatementLeft::ComputedMemberExpression(expr) => {
                        ctx.add_instruction(Instruction::Load);
                        expr.compile(ctx);
                        ctx.add_instruction(Instruction::Store);
                    }
                    ForStatementLeft::StaticMemberExpression(expr) => {
                        ctx.add_instruction(Instruction::Load);
                        expr.compile(ctx);
                        ctx.add_instruction(Instruction::Store);
                    }
                    ForStatementLeft::PrivateFieldExpression(_expr) => {
                        todo!("PrivateFieldExpression")
                    }
                    // Note: Assignments are handled above so these are
                    // unreachable.
                    ForStatementLeft::ArrayAssignmentTarget(_)
                    | ForStatementLeft::ObjectAssignmentTarget(_)
                    | ForStatementLeft::TSAsExpression(_)
                    | ForStatementLeft::TSSatisfiesExpression(_)
                    | ForStatementLeft::TSNonNullExpression(_)
                    | ForStatementLeft::TSTypeAssertion(_) => unreachable!(),
                }

                // 2. If lhsRef is an abrupt completion, then
                // a. Let status be lhsRef.
                // 3. Else,
                // a. Let status be Completion(PutValue(lhsRef.[[Value]], nextValue)).
                ctx.add_instruction(Instruction::PutValue);
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
                    if !entered_declarative_environment {
                        // Optimization: Only enter declarative environment if
                        // bound names exist.
                        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
                        ctx.current_lexical_depth += 1;

                        entered_declarative_environment = true;
                    }
                    let identifier =
                        String::from_str(ctx.agent, binding_identifier.name.as_str(), ctx.gc);
                    ctx.add_instruction_with_identifier(
                        if lhs.kind.is_const() {
                            Instruction::CreateImmutableBinding
                        } else {
                            Instruction::CreateMutableBinding
                        },
                        identifier,
                    );
                });
                // 1. Let status be
                //    Completion(ForDeclarationBindingInitialization of lhs
                //    with arguments nextValue and iterationEnv).
                assert!(!ctx.lexical_binding_state);
                ctx.lexical_binding_state = true;
                lhs.compile(ctx);
                ctx.lexical_binding_state = false;
            } else {
                // vii. Else,
                lhs.bound_names(&mut |binding_identifier| {
                    // iii. Let iterationEnv be NewDeclarativeEnvironment(oldEnv).
                    // iv. Perform ForDeclarationBindingInstantiation of lhs with argument iterationEnv.
                    // v. Set the running execution context's LexicalEnvironment to iterationEnv.
                    // 1. Assert: lhs binds a single name.
                    assert!(!entered_declarative_environment);
                    ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
                    ctx.current_lexical_depth += 1;
                    entered_declarative_environment = true;

                    // 2. Let lhsName be the sole element of the BoundNames of lhs.
                    let lhs_name =
                        String::from_str(ctx.agent, binding_identifier.name.as_str(), ctx.gc);
                    ctx.add_instruction_with_identifier(
                        if lhs.kind.is_const() {
                            Instruction::CreateImmutableBinding
                        } else {
                            Instruction::CreateMutableBinding
                        },
                        lhs_name,
                    );

                    // 3. Let lhsRef be ! ResolveBinding(lhsName).
                    ctx.add_instruction_with_identifier(Instruction::ResolveBinding, lhs_name);
                    // 4. Let status be Completion(InitializeReferencedBinding(lhsRef, nextValue)).
                    ctx.add_instruction(Instruction::InitializeReferencedBinding)
                });
            }
        }
    }
    // i. If status is an abrupt completion, then ...
    // j. Let result be Completion(Evaluation of stmt).
    stmt.compile(ctx);

    if entered_declarative_environment {
        ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        ctx.current_lexical_depth -= 1;
    }

    // k. Set the running execution context's LexicalEnvironment to oldEnv.
    // l. Corollary: If LoopContinues(result, labelSet) is true, then
    //    jump to loop start.
    let jump_target = ctx.take_current_jump_target(label_set);
    if pushed_exception_jump_target {
        // Note: If we've pushed an exception handler then we have to reverse
        // those changes before we continue our loop. Thus we put point
        // continue jumps here.
        for continue_entry in jump_target.continues {
            ctx.set_jump_target_here(continue_entry);
        }

        if pushed_exception_jump_target {
            ctx.add_instruction(Instruction::PopExceptionJumpTarget);
        }
    } else {
        // If the loop doesn't push an exception handlers then continues inside
        // the loop can simply jump directly to the loop start.
        for continue_entry in jump_target.continues {
            ctx.set_jump_target(continue_entry, loop_start.clone());
        }
    }

    // Note: this is the final fall-through jump to start.
    // TODO: Load V back from stack and compare with result, store.
    ctx.add_jump_instruction_to_index(Instruction::Jump, loop_start);

    // Note: this block is here for handling of exceptions in the loop binding
    // evaluation; these need to perform (Async)IteratorClose. ENUMERATE
    // iteration does not need this as its handling would simply rethrow.
    if let Some(jump_to_iterator_close_on_error) = jump_to_iterator_close_on_error {
        ctx.set_jump_target_here(jump_to_iterator_close_on_error);
        // i. Set the running execution context's LexicalEnvironment to oldEnv.
        // Note: the jump target has already returned to the old environment.
        // ii. If iteratorKind is ASYNC, return ? AsyncIteratorClose(iteratorRecord, status).
        if iterator_kind == IteratorKind::Async {
            ctx.add_instruction(Instruction::AsyncIteratorCloseWithError);
            // If AsyncIteratorCloseWithError ends up performing an Await then
            // it will have added the thrown error into the stack: we need to
            // rethrow it manually.
            ctx.add_instruction(Instruction::PopExceptionJumpTarget);
            ctx.add_instruction(Instruction::Store);
            ctx.add_instruction(Instruction::Throw);
        } else {
            // iv. Else,
            // 1. Assert: iterationKind is ITERATE.
            assert!(iteration_kind == IterationKind::Iterate);
            // 2. Return ? IteratorClose(iteratorRecord, status).
            ctx.add_instruction(Instruction::IteratorCloseWithError);
        }
        // Note: these instructions are a dead end; VM control flow will never
        // continue past this line.
    }

    // l. If LoopContinues(result, labelSet) is false, then
    for break_entry in jump_target.breaks {
        ctx.set_jump_target_here(break_entry);
    }
    // When breaking out of the loop, we need to pop the exception target.
    // Note that the declarative environment is automatically handled by our
    // break jump generation.
    if pushed_exception_jump_target {
        ctx.add_instruction(Instruction::PopExceptionJumpTarget);
    }
    // i. If iterationKind is ENUMERATE, then
    if iteration_kind == IterationKind::Enumerate {
        // 1. Return ? UpdateEmpty(result, V).
        // TODO: This is probably a no-op.
    } else {
        // ii. Else,
        // 1. Assert: iterationKind is ITERATE.
        // 2. Set status to Completion(UpdateEmpty(result, V)).
        // TODO: This is probably a no-op.
        // 3. If iteratorKind is ASYNC, return ? AsyncIteratorClose(iteratorRecord, status).
        if iterator_kind == IteratorKind::Async {
            ctx.add_instruction(Instruction::AsyncIteratorClose);
        } else {
            // 4. Return ? IteratorClose(iteratorRecord, status).
            ctx.add_instruction(Instruction::IteratorClose);
        }
    }

    // m. If result.[[Value]] is not EMPTY, set V to result.[[Value]].
    ctx.set_jump_target_here(jump_to_end);
    if let Some(key_result) = key_result {
        ctx.set_jump_target_here(key_result)
    }

    ctx.return_jump_target(previous_jump_target);
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
                    uninitialized_bound_names.push(String::from_str(
                        ctx.agent,
                        binding_identifier.name.as_str(),
                        ctx.gc,
                    ));
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
        ast::ForStatementLeft::TSAsExpression(_)
        | ast::ForStatementLeft::TSNonNullExpression(_)
        | ast::ForStatementLeft::TSSatisfiesExpression(_)
        | ast::ForStatementLeft::TSTypeAssertion(_) => unreachable!(),
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::ForInStatement<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        let mut uninitialized_bound_names = vec![];

        let lhs_kind =
            get_for_statement_left_hand_side_kind(&self.left, &mut uninitialized_bound_names, ctx);

        let key_result = for_in_of_head_evaluation(
            ctx,
            uninitialized_bound_names,
            &self.right,
            IterationKind::Enumerate,
        );
        for_in_of_body_evaluation(
            ctx,
            &self.left,
            &self.body,
            key_result,
            IterationKind::Enumerate,
            lhs_kind,
            label_set,
        );
    }
}

impl<'s> CompileLabelledEvaluation<'s> for ast::ForOfStatement<'s> {
    fn compile_labelled(
        &'s self,
        mut label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) {
        let previous_jump_target = ctx.push_new_jump_target(label_set.as_deref_mut());

        let mut uninitialized_bound_names = vec![];

        let lhs_kind =
            get_for_statement_left_hand_side_kind(&self.left, &mut uninitialized_bound_names, ctx);

        let iteration_kind = if self.r#await {
            IterationKind::AsyncIterate
        } else {
            IterationKind::Iterate
        };

        let key_result =
            for_in_of_head_evaluation(ctx, uninitialized_bound_names, &self.right, iteration_kind);
        // ForIn/OfHeadEvaluation should never return a jump for ITERATE or
        // ASYNC-ITERATE.
        debug_assert!(key_result.is_none());
        for_in_of_body_evaluation(
            ctx,
            &self.left,
            &self.body,
            None,
            iteration_kind,
            lhs_kind,
            label_set,
        );

        ctx.return_jump_target(previous_jump_target);
    }
}
