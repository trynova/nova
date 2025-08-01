// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::{self, LabelIdentifier, RegExpFlags, Statement};
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        builtins::{
            ordinary::{caches::PropertyLookupCache, shape::ObjectShape},
            regexp::RegExp,
        },
        execution::Agent,
        syntax_directed_operations::function_definitions::CompileFunctionBodyData,
        types::{BigInt, Number, PropertyKey, String, Value},
    },
    engine::{
        Executable, FunctionExpression, Instruction,
        bytecode::{
            bytecode_compiler::finaliser_stack::{
                compile_array_destructuring_exit, compile_if_statement_exit, compile_loop_exit,
                compile_sync_iterator_exit,
            },
            executable::ArrowFunctionExpression,
        },
        context::NoGcScope,
    },
};

use super::{
    executable_context::ExecutableContext,
    finaliser_stack::{
        ControlFlowFinallyEntry, ControlFlowStackEntry, compile_async_iterator_exit,
        compile_iterator_pop,
    },
    function_declaration_instantiation,
};

pub type IndexType = u16;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NamedEvaluationParameter {
    /// Name is in the result register.
    ///
    /// The name can be clobbered by the named evaluation.
    Result,
    /// Name is at the top of the stack.
    ///
    /// The name must not be clobbered by the named evaluation.
    Stack,
}

pub(crate) struct JumpTarget {
    /// Depth of the lexical of the jump target.
    ///
    /// This is used to determine how many ExitDeclarativeEnvironment
    /// instructions are needed before jumping to this target from a continue
    /// or break statement.
    pub(super) depth: u32,
    /// `continue;` statements that target this jump target.
    pub(crate) continues: Vec<JumpIndex>,
    /// `break;` statements that target this jump target.
    pub(crate) breaks: Vec<JumpIndex>,
}

impl JumpTarget {
    pub(super) fn new(depth: u32) -> Self {
        Self {
            depth,
            continues: vec![],
            breaks: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(transparent)]
pub(crate) struct JumpIndex {
    pub(crate) index: usize,
}

/// GeneratorKind of the currently compiled code.
///
/// This affects generator yield and return behaviour.
#[derive(PartialEq, Eq)]
pub(crate) enum GeneratorKind {
    Sync,
    Async,
}

/// Context for bytecode compilation.
///
/// The lifetimes on this context are:
/// - `'agent`: The lifetime of the Agent, which owns the heap.
/// - `'script`: The lifetime of the oxc Program struct which contains the AST.
/// - `'gc`: The garbage collector marker lifetime, needed for tracking garbage
///   collected data lifetime.
/// - `'scope`: The Javascript scope marker lifetime, only here because `gc`
///   tracks it.
pub(crate) struct CompileContext<'agent, 'script, 'gc, 'scope> {
    executable: ExecutableContext<'agent, 'gc, 'scope>,
    /// NamedEvaluation name parameter
    pub(super) name_identifier: Option<NamedEvaluationParameter>,
    /// If true, indicates that all bindings being created are lexical.
    ///
    /// Otherwise, all bindings being created are variable scoped.
    pub(super) lexical_binding_state: bool,
    /// `?.` chain jumps that were present in a chain expression.
    pub(super) optional_chains: Option<Vec<JumpIndex>>,
    /// In a `(a?.b).unbind()?.bind(gc.nogc()).()` chain the evaluation of `(a?.b)` must be considered a
    /// reference.
    pub(super) is_call_optional_chain_this: bool,
    /// Stores data needed to generate control flow graph transition points.
    control_flow_stack: Vec<ControlFlowStackEntry<'script>>,
    /// GeneratorKind of the currently compiled code.
    ///
    /// This affects generator yield and return behaviour.
    generator_kind: Option<GeneratorKind>,
}

impl<'agent, 'script, 'gc, 'scope> CompileContext<'agent, 'script, 'gc, 'scope> {
    pub(crate) fn new(
        agent: &'agent mut Agent,
        gc: NoGcScope<'gc, 'scope>,
    ) -> CompileContext<'agent, 'script, 'gc, 'scope> {
        CompileContext {
            executable: ExecutableContext::new(agent, gc),
            name_identifier: None,
            lexical_binding_state: false,
            optional_chains: None,
            is_call_optional_chain_this: false,
            control_flow_stack: Vec::new(),
            generator_kind: None,
        }
    }

    /// Set the compile context to be a type of generator.
    ///
    /// This affects generator yield and return behaviour.
    pub(crate) fn set_generator_kind(&mut self, kind: GeneratorKind) {
        self.generator_kind = Some(kind);
    }

    /// Returns true if we're compiling a generator (sync or async).
    pub(crate) fn is_generator(&self) -> bool {
        self.generator_kind.is_some()
    }

    /// Returns true if we're compiling an asynchronous generator.
    pub(crate) fn is_async_generator(&self) -> bool {
        self.generator_kind == Some(GeneratorKind::Async)
    }

    /// Get exclusive access to the Agent, and the GC scope, through the context.
    pub(crate) fn get_agent_and_gc(&mut self) -> (&mut Agent, NoGcScope<'gc, 'scope>) {
        self.executable.get_agent_and_gc()
    }

    /// Get shared access to the Agent through the context.
    pub(crate) fn get_agent(&self) -> &Agent {
        self.executable.get_agent()
    }

    /// Get exclusive access to the Agent through the context as mutable.
    pub(crate) fn get_agent_mut(&mut self) -> &mut Agent {
        self.executable.get_agent_mut()
    }

    /// Create a property lookup cache for a JavaScript String.
    pub(crate) fn create_property_lookup_cache(
        &mut self,
        identifier: PropertyKey<'gc>,
    ) -> PropertyLookupCache<'gc> {
        self.executable.create_property_lookup_cache(identifier)
    }

    /// Create a new JavaScript BigInt from a bigint literal and radix.
    pub(crate) fn create_bigint(&mut self, literal: &str, radix: u32) -> BigInt<'gc> {
        self.executable.create_bigint(literal, radix)
    }

    /// Create a new JavaScript Number from an f64.
    pub(crate) fn create_number(&mut self, value: f64) -> Number<'gc> {
        self.executable.create_number(value)
    }

    /// Create a new JavaScript PropertyKey from a string literal.
    pub(crate) fn create_property_key(&mut self, literal: &str) -> PropertyKey<'gc> {
        self.executable.create_property_key(literal)
    }

    /// Create a new JavaScript RegExp from a RegExp literal and flags.
    pub(crate) fn create_regexp(&mut self, literal: &str, flags: RegExpFlags) -> RegExp<'gc> {
        self.executable.create_regexp(literal, flags)
    }

    /// Create a new JavaScript String from a string literal.
    pub(crate) fn create_string(&mut self, literal: &str) -> String<'gc> {
        self.executable.create_string(literal)
    }

    /// Create a new JavaScript String from an owned string.
    pub(super) fn create_string_from_owned(&mut self, owned: std::string::String) -> String<'gc> {
        self.executable.create_string_from_owned(owned)
    }

    /// Create a new JavaScript String from an owned Wtf8Buf.
    pub(super) fn create_string_from_wtf8_buf(&mut self, buf: Wtf8Buf) -> String<'gc> {
        self.executable.create_string_from_wtf8_buf(buf)
    }

    /// Enter a labelled statement.
    pub(super) fn enter_label(&mut self, label: &'script LabelIdentifier<'script>) {
        self.control_flow_stack
            .push(ControlFlowStackEntry::LabelledStatement {
                label,
                incoming_control_flows: None,
            });
    }

    /// Exit a labelled statement.
    pub(super) fn exit_label(&mut self) {
        let Some(ControlFlowStackEntry::LabelledStatement {
            label: _,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(&mut self.executable);
        }
    }

    /// Enter a lexical scope.
    pub(super) fn enter_lexical_scope(&mut self) {
        self.add_instruction(Instruction::EnterDeclarativeEnvironment);
        self.control_flow_stack
            .push(ControlFlowStackEntry::LexicalScope);
    }

    /// Exit a lexical scope.
    pub(super) fn exit_lexical_scope(&mut self) {
        matches!(
            self.control_flow_stack.pop(),
            Some(ControlFlowStackEntry::LexicalScope)
        );
        if self.is_unreachable() {
            // OPTIMISATION: We don't need to add exit handling if this line is
            // unreachable.
            return;
        }
        self.add_instruction(Instruction::ExitDeclarativeEnvironment);
    }

    /// Enter a private environment scope.
    pub(super) fn enter_private_scope(&mut self, private_name_count: usize) {
        self.add_instruction_with_immediate(
            Instruction::EnterPrivateEnvironment,
            private_name_count,
        );
        self.control_flow_stack
            .push(ControlFlowStackEntry::PrivateScope);
    }

    /// Enter a private environment scope.
    pub(super) fn exit_private_scope(&mut self) {
        matches!(
            self.control_flow_stack.pop(),
            Some(ControlFlowStackEntry::PrivateScope)
        );
        if self.is_unreachable() {
            // OPTIMISATION: We don't need to add exit handling if this line is
            // unreachable.
            return;
        }
        self.add_instruction(Instruction::ExitPrivateEnvironment);
    }

    /// Enter a class static initialiser.
    pub(super) fn enter_class_static_block(&mut self) {
        self.add_instruction(Instruction::EnterClassStaticElementEnvironment);
        self.control_flow_stack
            .push(ControlFlowStackEntry::LexicalScope);
        self.control_flow_stack
            .push(ControlFlowStackEntry::VariableScope);
    }

    /// Exit a lexical scope.
    pub(super) fn exit_class_static_block(&mut self) {
        matches!(
            self.control_flow_stack.pop(),
            Some(ControlFlowStackEntry::VariableScope)
        );
        matches!(
            self.control_flow_stack.pop(),
            Some(ControlFlowStackEntry::LexicalScope)
        );
        if self.is_unreachable() {
            // OPTIMISATION: We don't need to add exit handling if this line is
            // unreachable.
            return;
        }
        self.add_instruction(Instruction::ExitVariableEnvironment);
        self.add_instruction(Instruction::ExitDeclarativeEnvironment);
    }

    /// Enter a try-catch block.
    #[must_use]
    pub(super) fn enter_try_catch_block(&mut self) -> JumpIndex {
        let jump_to_catch =
            self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget);
        self.control_flow_stack
            .push(ControlFlowStackEntry::CatchBlock);
        jump_to_catch
    }

    /// Exit a try-catch block.
    pub(super) fn exit_try_catch_block(&mut self) {
        let Some(ControlFlowStackEntry::CatchBlock) = self.control_flow_stack.pop() else {
            unreachable!()
        };
        if self.is_unreachable() {
            // OPTIMISATION: We don't need to add exit handling if this line is
            // unreachable.
            return;
        }
        self.add_instruction(Instruction::PopExceptionJumpTarget);
    }

    /// Enter a try-finally block.
    pub(super) fn enter_try_finally_block(&mut self) {
        let jump_to_catch =
            self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget);
        self.control_flow_stack
            .push(ControlFlowStackEntry::TryFinallyBlock {
                jump_to_catch,
                incoming_control_flows: None,
            });
    }

    /// Exit a try-finally block.
    pub(super) fn exit_try_finally_block(
        &mut self,
        block: &'script ast::BlockStatement<'script>,
        jump_over_catch_blocks: Option<JumpIndex>,
    ) {
        let Some(ControlFlowStackEntry::TryFinallyBlock {
            jump_to_catch,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        // Compile all finally-block variants here.
        // If we have a jump coming from the end of our try block, jumping over
        // the catch block then we'll put all of our abrupt completion paths
        // here, after the catch block, to make sure the normal control flow
        // only jumps once.
        if let Some(jump_over_catch_blocks) = jump_over_catch_blocks {
            let jump_to_finally_from_catch_end = if !self.is_unreachable() {
                // If the preceding catch block's end isn't unreachable, we
                // have to make sure that any fallthrough from it goes into the
                // normal finally-flow.
                Some(self.add_instruction_with_jump_slot(Instruction::Jump))
            } else {
                None
            };
            self.compile_abrupt_finally_blocks(block, jump_to_catch, incoming_control_flows);
            // Then compile the normal version: we jump over the catch blocks
            // and other abrupt completions, landing here to perform the
            // finally-work before continuing from the try-catch-finally block.
            self.set_jump_target_here(jump_over_catch_blocks);
            if let Some(jump_to_finally_from_catch_end) = jump_to_finally_from_catch_end {
                self.set_jump_target_here(jump_to_finally_from_catch_end);
            }
            // First we have to pop off the special finally-exception target.
            self.add_instruction(Instruction::PopExceptionJumpTarget);

            // Then we compile the finally-block.
            self.enter_finally_block(false);
            block.compile(self);
            self.exit_finally_block();
            // And continue on our merry way!
        } else {
            // No preceding catch-block exists or the try-block's end is
            // unreachable: this means that the normal code flow is right here,
            // right now, coming from either the lonely try-block or from the
            // end of the catch-block. If we're currently sitting in an
            // unreachable location then it means the normal version of the
            // finally block is not needed at all! Let's check for that.
            let jump_over_abrupt_completions = if !self.is_unreachable() {
                // We are reachable, so let's compile the normal finally-block
                // version here.
                self.add_instruction(Instruction::PopExceptionJumpTarget);
                self.enter_finally_block(false);
                block.compile(self);
                self.exit_finally_block();

                // We need to jump over the abrupt completion handling blocks,
                // unless of course we're now unreachable here!
                if !self.is_unreachable() {
                    Some(self.add_instruction_with_jump_slot(Instruction::Jump))
                } else {
                    None
                }
            } else {
                // We are unreachable indeed! Since there is no control flow
                // coming here, we don't need to add any control flow going out
                // of here either.
                None
            };

            self.compile_abrupt_finally_blocks(block, jump_to_catch, incoming_control_flows);
            if let Some(jump_over_abrupt_completions) = jump_over_abrupt_completions {
                self.set_jump_target_here(jump_over_abrupt_completions);
            }
        }
    }

    fn compile_abrupt_finally_blocks(
        &mut self,
        block: &'script ast::BlockStatement<'script>,
        jump_to_catch: JumpIndex,
        incoming_control_flows: Option<Box<ControlFlowFinallyEntry<'script>>>,
    ) {
        // TODO: there's a possible optimisation here to find an incoming
        // control flow from a directly preceding Jump instruction, and
        // generating that control flow block here directly.
        // A catch-version of finally stores the caught error and rethrows
        // it after performing the finally-work.
        self.set_jump_target_here(jump_to_catch);

        // Compile the finally-block...
        self.enter_finally_block(true);
        block.compile(self);
        self.exit_finally_block();
        // ... and rethrow the error.
        let end_of_finally_block_is_unreachable = self.is_unreachable();
        if !end_of_finally_block_is_unreachable {
            self.add_instruction(Instruction::Throw);
        }

        // Then, for each incoming control flow (break or continue), we need to
        // generate a finally block for them as well.
        if let Some(incoming_control_flows) = incoming_control_flows {
            for (break_source, label) in incoming_control_flows.breaks {
                // Make the original break jump here.
                self.set_jump_target_here(break_source);
                // Exit from the finally-block's grasp.
                self.add_instruction(Instruction::PopExceptionJumpTarget);

                // Compile the finally-block...
                self.enter_finally_block(false);
                block.compile(self);
                self.exit_finally_block();

                // ... then send the break on to its real target.
                if !end_of_finally_block_is_unreachable {
                    self.compile_break(label);
                }
            }

            for (continue_source, label) in incoming_control_flows.continues {
                // Make the original continue jump here.
                self.set_jump_target_here(continue_source);
                // Exit from the finally-block's grasp.
                self.add_instruction(Instruction::PopExceptionJumpTarget);

                // Compile the finally-block...
                self.enter_finally_block(false);
                block.compile(self);
                self.exit_finally_block();

                // ... then send the continue on to its real target.
                if !end_of_finally_block_is_unreachable {
                    self.compile_continue(label);
                }
            }

            if !incoming_control_flows.returns.is_empty() {
                for return_source in incoming_control_flows.returns {
                    self.set_jump_target_here(return_source);
                }
                self.add_instruction(Instruction::PopExceptionJumpTarget);

                // Compile the finally-block...
                self.enter_finally_block(true);
                block.compile(self);
                self.exit_finally_block();

                // ... then send the return on to its real target.
                if !end_of_finally_block_is_unreachable {
                    // Note: at this point we shouldn't inject a new Await here
                    // anymore, hence we pass false as `has_param`.
                    self.compile_return(false);
                }
            }
        }
    }

    /// Enter an if-statement; `UpdateEmpty(V, undefined)` must be performed at
    /// the end of the statement.
    pub(super) fn enter_if_statement(&mut self) {
        self.control_flow_stack
            .push(ControlFlowStackEntry::IfStatement);
    }

    /// Enter an if-statement; `UpdateEmpty(V, undefined)` must be performed at
    /// the end of the statement.
    ///
    /// Note: if we statically know a result exists, then the UpdateEmpty work
    /// can be skipped. The `has_result` boolean parameter is used for this.
    pub(super) fn exit_if_statement(&mut self, has_result: bool) {
        let Some(ControlFlowStackEntry::IfStatement) = self.control_flow_stack.pop() else {
            unreachable!()
        };
        if !self.is_unreachable() && !has_result {
            compile_if_statement_exit(&mut self.executable);
        }
    }

    /// Enter a finally block; a result value is present in the result register
    /// and must be stored onto the stack. When the block is exited, the result
    /// must be popped off the stack (and taken as the result of the
    /// fall-through case).
    pub(super) fn enter_finally_block(&mut self, has_result: bool) {
        self.control_flow_stack
            .push(ControlFlowStackEntry::FinallyBlock);
        if has_result {
            // We can load our result onto the stack directly.
            self.add_instruction(Instruction::Load);
        } else {
            // Our result might be empty currently; loading directly would
            // crash.
            self.add_instruction_with_constant(Instruction::LoadConstant, Value::Undefined);
            self.add_instruction(Instruction::UpdateEmpty);
            self.add_instruction(Instruction::Load);
        }
    }

    /// Exit a finally block; a result value is present on the stack and must
    /// be returned into the result register in the fall-through case.
    pub(super) fn exit_finally_block(&mut self) {
        let Some(ControlFlowStackEntry::FinallyBlock) = self.control_flow_stack.pop() else {
            unreachable!()
        };
        if !self.is_unreachable() {
            self.add_instruction(Instruction::Store);
        }
    }

    /// Enter a for, for-in, or while loop.
    #[must_use]
    pub(super) fn enter_loop(
        &mut self,
        label_set: Option<Vec<&'script LabelIdentifier<'script>>>,
    ) -> JumpIndex {
        self.control_flow_stack.push(ControlFlowStackEntry::Loop {
            label_set,
            incoming_control_flows: None,
        });
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Exit a for, for-in, or while loop.
    pub(super) fn exit_loop(&mut self, continue_target: JumpIndex) {
        let Some(ControlFlowStackEntry::Loop {
            label_set: _,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(
                continue_target,
                compile_loop_exit,
                &mut self.executable,
            );
        } else if !self.is_unreachable() {
            compile_loop_exit(&mut self.executable);
        }
    }

    /// Enter a switch block.
    pub(super) fn enter_switch(
        &mut self,
        label_set: Option<Vec<&'script LabelIdentifier<'script>>>,
    ) {
        self.control_flow_stack.push(ControlFlowStackEntry::Switch {
            label_set,
            incoming_control_flows: None,
        });
    }

    /// Exit a switch block.
    pub(super) fn exit_switch(&mut self) {
        let Some(ControlFlowStackEntry::Switch {
            label_set: _,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(&mut self.executable);
        }
    }

    /// Get an enumerator and push it into the iterator stack, and set up a
    /// catch block to pop the iterator stack on thrown error.
    pub(super) fn push_enumerator(&mut self) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::IteratorStackEntry);
        self.add_instruction(Instruction::EnumerateObjectProperties);
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Get a sync iterator and push it into the iterator stack, and set up a
    /// catch block to pop the iterator stack on thrown error.
    pub(super) fn push_sync_iterator(&mut self) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::IteratorStackEntry);
        self.add_instruction(Instruction::GetIteratorSync);
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Get an async iterator and push it into the iterator stack, and set up a
    /// catch block to pop the iterator stack on thrown error.
    pub(super) fn push_async_iterator(&mut self) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::IteratorStackEntry);
        self.add_instruction(Instruction::GetIteratorAsync);
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Pop the iterator stack and remove its catch handler.
    pub(super) fn pop_iterator_stack(&mut self) {
        let Some(ControlFlowStackEntry::IteratorStackEntry) = self.control_flow_stack.pop() else {
            unreachable!()
        };
        if !self.is_unreachable() {
            compile_iterator_pop(&mut self.executable);
        }
    }

    /// Enter a for-of loop or array destructuring.
    #[must_use]
    pub(super) fn enter_iterator(
        &mut self,
        label_set: Option<Vec<&'script LabelIdentifier<'script>>>,
    ) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::Iterator {
                label_set,
                incoming_control_flows: None,
            });
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Exit a for-of loop or an array destructuring. For array destructuring,
    /// the continue target should be None.
    pub(super) fn exit_iterator(&mut self, continue_target: JumpIndex) {
        let Some(ControlFlowStackEntry::Iterator {
            label_set: _,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        // Note: if we have a continue target it means that this is a for-of
        // loop where UpdateEmpty is performed as the last step in the work
        // before closing the iterator.
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(
                continue_target,
                compile_sync_iterator_exit,
                &mut self.executable,
            );
        } else if !self.is_unreachable() {
            compile_sync_iterator_exit(&mut self.executable);
        }
    }

    /// Get an iterator for array destructuring and push it into the iterator
    /// stack, and set up a catch block to close the iterator on thrown error.
    #[must_use]
    pub(super) fn enter_array_destructuring(&mut self) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::ArrayDestructuring);
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Exit array destructuring.
    pub(super) fn exit_array_destructuring(&mut self) {
        let Some(ControlFlowStackEntry::ArrayDestructuring) = self.control_flow_stack.pop() else {
            unreachable!()
        };
        if !self.is_unreachable() {
            compile_array_destructuring_exit(&mut self.executable);
        }
    }

    /// Enter a for-await-of loop.
    #[must_use]
    pub(super) fn enter_async_iterator(
        &mut self,
        label_set: Option<Vec<&'script LabelIdentifier<'script>>>,
    ) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::AsyncIterator {
                label_set,
                incoming_control_flows: None,
            });
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Exit a for-await-of loop.
    pub(super) fn exit_async_iterator(&mut self, continue_target: JumpIndex) {
        let Some(ControlFlowStackEntry::AsyncIterator {
            label_set: _,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(
                continue_target,
                compile_async_iterator_exit,
                &mut self.executable,
            );
        } else if !self.is_unreachable() {
            compile_async_iterator_exit(&mut self.executable);
        }
    }

    /// Compile a break statement targeting optional label.
    ///
    /// This helper injects all necessary finalisers at the break statement
    /// site before jumping to the target. If user-defined finally-blocks are
    /// present in the finaliser stack, the method instead jumps to a
    /// finally-block that ends with a jump to the final target.
    pub(super) fn compile_break(&mut self, label: Option<&'script LabelIdentifier<'script>>) {
        let mut has_result = false;
        for entry in self.control_flow_stack.iter_mut().rev() {
            if entry.is_break_target_for(label) {
                // Stop iterating the stack when we find our target and push
                // the current instruction pointer as a break source for our
                // target. Label is pushed in as well because finally-blocks
                // need to know about labelled breaks and continues.
                let break_source = self
                    .executable
                    .add_instruction_with_jump_slot(Instruction::Jump);
                entry.add_break_source(label, break_source);
                return;
            }
            // Compile the exit of each intermediate control flow stack entry.
            entry.compile_exit(&mut self.executable, has_result);
            has_result = has_result || entry.sets_result_during_exit();
        }
    }

    /// Compile a continue statement targeting optional label.
    ///
    /// This helper injects all necessary finalisers at the continue statement
    /// site before jumping to the target. If user-defined finally-blocks are
    /// present in the finaliser stack, the method instead jumps to a
    /// finally-block that ends with a jump to the final target.
    pub(super) fn compile_continue(&mut self, label: Option<&'script LabelIdentifier<'script>>) {
        let mut has_result = false;
        for entry in self.control_flow_stack.iter_mut().rev() {
            if entry.is_continue_target_for(label) {
                // Stop iterating the stack when we find our target and push
                // the current instruction pointer as a break source for our
                // target. Label is pushed in as well because finally-blocks
                // need to know about labelled breaks and continues.

                let continue_source = self
                    .executable
                    .add_instruction_with_jump_slot(Instruction::Jump);
                entry.add_continue_source(label, continue_source);
                break;
            }
            // Compile the exit of each intermediate control flow stack entry.
            entry.compile_exit(&mut self.executable, has_result);
            has_result = has_result || entry.sets_result_during_exit();
        }
    }

    /// Compile a return statement.
    ///
    /// This helper injects all necessary finalisers at the return site before
    /// performing the final return. If user-defined finally-blocks are
    /// present in the finaliser stack, the method instead jumps to a
    /// finally-block that ends with a return.
    pub(super) fn compile_return(&mut self, has_param: bool) {
        if self.is_async_generator() && has_param {
            // AsyncGenerators perform an Await before wrapping the result in a
            // ReturnCompletion and returning it.
            // Because this happens before the ReturnCompletion wrapping, it
            // means that the Await instruction should be injected before the
            // finalisers run.
            self.add_instruction(Instruction::Await);
        }
        let (stack_contains_finally_blocks, stack_contains_finalisers) = self
            .control_flow_stack
            .iter()
            .fold((false, false), |acc, entry| {
                (
                    acc.0 || entry.is_return_target(),
                    acc.1 || entry.requires_return_finalisation(false),
                )
            });
        if !stack_contains_finalisers {
            // If there are no finalisers to be called, then we can just jump
            // straight to Return. This is the common case.
            self.add_instruction(Instruction::Return);
            return;
        } else if !stack_contains_finally_blocks {
            // If there are no finally-blocks to be visited, then we can just
            // directly inline all the finalisers at the return site. Note that
            // we can skip exiting declarative environments but must exit catch
            // blocks so as to ensure they don't interfere with closing of
            // iterators.
            for entry in self.control_flow_stack.iter().rev() {
                if entry.requires_return_finalisation(true) {
                    // Note: return always sets a value to the result register.
                    entry.compile_exit(&mut self.executable, true);
                }
            }
            self.add_instruction(Instruction::Return);
            return;
        }
        // The rare case: We have at least one finally-block in the stack. In
        // this case we have to perform normal unwinding of the stack until the
        // first finally-block.
        for entry in self.control_flow_stack.iter_mut().rev() {
            if entry.is_return_target() {
                let return_source = self
                    .executable
                    .add_instruction_with_jump_slot(Instruction::Jump);
                entry.add_return_source(return_source);
                return;
            }
            // Note: return always sets a value to the result register.
            entry.compile_exit(&mut self.executable, true);
        }
        unreachable!()
    }

    /// Returns true if the current instruction pointer is unreachable.
    pub(crate) fn is_unreachable(&self) -> bool {
        self.executable.is_unreachable()
    }

    /// Compile a function body into the current context.
    ///
    /// This is useful when the function body is part of a larger whole, namely
    /// with class constructors.
    pub(crate) fn compile_function_body(&mut self, data: CompileFunctionBodyData<'script>) {
        if self.executable.agent.options.print_internals {
            eprintln!();
            eprintln!("=== Compiling Function ===");
            eprintln!();
        }

        function_declaration_instantiation::instantiation(
            self,
            data.params,
            data.body,
            data.is_strict,
            data.is_lexical,
        );

        if self.is_generator() {
            // Perform a Yield after FunctionDeclarationInstantiation; this is
            // when the Generator object actually gets created.
            self.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            self.add_instruction(Instruction::Yield);
        }

        // SAFETY: Script referred by the Function uniquely owns the Program
        // and the body buffer does not move under any circumstances during
        // heap operations.
        let body: &[Statement] = unsafe { core::mem::transmute(data.body.statements.as_slice()) };

        self.compile_statements(body);
    }

    pub(crate) fn compile_statements(&mut self, body: &'script [Statement<'script>]) {
        let iter = body.iter();

        for stmt in iter {
            stmt.compile(self);
        }
    }

    pub(crate) fn do_implicit_return(&mut self) {
        if !self.is_unreachable() {
            if self.is_async_generator() {
                self.add_instruction(Instruction::Await);
            }
            self.add_instruction(Instruction::Return);
        }
    }

    pub(crate) fn finish(self) -> Executable<'gc> {
        debug_assert!(self.control_flow_stack.is_empty());
        self.executable.finish()
    }

    pub(super) fn add_instruction(&mut self, instruction: Instruction) {
        self.executable.add_instruction(instruction);
    }

    pub(super) fn add_instruction_with_jump_slot(&mut self, instruction: Instruction) -> JumpIndex {
        self.executable.add_instruction_with_jump_slot(instruction)
    }

    pub(super) fn add_jump_instruction_to_index(
        &mut self,
        instruction: Instruction,
        jump_index: JumpIndex,
    ) {
        self.executable
            .add_jump_instruction_to_index(instruction, jump_index);
    }

    pub(super) fn get_jump_index_to_here(&mut self) -> JumpIndex {
        self.executable.get_jump_index_to_here()
    }

    pub(super) fn add_identifier(&mut self, identifier: String<'gc>) -> usize {
        self.executable.add_identifier(identifier)
    }

    pub(super) fn add_instruction_with_immediate(
        &mut self,
        instruction: Instruction,
        immediate: usize,
    ) {
        self.executable
            .add_instruction_with_immediate(instruction, immediate);
    }

    pub(super) fn add_instruction_with_constant(
        &mut self,
        instruction: Instruction,
        constant: impl Into<Value<'gc>>,
    ) {
        self.executable
            .add_instruction_with_constant(instruction, constant);
    }

    pub(super) fn add_instruction_with_identifier(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
    ) {
        self.executable
            .add_instruction_with_identifier(instruction, identifier);
    }

    pub(super) fn add_instruction_with_cache(
        &mut self,
        instruction: Instruction,
        cache: PropertyLookupCache<'gc>,
    ) {
        self.executable
            .add_instruction_with_cache(instruction, cache);
    }

    pub(super) fn add_instruction_with_identifier_and_constant(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
        constant: impl Into<Value<'gc>>,
    ) {
        self.executable
            .add_instruction_with_identifier_and_constant(instruction, identifier, constant);
    }

    pub(super) fn add_instruction_with_identifier_and_immediate(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
        immediate: usize,
    ) {
        self.executable
            .add_instruction_with_identifier_and_immediate(instruction, identifier, immediate);
    }

    pub(super) fn add_instruction_with_immediate_and_immediate(
        &mut self,
        instruction: Instruction,
        immediate1: usize,
        immediate2: usize,
    ) {
        self.executable
            .add_instruction_with_immediate_and_immediate(instruction, immediate1, immediate2);
    }

    pub(super) fn add_instruction_with_function_expression(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
    ) {
        self.executable
            .add_instruction_with_function_expression(instruction, function_expression);
    }

    /// Add an Instruction that takes a function expression and an immediate
    /// as its bytecode parameters.
    ///
    /// Returns the function expression's index.
    pub(super) fn add_instruction_with_function_expression_and_immediate(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
        immediate: usize,
    ) -> IndexType {
        self.executable
            .add_instruction_with_function_expression_and_immediate(
                instruction,
                function_expression,
                immediate,
            )
    }

    pub(super) fn add_instruction_with_shape(
        &mut self,
        instruction: Instruction,
        shape: ObjectShape<'gc>,
    ) {
        self.executable
            .add_instruction_with_shape(instruction, shape);
    }

    pub(super) fn add_arrow_function_expression(
        &mut self,
        arrow_function_expression: ArrowFunctionExpression,
    ) {
        self.executable
            .add_arrow_function_expression(arrow_function_expression);
    }

    pub(super) fn set_jump_target_here(&mut self, jump: JumpIndex) {
        self.executable.set_jump_target_here(jump);
    }

    pub(super) fn set_jump_target(&mut self, source: JumpIndex, target: JumpIndex) {
        self.executable.set_jump_target(source, target);
    }

    pub(super) fn get_next_class_initializer_index(&self) -> IndexType {
        self.executable.get_next_class_initializer_index()
    }

    pub(super) fn set_function_expression_bytecode(
        &mut self,
        index: IndexType,
        executable: Executable<'gc>,
    ) {
        self.executable
            .set_function_expression_bytecode(index, executable);
    }

    pub(super) fn add_class_initializer_bytecode(
        &mut self,
        executable: Executable<'gc>,
        has_constructor_parent: bool,
    ) {
        self.executable
            .add_class_initializer_bytecode(executable, has_constructor_parent);
    }

    pub(super) fn add_class_initializer(&mut self, has_constructor_parent: bool) {
        self.executable
            .add_class_initializer(has_constructor_parent);
    }
}

pub(crate) trait CompileEvaluation<'a, 's, 'gc, 'scope> {
    type Output;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output;
}

pub(crate) trait CompileLabelledEvaluation<'a, 's, 'gc, 'scope> {
    type Output;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'a, 's, 'gc, 'scope>,
    ) -> Self::Output;
}
