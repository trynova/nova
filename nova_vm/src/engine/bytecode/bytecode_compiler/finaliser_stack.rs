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

use oxc_ast::ast;

pub(super) enum FinalisableBlock<'a> {
    /// A lexical scope was entered. Exiting it requires calling the
    /// ExitDeclarativeEnvironment instruction.
    LexicalScope,
    /// A variable scope was entered. Exiting it requires calling the
    /// ExitVariableEnvironment instruction.
    VariableScope,
    /// A for-of iterator loop was entered. Exiting it requires calling the
    /// IteratorClose instruction.
    IteratorScope,
    /// A for-await-of iterator loop was entered. Exiting it requires calling
    /// the AsyncIteratorClose instruction and conditionally running Await,
    /// among other things.
    AsyncIteratorScope,
    /// A try-finally-block was entered. Exiting it requires generating the
    /// bytecode for the finally-block's contents.
    FinallyBlock(&'a ast::BlockStatement<'a>),
}

pub(super) struct ControlFlowLoopEntry<'a> {
    continues: Option<Box<Vec<FinalisableBlock<'a>>>>,
    breaks: Option<Box<Vec<FinalisableBlock<'a>>>>,
}

pub(super) enum ControlFlowStackEntry<'a> {
    /// A lexical scope was entered.
    LexicalScope,
    /// A try-finally-block was entered.
    FinallyBlock(&'a ast::BlockStatement<'a>),
    /// Traditional for-, while-, or for-in loop. Does not require finalisation.
    Loop(ControlFlowLoopEntry<'a>),
    /// Synchronous for-of loop. Requires closing the iterator on exit.
    Iterator(ControlFlowLoopEntry<'a>),
    /// Asynchronous for-await-of loop. Requires closing the iterator and
    /// awaiting the "return" call result on exit.
    AsyncIterator(ControlFlowLoopEntry<'a>),
}
