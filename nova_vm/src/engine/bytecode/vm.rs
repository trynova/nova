// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod binding_methods;
mod execute_instructions;

use execute_instructions::*;

use std::hint::unreachable_unchecked;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, BigInt, Environment, ExceptionType, JsError,
        JsResult, Number, Object, Primitive, Promise, Reference, ScopedArgumentsList, String,
        Value, call_function, get_method, is_callable, ordinary_has_instance, to_boolean,
        to_numeric, to_numeric_primitive, to_primitive, to_property_key, to_string_primitive,
        try_get_object_method, try_result_into_option_js,
    },
    engine::{
        Bindable, GcScope, NoGcScope, Scopable, bindable_handle,
        bytecode::{
            IndexType, Instruction, InstructionIter, instructions::Instr,
            iterator::VmIteratorRecord,
        },
        trivially_bindable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbols, WorkQueues},
};

#[derive(Debug)]
pub(crate) enum ExecutionResult<'a> {
    Return(Value<'a>),
    Throw(JsError<'a>),
    Await {
        vm: SuspendedVm,
        promise: Promise<'a>,
    },
    Yield {
        vm: SuspendedVm,
        yielded_value: Value<'a>,
    },
}
impl<'a> ExecutionResult<'a> {
    pub(crate) fn into_js_result(self) -> JsResult<'a, Value<'a>> {
        match self {
            ExecutionResult::Return(value) => Ok(value),
            ExecutionResult::Throw(err) => Err(err.unbind()),
            _ => panic!("Unexpected yield or await"),
        }
    }
}

bindable_handle!(ExecutionResult);

/// Indicates how the execution of an instruction should affect the remainder of
/// execution that contains it.
#[must_use]
enum ContinuationKind {
    Normal,
    Return,
    Yield,
    Await,
}
trivially_bindable!(ContinuationKind);

/// VM exception handler.
#[derive(Debug)]
pub(crate) enum ExceptionHandler<'a> {
    /// Indicates a jump to catch block.
    CatchBlock {
        /// Instruction pointer.
        ip: u32,
        /// The lexical environment which contains this exception jump target.
        lexical_environment: Environment<'a>,
    },
    /// Indicates that any error should be ignored and the next instruction
    /// skipped. This is used in AsyncIteratorClose handling.
    IgnoreErrorAndNextInstruction,
}
bindable_handle!(ExceptionHandler);

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug, Default)]
pub(crate) struct Vm {
    /// Instruction pointer.
    ip: usize,
    stack: Vec<Value<'static>>,
    stack_base: u32,
    reference_stack: Vec<Reference<'static>>,
    #[cfg(debug_assertions)]
    reference_stack_base: u32,
    iterator_stack: Vec<VmIteratorRecord<'static>>,
    #[cfg(debug_assertions)]
    iterator_stack_base: u32,
    exception_handler_stack: Vec<ExceptionHandler<'static>>,
    exception_handler_stack_base: u32,
    result: Option<Value<'static>>,
    reference: Option<Reference<'static>>,
}

#[derive(Debug)]
pub(crate) struct SuspendedVm {
    ip: usize,
    /// Note: Stack is empty only if the code contains no local variables
    /// optimised into stack slots or temporarily stored Values. A heap clone is
    /// probably often performed by the `stack.into_boxed_slice()` call.
    stack: Box<[Value<'static>]>,
    /// Note: Reference stack is non-empty only if the code awaits inside a call
    /// expression. This means that usually no heap data clone is required.
    reference_stack: Box<[Reference<'static>]>,
    /// Note: Iterator stack is non-empty only if the code awaits inside a
    /// for-in or for-of loop. This means that often no heap data clone is
    /// required.
    iterator_stack: Box<[VmIteratorRecord<'static>]>,
    /// Note: Exception jump stack is non-empty only if the code awaits inside a
    /// try block or an await for-of loop. This means that often no heap data
    /// clone is required.
    exception_handler_stack: Box<[ExceptionHandler<'static>]>,
}

impl SuspendedVm {
    pub(crate) fn get_ip_mut(&mut self) -> &mut usize {
        &mut self.ip
    }

    #[inline(always)]
    pub(crate) fn has_catch_handler(&self) -> bool {
        !self.exception_handler_stack.is_empty()
    }
}

impl Vm {
    fn suspend(
        &mut self,
        (stack_base, reference_stack_base, iterator_stack_base, exception_handler_stack_base): (
            u32,
            u32,
            u32,
            u32,
        ),
    ) -> SuspendedVm {
        debug_assert!(self.stack.len() >= stack_base as usize);
        debug_assert!(self.reference_stack.len() >= reference_stack_base as usize);
        debug_assert!(self.iterator_stack.len() >= iterator_stack_base as usize);
        debug_assert!(self.exception_handler_stack.len() >= exception_handler_stack_base as usize);
        SuspendedVm {
            ip: self.ip,
            stack: self.stack.split_off(stack_base as usize).into_boxed_slice(),
            reference_stack: self
                .reference_stack
                .split_off(reference_stack_base as usize)
                .into_boxed_slice(),
            iterator_stack: self
                .iterator_stack
                .split_off(iterator_stack_base as usize)
                .into_boxed_slice(),
            exception_handler_stack: self
                .exception_handler_stack
                .split_off(exception_handler_stack_base as usize)
                .into_boxed_slice(),
        }
    }

    pub(crate) fn get_instruction_pointer(&mut self) -> usize {
        self.ip
    }

    pub(crate) fn set_instruction_pointer(&mut self, ip: usize) {
        self.ip = ip;
    }

    pub(crate) fn get_stack_base(&self) -> u32 {
        self.stack_base
    }

    pub(crate) fn set_stack_bases(
        &mut self,
        stack_base: u32,
        #[cfg(debug_assertions)] reference_stack_base: u32,
        #[cfg(debug_assertions)] iterator_stack_base: u32,
        exception_handler_stack_base: u32,
    ) {
        self.stack_base = stack_base;
        #[cfg(debug_assertions)]
        {
            self.reference_stack_base = reference_stack_base;
            self.iterator_stack_base = iterator_stack_base;
        }
        self.exception_handler_stack_base = exception_handler_stack_base;
    }

    pub(crate) fn push_stack_frame(&mut self) -> (u32, u32, u32, u32) {
        let stack_base = u32::try_from(self.stack.len()).unwrap();
        self.stack_base = stack_base;
        let reference_stack_base = u32::try_from(self.reference_stack.len()).unwrap();
        let iterator_stack_base = u32::try_from(self.iterator_stack.len()).unwrap();
        let exception_handler_stack_base =
            u32::try_from(self.exception_handler_stack.len()).unwrap();
        self.exception_handler_stack_base = exception_handler_stack_base;
        #[cfg(debug_assertions)]
        {
            self.reference_stack_base = reference_stack_base;
            self.iterator_stack_base = iterator_stack_base;
        }
        (
            stack_base,
            reference_stack_base,
            iterator_stack_base,
            exception_handler_stack_base,
        )
    }

    pub(crate) fn pop_stack_frame(&mut self, stack_frame: (u32, u32, u32, u32)) {
        let (stack_base, reference_stack_base, iterator_stack_base, exception_handler_stack_base) =
            stack_frame;
        // Truncate the stacks to what we're told.
        debug_assert!(self.stack.len() >= stack_base as usize);
        debug_assert!(self.reference_stack.len() >= reference_stack_base as usize);
        debug_assert!(self.iterator_stack.len() >= iterator_stack_base as usize);
        debug_assert!(self.exception_handler_stack.len() >= exception_handler_stack_base as usize);
        self.stack.truncate(stack_base as usize);
        self.reference_stack.truncate(reference_stack_base as usize);
        self.iterator_stack.truncate(iterator_stack_base as usize);
        self.exception_handler_stack
            .truncate(exception_handler_stack_base as usize);
    }

    pub(crate) fn clear_stack(&mut self) {
        self.stack.clear();
        self.reference_stack.clear();
        self.iterator_stack.clear();
        self.exception_handler_stack.clear();
        self.ip = 0;
        self.stack_base = 0;
        #[cfg(debug_assertions)]
        {
            self.reference_stack_base = 0;
            self.iterator_stack_base = 0;
        }
        self.exception_handler_stack_base = 0;
    }

    pub(crate) fn pop_stack<'gc>(&mut self, gc: NoGcScope<'gc, '_>) -> Value<'gc> {
        debug_assert!(self.stack.len() > self.stack_base as usize);
        self.stack
            .pop()
            .expect("Attempted to pop from an empty stack")
            .bind(gc)
    }

    pub(crate) fn take_result<'gc>(&mut self, gc: NoGcScope<'gc, '_>) -> Value<'gc> {
        self.result
            .take()
            .expect("Attempted to take an empty result")
            .bind(gc)
    }

    pub(crate) fn unsuspend(
        &mut self,
        SuspendedVm {
            ip,
            stack,
            reference_stack,
            iterator_stack,
            exception_handler_stack,
        }: SuspendedVm,
    ) {
        self.ip = ip;
        self.stack.extend(stack);
        self.reference_stack.extend(reference_stack);
        self.iterator_stack.extend(iterator_stack);
        self.exception_handler_stack.extend(exception_handler_stack);
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute<'gc>(
        agent: &mut Agent,
        arguments: Option<&mut [Value]>,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        if let Some(arguments) = arguments {
            ArgumentsList::from_mut_slice(arguments).with_scoped(
                agent,
                |agent, arguments, gc| {
                    // SAFETY: awaits and yields are invalid syntax inside an arguments
                    // list, so this reference shouldn't remain alive after this
                    // function returns.
                    let arguments = unsafe {
                        core::mem::transmute::<ScopedArgumentsList, ScopedArgumentsList<'static>>(
                            arguments,
                        )
                    };
                    agent
                        .vm
                        .iterator_stack
                        .push(VmIteratorRecord::SliceIterator(arguments));
                    if agent.options.print_internals {
                        Vm::print_internals(agent, gc.nogc());
                    }

                    Vm::inner_execute(agent, gc)
                },
                gc,
            )
        } else {
            if agent.options.print_internals {
                Vm::print_internals(agent, gc.nogc());
            }

            Vm::inner_execute(agent, gc)
        }
    }

    fn print_internals(agent: &mut Agent, gc: NoGcScope) {
        let executable = agent.current_executable(gc);
        eprintln!();
        eprintln!("=== Executing Executable ===");
        eprintln!("Constants: {:?}", executable.get_constants(agent, gc));
        eprintln!();

        eprintln!("Instructions:");
        let iter = InstructionIter::new(executable.get_instructions(agent));
        for (ip, instr) in iter {
            instr.debug_print(agent, ip, executable, gc);
        }
        eprintln!();
    }

    pub(crate) fn resume<'gc>(
        agent: &mut Agent,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        agent.vm.result = Some(value.unbind());
        Vm::inner_execute(agent, gc)
    }

    pub(crate) fn resume_throw<'gc>(
        agent: &mut Agent,
        err: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let err = err.bind(gc.nogc());
        let err = JsError::new(err.unbind());
        if !Vm::handle_error(agent, err) {
            if agent.options.print_internals {
                eprintln!("Exiting function with error\n");
            }
            return ExecutionResult::Throw(err);
        }
        Vm::inner_execute(agent, gc)
    }

    fn inner_execute<'gc>(agent: &mut Agent, mut gc: GcScope<'gc, '_>) -> ExecutionResult<'gc> {
        // SAFETY: The instructions buffer is owned by the current Executable,
        // which is currently rooted by the execution context stack. The buffer
        // is not accessible as exclusive excecpt for dropping the Executable's
        // heap data, and isn't dropped while the Executable is rooted. Thus,
        // the instructions slice is guaranteed to be valid while we perform the
        // execution inner loop.
        let instructions =
            unsafe { core::mem::transmute::<&[u8], &[u8]>(agent.current_instructions()) };
        let stack_depth = agent.stack_refs.borrow().len();
        let print_internals = agent.options.print_internals;
        while let Some(instr) = Instr::consume_instruction(instructions, &mut agent.vm.ip) {
            if agent.check_gc() {
                agent.gc(gc.reborrow());
            }
            if print_internals {
                Self::print_executing(instr.kind);
            }
            let result = Self::execute_instruction(agent, instr, gc.reborrow());
            match result {
                Ok(ContinuationKind::Normal) => {}
                // SAFETY: result is not Ok(ContinuationKind::Normal).
                _ => unsafe {
                    if let Some(r) = Vm::handle_execute_instruction_abnormal_result(
                        agent,
                        result.unbind(),
                        gc.nogc(),
                    ) {
                        return r.unbind().bind(gc.into_nogc());
                    }
                },
            }
            agent.stack_refs.borrow_mut().truncate(stack_depth);
        }

        ExecutionResult::Return(Value::Undefined)
    }

    /// ## Safety
    ///
    /// result must not be Ok(ContinuationKind::Normal).
    #[inline(never)]
    #[cold]
    unsafe fn handle_execute_instruction_abnormal_result<'a>(
        agent: &mut Agent,
        result: JsResult<'a, ContinuationKind>,
        gc: NoGcScope<'a, '_>,
    ) -> Option<ExecutionResult<'a>> {
        match result {
            // SAFETY: method only called if result is not normal.
            Ok(ContinuationKind::Normal) => unsafe { unreachable_unchecked() },
            Ok(ContinuationKind::Return) => {
                if agent.options.print_internals {
                    Self::print_exiting();
                }
                let result = agent.vm.result.unwrap_or(Value::Undefined);
                Some(ExecutionResult::Return(result))
            }
            Ok(ContinuationKind::Yield) => {
                let yielded_value = agent.vm.take_result(gc);
                if agent.options.print_internals {
                    Self::print_yielding(yielded_value);
                }
                Some(ExecutionResult::Yield {
                    vm: agent
                        .vm
                        .suspend(agent.running_execution_context().get_stack_bases()),
                    yielded_value,
                })
            }
            Ok(ContinuationKind::Await) => {
                if agent.options.print_internals {
                    Self::print_awaiting();
                }
                let Value::Promise(promise) = agent.vm.result.take().unwrap() else {
                    unreachable!()
                };
                Some(ExecutionResult::Await {
                    vm: agent
                        .vm
                        .suspend(agent.running_execution_context().get_stack_bases()),
                    promise,
                })
            }
            Err(err) => {
                if !Vm::handle_error(agent, err) {
                    if agent.options.print_internals {
                        Self::print_exiting_with_error();
                    }
                    Some(ExecutionResult::Throw(err.unbind()))
                } else {
                    eprintln!("None?");
                    None
                }
            }
        }
    }

    #[inline(never)]
    #[cold]
    fn print_executing(instruction: Instruction) {
        eprintln!("Executing: {instruction:?}");
    }

    #[inline(never)]
    #[cold]
    fn print_exiting() {
        eprintln!("Exiting function with result\n");
    }

    #[inline(never)]
    #[cold]
    fn print_exiting_with_error() {
        eprintln!("Exiting function with error\n");
    }

    #[inline(never)]
    #[cold]
    fn print_awaiting() {
        eprintln!("Awaiting value in function\n");
    }

    #[inline(never)]
    #[cold]
    fn print_yielding(yielded_value: Value) {
        eprintln!("Yielding value from function {yielded_value:?}\n");
    }

    #[inline(never)]
    #[cold]
    #[must_use]
    fn handle_error(agent: &mut Agent, err: JsError) -> bool {
        if let Some(handler) = agent.vm.pop_exception_handler_stack() {
            match handler {
                ExceptionHandler::CatchBlock {
                    ip,
                    lexical_environment,
                } => {
                    if agent.options.print_internals {
                        eprintln!("Error: {:?}", err.value());
                        eprintln!("Jumping to catch block in {ip}\n");
                    }
                    agent.vm.ip = ip as usize;
                    agent.set_current_lexical_environment(lexical_environment);
                    agent.vm.result = Some(err.value().unbind());
                }
                ExceptionHandler::IgnoreErrorAndNextInstruction => {
                    if agent.options.print_internals {
                        eprintln!("Ignoring throw error and skipping to {}\n", agent.vm.ip + 1);
                    }
                    agent.vm.ip += 1;
                }
            }
            true
        } else {
            false
        }
    }

    fn execute_instruction<'a>(
        agent: &mut Agent,
        instr: Instr,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ContinuationKind> {
        // Hot instructions; apply #[inline(always)] to the execute methods.
        match instr.kind {
            Instruction::Load => {
                agent.vm.execute_load();
            }
            Instruction::LoadCopy => {
                agent.vm.execute_load_copy();
            }
            Instruction::PutValueToIndex => {
                agent
                    .vm
                    .execute_load_to_index(instr.get_first_stack_slot(agent));
            }
            Instruction::Store => {
                agent.vm.execute_store();
            }
            Instruction::GetValueFromIndex => {
                agent
                    .vm
                    .execute_store_from_index(instr.get_first_stack_slot(agent));
            }
            Instruction::StoreConstant => {
                execute_store_constant(agent, instr, gc.into_nogc());
            }
            Instruction::PopStack => {
                agent.vm.execute_pop_stack();
            }
            Instruction::Jump => execute_jump(agent, instr),
            Instruction::JumpIfNot => execute_jump_if_not(agent, instr, gc.into_nogc()),
            Instruction::ResolveBinding => {
                execute_resolve_binding(agent, instr, gc)?;
            }
            Instruction::GetValue
            | Instruction::GetValueWithCache
            | Instruction::GetValueKeepReference
            | Instruction::GetValueWithCacheKeepReference => {
                let cache = matches!(
                    instr.kind,
                    Instruction::GetValueWithCache | Instruction::GetValueWithCacheKeepReference
                );
                let keep_reference = matches!(
                    instr.kind,
                    Instruction::GetValueKeepReference
                        | Instruction::GetValueWithCacheKeepReference
                );
                execute_get_value(agent, instr, cache, keep_reference, gc)?
            }
            Instruction::PutValue | Instruction::PutValueWithCache => {
                let cache = matches!(instr.kind, Instruction::PutValueWithCache);
                execute_put_value(agent, instr, cache, gc)?
            }
            Instruction::ToNumeric => execute_to_numeric(agent, gc)?,
            Instruction::CreateImmutableBinding => {
                execute_create_immutable_binding(agent, instr, gc.into_nogc())
            }
            Instruction::CreateMutableBinding => {
                execute_create_mutable_binding(agent, instr, gc.into_nogc())
            }
            Instruction::InitializeReferencedBinding => {
                execute_initialize_referenced_binding(agent, gc.into_nogc())
            }
            Instruction::PushReference => agent.vm.execute_push_reference(),
            Instruction::PopReference => agent.vm.execute_pop_reference(),
            Instruction::EvaluatePropertyAccessWithIdentifierKey => {
                execute_evaluate_property_access_with_identifier_key(agent, instr, gc.into_nogc())
            }
            Instruction::EvaluatePropertyAccessWithExpressionKey => {
                execute_evaluate_property_access_with_expression_key(agent, gc.into_nogc())
            }
            _ => return Self::execute_cold_instruction(agent, instr, gc),
        }

        Ok(ContinuationKind::Normal)
    }

    #[inline(never)]
    fn execute_cold_instruction<'a>(
        agent: &mut Agent,
        instr: Instr,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ContinuationKind> {
        let _: () = match instr.kind {
            Instruction::Load
            | Instruction::LoadCopy
            | Instruction::PutValueToIndex
            | Instruction::Store
            | Instruction::StoreConstant
            | Instruction::GetValueFromIndex
            | Instruction::PopStack
            | Instruction::Jump
            | Instruction::JumpIfNot
            | Instruction::ResolveBinding
            | Instruction::GetValue
            | Instruction::GetValueWithCache
            | Instruction::GetValueKeepReference
            | Instruction::GetValueWithCacheKeepReference
            | Instruction::PutValue
            | Instruction::PutValueWithCache
            | Instruction::ToNumeric
            | Instruction::CreateImmutableBinding
            | Instruction::CreateMutableBinding
            | Instruction::InitializeReferencedBinding
            | Instruction::PushReference
            | Instruction::PopReference
            | Instruction::EvaluatePropertyAccessWithIdentifierKey
            | Instruction::EvaluatePropertyAccessWithExpressionKey => {
                unreachable!("hot instruction not handled before execute_cold_instruction")
            }
            Instruction::Return => return Ok(ContinuationKind::Return),
            Instruction::Await => {
                execute_await_promise_resolve(agent, gc)?;
                return Ok(ContinuationKind::Await);
            }
            Instruction::Yield => return Ok(ContinuationKind::Yield),
            Instruction::IsStrictlyEqual => execute_is_strictly_equal(agent, gc.into_nogc()),
            Instruction::IsNullOrUndefined => agent.vm.execute_is_null_or_undefined(),
            Instruction::IsNull => agent.vm.execute_is_null(),
            Instruction::IsUndefined => agent.vm.execute_is_undefined(),
            Instruction::IsObject => agent.vm.execute_is_object(),
            Instruction::IsConstructor => execute_is_constructor(agent, gc.into_nogc()),
            Instruction::JumpIfTrue => execute_jump_if_true(agent, instr, gc.into_nogc()),
            Instruction::LoadReplace => agent.vm.execute_load_replace(),
            Instruction::LoadConstant => execute_load_constant(agent, instr, gc.into_nogc()),
            Instruction::LoadStoreSwap => agent.vm.execute_load_store_swap(),
            Instruction::UpdateEmpty => agent.vm.execute_update_empty(),
            Instruction::Swap => agent.vm.execute_swap(),
            Instruction::Empty => agent.vm.execute_empty(),
            Instruction::LogicalNot => execute_logical_not(agent, gc.into_nogc()),
            Instruction::ApplyAdditionBinaryOperator => {
                execute_apply_addition_binary_operator(agent, gc)?
            }
            Instruction::ApplySubtractionBinaryOperator
            | Instruction::ApplyMultiplicationBinaryOperator
            | Instruction::ApplyDivisionBinaryOperator
            | Instruction::ApplyRemainderBinaryOperator
            | Instruction::ApplyExponentialBinaryOperator
            | Instruction::ApplyShiftLeftBinaryOperator
            | Instruction::ApplyShiftRightBinaryOperator
            | Instruction::ApplyShiftRightZeroFillBinaryOperator
            | Instruction::ApplyBitwiseORBinaryOperator
            | Instruction::ApplyBitwiseXORBinaryOperator
            | Instruction::ApplyBitwiseAndBinaryOperator => {
                execute_apply_binary_operator(agent, instr.kind, gc)?
            }
            Instruction::ArrayCreate => execute_array_create(agent, instr, gc.into_nogc())?,
            Instruction::ArrayPush => execute_array_push(agent, gc)?,
            Instruction::ArrayElision => execute_array_elision(agent, gc)?,
            Instruction::BitwiseNot => execute_bitwise_not(agent, gc.into_nogc())?,
            Instruction::CreateUnmappedArgumentsObject => {
                execute_create_unmapped_arguments_object(agent, gc.into_nogc())?
            }
            Instruction::CopyDataProperties => execute_copy_data_properties(agent, gc)?,
            Instruction::CopyDataPropertiesIntoObject => {
                execute_copy_data_properties_into_object(agent, instr, gc)?
            }
            Instruction::Delete => execute_delete(agent, gc)?,
            Instruction::DirectEvalCall => execute_direct_eval_call(agent, instr, gc)?,
            Instruction::EvaluateCall => execute_evaluate_call(agent, instr, gc)?,
            Instruction::EvaluateNew => execute_evaluate_new(agent, instr, gc)?,
            Instruction::EvaluateSuper => execute_evaluate_super(agent, instr, gc)?,
            Instruction::MakePrivateReference => {
                execute_make_private_reference(agent, instr, gc.into_nogc())
            }
            Instruction::MakeSuperPropertyReferenceWithExpressionKey => {
                execute_make_super_property_reference_with_expression_key(agent, gc.into_nogc())?
            }
            Instruction::MakeSuperPropertyReferenceWithIdentifierKey => {
                execute_make_super_property_reference_with_identifier_key(
                    agent,
                    instr,
                    gc.into_nogc(),
                )?
            }
            Instruction::GreaterThan => execute_greater_than(agent, gc)?,
            Instruction::GreaterThanEquals => execute_greater_than_equals(agent, gc)?,
            Instruction::HasProperty => execute_has_property(agent, gc)?,
            Instruction::HasPrivateElement => execute_has_private_element(agent, gc.into_nogc())?,
            Instruction::Increment => execute_increment(agent, gc.into_nogc()),
            Instruction::Decrement => execute_decrement(agent, gc.into_nogc()),
            Instruction::InstanceofOperator => execute_instanceof_operator(agent, gc)?,
            Instruction::InstantiateArrowFunctionExpression => {
                execute_instantiate_arrow_function_expression(agent, instr, gc)?
            }
            Instruction::InstantiateOrdinaryFunctionExpression => {
                execute_instantiate_ordinary_function_expression(agent, instr, gc)?
            }
            Instruction::ClassDefineConstructor => {
                execute_class_define_constructor(agent, instr, gc)?
            }
            Instruction::ClassDefineDefaultConstructor => {
                execute_class_define_default_constructor(agent, instr, gc)?
            }
            Instruction::ClassDefinePrivateMethod => {
                execute_class_define_private_method(agent, instr, gc)?
            }
            Instruction::ClassDefinePrivateProperty => {
                execute_class_define_private_property(agent, instr, gc.into_nogc())?
            }
            Instruction::ClassInitializePrivateElements => {
                execute_class_initialize_private_elements(agent, gc.into_nogc())?
            }
            Instruction::ClassInitializePrivateValue => {
                execute_class_initialize_private_value(agent, instr, gc.into_nogc())?
            }
            Instruction::LessThan => execute_less_than(agent, gc)?,
            Instruction::LessThanEquals => execute_less_than_equals(agent, gc)?,
            Instruction::IsLooselyEqual => execute_is_loosely_equal(agent, gc)?,
            Instruction::ObjectCreate => execute_object_create(agent, gc.into_nogc()),
            Instruction::ObjectCreateWithShape => {
                execute_object_create_with_shape(agent, instr, gc.into_nogc())
            }
            Instruction::ObjectDefineProperty => execute_object_define_property(agent, gc)?,
            Instruction::ObjectDefineMethod => execute_object_define_method(agent, instr, gc)?,
            Instruction::ObjectDefineGetter => execute_object_define_getter(agent, instr, gc)?,
            Instruction::ObjectDefineSetter => execute_object_define_setter(agent, instr, gc)?,
            Instruction::ObjectSetPrototype => execute_object_set_prototype(agent, gc)?,
            Instruction::PopExceptionJumpTarget => agent.vm.execute_pop_exception_jump_target(),
            Instruction::PushExceptionJumpTarget => {
                execute_push_exception_jump_target(agent, instr, gc.into_nogc())
            }
            Instruction::TruncateStack => {
                agent.vm.stack.truncate(instr.get_first_stack_slot(agent))
            }
            Instruction::ResolveBindingWithCache => {
                execute_resolve_binding_with_cache(agent, instr, gc)?
            }
            Instruction::ResolveThisBinding => execute_resolve_this_binding(agent, gc.into_nogc())?,
            Instruction::StoreCopy => agent.vm.execute_store_copy(),
            Instruction::StringConcat => execute_string_concat(agent, instr, gc)?,
            Instruction::Throw => agent.vm.execute_throw(gc.into_nogc())?,
            Instruction::ThrowError => execute_throw_error(agent, instr, gc.into_nogc())?,
            Instruction::ToNumber => execute_to_number(agent, gc)?,
            Instruction::ToObject => execute_to_object(agent, gc.into_nogc())?,
            Instruction::Typeof => execute_typeof(agent, gc)?,
            Instruction::UnaryMinus => execute_unary_minus(agent, gc.into_nogc()),
            Instruction::InitializeVariableEnvironment => {
                execute_initialize_variable_environment(agent, instr, gc.into_nogc())
            }
            Instruction::EnterDeclarativeEnvironment => {
                execute_enter_declarative_environment(agent, gc.into_nogc())
            }
            Instruction::EnterClassStaticElementEnvironment => {
                execute_enter_class_static_element_environment(agent, gc.into_nogc())
            }
            Instruction::EnterPrivateEnvironment => {
                execute_enter_private_environment(agent, instr, gc.into_nogc())
            }
            Instruction::ExitDeclarativeEnvironment => {
                execute_exit_declarative_environment(agent, gc.into_nogc())
            }
            Instruction::ExitVariableEnvironment => {
                execute_exit_variable_environment(agent, gc.into_nogc())
            }
            Instruction::ExitPrivateEnvironment => {
                execute_exit_private_environment(agent, gc.into_nogc())
            }
            Instruction::BeginSimpleObjectBindingPattern => {
                execute_begin_simple_object_binding_pattern(agent, instr, gc)?
            }
            Instruction::BeginSimpleArrayBindingPattern => {
                execute_begin_simple_array_binding_pattern(agent, instr, gc)?
            }
            Instruction::BindingPatternBind
            | Instruction::BindingPatternBindNamed
            | Instruction::BindingPatternBindToIndex
            | Instruction::BindingPatternBindRest
            | Instruction::BindingPatternBindRestToIndex
            | Instruction::BindingPatternSkip
            | Instruction::BindingPatternGetValue
            | Instruction::BindingPatternGetValueNamed
            | Instruction::BindingPatternGetRestValue
            | Instruction::FinishBindingPattern => execute_binding_pattern(),
            Instruction::EnumerateObjectProperties => {
                execute_enumerate_object_properties(agent, gc.into_nogc())
            }
            Instruction::GetIteratorSync => execute_get_iterator_sync(agent, gc)?,
            Instruction::GetIteratorAsync => execute_get_iterator_async(agent, gc)?,
            Instruction::IteratorStepValue => execute_iterator_step_value(agent, instr, gc)?,
            Instruction::IteratorStepValueOrUndefined => {
                execute_iterator_step_value_or_undefined(agent, gc)?
            }
            Instruction::IteratorCallNextMethod => execute_iterator_call_next_method(agent, gc)?,
            Instruction::IteratorComplete => execute_iterator_complete(agent, instr, gc)?,
            Instruction::IteratorValue => execute_iterator_value(agent, gc)?,
            Instruction::IteratorThrow => execute_iterator_throw(agent, instr, gc)?,
            Instruction::IteratorReturn => execute_iterator_return(agent, instr, gc)?,
            Instruction::IteratorRestIntoArray => execute_iterator_rest_into_array(agent, gc)?,
            Instruction::IteratorClose => execute_iterator_close(agent, gc)?,
            Instruction::AsyncIteratorClose => {
                if execute_async_iterator_close(agent, gc)? {
                    return Ok(ContinuationKind::Await);
                }
            }
            Instruction::IteratorCloseWithError => execute_iterator_close_with_error(agent, gc),
            Instruction::AsyncIteratorCloseWithError => {
                if execute_async_iterator_close_with_error(agent, gc) {
                    return Ok(ContinuationKind::Await);
                }
            }
            Instruction::IteratorPop => {
                let _ = agent.vm.pop_iterator(gc.into_nogc());
            }
            Instruction::GetNewTarget => execute_get_new_target(agent, gc.into_nogc()),
            Instruction::ImportCall => execute_import_call(agent, gc),
            Instruction::ImportMeta => execute_import_meta(agent, gc.into_nogc()),
            Instruction::VerifyIsObject => execute_verify_is_object(agent, instr, gc.into_nogc())?,
            Instruction::Debug => execute_debug(agent),
        };
        Ok(ContinuationKind::Normal)
    }

    fn get_call_args<'gc>(&mut self, instr: Instr, _gc: NoGcScope<'gc, '_>) -> Vec<Value<'gc>> {
        let instr_arg0 = instr.get_first_arg();
        if instr_arg0 != IndexType::MAX {
            // Static number of arguments less than IndexType::MAX.
            let arg_count = instr_arg0 as usize;
            debug_assert!(self.stack.len() >= self.stack_base as usize + arg_count);
            self.stack.split_off(self.stack.len() - arg_count)
        } else {
            // Dynamic number of arguments, or exactly IndexType::MAX or more
            // arguments. In this case the number of arguments is stored in the
            // result register for us. Additionally, an extra accumulator value
            // is stored on the stack before the arguments.
            let Value::Integer(integer) = self.result.take().unwrap() else {
                panic!("Expected the number of function arguments to be an integer")
            };
            let arg_count = usize::try_from(integer.into_i64()).unwrap();
            debug_assert!(self.stack.len() > self.stack_base as usize + arg_count);
            let args = self.stack.split_off(self.stack.len() - arg_count);
            debug_assert!(self.stack.len() > self.stack_base as usize);
            let integer_copy = self.stack.pop().unwrap();
            debug_assert_eq!(Value::Integer(integer), integer_copy);
            args
        }
    }

    /// Pop the active (top-most) iterator from the iterator stack.
    ///
    /// ### Panics
    ///
    /// Panics if the iterator stack is empty.
    pub(super) fn pop_iterator<'gc>(&mut self, gc: NoGcScope<'gc, '_>) -> VmIteratorRecord<'gc> {
        debug_assert!(self.iterator_stack.len() > self.iterator_stack_base as usize);
        self.iterator_stack
            .pop()
            .expect("Iterator stack is empty")
            .bind(gc)
    }

    /// Get the active (top-most) iterator from the iterator stack.
    ///
    /// ### Panics
    ///
    /// Panics if the iterator stack is empty.
    pub(super) fn get_active_iterator(&self) -> &VmIteratorRecord<'static> {
        debug_assert!(self.iterator_stack.len() > self.iterator_stack_base as usize);
        self.iterator_stack.last().expect("Iterator stack is empty")
    }

    /// Get the active (top-most) iterator from the iterator stack as mutable.
    ///
    /// ### Panics
    ///
    /// Panics if the iterator stack is empty.
    pub(super) fn get_active_iterator_mut(&mut self) -> &mut VmIteratorRecord<'static> {
        debug_assert!(self.iterator_stack.len() > self.iterator_stack_base as usize);
        self.iterator_stack
            .last_mut()
            .expect("Iterator stack is empty")
    }

    #[inline(always)]
    fn execute_is_null_or_undefined(&mut self) {
        let val = self.result.take().unwrap();
        let result = val.is_null() || val.is_undefined();
        self.result = Some(result.into());
    }

    #[inline(always)]
    fn execute_load(&mut self) {
        self.stack.push(self.result.take().unwrap());
    }

    #[inline(always)]
    fn execute_load_copy(&mut self) {
        self.stack.push(self.result.unwrap());
    }

    #[inline(always)]
    fn execute_load_to_index(&mut self, index: usize) {
        self.stack[index] = self.result.take().unwrap();
    }

    #[inline(always)]
    fn execute_load_store_swap(&mut self) {
        let temp = self
            .result
            .take()
            .expect("Expected result value to not be empty");
        debug_assert!(self.stack.len() > self.stack_base as usize);
        self.result = Some(self.stack.pop().expect("Trying to pop from empty stack"));
        self.stack.push(temp);
    }

    #[inline(always)]
    fn execute_load_replace(&mut self) {
        // Take result, if present, and replace the top of the stack
        // value with it.
        if let Some(result) = self.result.take() {
            let temp = self
                .stack
                .last_mut()
                .expect("Trying to replace top of empty stack");
            *temp = result;
        }
    }

    #[inline(always)]
    fn execute_update_empty(&mut self) {
        debug_assert!(self.stack.len() > self.stack_base as usize);
        // Take top of the stack value, set it as the result if no
        // result exists yet.
        let temp = self.stack.pop().expect("Trying to pop from empty stack");
        if self.result.is_none() {
            self.result = Some(temp);
        }
    }

    #[inline(always)]
    fn execute_store(&mut self) {
        debug_assert!(self.stack.len() > self.stack_base as usize);
        self.result = Some(self.stack.pop().expect("Trying to pop from empty stack"));
    }

    #[inline(always)]
    fn execute_store_from_index(&mut self, index: usize) {
        self.result = Some(self.stack[index]);
    }

    #[inline(always)]
    fn execute_store_copy(&mut self) {
        self.result = Some(*self.stack.last().expect("Trying to get from empty stack"));
    }

    #[inline(always)]
    fn execute_pop_stack(&mut self) {
        debug_assert!(self.stack.len() > self.stack_base as usize);
        let _ = self.stack.pop().expect("Trying to pop from empty stack");
    }

    #[inline(always)]
    fn execute_push_reference(&mut self) {
        self.reference_stack.push(self.reference.take().unwrap());
    }

    fn pop_reference_stack(&mut self) -> Reference<'static> {
        debug_assert!(self.reference_stack.len() > self.reference_stack_base as usize);
        self.reference_stack
            .pop()
            .expect("Attempted to pop a Reference from an empty stack")
    }

    #[inline(always)]
    fn execute_pop_reference(&mut self) {
        self.reference = Some(self.pop_reference_stack());
    }

    #[inline(always)]
    fn execute_swap(&mut self) {
        debug_assert!(self.stack.len() > self.stack_base as usize);
        let a = self.stack.pop().unwrap();
        debug_assert!(self.stack.len() > self.stack_base as usize);
        let b = self.stack.pop().unwrap();
        self.stack.push(a);
        self.stack.push(b);
    }

    #[inline(always)]
    fn execute_empty(&mut self) {
        self.result = None;
    }

    #[inline(always)]
    fn execute_is_undefined(&mut self) {
        let val = self.result.take().unwrap();
        let result = val.is_undefined();
        self.result = Some(result.into());
    }

    #[inline(always)]
    fn execute_is_null(&mut self) {
        let val = self.result.take().unwrap();
        let result = val.is_null();
        self.result = Some(result.into());
    }

    #[inline(always)]
    fn execute_is_object(&mut self) {
        let val = self.result.take().unwrap();
        let result = val.is_object();
        self.result = Some(result.into());
    }

    pub(crate) fn pop_exception_handler_stack(&mut self) -> Option<ExceptionHandler<'static>> {
        if self.exception_handler_stack.len() > self.exception_handler_stack_base as usize {
            self.exception_handler_stack.pop()
        } else {
            None
        }
    }

    #[inline(always)]
    fn execute_pop_exception_jump_target(&mut self) {
        // Bytecode should never attempt to pop an exception handler past the
        // stack base.
        debug_assert!(
            self.exception_handler_stack.len() > self.exception_handler_stack_base as usize
        );
        self.pop_exception_handler_stack().unwrap();
    }

    #[inline(always)]
    fn execute_throw<'gc>(&mut self, gc: NoGcScope<'gc, '_>) -> JsResult<'gc, ()> {
        let result = self.result.take().unwrap();
        Err(JsError::new(result).bind(gc))
    }
}

fn concat_string_from_slice<'gc>(
    agent: &mut Agent,
    slice: &[String],
    string_length: usize,
    gc: NoGcScope<'gc, '_>,
) -> String<'gc> {
    let mut result_string = Wtf8Buf::with_capacity(string_length);
    for string in slice.iter() {
        result_string.push_wtf8(string.as_wtf8_(agent));
    }
    String::from_wtf8_buf(agent, result_string, gc)
}

/// ### [13.15.3 ApplyStringOrNumericBinaryOperator ( lval, opText, rval )](https://tc39.es/ecma262/#sec-applystringornumericbinaryoperator)
///
/// The abstract operation ApplyStringOrNumericBinaryOperator takes
/// arguments lval (an ECMAScript language value), opText (**, *, /, %,
/// -, <<, >>, >>>, &, ^, or |), and rval (an ECMAScript language value) and
/// returns either a normal completion containing either a String, a BigInt,
/// or a Number, or a throw completion.
fn apply_string_or_numeric_binary_operator<'gc>(
    agent: &mut Agent,
    lval: Value,
    op_text: Instruction,
    rval: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let lval = lval.bind(gc.nogc());
    let rval = rval.bind(gc.nogc());
    // 1. If opText is +, then
    let rval = rval.scope(agent, gc.nogc());
    // 3. Let lnum be ? ToNumeric(lval).
    let lnum = to_numeric(agent, lval.unbind(), gc.reborrow())
        .unbind()?
        .scope(agent, gc.nogc());
    // 4. Let rnum be ? ToNumeric(rval).
    // SAFETY: not shared.
    let rnum = to_numeric(agent, unsafe { rval.take(agent) }, gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let rnum = rnum.bind(gc);
    // SAFETY: not shared.
    let lnum = unsafe { lnum.take(agent) }.bind(gc);

    // 6. If lnum is a BigInt, then
    if let (Ok(lnum), Ok(rnum)) = (BigInt::try_from(lnum), BigInt::try_from(rnum)) {
        bigint_binary_operator(agent, op_text, lnum, rnum, gc).map(|v| v.into())
    } else if let (Ok(lnum), Ok(rnum)) = (Number::try_from(lnum), Number::try_from(rnum)) {
        number_binary_operator(agent, op_text, lnum, rnum, gc).map(|v| v.into())
    } else {
        // 5. If Type(lnum) is not Type(rnum), throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "The left and right-hand sides do not have the same type.",
            gc,
        ))
    }
}

/// ### [13.15.3 ApplyStringOrNumericBinaryOperator ( lval, opText, rval )](https://tc39.es/ecma262/#sec-applystringornumericbinaryoperator)
///
/// The abstract operation ApplyStringOrNumericBinaryOperator takes
/// arguments lval (an ECMAScript language value), opText (+), and rval (an
/// ECMAScript language value) and returns either a normal completion
/// containing either a String, a BigInt, or a Number, or a throw completion.
fn apply_string_or_numeric_addition<'gc>(
    agent: &mut Agent,
    lval: Value,
    rval: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let lval = lval.bind(gc.nogc());
    let rval = rval.bind(gc.nogc());
    // 1. If opText is +, then
    // a. Let lprim be ? ToPrimitive(lval).
    // b. Let rprim be ? ToPrimitive(rval).
    let (lprim, rprim, gc) = match (Primitive::try_from(lval), Primitive::try_from(rval)) {
        (Ok(lprim), Ok(rprim)) => {
            let lprim = lprim.unbind();
            let rprim = rprim.unbind();
            let gc = gc.into_nogc();
            (lprim.bind(gc), rprim.bind(gc), gc)
        }
        (Ok(lprim), Err(_)) => {
            let lprim = lprim.scope(agent, gc.nogc());
            let rprim = to_primitive(agent, rval.unbind(), None, gc.reborrow()).unbind()?;
            let gc = gc.into_nogc();
            // SAFETY: not shared.
            let lprim = unsafe { lprim.take(agent) };
            (lprim.bind(gc), rprim.bind(gc), gc)
        }
        (Err(_), Ok(rprim)) => {
            let rprim = rprim.scope(agent, gc.nogc());
            let lprim = to_primitive(agent, lval.unbind(), None, gc.reborrow()).unbind()?;
            let gc = gc.into_nogc();
            // SAFETY: not shared.
            let rprim = unsafe { rprim.take(agent) };
            (lprim.bind(gc), rprim.bind(gc), gc)
        }
        (Err(_), Err(_)) => {
            let rval = rval.scope(agent, gc.nogc());
            let lprim = to_primitive(agent, lval.unbind(), None, gc.reborrow())
                .unbind()?
                .scope(agent, gc.nogc());
            // SAFETY: not shared.
            let rprim =
                to_primitive(agent, unsafe { rval.take(agent) }, None, gc.reborrow()).unbind()?;
            let gc = gc.into_nogc();
            // SAFETY: not shared.
            let lprim = unsafe { lprim.take(agent) };
            (lprim.bind(gc), rprim.bind(gc), gc)
        }
    };

    // c. If lprim is a String or rprim is a String, then
    match (String::try_from(lprim), String::try_from(rprim)) {
        (Ok(lstr), Ok(rstr)) => {
            // iii. Return the string-concatenation of lstr and rstr.
            return Ok(String::concat(agent, [lstr, rstr], gc).into());
        }
        (Ok(lstr), Err(_)) => {
            let lstr = lstr.scope(agent, gc);
            // ii. Let rstr be ? ToString(rprim).
            let rstr = to_string_primitive(agent, rprim, gc)?;
            // iii. Return the string-concatenation of lstr and rstr.
            return Ok(String::concat(agent, [lstr.get(agent).bind(gc), rstr], gc).into());
        }
        (Err(_), Ok(rstr)) => {
            let rstr = rstr.scope(agent, gc);
            // i. Let lstr be ? ToString(lprim).
            let lstr = to_string_primitive(agent, lprim, gc)?;
            // iii. Return the string-concatenation of lstr and rstr.
            return Ok(String::concat(agent, [lstr, rstr.get(agent).bind(gc)], gc).into());
        }
        (Err(_), Err(_)) => {}
    }

    // d. Set lval to lprim.
    // e. Set rval to rprim.
    // 2. NOTE: At this point, it must be a numeric operation.
    // 3. Let lnum be ? ToNumeric(lval).
    let lnum = to_numeric_primitive(agent, lprim, gc)?;
    // 4. Let rnum be ? ToNumeric(rval).
    let rnum = to_numeric_primitive(agent, rprim, gc)?;

    // 6. If lnum is a BigInt, then
    if let (Ok(lnum), Ok(rnum)) = (BigInt::try_from(lnum), BigInt::try_from(rnum)) {
        Ok(BigInt::add(agent, lnum, rnum).into())
    } else if let (Ok(lnum), Ok(rnum)) = (Number::try_from(lnum), Number::try_from(rnum)) {
        Ok(Number::add(agent, lnum, rnum).into())
    } else {
        // 5. If Type(lnum) is not Type(rnum), throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "The left and right-hand sides do not have the same type.",
            gc,
        ))
    }
}

fn bigint_binary_operator<'a>(
    agent: &mut Agent,
    op_text: Instruction,
    lnum: BigInt<'a>,
    rnum: BigInt<'a>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, BigInt<'a>> {
    match op_text {
        // a. If opText is **, return ? BigInt::exponentiate(lnum, rnum).
        Instruction::ApplyExponentialBinaryOperator => BigInt::exponentiate(agent, lnum, rnum, gc),
        // b. If opText is /, return ? BigInt::divide(lnum, rnum).
        Instruction::ApplyDivisionBinaryOperator => BigInt::divide(agent, lnum, rnum, gc),
        // c. If opText is %, return ? BigInt::remainder(lnum, rnum).
        Instruction::ApplyRemainderBinaryOperator => BigInt::remainder(agent, lnum, rnum, gc),
        // d. If opText is >>>, return ? BigInt::unsignedRightShift(lnum, rnum).
        Instruction::ApplyShiftRightZeroFillBinaryOperator => {
            BigInt::unsigned_right_shift(agent, lnum, rnum, gc)
        }
        // <<	BigInt	BigInt::leftShift
        Instruction::ApplyShiftLeftBinaryOperator => BigInt::left_shift(agent, lnum, rnum, gc),
        // >>	BigInt	BigInt::signedRightShift
        Instruction::ApplyShiftRightBinaryOperator => {
            BigInt::signed_right_shift(agent, lnum, rnum, gc)
        }
        // -	BigInt	BigInt::subtract
        Instruction::ApplySubtractionBinaryOperator => Ok(BigInt::subtract(agent, lnum, rnum)),
        // *	BigInt	BigInt::multiply
        Instruction::ApplyMultiplicationBinaryOperator => Ok(BigInt::multiply(agent, lnum, rnum)),
        // |	BigInt	BigInt::bitwiseOR
        Instruction::ApplyBitwiseORBinaryOperator => Ok(BigInt::bitwise_or(agent, lnum, rnum)),
        // ^	BigInt	BigInt::bitwiseXOR
        Instruction::ApplyBitwiseXORBinaryOperator => Ok(BigInt::bitwise_xor(agent, lnum, rnum)),
        // &	BigInt	BigInt::bitwiseAND
        Instruction::ApplyBitwiseAndBinaryOperator => Ok(BigInt::bitwise_and(agent, lnum, rnum)),
        _ => unreachable!(),
    }
}

fn number_binary_operator<'a>(
    agent: &mut Agent,
    op_text: Instruction,
    lnum: Number<'a>,
    rnum: Number<'a>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, Number<'a>> {
    // 7. Let operation be the abstract operation associated with opText and
    // Type(lnum) in the following table:
    // 8. Return operation(lnum, rnum).
    // NOTE: We do step 8. explicitly in branch.
    Ok(match op_text {
        // opText	Type(lnum)	operation
        // **	Number	Number::exponentiate
        Instruction::ApplyExponentialBinaryOperator => Number::exponentiate(agent, lnum, rnum),
        // *	Number	Number::multiply
        Instruction::ApplyMultiplicationBinaryOperator => Number::multiply(agent, lnum, rnum, gc),
        // /	Number	Number::divide
        Instruction::ApplyDivisionBinaryOperator => Number::divide(agent, lnum, rnum, gc),
        // %	Number	Number::remainder
        Instruction::ApplyRemainderBinaryOperator => Number::remainder(agent, lnum, rnum, gc),
        // -	Number	Number::subtract
        Instruction::ApplySubtractionBinaryOperator => Number::subtract(agent, lnum, rnum),
        // <<	Number	Number::leftShift
        Instruction::ApplyShiftLeftBinaryOperator => Number::left_shift(agent, lnum, rnum),
        // >>	Number	Number::signedRightShift
        Instruction::ApplyShiftRightBinaryOperator => Number::signed_right_shift(agent, lnum, rnum),
        // >>>	Number	Number::unsignedRightShift
        Instruction::ApplyShiftRightZeroFillBinaryOperator => {
            Number::unsigned_right_shift(agent, lnum, rnum)
        }
        // |	Number	Number::bitwiseOR
        Instruction::ApplyBitwiseORBinaryOperator => Number::bitwise_or(agent, lnum, rnum).into(),
        // ^	Number	Number::bitwiseXOR
        Instruction::ApplyBitwiseXORBinaryOperator => Number::bitwise_xor(agent, lnum, rnum).into(),
        // &	Number	Number::bitwiseAND
        Instruction::ApplyBitwiseAndBinaryOperator => Number::bitwise_and(agent, lnum, rnum).into(),
        _ => unreachable!(),
    })
}

/// ### [13.5.3 The typeof operator](https://tc39.es/ecma262/#sec-typeof-operator)
#[inline]
pub(crate) fn typeof_operator(agent: &Agent, val: Value, gc: NoGcScope) -> String<'static> {
    match val {
        // 4. If val is undefined, return "undefined".
        Value::Undefined => BUILTIN_STRING_MEMORY.undefined,
        // 8. If val is a Boolean, return "boolean".
        Value::Boolean(_) => BUILTIN_STRING_MEMORY.boolean,
        // 6. If val is a String, return "string".
        Value::String(_) |
        Value::SmallString(_) => BUILTIN_STRING_MEMORY.string,
        // 7. If val is a Symbol, return "symbol".
        Value::Symbol(_) => BUILTIN_STRING_MEMORY.symbol,
        // 9. If val is a Number, return "number".
        Value::Number(_) |
        Value::Integer(_) |
        Value::SmallF64(_) => BUILTIN_STRING_MEMORY.number,
        // 10. If val is a BigInt, return "bigint".
        Value::BigInt(_) |
        Value::SmallBigInt(_) => BUILTIN_STRING_MEMORY.bigint,
        // 5. If val is null, return "object".
        Value::Null |
        // 11. Assert: val is an Object.
        // 12. NOTE: This step is replaced in section B.3.6.3.
        Value::Object(_)  |
        Value::Array(_)  |
        Value::Error(_)  |
        // 14. Return "object".
        Value::PrimitiveObject(_) |
        Value::Arguments(_) |
        Value::FinalizationRegistry(_) |
        Value::Map(_) |
        Value::Promise(_) |
        Value::AsyncGenerator(_) |
        Value::ArrayIterator(_) |
        Value::MapIterator(_) |
        Value::StringIterator(_) |
        Value::Generator(_) |
        Value::Module(_) |
        Value::EmbedderObject(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "regexp")]
        Value::RegExp(_) | Value::RegExpStringIterator(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "weak-refs")]
        Value::WeakMap(_) |
        Value::WeakRef(_) |
        Value::WeakSet(_)  => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "set")]
        Value::Set(_) |
        Value::SetIterator(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "array-buffer")]
        Value::ArrayBuffer(_) |
        Value::Int8Array(_) |
        Value::Uint8Array(_) |
        Value::Uint8ClampedArray(_) |
        Value::Int16Array(_) |
        Value::Uint16Array(_) |
        Value::Int32Array(_) |
        Value::Uint32Array(_) |
        Value::BigInt64Array(_) |
        Value::BigUint64Array(_) |
        Value::Float32Array(_) |
        Value::Float64Array(_) |
        Value::DataView(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "proposal-float16array")]
        Value::Float16Array(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "shared-array-buffer")]
        Value::SharedArrayBuffer(_) |
        Value::SharedInt8Array(_) |
        Value::SharedUint8Array(_) |
        Value::SharedUint8ClampedArray(_) |
        Value::SharedInt16Array(_) |
        Value::SharedUint16Array(_) |
        Value::SharedInt32Array(_) |
        Value::SharedUint32Array(_) |
        Value::SharedBigInt64Array(_) |
        Value::SharedBigUint64Array(_) |
        Value::SharedFloat32Array(_) |
        Value::SharedFloat64Array(_) |
        Value::SharedDataView(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(all(feature = "proposal-float16array", feature = "shared-array-buffer"))]
        Value::SharedFloat16Array(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "date")]
        Value::Date(_)  => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "temporal")]
        Value::Instant(_)  => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "temporal")]
        Value::Duration(_)  => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "temporal")]
        Value::PlainTime(_)  => BUILTIN_STRING_MEMORY.object,
        // 13. If val has a [[Call]] internal slot, return "function".
        Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_) |
        Value::BuiltinConstructorFunction(_) |
        Value::BuiltinPromiseResolvingFunction(_) |
        Value::BuiltinPromiseFinallyFunction(_) |
        Value::BuiltinProxyRevokerFunction => BUILTIN_STRING_MEMORY.function,
        Value::Proxy(proxy) => {
            if proxy.is_callable(agent, gc) {
                BUILTIN_STRING_MEMORY.function
            } else {
                BUILTIN_STRING_MEMORY.object
            }
        },
    }
}

/// ### [13.10.2 InstanceofOperator ( V, target )](https://tc39.es/ecma262/#sec-instanceofoperator)
///
/// The abstract operation InstanceofOperator takes arguments V (an ECMAScript
/// language value) and target (an ECMAScript language value) and returns
/// either a normal completion containing a Boolean or a throw completion. It
/// implements the generic algorithm for determining if V is an instance of
/// target either by consulting target's @@hasInstance method or, if absent,
/// determining whether the value of target's "prototype" property is present
/// in V's prototype chain.
///
/// > #### Note
/// > Steps 4 and 5 provide compatibility with previous editions of ECMAScript
/// > that did not use a @@hasInstance method to define the instanceof operator
/// > semantics. If an object does not define or inherit @@hasInstance it uses
/// > the default instanceof semantics.
pub(crate) fn instanceof_operator<'a, 'b>(
    agent: &mut Agent,
    value: impl Into<Value<'b>>,
    target: impl Into<Value<'b>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let mut value = value.into().bind(gc.nogc());
    let target = target.into().bind(gc.nogc());
    // 1. If target is not an Object, throw a TypeError exception.
    let Ok(mut target) = Object::try_from(target) else {
        let error_message = format!(
            "Invalid instanceof target {}.",
            target
                .unbind()
                .string_repr(agent, gc.reborrow())
                .to_string_lossy_(agent)
        );
        return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.into_nogc()));
    };
    // 2. Let instOfHandler be ? GetMethod(target, @@hasInstance).
    let inst_of_handler = if let Some(handler) = try_result_into_option_js(try_get_object_method(
        agent,
        target,
        WellKnownSymbols::HasInstance.into(),
        gc.nogc(),
    )) {
        handler.unbind()?.bind(gc.nogc())
    } else {
        let scoped_value = value.scope(agent, gc.nogc());
        let scoped_target = target.scope(agent, gc.nogc());
        let inst_of_handler = get_method(
            agent,
            target.unbind().into(),
            WellKnownSymbols::HasInstance.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // SAFETY: not shared.
        value = unsafe { scoped_value.take(agent) }.bind(gc.nogc());
        // SAFETY: not shared.
        target = unsafe { scoped_target.take(agent) }.bind(gc.nogc());
        inst_of_handler
    };
    // 3. If instOfHandler is not undefined, then
    if let Some(inst_of_handler) = inst_of_handler {
        // a. Return ToBoolean(? Call(instOfHandler, target, « V »)).
        let result = call_function(
            agent,
            inst_of_handler.unbind(),
            target.unbind().into(),
            Some(ArgumentsList::from_mut_slice(&mut [value.unbind()])),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        Ok(to_boolean(agent, result))
    } else {
        // 4. If IsCallable(target) is false, throw a TypeError exception.
        let Some(target) = is_callable(target, gc.nogc()) else {
            let error_message = format!(
                "Invalid instanceof target {} is not a function.",
                Value::from(target)
                    .unbind()
                    .string_repr(agent, gc.reborrow())
                    .to_string_lossy_(agent)
            );
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                error_message,
                gc.into_nogc(),
            ));
        };
        // 5. Return ? OrdinaryHasInstance(target, V).
        Ok(ordinary_has_instance(
            agent,
            target.unbind(),
            value.unbind(),
            gc,
        )?)
    }
}

impl HeapMarkAndSweep for ExceptionHandler<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            ExceptionHandler::CatchBlock {
                ip: _,
                lexical_environment,
            } => {
                lexical_environment.mark_values(queues);
            }
            ExceptionHandler::IgnoreErrorAndNextInstruction => {}
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            ExceptionHandler::CatchBlock {
                ip: _,
                lexical_environment,
            } => {
                lexical_environment.sweep_values(compactions);
            }
            ExceptionHandler::IgnoreErrorAndNextInstruction => {}
        }
    }
}

impl HeapMarkAndSweep for Vm {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Vm {
            ip: _,
            stack,
            stack_base: _,
            reference_stack,
            #[cfg(debug_assertions)]
                reference_stack_base: _,
            iterator_stack,
            #[cfg(debug_assertions)]
                iterator_stack_base: _,
            exception_handler_stack,
            #[cfg(debug_assertions)]
                exception_handler_stack_base: _,
            result,
            reference,
        } = self;
        stack.as_slice().mark_values(queues);
        reference_stack.as_slice().mark_values(queues);
        iterator_stack.as_slice().mark_values(queues);
        exception_handler_stack.as_slice().mark_values(queues);
        result.mark_values(queues);
        reference.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Vm {
            ip: _,
            stack,
            stack_base: _,
            reference_stack,
            #[cfg(debug_assertions)]
                reference_stack_base: _,
            iterator_stack,
            #[cfg(debug_assertions)]
                iterator_stack_base: _,
            exception_handler_stack,
            #[cfg(debug_assertions)]
                exception_handler_stack_base: _,
            result,
            reference,
        } = self;
        stack.as_mut_slice().sweep_values(compactions);
        reference_stack.as_mut_slice().sweep_values(compactions);
        iterator_stack.as_mut_slice().sweep_values(compactions);
        exception_handler_stack
            .as_mut_slice()
            .sweep_values(compactions);
        result.sweep_values(compactions);
        reference.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for SuspendedVm {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            ip: _,
            stack,
            reference_stack,
            iterator_stack,
            exception_handler_stack: exception_jump_target_stack,
        } = self;
        stack.mark_values(queues);
        reference_stack.mark_values(queues);
        iterator_stack.mark_values(queues);
        exception_jump_target_stack.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            ip: _,
            stack,
            reference_stack,
            iterator_stack,
            exception_handler_stack: exception_jump_target_stack,
        } = self;
        stack.sweep_values(compactions);
        reference_stack.sweep_values(compactions);
        iterator_stack.sweep_values(compactions);
        exception_jump_target_stack.sweep_values(compactions);
    }
}

/// SetFunctionName version for class constructors, only calculates the correct
/// function name and returns it.
fn set_class_name<'a>(
    agent: &mut Agent,
    name: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, String<'a>> {
    if let Ok(name) = String::try_from(name) {
        Ok(name.bind(gc.into_nogc()))
    } else if let Value::Symbol(name) = name {
        // OPTIMISATION: Specification wise, we should go to the
        // below else-branch and perform ToPropertyKey, but that'd
        // just return our Symbol at the cost of some scoping.
        // Symbols are the most likely non-String value here, so
        // we'll check them separately first.
        Ok(name
            .unbind()
            .get_symbol_function_name(agent, gc.into_nogc()))
    } else {
        // ## 13.2.5.5 Runtime Semantics: PropertyDefinitionEvaluation
        // ### PropertyDefinition : PropertyName : AssignmentExpression
        // 1. Let propKey be ? Evaluation of PropertyName.
        // 3. Return ? ToPropertyKey(propName).
        let prop_key = {
            let name = name.unbind();
            to_property_key(agent, name, gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };

        let name = prop_key.convert_to_value(agent, gc.nogc());
        set_class_name(agent, name.unbind().into(), gc)
    }
}

fn verify_is_object<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    if !value.is_object() {
        let message =
            String::from_static_str(agent, "iterator.return() returned a non-object value", gc);
        Err(agent.throw_exception_with_message(ExceptionType::TypeError, message.unbind(), gc))
    } else {
        Ok(())
    }
}

fn throw_error_in_target_not_object<'a>(
    agent: &mut Agent,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    let error_message = format!(
        "right-hand side of 'in' should be an object, got {}.",
        typeof_operator(agent, value, gc).to_string_lossy_(agent)
    );
    agent.throw_exception(ExceptionType::TypeError, error_message, gc)
}
