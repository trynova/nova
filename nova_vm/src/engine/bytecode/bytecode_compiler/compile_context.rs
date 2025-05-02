// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast;
use oxc_semantic::Semantic;

use crate::{
    ecmascript::{
        execution::Agent,
        syntax_directed_operations::function_definitions::CompileFunctionBodyData,
        types::{IntoValue, String, Value},
    },
    engine::{
        Executable, ExecutableHeapData, FunctionExpression, Instruction,
        bytecode::executable::ArrowFunctionExpression,
        context::{Bindable, NoGcScope},
    },
    heap::CreateHeapData,
};

use super::{
    finaliser_stack::{ControlFlowFinallyEntry, ControlFlowStackEntry},
    function_declaration_instantiation, is_reference,
};

pub type IndexType = u16;

#[derive(Debug, Clone, Copy)]
pub(crate) enum NamedEvaluationParameter {
    /// Name is in the result register
    Result,
    /// Name is at the top of the stack
    Stack,
    /// Name is in the reference register
    Reference,
    /// Name is at the top of the reference stack
    ReferenceStack,
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
    pub(crate) agent: &'agent mut Agent,
    pub(crate) semantic: &'gc Semantic<'static>,
    pub(crate) gc: NoGcScope<'gc, 'scope>,
    /// true if the current last instruction is a terminal instruction and no
    /// jumps point past it.
    last_instruction_is_terminal: bool,
    /// Instructions being built
    instructions: Vec<u8>,
    /// Constants being built
    constants: Vec<Value<'gc>>,
    /// Function expressions being built
    function_expressions: Vec<FunctionExpression<'gc>>,
    /// Arrow function expressions being built
    arrow_function_expressions: Vec<ArrowFunctionExpression>,
    class_initializer_bytecodes: Vec<(Option<Executable<'gc>>, bool)>,
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
}

impl<'a, 's, 'gc, 'scope> CompileContext<'a, 's, 'gc, 'scope> {
    pub(crate) fn new(
        agent: &'a mut Agent,
        semantic: &'gc Semantic<'static>,
        gc: NoGcScope<'gc, 'scope>,
    ) -> CompileContext<'a, 's, 'gc, 'scope> {
        CompileContext {
            agent,
            semantic,
            gc,
            // Note: when no instructions exist, we are indeed terminal.
            last_instruction_is_terminal: true,
            instructions: Vec::new(),
            constants: Vec::new(),
            function_expressions: Vec::new(),
            arrow_function_expressions: Vec::new(),
            class_initializer_bytecodes: Vec::new(),
            name_identifier: None,
            lexical_binding_state: false,
            optional_chains: None,
            is_call_optional_chain_this: false,
            control_flow_stack: Vec::new(),
        }
    }

    /// Enter a labelled statement.
    pub(super) fn enter_label(&mut self, label: &'s ast::LabelIdentifier<'s>) {
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
        let break_target = self.get_jump_index_to_here();
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(break_target, self);
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
        self.add_instruction(Instruction::ExitDeclarativeEnvironment);
    }

    /// Exit a lexical scope without performing the instruction.
    pub(super) fn pop_lexical_scope(&mut self) {
        matches!(
            self.control_flow_stack.pop(),
            Some(ControlFlowStackEntry::LexicalScope)
        );
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
        self.add_instruction(Instruction::ExitVariableEnvironment);
        matches!(
            self.control_flow_stack.pop(),
            Some(ControlFlowStackEntry::LexicalScope)
        );
        self.add_instruction(Instruction::ExitDeclarativeEnvironment);
    }

    /// Enter a try-catch block.
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
        self.add_instruction(Instruction::PopExceptionJumpTarget);
    }

    /// Enter a try-finally block.
    pub(super) fn enter_try_finally_block(&mut self) {
        let jump_to_catch =
            self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget);
        self.control_flow_stack
            .push(ControlFlowStackEntry::FinallyBlock {
                jump_to_catch,
                incoming_control_flows: None,
            });
    }

    /// Exit a try-finally block.
    pub(super) fn exit_try_finally_block(
        &mut self,
        block: &'s ast::BlockStatement<'s>,
        jump_over_catch_blocks: Option<JumpIndex>,
        catch_block_is_terminal: bool,
    ) {
        let Some(ControlFlowStackEntry::FinallyBlock {
            jump_to_catch,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        // Compile all finally-block variants here.
        // If we have a preceding catch block then we'll put all of our
        // abrupt completion paths after it to make sure the normal control
        // flow only jumps once.
        if let Some(jump_over_catch_blocks) = jump_over_catch_blocks {
            let jump_to_finally_from_catch_end = if !catch_block_is_terminal {
                // If the preceding catch doesn't end in a terminal
                // instruction, we have to make sure that any fallthrough from
                // it goes into the normal finally-flow.
                Some(self.add_instruction_with_jump_slot(Instruction::Jump))
            } else {
                None
            };
            self.compile_abrupt_finally_blocks(block, jump_to_catch, incoming_control_flows);
            // Then compile the normal version: we jump over the catch blocks
            // and other abrupt completions, landing here to perform the
            // finally-work before continuing from the try-catch-finally block.
            // First we have to pop off the special finally-exception target.
            self.set_jump_target_here(jump_over_catch_blocks);
            if let Some(jump_to_finally_from_catch_end) = jump_to_finally_from_catch_end {
                self.set_jump_target_here(jump_to_finally_from_catch_end);
            }
            self.add_instruction(Instruction::PopExceptionJumpTarget);
            // Then we compile the finally-block.
            block.compile(self);
            // And continue on our merry way!
        } else {
            // No preceding catch-block exists: this means that the normal
            // code flow is right here, right now. We should first compile the
            // normal version of the finally-block and then make it jump
            self.add_instruction(Instruction::PopExceptionJumpTarget);
            block.compile(self);
            // We need to jump over the abrupt completion handling blocks,
            // unless our finally block ends in a terminal instruction.
            let jump_over_abrupt_completions = if !self.is_terminal() {
                Some(self.add_instruction_with_jump_slot(Instruction::Jump))
            } else {
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
        block: &'s ast::BlockStatement<'s>,
        jump_to_catch: JumpIndex,
        incoming_control_flows: Option<Box<ControlFlowFinallyEntry<'s>>>,
    ) {
        // A catch-version of finally stores the caught error and rethrows
        // it after performing the finally-work.
        self.set_jump_target_here(jump_to_catch);
        self.add_instruction(Instruction::Load);
        // Compile the finally-block.
        block.compile(self);
        // Take the error back from the stack and rethrow.
        self.add_instruction(Instruction::Store);
        self.add_instruction(Instruction::Throw);
        // Then, for each incoming control flow (break or continue), we need to
        // generate a finally block for them as well.
        if let Some(incoming_control_flows) = incoming_control_flows {
            for (break_source, label) in incoming_control_flows.breaks {
                // Make the original break jump here.
                self.set_jump_target_here(break_source);
                // Exit from the finally-block's grasp.
                self.add_instruction(Instruction::PopExceptionJumpTarget);
                // Compile the finally-block.
                block.compile(self);
                // Then send the break on to its real target.
                self.compile_break(label);
            }

            for (continue_source, label) in incoming_control_flows.continues {
                // Make the original continue jump here.
                self.set_jump_target_here(continue_source);
                // Exit from the finally-block's grasp.
                self.add_instruction(Instruction::PopExceptionJumpTarget);
                // Compile the finally-block.
                block.compile(self);
                // Then send the continue on to its real target.
                self.compile_continue(label);
            }

            if !incoming_control_flows.returns.is_empty() {
                for return_source in incoming_control_flows.returns {
                    self.set_jump_target_here(return_source);
                }
                self.add_instruction(Instruction::PopExceptionJumpTarget);
                // Load the return result onto the stack.
                self.add_instruction(Instruction::Load);
                block.compile(self);
                // Store the return result back into the result register.
                self.add_instruction(Instruction::Store);
                self.compile_return();
            }
        }
    }

    /// Enter a for, for-in, or while loop.
    pub(super) fn enter_loop(&mut self, label_set: Option<Vec<&'s ast::LabelIdentifier<'s>>>) {
        self.control_flow_stack.push(ControlFlowStackEntry::Loop {
            label_set,
            incoming_control_flows: None,
        });
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
        let break_target = self.get_jump_index_to_here();
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(continue_target, break_target, self);
        }
    }

    /// Enter a switch block.
    pub(super) fn enter_switch(&mut self, label_set: Option<Vec<&'s ast::LabelIdentifier<'s>>>) {
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
        let break_target = self.get_jump_index_to_here();
        if let Some(incoming_control_flows) = incoming_control_flows {
            incoming_control_flows.compile(break_target, self);
        }
    }

    /// Enter a for-of loop.
    pub(super) fn enter_iterator(
        &mut self,
        label_set: Option<Vec<&'s ast::LabelIdentifier<'s>>>,
    ) -> JumpIndex {
        self.control_flow_stack
            .push(ControlFlowStackEntry::Iterator {
                label_set,
                incoming_control_flows: None,
            });
        self.add_instruction_with_jump_slot(Instruction::PushExceptionJumpTarget)
    }

    /// Exit a for-of loop.
    pub(super) fn exit_iterator(&mut self, continue_target: JumpIndex) {
        let Some(ControlFlowStackEntry::Iterator {
            label_set: _,
            incoming_control_flows,
        }) = self.control_flow_stack.pop()
        else {
            unreachable!()
        };
        if let Some(incoming_control_flows) = incoming_control_flows {
            let break_target = self.get_jump_index_to_here();
            if incoming_control_flows.has_breaks() {
                // When breaking out of iterator, it needs to be closed and its
                // exception handler removed.
                self.add_instruction(Instruction::PopExceptionJumpTarget);
                self.add_instruction(Instruction::IteratorClose);
            }
            incoming_control_flows.compile(continue_target, break_target, self);
        }
    }

    /// Enter a for-await-of loop.
    pub(super) fn enter_async_iterator(
        &mut self,
        label_set: Option<Vec<&'s ast::LabelIdentifier<'s>>>,
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
            let break_target = self.get_jump_index_to_here();
            // When breaking out of AsyncIterator, we need to close the iterator
            // and await the "return" function result, if any.
            self.add_instruction(Instruction::PopExceptionJumpTarget);
            self.add_instruction(Instruction::AsyncIteratorClose);
            // If async iterator close returned a Value, then it'll push the
            // previous result value into the stack, add an "ignore" exception
            // handler, and put the received Value
            self.add_instruction(Instruction::Await);
            self.add_instruction(Instruction::PopExceptionJumpTarget);
            self.add_instruction(Instruction::Store);
            incoming_control_flows.compile(continue_target, break_target, self);
        }
    }

    /// Compile a break statement targeting optional label.
    ///
    /// This helper injects all necessary finalisers at the break statement
    /// site before jumping to the target. If user-defined finally-blocks are
    /// present in the finaliser stack, the method instead jumps to a
    /// finally-block that ends with a jump to the final target.
    pub(super) fn compile_break(&mut self, label: Option<&'s ast::LabelIdentifier<'s>>) {
        for entry in self.control_flow_stack.iter_mut().rev() {
            if entry.is_break_target_for(label) {
                // Stop iterating the stack when we find our target and push
                // the current instruction pointer as a break source for our
                // target. Label is pushed in as well because finally-blocks
                // need to know about labelled breaks and continues.
                // Jump
                self.instructions.push(Instruction::Jump.as_u8());
                self.last_instruction_is_terminal = true;
                entry.add_break_source(
                    label,
                    JumpIndex {
                        index: self.instructions.len(),
                    },
                );
                // JumpSlot
                self.instructions.extend_from_slice(&[0, 0, 0, 0]);
                return;
            }
            // Compile the exit of each intermediate control flow stack entry.
            entry.compile_exit(&mut self.instructions);
        }
        // Note: if we got here then we're at the end of a loop to do teardown
        // for a break. The teardown instructions are not terminal.
        self.last_instruction_is_terminal = false;
    }

    /// Compile a continue statement targeting optional label.
    ///
    /// This helper injects all necessary finalisers at the continue statement
    /// site before jumping to the target. If user-defined finally-blocks are
    /// present in the finaliser stack, the method instead jumps to a
    /// finally-block that ends with a jump to the final target.
    pub(super) fn compile_continue(&mut self, label: Option<&'s ast::LabelIdentifier<'s>>) {
        for entry in self.control_flow_stack.iter_mut().rev() {
            if entry.is_continue_target_for(label) {
                // Stop iterating the stack when we find our target and push
                // the current instruction pointer as a break source for our
                // target. Label is pushed in as well because finally-blocks
                // need to know about labelled breaks and continues.

                // Jump
                self.instructions.push(Instruction::Jump.as_u8());
                self.last_instruction_is_terminal = true;
                entry.add_continue_source(
                    label,
                    JumpIndex {
                        index: self.instructions.len(),
                    },
                );
                // JumpSlot
                self.instructions.extend_from_slice(&[0, 0, 0, 0]);
                break;
            }
            // Compile the exit of each intermediate control flow stack entry.
            entry.compile_exit(&mut self.instructions);
        }
        // Note: if we got here then we're at the end of a loop to do teardown
        // for a continue. The teardown instructions are not terminal.
        self.last_instruction_is_terminal = false;
    }

    /// Compile a return statement.
    ///
    /// This helper injects all necessary finalisers at the return site before
    /// performing the final return. If user-defined finally-blocks are
    /// present in the finaliser stack, the method instead jumps to a
    /// finally-block that ends with a return.
    pub(super) fn compile_return(&mut self) {
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
            // First we need to load our return result to the stack.
            self.add_instruction(Instruction::Load);
            for entry in self.control_flow_stack.iter().rev() {
                if entry.requires_return_finalisation(true) {
                    entry.compile_exit(&mut self.instructions);
                }
            }
            // Before returning we need to store the return result back from
            // the stack.
            self.add_instruction(Instruction::Store);
            self.add_instruction(Instruction::Return);
            return;
        }
        // The rare case: We have at least one finally-block in the stack. In
        // this case we have to perform normal unwinding of the stack until the
        // first finally-block.
        // First we need to load our return result to the stack.
        self.add_instruction(Instruction::Load);
        for entry in self.control_flow_stack.iter_mut().rev() {
            if entry.is_return_target() {
                // Before continuing our unwind jump we need to take the return
                // result from the stack.
                self.instructions.push(Instruction::Store.as_u8());
                // Jump
                self.instructions.push(Instruction::Jump.as_u8());
                self.last_instruction_is_terminal = true;
                entry.add_return_source(JumpIndex {
                    index: self.instructions.len(),
                });
                // JumpSlot
                self.instructions.extend_from_slice(&[0, 0, 0, 0]);
                return;
            }
            entry.compile_exit(&mut self.instructions);
        }
        unreachable!()
    }

    /// Returns true if the last instruction is a terminal instruction and no
    /// jumps point past it.
    pub(crate) fn is_terminal(&self) -> bool {
        self.last_instruction_is_terminal
    }

    /// Compile a class static field with an optional initializer into the
    /// current context.
    pub(crate) fn compile_class_static_field(
        &mut self,
        identifier_name: &'s ast::IdentifierName<'s>,
        value: &'s Option<ast::Expression<'s>>,
    ) {
        let identifier = String::from_str(self.agent, identifier_name.name.as_str(), self.gc);
        // Turn the static name to a 'this' property access.
        self.add_instruction(Instruction::ResolveThisBinding);
        self.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            identifier,
        );
        if let Some(value) = value {
            // Minor optimisation: We do not need to push and pop the
            // reference if we know we're not using the reference stack.
            let is_literal = value.is_literal();
            if !is_literal {
                self.add_instruction(Instruction::PushReference);
            }
            value.compile(self);
            if is_reference(value) {
                self.add_instruction(Instruction::GetValue);
            }
            if !is_literal {
                self.add_instruction(Instruction::PopReference);
            }
        } else {
            // Same optimisation is unconditionally valid here.
            self.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        self.add_instruction(Instruction::PutValue);
    }

    /// Compile a class computed field with an optional initializer into the
    /// current context.
    pub(crate) fn compile_class_computed_field(
        &mut self,
        property_key_id: String<'gc>,
        value: &'s Option<ast::Expression<'s>>,
    ) {
        // Resolve 'this' into the stack.
        self.add_instruction(Instruction::ResolveThisBinding);
        self.add_instruction(Instruction::Load);
        // Resolve the static computed key ID to the actual computed key value.
        self.add_instruction_with_identifier(Instruction::ResolveBinding, property_key_id);
        // Store the computed key value as the result.
        self.add_instruction(Instruction::GetValue);
        // Evaluate access to 'this' with the computed key.
        self.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
        if let Some(value) = value {
            // Minor optimisation: We do not need to push and pop the
            // reference if we know we're not using the reference stack.
            let is_literal = value.is_literal();
            if !is_literal {
                self.add_instruction(Instruction::PushReference);
            }
            value.compile(self);
            if is_reference(value) {
                self.add_instruction(Instruction::GetValue);
            }
            if !is_literal {
                self.add_instruction(Instruction::PopReference);
            }
        } else {
            // Same optimisation is unconditionally valid here.
            self.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        self.add_instruction(Instruction::PutValue);
    }

    /// Compile a function body into the current context.
    ///
    /// This is useful when the function body is part of a larger whole, namely
    /// with class constructors.
    pub(crate) fn compile_function_body(&mut self, data: CompileFunctionBodyData<'s>) {
        if self.agent.options.print_internals {
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

        // SAFETY: Script referred by the Function uniquely owns the Program
        // and the body buffer does not move under any circumstances during
        // heap operations.
        let body: &[ast::Statement] =
            unsafe { core::mem::transmute(data.body.statements.as_slice()) };

        self.compile_statements(body);
    }

    pub(crate) fn compile_statements(&mut self, body: &'s [ast::Statement<'s>]) {
        let iter = body.iter();

        for stmt in iter {
            stmt.compile(self);
        }
    }

    pub(crate) fn do_implicit_return(&mut self) {
        if !self.is_terminal() {
            self.add_instruction(Instruction::Return);
        }
    }

    pub(crate) fn finish(self) -> Executable<'gc> {
        self.agent.heap.create(ExecutableHeapData {
            instructions: self.instructions.into_boxed_slice(),
            constants: self.constants.unbind().into_boxed_slice(),
            function_expressions: self.function_expressions.unbind().into_boxed_slice(),
            arrow_function_expressions: self.arrow_function_expressions.into_boxed_slice(),
            class_initializer_bytecodes: self
                .class_initializer_bytecodes
                .into_iter()
                .map(|(exe, b)| (exe.unbind(), b))
                .collect(),
        })
    }

    pub(crate) fn create_identifier(&mut self, atom: &oxc_span::Atom<'_>) -> String<'gc> {
        let existing = self.constants.iter().find_map(|constant| {
            if let Ok(existing_identifier) = String::try_from(*constant) {
                if existing_identifier.as_str(self.agent) == atom.as_str() {
                    Some(existing_identifier)
                } else {
                    None
                }
            } else {
                None
            }
        });
        if let Some(existing) = existing {
            existing
        } else {
            String::from_str(self.agent, atom.as_str(), self.gc)
        }
    }

    fn _push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction.as_u8());
        self.last_instruction_is_terminal = instruction.is_terminal();
    }

    pub(super) fn add_instruction(&mut self, instruction: Instruction) {
        debug_assert_eq!(instruction.argument_count(), 0);
        debug_assert!(
            !instruction.has_constant_index()
                && !instruction.has_function_expression_index()
                && !instruction.has_identifier_index()
        );
        self._push_instruction(instruction);
    }

    pub(super) fn add_instruction_with_jump_slot(&mut self, instruction: Instruction) -> JumpIndex {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_jump_slot());
        self._push_instruction(instruction);
        self.add_jump_index()
    }

    pub(super) fn add_jump_instruction_to_index(
        &mut self,
        instruction: Instruction,
        jump_index: JumpIndex,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_jump_slot());
        self._push_instruction(instruction);
        self.add_double_index(jump_index.index);
    }

    pub(super) fn get_jump_index_to_here(&mut self) -> JumpIndex {
        self.last_instruction_is_terminal = false;
        JumpIndex {
            index: self.instructions.len(),
        }
    }

    fn add_constant(&mut self, constant: Value<'gc>) -> usize {
        let duplicate = self
            .constants
            .iter()
            .enumerate()
            .find(|item| item.1.eq(&constant))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.constants.len();
            self.constants.push(constant);
            index
        })
    }

    pub(super) fn add_identifier(&mut self, identifier: String<'gc>) -> usize {
        let duplicate = self
            .constants
            .iter()
            .enumerate()
            .find(|item| String::try_from(*item.1) == Ok(identifier))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.constants.len();
            self.constants.push(identifier.into_value());
            index
        })
    }

    pub(super) fn add_instruction_with_immediate(
        &mut self,
        instruction: Instruction,
        immediate: usize,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        self._push_instruction(instruction);
        self.add_index(immediate);
    }

    pub(super) fn add_instruction_with_constant(
        &mut self,
        instruction: Instruction,
        constant: impl Into<Value<'gc>>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_constant_index());
        self._push_instruction(instruction);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    pub(super) fn add_instruction_with_identifier(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_identifier_index());
        self._push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
    }

    pub(super) fn add_instruction_with_identifier_and_constant(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
        constant: impl Into<Value<'gc>>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_identifier_index() && instruction.has_constant_index());
        self._push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    pub(super) fn add_instruction_with_immediate_and_immediate(
        &mut self,
        instruction: Instruction,
        immediate1: usize,
        immediate2: usize,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        self._push_instruction(instruction);
        self.add_index(immediate1);
        self.add_index(immediate2)
    }

    fn add_index(&mut self, index: usize) {
        let index = IndexType::try_from(index).expect("Immediate value is too large");
        let bytes: [u8; 2] = index.to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_double_index(&mut self, index: usize) {
        let index = u32::try_from(index).expect("Immediate value is too large");
        let bytes: [u8; 4] = index.to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    pub(super) fn add_instruction_with_function_expression(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.function_expressions.push(function_expression);
        let index = self.function_expressions.len() - 1;
        self.add_index(index);
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
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.function_expressions.push(function_expression);
        let index = self.function_expressions.len() - 1;
        self.add_index(index);
        self.add_index(immediate);
        // Note: add_index would have panicked if this was not a lossless
        // conversion.
        index as IndexType
    }

    pub(super) fn add_arrow_function_expression(
        &mut self,
        arrow_function_expression: ArrowFunctionExpression,
    ) {
        let instruction = Instruction::InstantiateArrowFunctionExpression;
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self._push_instruction(instruction);
        self.arrow_function_expressions
            .push(arrow_function_expression);
        let index = self.arrow_function_expressions.len() - 1;
        self.add_index(index);
    }

    fn add_jump_index(&mut self) -> JumpIndex {
        self.last_instruction_is_terminal = false;
        self.add_double_index(0);
        JumpIndex {
            index: self.instructions.len() - core::mem::size_of::<u32>(),
        }
    }

    pub(super) fn set_jump_target(&mut self, source: JumpIndex, target: JumpIndex) {
        assert!(target.index < u32::MAX as usize);
        let bytes: [u8; 4] = (target.index as u32).to_ne_bytes();
        self.instructions[source.index..source.index + 4].copy_from_slice(&bytes);
    }

    pub(super) fn set_jump_target_here(&mut self, jump: JumpIndex) {
        self.set_jump_target(
            jump,
            JumpIndex {
                index: self.instructions.len(),
            },
        );
        self.last_instruction_is_terminal = false;
    }

    pub(super) fn get_next_class_initializer_index(&self) -> IndexType {
        IndexType::try_from(self.class_initializer_bytecodes.len()).unwrap()
    }

    pub(super) fn set_function_expression_bytecode(
        &mut self,
        index: IndexType,
        executable: Executable<'gc>,
    ) {
        self.function_expressions[index as usize].compiled_bytecode = Some(executable);
    }

    pub(super) fn add_class_initializer_bytecode(
        &mut self,
        executable: Executable<'gc>,
        has_constructor_parent: bool,
    ) {
        self.class_initializer_bytecodes
            .push((Some(executable), has_constructor_parent));
    }

    pub(super) fn add_class_initializer(&mut self, has_constructor_parent: bool) {
        self.class_initializer_bytecodes
            .push((None, has_constructor_parent));
    }
}

pub(crate) trait CompileEvaluation<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>);
}

pub(crate) trait CompileLabelledEvaluation<'s> {
    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    );
}
