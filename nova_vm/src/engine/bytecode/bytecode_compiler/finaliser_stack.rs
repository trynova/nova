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

use crate::engine::Instruction;

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
    /// A try-finally-block was entered.
    FinallyBlock {
        jump_to_catch: JumpIndex,
        incoming_control_flows: Option<Box<ControlFlowFinallyEntry<'a>>>,
    },
    /// Traditional for-, while-, or for-in loop. Does not require finalisation.
    Loop {
        label_set: Option<Vec<&'a LabelIdentifier<'a>>>,
        incoming_control_flows: Option<Box<ControlFlowLoopEntry>>,
    },
    /// Switch block. Does not require finalisation.
    Switch {
        label_set: Option<Vec<&'a LabelIdentifier<'a>>>,
        incoming_control_flows: Option<Box<ControlFlowSwitchEntry>>,
    },
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
            | ControlFlowStackEntry::CatchBlock { .. } => false,
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
            | ControlFlowStackEntry::CatchBlock { .. }
            | ControlFlowStackEntry::Switch { .. } => false,
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
        match self {
            ControlFlowStackEntry::LabelledStatement { .. }
            | ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
            | ControlFlowStackEntry::PrivateScope
            | ControlFlowStackEntry::CatchBlock { .. }
            | ControlFlowStackEntry::Switch { .. }
            | ControlFlowStackEntry::Loop { .. }
            | ControlFlowStackEntry::Iterator { .. }
            | ControlFlowStackEntry::AsyncIterator { .. } => false,
            // Finally-block needs to intercept return.
            ControlFlowStackEntry::FinallyBlock { .. } => true,
        }
    }

    /// Returns true if the entry requires finalisation on return.
    pub(super) fn requires_return_finalisation(&self, will_perform_other_finalisers: bool) -> bool {
        match self {
            ControlFlowStackEntry::LabelledStatement { .. }
            | ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
            | ControlFlowStackEntry::PrivateScope
            | ControlFlowStackEntry::Loop { .. }
            | ControlFlowStackEntry::Switch { .. } => false,
            // User-controlled finally-blocks, and iterator closes must be
            // called on return.
            ControlFlowStackEntry::Iterator { .. }
            | ControlFlowStackEntry::AsyncIterator { .. }
            | ControlFlowStackEntry::FinallyBlock { .. } => true,
            // Catch blocks don't require finalisation on
            // their own, but they can affect iterator and finally block work
            // if those throw errors.
            ControlFlowStackEntry::CatchBlock => will_perform_other_finalisers,
        }
    }

    pub(super) fn compile_exit<'gc>(&self, executable: &mut ExecutableContext) {
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
            ControlFlowStackEntry::CatchBlock { .. } => {
                executable.add_instruction(Instruction::PopExceptionJumpTarget);
            }
            ControlFlowStackEntry::FinallyBlock { .. } => {
                // Finally-blocks should always intercept incoming work.
                unreachable!()
            }
            ControlFlowStackEntry::Loop { .. } | ControlFlowStackEntry::Switch { .. } => {
                // Loops and switches don't need finalisation.
            }
            ControlFlowStackEntry::Iterator { .. } => {
                // Iterators have to be closed and their catch handler popped.
                executable.add_instruction(Instruction::PopExceptionJumpTarget);
                executable.add_instruction(Instruction::IteratorClose);
            }
            ControlFlowStackEntry::AsyncIterator { .. } => {
                compile_async_iterator_exit(executable);
            }
        }
    }
}

/// Helper method to compile async iterator exit handling.
///
/// Async iterators have to be closed and the "return" function result, if any,
/// awaited. Any errors thrown during this process are immediately rethrown,
/// and if the process finishes successfully then the original result will be
/// restored as the result value.
pub(super) fn compile_async_iterator_exit(executable: &mut ExecutableContext) {
    let error_message = executable.create_string("iterator.return() returned a non-object value");
    executable.add_instruction(Instruction::PopExceptionJumpTarget);
    executable.add_instruction(Instruction::AsyncIteratorClose);
    // If async iterator close returned a Value, then it'll push the previous
    // result value into the stack. We should await the returned value, verify
    // that it is an object, and then return the original result.
    executable.add_instruction_with_identifier(Instruction::VerifyIsObject, error_message);
    executable.add_instruction(Instruction::Store);
}

impl ControlFlowSwitchEntry {
    pub(super) fn compile(self, break_target: JumpIndex, ctx: &mut ExecutableContext) {
        for break_source in self.breaks {
            ctx.set_jump_target(break_source, break_target.clone());
        }
    }
}

impl ControlFlowLoopEntry {
    pub(super) fn compile(
        self,
        continue_target: JumpIndex,
        break_target: JumpIndex,
        ctx: &mut ExecutableContext,
    ) {
        for break_source in self.breaks {
            ctx.set_jump_target(break_source, break_target.clone());
        }
        for continue_source in self.continues {
            ctx.set_jump_target(continue_source, continue_target.clone());
        }
    }

    pub(super) fn has_breaks(&self) -> bool {
        !self.breaks.is_empty()
    }
}
