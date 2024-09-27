// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{is_reference, CompileContext, CompileEvaluation, Instruction, JumpIndex};
use crate::ecmascript::types::{String, Value};
use oxc_ast::{ast, syntax_directed_operations::BoundNames};

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

fn for_in_of_head_evaluation(
    ctx: &mut CompileContext,
    uninitialized_bound_names: Vec<String>,
    expr: &ast::Expression<'_>,
    iteration_kind: IterationKind,
) -> Option<JumpIndex> {
    // 1. Let oldEnv be the running execution context's LexicalEnvironment.
    // 2. If uninitializedBoundNames is not empty, then
    if !uninitialized_bound_names.is_empty() {
        // a. Assert: uninitializedBoundNames has no duplicate entries.
        // b. Let newEnv be NewDeclarativeEnvironment(oldEnv).
        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
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

fn for_in_of_body_evaluation(
    ctx: &mut CompileContext,
    lhs: &ast::ForStatementLeft<'_>,
    stmt: &ast::Statement<'_>,
    // In the spec, keyResult contains the iteratorRecord.
    // For us that is on the iterator stack but we do have a potential jump to
    // the end from an undefined or null for-in iterator target value.
    key_result: Option<JumpIndex>,
    iteration_kind: IterationKind,
    lhs_kind: LeftHandSideKind,
    // _label_set: (),
) {
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
    let _assignment_pattern = if destructuring && lhs_kind == LeftHandSideKind::Assignment {
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

    let previous_continue = ctx.current_continue.replace(vec![]);
    let previous_break = ctx.current_break.replace(vec![]);

    // 6. Repeat,
    let repeat_jump = ctx.get_jump_index_to_here();
    // a. Let nextResult be ? Call(iteratorRecord.[[NextMethod]], iteratorRecord.[[Iterator]]).
    // b. If iteratorKind is ASYNC, set nextResult to ? Await(nextResult).
    // c. If nextResult is not an Object, throw a TypeError exception.
    // d. Let done be ? IteratorComplete(nextResult).
    // e. If done is true, return V.
    // f. Let nextValue be ? IteratorValue(nextResult).
    let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue);
    let mut entered_declarative_environment = false;
    // g. If lhsKind is either ASSIGNMENT or VAR-BINDING, then
    match lhs_kind {
        LeftHandSideKind::Assignment | LeftHandSideKind::VarBinding => {
            // i. If destructuring is true, then
            if destructuring {
                // 1. If lhsKind is ASSIGNMENT, then
                if lhs_kind == LeftHandSideKind::Assignment {
                    // a. Let status be Completion(DestructuringAssignmentEvaluation of assignmentPattern with argument nextValue).
                    todo!();
                } else {
                    // 2. Else,
                    // a. Assert: lhsKind is VAR-BINDING.
                    // b. Assert: lhs is a ForBinding.
                    // c. Let status be Completion(BindingInitialization of lhs with arguments nextValue and undefined).
                    todo!();
                }
            } else {
                // ii. Else,
                // 1. Let lhsRef be Completion(Evaluation of lhs). (It may be evaluated repeatedly.)
                match lhs {
                    ast::ForStatementLeft::VariableDeclaration(decl) => {
                        assert_eq!(decl.declarations.len(), 1);
                        let declaration = decl.declarations.first().unwrap();
                        match &declaration.id.kind {
                            ast::BindingPatternKind::BindingIdentifier(binding_identifier) => {
                                let identifier =
                                    String::from_str(ctx.agent, binding_identifier.name.as_str());
                                ctx.add_instruction_with_identifier(
                                    Instruction::ResolveBinding,
                                    identifier,
                                );
                            }
                            ast::BindingPatternKind::AssignmentPattern(_)
                            | ast::BindingPatternKind::ObjectPattern(_)
                            | ast::BindingPatternKind::ArrayPattern(_) => unreachable!(),
                        }
                    }
                    ast::ForStatementLeft::AssignmentTargetIdentifier(identifier_reference) => {
                        let identifier =
                            String::from_str(ctx.agent, identifier_reference.name.as_str());
                        ctx.add_instruction_with_identifier(
                            Instruction::ResolveBinding,
                            identifier,
                        );
                    }
                    _ => unreachable!(),
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
            // iii. Let iterationEnv be NewDeclarativeEnvironment(oldEnv).
            // iv. Perform ForDeclarationBindingInstantiation of lhs with argument iterationEnv.
            lhs.bound_names(&mut |binding_identifier| {
                if !entered_declarative_environment {
                    // Optimization: Only enter declarative environment if
                    // bound names exist.
                    ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
                    entered_declarative_environment = true;
                }
                let identifier = String::from_str(ctx.agent, binding_identifier.name.as_str());
                ctx.add_instruction_with_identifier(
                    if lhs.kind.is_const() {
                        Instruction::CreateImmutableBinding
                    } else {
                        Instruction::CreateMutableBinding
                    },
                    identifier,
                );
            });
            // v. Set the running execution context's LexicalEnvironment to iterationEnv.
            // vi. If destructuring is true, then
            if destructuring {
                // 1. Let status be Completion(ForDeclarationBindingInitialization of lhs with arguments nextValue and iterationEnv).
                todo!();
            } else {
                // vii. Else,
                // 1. Assert: lhs binds a single name.
                let mut bound = false;
                lhs.bound_names(&mut |binding_identifier| {
                    assert!(!bound);
                    bound = true;
                    // 2. Let lhsName be the sole element of the BoundNames of lhs.
                    let lhs_name = String::from_str(ctx.agent, binding_identifier.name.as_str());
                    // 3. Let lhsRef be ! ResolveBinding(lhsName).
                    ctx.add_instruction_with_identifier(Instruction::ResolveBinding, lhs_name);
                    // 4. Let status be Completion(InitializeReferencedBinding(lhsRef, nextValue)).
                    ctx.add_instruction(Instruction::InitializeReferencedBinding)
                });
            }
        }
    }
    // TODO: Abrupt completion should be handled through completion handling
    // in vm.rs.
    // i. If status is an abrupt completion, then
    //      i. Set the running execution context's LexicalEnvironment to oldEnv.
    //      ii. If iteratorKind is ASYNC, return ? AsyncIteratorClose(iteratorRecord, status).
    //      iii. If iterationKind is ENUMERATE, then
    //      1. Return ? status.
    //      iv. Else,
    //      1. Assert: iterationKind is ITERATE.
    //      2. Return ? IteratorClose(iteratorRecord, status).

    // j. Let result be Completion(Evaluation of stmt).
    stmt.compile(ctx);

    // k. Set the running execution context's LexicalEnvironment to oldEnv.
    // l. Corollary: If LoopContinues(result, labelSet) is true, then
    // jump to repeat_jump.
    let own_continues = ctx.current_continue.take().unwrap();
    ctx.current_continue = previous_continue;
    if entered_declarative_environment {
        // Note: If we've entered a declarative environment then we have to
        // exit it before we continue back to repeat_jump.
        ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        for continue_entry in own_continues {
            ctx.set_jump_target_here(continue_entry);
        }
    } else {
        for continue_entry in own_continues {
            ctx.set_jump_target(continue_entry, repeat_jump.clone());
        }
    }

    // TODO: Load V back from stack and compare with result, store.
    ctx.add_jump_instruction_to_index(Instruction::Jump, repeat_jump);

    // l. If LoopContinues(result, labelSet) is false, then
    let own_breaks = ctx.current_break.take().unwrap();
    ctx.current_break = previous_break;
    for break_entry in own_breaks {
        ctx.set_jump_target_here(break_entry);
    }
    // i. If iterationKind is ENUMERATE, then
    if iteration_kind == IterationKind::Enumerate {
        // 1. Return ? UpdateEmpty(result, V).
        // TODO: This is probably a no-op.
    } else {
        // ii. Else,
        // 1. Assert: iterationKind is ITERATE.
        assert_eq!(iteration_kind, IterationKind::Iterate);
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
}

impl CompileEvaluation for ast::ForInStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let mut uninitialized_bound_names = vec![];

        let lhs_kind = match &self.left {
            ast::ForStatementLeft::VariableDeclaration(var_decl) => {
                if var_decl.kind.is_lexical() {
                    var_decl.bound_names(&mut |binding_identifier| {
                        uninitialized_bound_names.push(String::from_str(
                            ctx.agent,
                            binding_identifier.name.as_str(),
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
            | ast::ForStatementLeft::TSInstantiationExpression(_)
            | ast::ForStatementLeft::TSNonNullExpression(_)
            | ast::ForStatementLeft::TSSatisfiesExpression(_)
            | ast::ForStatementLeft::TSTypeAssertion(_) => unreachable!(),
        };

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
            // label_set,
        );
    }
}

impl CompileEvaluation for ast::ForOfStatement<'_> {
    fn compile(&self, ctx: &mut CompileContext) {
        let previous_continue = ctx.current_continue.replace(vec![]);
        let previous_break = ctx.current_break.replace(vec![]);

        let mut uninitialized_bound_names = vec![];

        let lhs_kind = match &self.left {
            ast::ForStatementLeft::VariableDeclaration(var_decl) => {
                if var_decl.kind.is_lexical() {
                    var_decl.bound_names(&mut |binding_identifier| {
                        uninitialized_bound_names.push(String::from_str(
                            ctx.agent,
                            binding_identifier.name.as_str(),
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
            | ast::ForStatementLeft::TSInstantiationExpression(_)
            | ast::ForStatementLeft::TSNonNullExpression(_)
            | ast::ForStatementLeft::TSSatisfiesExpression(_)
            | ast::ForStatementLeft::TSTypeAssertion(_) => unreachable!(),
        };

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
            // label_set,
        );

        ctx.current_break = previous_break;
        ctx.current_continue = previous_continue;
    }
}
