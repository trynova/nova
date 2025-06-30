// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Contains code for performing lexical scope entry and exit during bytecode
//! compilation, including generating proper finaliser blocks for various scope
//! exits.
//!
//! This includes:
//! - Entering and exiting declarative scopes.
//! - Entering and exiting variable scopes.
//! - Entering and exiting try-catch blocks.
//! - Closing iterators on for-of loop exit.
//! - Visiting finally blocks on try-finally block exit.

use oxc_ast::ast::LabelIdentifier;

use crate::{ecmascript::types::Value, engine::Instruction};

use super::{JumpIndex, executable_context::ExecutableContext};

#[derive(Debug, Clone)]
pub(super) struct ControlFlowFinallyEntry<'a> {
    pub(super) continues: Vec<(JumpIndex, Option<&'a LabelIdentifier<'a>>)>,
    pub(super) breaks: Vec<(JumpIndex, Option<&'a LabelIdentifier<'a>>)>,
    pub(super) returns: Vec<JumpIndex>,
}

#[derive(Debug, Clone)]
pub(super) struct ControlFlowLoopEntry {
    continues: Vec<JumpIndex>,
    breaks: Vec<JumpIndex>,
}

#[derive(Debug, Clone)]
pub(super) struct ControlFlowSwitchEntry {
    breaks: Vec<JumpIndex>,
}

#[derive(Debug, Clone)]
pub(super) enum ControlFlowStackEntry<'a> {
    /// A labelled statement was entered.
    LabelledStatement {
        label: &'a LabelIdentifier<'a>,
        incoming_control_flows: Option<Box<ControlFlowSwitchEntry>>,
    },
    /// A lexical scope was entered.
    LexicalScope,
    /// A variable scope was entered.
    VariableScope,
    /// A private environment was scoped.
    PrivateScope,
    /// A try-catch block was entered.
    CatchBlock,
    /// An if-statement was entered.
    IfStatement,
    /// A try-finally-block was entered.
    FinallyBlock {
        jump_to_catch: JumpIndex,
        incoming_control_flows: Option<Box<ControlFlowFinallyEntry<'a>>>,
    },
    /// Traditional for or while loop. Does not require finalisation.
    Loop {
        label_set: Option<Vec<&'a LabelIdentifier<'a>>>,
        incoming_control_flows: Option<Box<ControlFlowLoopEntry>>,
    },
    /// Switch block. Does not require finalisation.
    Switch {
        label_set: Option<Vec<&'a LabelIdentifier<'a>>>,
        incoming_control_flows: Option<Box<ControlFlowSwitchEntry>>,
    },
    /// An iterator stack entry. Requires popping the iterator from the
    /// iterator stack on exit.
    IteratorStackEntry,
    /// An iterator stack entry for array destructuring. Requires closing and
    /// popping the iterator stack on exit.
    ArrayDestructuring,
    /// Synchronous for-of loop. Requires closing the iterator on exit.
    Iterator {
        label_set: Option<Vec<&'a LabelIdentifier<'a>>>,
        incoming_control_flows: Option<Box<ControlFlowLoopEntry>>,
    },
    /// Asynchronous for-await-of loop. Requires closing the iterator and
    /// awaiting the "return" call result, if any, on exit.
    AsyncIterator {
        label_set: Option<Vec<&'a LabelIdentifier<'a>>>,
        incoming_control_flows: Option<Box<ControlFlowLoopEntry>>,
    },
}

impl<'a> ControlFlowStackEntry<'a> {
    pub(super) fn add_break_source(
        &mut self,
        label: Option<&'a LabelIdentifier<'a>>,
        break_source: JumpIndex,
    ) {
        match self {
            ControlFlowStackEntry::FinallyBlock {
                incoming_control_flows,
                ..
            } => {
                if let Some(incoming_control_flows) = incoming_control_flows {
                    incoming_control_flows.breaks.push((break_source, label));
                } else {
                    *incoming_control_flows = Some(Box::new(ControlFlowFinallyEntry {
                        continues: vec![],
                        breaks: vec![(break_source, label)],
                        returns: vec![],
                    }));
                }
            }
            ControlFlowStackEntry::Loop {
                incoming_control_flows,
                ..
            }
            | ControlFlowStackEntry::Iterator {
                incoming_control_flows,
                ..
            }
            | ControlFlowStackEntry::AsyncIterator {
                incoming_control_flows,
                ..
            } => {
                if let Some(incoming_control_flows) = incoming_control_flows {
                    incoming_control_flows.breaks.push(break_source);
                } else {
                    *incoming_control_flows = Some(Box::new(ControlFlowLoopEntry {
                        continues: vec![],
                        breaks: vec![break_source],
                    }));
                }
            }
            ControlFlowStackEntry::LabelledStatement {
                incoming_control_flows,
                ..
            }
            | ControlFlowStackEntry::Switch {
                incoming_control_flows,
                ..
            } => {
                if let Some(incoming_control_flows) = incoming_control_flows {
                    incoming_control_flows.breaks.push(break_source);
                } else {
                    *incoming_control_flows = Some(Box::new(ControlFlowSwitchEntry {
                        breaks: vec![break_source],
                    }));
                }
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn add_continue_source(
        &mut self,
        label: Option<&'a LabelIdentifier<'a>>,
        continue_source: JumpIndex,
    ) {
        match self {
            ControlFlowStackEntry::FinallyBlock {
                incoming_control_flows,
                ..
            } => {
                if let Some(incoming_control_flows) = incoming_control_flows {
                    incoming_control_flows
                        .continues
                        .push((continue_source, label));
                } else {
                    *incoming_control_flows = Some(Box::new(ControlFlowFinallyEntry {
                        continues: vec![(continue_source, label)],
                        breaks: vec![],
                        returns: vec![],
                    }));
                }
            }
            ControlFlowStackEntry::Loop {
                incoming_control_flows,
                ..
            }
            | ControlFlowStackEntry::Iterator {
                incoming_control_flows,
                ..
            }
            | ControlFlowStackEntry::AsyncIterator {
                incoming_control_flows,
                ..
            } => {
                if let Some(incoming_control_flows) = incoming_control_flows {
                    incoming_control_flows.continues.push(continue_source);
                } else {
                    *incoming_control_flows = Some(Box::new(ControlFlowLoopEntry {
                        continues: vec![continue_source],
                        breaks: vec![],
                    }));
                }
            }
            _ => unreachable!(),
        }
    }

    pub(super) fn add_return_source(&mut self, return_source: JumpIndex) {
        let ControlFlowStackEntry::FinallyBlock {
            incoming_control_flows,
            ..
        } = self
        else {
            unreachable!()
        };
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.returns.push(return_source);
        } else {
            *incoming_control_flows = Some(Box::new(ControlFlowFinallyEntry {
                continues: vec![],
                breaks: vec![],
                returns: vec![return_source],
            }));
        }
    }

    pub(super) fn is_break_target_for(&self, label: Option<&'a LabelIdentifier<'a>>) -> bool {
        match self {
            ControlFlowStackEntry::LabelledStatement { label: l, .. } => {
                label.is_some_and(|label| l.name == label.name)
            }
            ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
            | ControlFlowStackEntry::PrivateScope
            | ControlFlowStackEntry::CatchBlock
            | ControlFlowStackEntry::IfStatement
            | ControlFlowStackEntry::IteratorStackEntry { .. }
            | ControlFlowStackEntry::ArrayDestructuring => false,
            // Finally-block needs to intercept every break and continue.
            ControlFlowStackEntry::FinallyBlock { .. } => true,
            ControlFlowStackEntry::Loop { label_set, .. }
            | ControlFlowStackEntry::Switch { label_set, .. }
            | ControlFlowStackEntry::Iterator { label_set, .. }
            | ControlFlowStackEntry::AsyncIterator { label_set, .. } => {
                if let Some(label) = label {
                    // Labelled break only matches a breakable statement with
                    // that label.
                    let Some(label_set) = label_set else {
                        return false;
                    };
                    label_set.iter().any(|l| l.name == label.name)
                } else {
                    // Unlabelled break matches any breakable statement.
                    true
                }
            }
        }
    }

    pub(super) fn is_continue_target_for(&self, label: Option<&'a LabelIdentifier<'a>>) -> bool {
        match self {
            ControlFlowStackEntry::LabelledStatement { label: l, .. } => {
                label.is_some_and(|label| l.name == label.name)
            }
            ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
            | ControlFlowStackEntry::PrivateScope
            | ControlFlowStackEntry::IfStatement
            | ControlFlowStackEntry::CatchBlock { .. }
            | ControlFlowStackEntry::Switch { .. }
            | ControlFlowStackEntry::IteratorStackEntry
            | ControlFlowStackEntry::ArrayDestructuring => false,
            // Finally-block needs to intercept every break and continue.
            ControlFlowStackEntry::FinallyBlock { .. } => true,
            ControlFlowStackEntry::Loop { label_set, .. }
            | ControlFlowStackEntry::Iterator { label_set, .. }
            | ControlFlowStackEntry::AsyncIterator { label_set, .. } => {
                if let Some(label) = label {
                    // Labelled continue only matches a continuable statement
                    // with that label.
                    let Some(label_set) = label_set else {
                        return false;
                    };
                    label_set.iter().any(|l| l.name == label.name)
                } else {
                    // Unlabelled continue matches any continuable statement.
                    true
                }
            }
        }
    }

    /// Return cannot target any block in particular, but finally-blocks do
    /// intercept returns and thus are an indirect target for them.
    pub(super) fn is_return_target(&self) -> bool {
        // Finally-block needs to intercept return.
        matches!(self, ControlFlowStackEntry::FinallyBlock { .. })
    }

    /// Returns true if the entry requires finalisation on return.
    pub(super) fn requires_return_finalisation(&self, will_perform_other_finalisers: bool) -> bool {
        match self {
            // Exiting these cannot be observed by users.
            ControlFlowStackEntry::LabelledStatement { .. }
            | ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
            | ControlFlowStackEntry::PrivateScope
            | ControlFlowStackEntry::Loop { .. }
            | ControlFlowStackEntry::Switch { .. } => false,
            // If-statements, user-controlled finally-blocks, and iterator
            // closes must be called on return.
            ControlFlowStackEntry::IfStatement
            | ControlFlowStackEntry::ArrayDestructuring
            | ControlFlowStackEntry::Iterator { .. }
            | ControlFlowStackEntry::AsyncIterator { .. }
            | ControlFlowStackEntry::FinallyBlock { .. } => true,
            // Catch blocks and the iterator stack don't require finalisation
            // on their own, but they do affect iterator closing and finally
            // block work.
            ControlFlowStackEntry::CatchBlock | ControlFlowStackEntry::IteratorStackEntry => {
                will_perform_other_finalisers
            }
        }
    }

    /// Returns true if the entry sets a defined value to the result register
    /// in compile_exit.
    pub(super) fn sets_result_during_exit(&self) -> bool {
        matches!(
            self,
            ControlFlowStackEntry::IfStatement
                | ControlFlowStackEntry::Loop { .. }
                | ControlFlowStackEntry::Iterator { .. }
                | ControlFlowStackEntry::AsyncIterator { .. }
        )
    }

    pub(super) fn compile_exit(&self, executable: &mut ExecutableContext, has_result: bool) {
        match self {
            ControlFlowStackEntry::LabelledStatement { .. } => {
                // Labelled statements don't need finalisation.
            }
            ControlFlowStackEntry::LexicalScope => {
                executable.add_instruction(Instruction::ExitDeclarativeEnvironment);
            }
            ControlFlowStackEntry::VariableScope => {
                executable.add_instruction(Instruction::ExitVariableEnvironment);
            }
            ControlFlowStackEntry::PrivateScope => {
                executable.add_instruction(Instruction::ExitPrivateEnvironment);
            }
            ControlFlowStackEntry::IfStatement => {
                if has_result {
                    // OPTIMISATION: if we statically know we have a result,
                    // then we don't need to perform our
                    // `UpdateEmpty(V, undefined)`.
                    return;
                }
                compile_if_statement_exit(executable);
            }
            ControlFlowStackEntry::CatchBlock { .. } => {
                executable.add_instruction(Instruction::PopExceptionJumpTarget);
            }
            ControlFlowStackEntry::FinallyBlock { .. } => {
                // Finally-blocks should always intercept incoming work.
                unreachable!()
            }
            ControlFlowStackEntry::Switch { .. } => {
                // Switches don't need finalisation.
            }
            ControlFlowStackEntry::IteratorStackEntry => {
                // Enumerator loops need to pop the iterator stack.
                compile_iterator_pop(executable);
            }
            ControlFlowStackEntry::ArrayDestructuring => {
                compile_array_destructuring_exit(executable);
            }
            ControlFlowStackEntry::Loop { .. } => {
                compile_loop_exit(executable);
            }
            ControlFlowStackEntry::Iterator { .. } => {
                compile_sync_iterator_exit(executable);
            }
            ControlFlowStackEntry::AsyncIterator { .. } => {
                compile_async_iterator_exit(executable);
            }
        }
    }
}

pub(super) fn compile_iterator_pop(executable: &mut ExecutableContext) {
    executable.add_instruction(Instruction::PopExceptionJumpTarget);
    executable.add_instruction(Instruction::IteratorPop);
}

/// Helper method to compile if-statement exit handling.
///
/// If-statements have to perform `UpdateEmpty(V, undefined)` at the end of the
/// statement.
pub(super) fn compile_if_statement_exit(executable: &mut ExecutableContext) {
    executable.add_instruction_with_constant(Instruction::LoadConstant, Value::Undefined);
    executable.add_instruction(Instruction::UpdateEmpty);
}

/// Helper method to compile loop exit handling.
///
/// Loops have an exception handler for exceptional loop exit handling: that
/// needs to be removed. Next, the loop will have placed a JavaScript Value `V`
/// onto the stack: this needs to be popped and it should become our result if
/// the exit was reached with an empty result value.
pub(super) fn compile_loop_exit(executable: &mut ExecutableContext) {
    // When breaking out of a loop its exception handler needs to be removed
    // and the pushed JavaScript stack value popped.
    executable.add_instruction(Instruction::PopExceptionJumpTarget);
    executable.add_instruction(Instruction::UpdateEmpty);
}

/// Helper method to compile array destructuring iterator exit handling.
///
/// Iterators have an exception handler for exceptional loop exit handling:
/// that needs to be removed. Array destructuring iterators must always be
/// closed as well.
pub(super) fn compile_array_destructuring_exit(executable: &mut ExecutableContext) {
    executable.add_instruction(Instruction::PopExceptionJumpTarget);
    executable.add_instruction(Instruction::IteratorClose);
}

/// Helper method to compile sync iterator exit handling.
///
/// Iterators have an exception handler for exceptional loop exit handling:
/// that needs to be removed. Next, the iterator will have placed a JavaScript
/// Value `V` onto the stack: this needs to be popped and it should become our
/// result if the exit was reached with an empty result value. Finally, the
/// iterator's "return" function, if found, must be called and its result
/// ignored.
pub(super) fn compile_sync_iterator_exit(executable: &mut ExecutableContext) {
    compile_loop_exit(executable);
    executable.add_instruction(Instruction::IteratorClose);
}

/// Helper method to compile async iterator exit handling.
///
/// Iterators have an exception handler for exceptional loop exit handling:
/// that needs to be removed. Next, the iterator will have placed a JavaScript
/// Value `V` onto the stack: this needs to be popped and it should become our
/// result if the exit was reached with an empty result value. Finally, the
/// iterator's "return" function, if found, must be called and its result
/// awaited.
pub(super) fn compile_async_iterator_exit(executable: &mut ExecutableContext) {
    compile_loop_exit(executable);
    executable.add_instruction(Instruction::AsyncIteratorClose);
    // If async iterator close returned a Value, then it'll push the previous
    // result value into the stack and perform an implicit Await.
    // We should verify that the result of the await is an object, and then
    // return the original result.
    let error_message = executable.create_string("iterator.return() returned a non-object value");
    executable.add_instruction_with_identifier(Instruction::VerifyIsObject, error_message);
    executable.add_instruction(Instruction::Store);
}

impl ControlFlowSwitchEntry {
    pub(super) fn compile(self, ctx: &mut ExecutableContext) {
        // Note: iterate breaks in reverse, in case the last one is our current
        // instruction. If that is the case, we can remove the last Jump and
        // make it a fallthrough.
        for break_source in self.breaks.into_iter().rev() {
            ctx.set_jump_target_here(break_source);
        }
    }
}

impl ControlFlowLoopEntry {
    pub(super) fn compile(
        self,
        continue_target: JumpIndex,
        compile_break: impl FnOnce(&mut ExecutableContext),
        ctx: &mut ExecutableContext,
    ) {
        for continue_source in self.continues {
            ctx.set_jump_target(continue_source, continue_target.clone());
        }
        if ctx.is_unreachable() && self.breaks.is_empty() {
            return;
        }
        // Note: iterate breaks in reverse, in case the last one is our current
        // instruction. If that is the case, we can remove the last Jump and
        // make it a fallthrough.
        for break_source in self.breaks.into_iter().rev() {
            ctx.set_jump_target_here(break_source);
        }
        compile_break(ctx);
    }

    pub(super) fn has_breaks(&self) -> bool {
        !self.breaks.is_empty()
    }
}
