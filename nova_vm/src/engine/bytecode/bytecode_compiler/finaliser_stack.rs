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

use super::{CompileContext, JumpIndex};

#[derive(Debug, Clone)]
pub(super) struct ControlFlowFinallyEntry<'a> {
    pub(super) continues: Vec<(JumpIndex, Option<&'a LabelIdentifier<'a>>)>,
    pub(super) breaks: Vec<(JumpIndex, Option<&'a LabelIdentifier<'a>>)>,
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

    pub(super) fn is_break_target_for(&self, label: Option<&'a LabelIdentifier<'a>>) -> bool {
        match self {
            ControlFlowStackEntry::LabelledStatement { label: l, .. } => {
                label.map_or(false, |label| l.name == label.name)
            }
            ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
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
                label.map_or(false, |label| l.name == label.name)
            }
            ControlFlowStackEntry::LexicalScope
            | ControlFlowStackEntry::VariableScope
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

    pub(super) fn compile_exit(&self, instructions: &mut Vec<u8>) {
        match self {
            ControlFlowStackEntry::LabelledStatement { .. } => {
                // Labelled statements don't need finalisation.
            }
            ControlFlowStackEntry::LexicalScope => {
                instructions.push(Instruction::ExitDeclarativeEnvironment.as_u8());
            }
            ControlFlowStackEntry::VariableScope => {
                instructions.push(Instruction::ExitVariableEnvironment.as_u8());
            }
            ControlFlowStackEntry::CatchBlock { .. } => {
                eprintln!("FinaliserStack Catch");
                instructions.push(Instruction::PopExceptionJumpTarget.as_u8());
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
                instructions.push(Instruction::PopExceptionJumpTarget.as_u8());
                instructions.push(Instruction::IteratorClose.as_u8());
            }
            ControlFlowStackEntry::AsyncIterator { .. } => {
                // Async iterators have to be closed and the "return" function
                // result, if any, awaited.
                instructions.push(Instruction::PopExceptionJumpTarget.as_u8());
                instructions.push(Instruction::AsyncIteratorClose.as_u8());
                instructions.push(Instruction::Await.as_u8());
            }
        }
    }
}

impl ControlFlowSwitchEntry {
    pub(super) fn compile(self, break_target: JumpIndex, ctx: &mut CompileContext) {
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
        ctx: &mut CompileContext,
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
