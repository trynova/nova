// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod binding_methods;

use std::{ptr::NonNull, sync::OnceLock};

use binding_methods::{execute_simple_array_binding, execute_simple_object_binding};
use oxc_ast::ast;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{iterator_complete, iterator_value},
            operations_on_objects::{
                call, call_function, construct, copy_data_properties,
                copy_data_properties_into_object, create_data_property_or_throw,
                define_property_or_throw, get_method, has_property, ordinary_has_instance,
                private_element_find, set, throw_no_proxy_private_names,
                try_copy_data_properties_into_object, try_create_data_property,
                try_define_property_or_throw, try_has_property,
            },
            testing_and_comparison::{
                is_callable, is_constructor, is_less_than, is_loosely_equal, is_strictly_equal,
            },
            type_conversion::{
                to_boolean, to_number, to_numeric, to_numeric_primitive, to_object, to_primitive,
                to_property_key, to_property_key_complex, to_property_key_simple, to_string,
                to_string_primitive,
            },
        },
        builtins::{
            ArgumentsList, Array, BuiltinConstructorArgs, ConstructorStatus,
            OrdinaryFunctionCreateParams, ScopedArgumentsList, SetFunctionNamePrefix, array_create,
            create_builtin_constructor, create_unmapped_arguments_object,
            global_object::perform_eval, make_constructor, make_method,
            ordinary::ordinary_object_create_with_intrinsics, ordinary_function_create,
            set_function_name,
        },
        execution::{
            Agent, Environment, JsResult, PrivateMethod, ProtoIntrinsics,
            agent::{ExceptionType, JsError, resolve_binding, try_resolve_binding},
            get_this_environment, new_class_static_element_environment,
            new_declarative_environment, new_private_environment, resolve_private_identifier,
            resolve_this_binding,
        },
        scripts_and_modules::{ScriptOrModule, module::evaluate_import_call},
        types::{
            BUILTIN_STRING_MEMORY, BigInt, Function, InternalMethods, InternalSlots, IntoFunction,
            IntoObject, IntoValue, Number, Numeric, Object, OrdinaryObject, Primitive,
            PropertyDescriptor, PropertyKey, PropertyKeySet, Reference, String, Value,
            get_this_value, get_value, initialize_referenced_binding, is_private_reference,
            is_property_reference, is_super_reference, is_unresolvable_reference, put_value,
            throw_read_undefined_or_null_error, try_get_value, try_initialize_referenced_binding,
        },
    },
    engine::{
        ScopableCollection, Scoped, TryResult,
        bytecode::{
            Executable, FunctionExpression, IndexType, Instruction, InstructionIter,
            NamedEvaluationParameter,
            executable::ArrowFunctionExpression,
            instructions::Instr,
            iterator::{ObjectPropertiesIteratorRecord, VmIteratorRecord},
        },
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        unwrap_try,
    },
    heap::{CompactionLists, HeapMarkAndSweep, ObjectEntry, WellKnownSymbolIndexes, WorkQueues},
};

use super::iterator::{ActiveIterator, throw_iterator_returned_non_object};

struct EmptyParametersList(ast::FormalParameters<'static>);
unsafe impl Send for EmptyParametersList {}
unsafe impl Sync for EmptyParametersList {}

#[derive(Debug)]
pub(crate) enum ExecutionResult<'a> {
    Return(Value<'a>),
    Throw(JsError<'a>),
    Await {
        vm: SuspendedVm,
        awaited_value: Value<'a>,
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

// SAFETY: Property implemented as a recursive bind.
unsafe impl Bindable for ExecutionResult<'_> {
    type Of<'a> = ExecutionResult<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        match self {
            Self::Return(value) => ExecutionResult::Return(value.unbind()),
            Self::Throw(js_error) => ExecutionResult::Throw(js_error.unbind()),
            Self::Await { vm, awaited_value } => ExecutionResult::Await {
                vm,
                awaited_value: awaited_value.unbind(),
            },
            Self::Yield { vm, yielded_value } => ExecutionResult::Yield {
                vm,
                yielded_value: yielded_value.unbind(),
            },
        }
    }

    #[inline(always)]
    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        match self {
            Self::Return(value) => ExecutionResult::Return(value.bind(gc)),
            Self::Throw(js_error) => ExecutionResult::Throw(js_error.bind(gc)),
            Self::Await { vm, awaited_value } => ExecutionResult::Await {
                vm,
                awaited_value: awaited_value.bind(gc),
            },
            Self::Yield { vm, yielded_value } => ExecutionResult::Yield {
                vm,
                yielded_value: yielded_value.bind(gc),
            },
        }
    }
}

/// Indicates how the execution of an instruction should affect the remainder of
/// execution that contains it.
#[must_use]
enum ContinuationKind {
    Normal,
    Return,
    Yield,
    Await,
}

/// VM exception handler.
#[derive(Debug)]
enum ExceptionHandler<'a> {
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

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug)]
pub(crate) struct Vm {
    /// Instruction pointer.
    ip: usize,
    stack: Vec<Value<'static>>,
    reference_stack: Vec<Reference<'static>>,
    iterator_stack: Vec<VmIteratorRecord<'static>>,
    exception_handler_stack: Vec<ExceptionHandler<'static>>,
    result: Option<Value<'static>>,
    reference: Option<Reference<'static>>,
}

#[derive(Debug)]
pub(crate) struct SuspendedVm {
    ip: usize,
    /// Note: Stack is non-empty only if the code awaits inside a call
    /// expression. This is reasonably rare that we can expect the stack to
    /// usually be empty. In this case this Box is an empty dangling pointer
    /// and no heap data clone is required.
    stack: Box<[Value<'static>]>,
    /// Note: Reference stack is non-empty only if the code awaits inside a
    /// call expression. This means that usually no heap data clone is
    /// required.
    reference_stack: Box<[Reference<'static>]>,
    /// Note: Iterator stack is non-empty only if the code awaits inside a
    /// for-in or for-of loop. This means that often no heap data clone is
    /// required.
    iterator_stack: Box<[VmIteratorRecord<'static>]>,
    /// Note: Exception jump stack is non-empty only if the code awaits inside
    /// a try block. This means that often no heap data clone is required.
    exception_jump_target_stack: Box<[ExceptionHandler<'static>]>,
}

impl SuspendedVm {
    /// Returns true if the suspended VM is safe to keep past a GC safepoint.
    ///
    /// This requires that the VMs stacks are all empty.
    pub(crate) fn is_gc_safe(&self) -> bool {
        self.stack.is_empty()
            && self.reference_stack.is_empty()
            && self.iterator_stack.is_empty()
            && self.exception_jump_target_stack.is_empty()
    }

    pub(crate) fn resume<'gc>(
        self,
        agent: &mut Agent,
        executable: Scoped<Executable>,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        if agent.options.print_internals {
            eprintln!("Resuming function with value\n");
        }
        let vm = Vm::from_suspended(self);
        vm.resume(agent, executable, value, gc)
    }

    pub(crate) fn resume_throw<'gc>(
        self,
        agent: &mut Agent,
        executable: Scoped<Executable>,
        err: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        if agent.options.print_internals {
            eprintln!("Resuming function with error\n");
        }
        // Optimisation: Avoid unsuspending the Vm if we're just going to throw
        // out of it immediately.
        if self.exception_jump_target_stack.is_empty() {
            let err = JsError::new(err.unbind());
            return ExecutionResult::Throw(err);
        }
        let vm = Vm::from_suspended(self);
        vm.resume_throw(agent, executable, err, gc)
    }

    pub(crate) fn resume_return<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Scoped<Executable>,
        result: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        if agent.options.print_internals {
            eprintln!("Resuming function with return\n");
        }
        // Following a yield point, the next instruction is a Jump to the
        // Normal continue handling. We need to ignore that.
        let next_instruction = executable.get_instruction(agent, &mut self.ip);
        assert_eq!(next_instruction.map(|i| i.kind), Some(Instruction::Jump));
        let peek_next_instruction = executable.get_instructions(agent).get(self.ip).copied();
        if peek_next_instruction == Some(Instruction::Return.as_u8()) {
            // Our return handling is to just return; we can do that without
            // unsuspending the VM.
            return ExecutionResult::Return(result.bind(gc.into_nogc()));
        }
        let vm = Vm::from_suspended(self);
        vm.resume(agent, executable, result, gc)
    }
}

impl Vm {
    fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            iterator_stack: Vec::new(),
            exception_handler_stack: Vec::new(),
            result: None,
            reference: None,
        }
    }

    fn suspend(self) -> SuspendedVm {
        SuspendedVm {
            ip: self.ip,
            stack: self.stack.into_boxed_slice(),
            reference_stack: self.reference_stack.into_boxed_slice(),
            iterator_stack: self.iterator_stack.into_boxed_slice(),
            exception_jump_target_stack: self.exception_handler_stack.into_boxed_slice(),
        }
    }

    fn from_suspended(suspended: SuspendedVm) -> Self {
        Self {
            ip: suspended.ip,
            stack: suspended.stack.into_vec(),
            reference_stack: suspended.reference_stack.into_vec(),
            iterator_stack: suspended.iterator_stack.into_vec(),
            exception_handler_stack: suspended.exception_jump_target_stack.into_vec(),
            result: None,
            reference: None,
        }
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute<'gc>(
        agent: &mut Agent,
        executable: Scoped<Executable>,
        arguments: Option<&mut [Value<'static>]>,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let mut vm = Vm::new();

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
                    vm.iterator_stack
                        .push(VmIteratorRecord::SliceIterator(arguments));
                    if agent.options.print_internals {
                        vm.print_internals(agent, executable.clone(), gc.nogc());
                    }

                    vm.inner_execute(agent, executable, gc)
                },
                gc,
            )
        } else {
            if agent.options.print_internals {
                vm.print_internals(agent, executable.clone(), gc.nogc());
            }

            vm.inner_execute(agent, executable, gc)
        }
    }

    fn print_internals(&self, agent: &mut Agent, executable: Scoped<Executable>, gc: NoGcScope) {
        eprintln!();
        eprintln!("=== Executing Executable ===");
        eprintln!("Constants: {:?}", executable.get_constants(agent, gc));
        eprintln!();

        eprintln!("Instructions:");
        let iter = InstructionIter::new(executable.get_instructions(agent));
        for (ip, instr) in iter {
            instr.debug_print(agent, ip, executable.clone(), gc);
        }
        eprintln!();
    }

    pub fn resume<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Scoped<Executable>,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        self.result = Some(value.unbind());
        self.inner_execute(agent, executable, gc)
    }

    pub fn resume_throw<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Scoped<Executable>,
        err: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let err = err.bind(gc.nogc());
        let err = JsError::new(err.unbind());
        if !self.handle_error(agent, err) {
            if agent.options.print_internals {
                eprintln!("Exiting function with error\n");
            }
            return ExecutionResult::Throw(err);
        }
        self.inner_execute(agent, executable, gc)
    }

    fn inner_execute<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Scoped<Executable>,
        mut gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let stack_depth = agent.stack_refs.borrow().len();
        let instructions = executable.get_instructions(agent);
        while let Some(instr) = Instr::consume_instruction(instructions, &mut self.ip) {
            if agent.check_gc() {
                with_vm_gc(agent, &mut self, |agent, gc| agent.gc(gc), gc.reborrow());
            }
            if agent.options.print_internals {
                eprintln!("Executing: {:?}", instr.kind);
            }
            match Self::execute_instruction(
                agent,
                &mut self,
                executable.clone(),
                &instr,
                gc.reborrow(),
            ) {
                Ok(ContinuationKind::Normal) => {}
                Ok(ContinuationKind::Return) => {
                    if agent.options.print_internals {
                        eprintln!("Exiting function with result\n");
                    }
                    let result = self.result.unwrap_or(Value::Undefined);
                    return ExecutionResult::Return(result);
                }
                Ok(ContinuationKind::Yield) => {
                    let yielded_value = self.result.take().unwrap();
                    if agent.options.print_internals {
                        eprintln!("Yielding value from function {yielded_value:?}\n");
                    }
                    return ExecutionResult::Yield {
                        vm: self.suspend(),
                        yielded_value,
                    };
                }
                Ok(ContinuationKind::Await) => {
                    if agent.options.print_internals {
                        eprintln!("Awaiting value in function\n");
                    }
                    let awaited_value = self.result.take().unwrap();
                    return ExecutionResult::Await {
                        vm: self.suspend(),
                        awaited_value,
                    };
                }
                Err(err) => {
                    if !self.handle_error(agent, err) {
                        if agent.options.print_internals {
                            eprintln!("Exiting function with error\n");
                        }
                        return ExecutionResult::Throw(err.unbind().bind(gc.into_nogc()));
                    }
                }
            }
            agent.stack_refs.borrow_mut().truncate(stack_depth);
        }

        ExecutionResult::Return(Value::Undefined)
    }

    #[must_use]
    fn handle_error(&mut self, agent: &mut Agent, err: JsError) -> bool {
        if let Some(handler) = self.exception_handler_stack.pop() {
            match handler {
                ExceptionHandler::CatchBlock {
                    ip,
                    lexical_environment,
                } => {
                    if agent.options.print_internals {
                        eprintln!("Error: {:?}", err.value());
                        eprintln!("Jumping to catch block in {ip}\n");
                    }
                    self.ip = ip as usize;
                    agent.set_current_lexical_environment(lexical_environment);
                    self.result = Some(err.value().unbind());
                }
                ExceptionHandler::IgnoreErrorAndNextInstruction => {
                    if agent.options.print_internals {
                        eprintln!("Ignoring throw error and skipping to {}\n", self.ip + 1);
                    }
                    self.ip += 1;
                }
            }
            true
        } else {
            false
        }
    }

    fn execute_instruction<'a>(
        agent: &mut Agent,
        vm: &mut Vm,
        executable: Scoped<Executable>,
        instr: &Instr,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ContinuationKind> {
        match instr.kind {
            Instruction::ArrayCreate => {
                let result = array_create(agent, 0, instr.get_first_index(), None, gc.into_nogc())?
                    .into_value();
                vm.result = Some(result.unbind());
            }
            Instruction::ArrayPush => {
                let value = vm.result.take().unwrap().bind(gc.nogc());
                let array = vm.stack.last().unwrap().bind(gc.nogc());
                let Ok(array) = Array::try_from(array) else {
                    unreachable!();
                };
                let len = array.len(agent);
                let key = PropertyKey::Integer(len.into());
                let array = array.unbind();
                let value = value.unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| create_data_property_or_throw(agent, array, key, value, gc),
                    gc,
                )?;
            }
            Instruction::ArrayElision => {
                let array = vm.stack.last().unwrap().bind(gc.nogc());
                let Ok(array) = Array::try_from(array) else {
                    unreachable!();
                };
                let length = array.len(agent) + 1;
                let array = array.into_object().unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        set(
                            agent,
                            array,
                            BUILTIN_STRING_MEMORY.length.into(),
                            length.into(),
                            true,
                            gc,
                        )
                    },
                    gc,
                )?;
            }
            Instruction::Await => return Ok(ContinuationKind::Await),
            Instruction::BitwiseNot => {
                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                // Note: This step is a separate instruction.
                let old_value = Numeric::try_from(vm.result.take().unwrap())
                    .unwrap()
                    .bind(gc.nogc());

                // 3. If oldValue is a Number, then
                if let Ok(old_value) = Number::try_from(old_value) {
                    // a. Return Number::bitwiseNOT(oldValue).
                    vm.result = Some(Number::bitwise_not(agent, old_value).into_value().unbind());
                } else {
                    // 4. Else,
                    // a. Assert: oldValue is a BigInt.
                    let Ok(old_value) = BigInt::try_from(old_value) else {
                        unreachable!();
                    };

                    // b. Return BigInt::bitwiseNOT(oldValue).
                    vm.result = Some(BigInt::bitwise_not(agent, old_value).into_value().unbind());
                }
            }
            Instruction::Debug => {
                if agent.options.print_internals {
                    eprintln!("Debug: {vm:#?}");
                }
            }
            Instruction::ResolveBinding => {
                let identifier =
                    executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());

                let reference = if let TryResult::Continue(reference) =
                    try_resolve_binding(agent, identifier, gc.nogc())
                {
                    reference
                } else {
                    let identifier = identifier.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| resolve_binding(agent, identifier, None, gc),
                        gc,
                    )?
                };

                vm.reference = Some(reference.unbind());
            }
            Instruction::ResolveThisBinding => {
                let this = resolve_this_binding(agent, gc.into_nogc())?.unbind();
                vm.result = Some(this);
            }
            Instruction::LoadConstant => {
                let constant =
                    executable.fetch_constant(agent, instr.get_first_index(), gc.into_nogc());
                vm.stack.push(constant.unbind());
            }
            Instruction::Load => {
                vm.stack.push(vm.result.take().unwrap());
            }
            Instruction::LoadCopy => {
                vm.stack.push(vm.result.unwrap());
            }
            Instruction::LoadStoreSwap => {
                let temp = vm
                    .result
                    .take()
                    .expect("Expected result value to not be empty");
                vm.result = Some(vm.stack.pop().expect("Trying to pop from empty stack"));
                vm.stack.push(temp);
            }
            Instruction::LoadReplace => {
                // Take result, if present, and replace the top of the stack
                // value with it.
                if let Some(result) = vm.result.take() {
                    let temp = vm
                        .stack
                        .last_mut()
                        .expect("Trying to replace top of empty stack");
                    *temp = result;
                }
            }
            Instruction::UpdateEmpty => {
                // Take top of the stack value, set it as the result if no
                // result exists yet.
                let temp = vm.stack.pop().expect("Trying to pop from empty stack");
                if vm.result.is_none() {
                    vm.result = Some(temp);
                }
            }
            Instruction::Return => {
                return Ok(ContinuationKind::Return);
            }
            Instruction::Store => {
                vm.result = Some(vm.stack.pop().expect("Trying to pop from empty stack"));
            }
            Instruction::StoreCopy => {
                vm.result = Some(*vm.stack.last().expect("Trying to get from empty stack"));
            }
            Instruction::StoreConstant => {
                let constant = executable.fetch_constant(agent, instr.get_first_index(), gc.nogc());
                vm.result = Some(constant.unbind());
            }
            Instruction::UnaryMinus => {
                let old_value = vm.result.unwrap().bind(gc.nogc());

                // 3. If oldValue is a Number, then
                let result = if let Ok(old_value) = Number::try_from(old_value) {
                    // a. Return Number::unaryMinus(oldValue).
                    Number::unary_minus(agent, old_value).into_value()
                }
                // 4. Else,
                else {
                    // a. Assert: oldValue is a BigInt.
                    let old_value = BigInt::try_from(old_value).unwrap();

                    // b. Return BigInt::unaryMinus(oldValue).
                    BigInt::unary_minus(agent, old_value).into_value()
                };
                vm.result = Some(result.unbind());
            }
            Instruction::ToNumber => {
                let arg0 = vm.result.unwrap();
                let result = with_vm_gc(agent, vm, |agent, gc| to_number(agent, arg0, gc), gc)?;
                vm.result = Some(result.into_value().unbind());
            }
            Instruction::ToNumeric => {
                let arg0 = vm.result.unwrap();
                let result = with_vm_gc(agent, vm, |agent, gc| to_numeric(agent, arg0, gc), gc)?;
                vm.result = Some(result.into_value().unbind());
            }
            Instruction::ToObject => {
                vm.result = Some(
                    to_object(agent, vm.result.unwrap(), gc.into_nogc())?
                        .into_value()
                        .unbind(),
                );
            }
            Instruction::ApplyStringOrNumericBinaryOperator(op_text) => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        if op_text == BinaryOperator::Addition {
                            apply_string_or_numeric_addition(agent, lval, rval, gc)
                        } else {
                            apply_string_or_numeric_binary_operator(agent, lval, op_text, rval, gc)
                        }
                    },
                    gc,
                )?;
                vm.result = Some(result.unbind());
            }
            Instruction::ObjectDefineProperty => {
                let key = vm.stack.pop().unwrap();
                let key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, key, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let key = key.unbind().bind(gc.nogc());
                let value = vm.result.take().unwrap().bind(gc.nogc());
                let object = vm.stack.last().unwrap().bind(gc.nogc());
                let object = Object::try_from(object).unwrap();

                create_data_property_or_throw(
                    agent,
                    object.unbind(),
                    key.unbind(),
                    value.unbind(),
                    gc,
                )?;
            }
            Instruction::ObjectDefineMethod => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                let function_expression = expression.get();
                let enumerable = instr.get_second_bool();
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = vm.stack.pop().unwrap();
                let prop_key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, prop_key, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let object = Object::try_from(*vm.stack.last().unwrap())
                    .unwrap()
                    .bind(gc.nogc());

                // 2. Let env be the running execution context's LexicalEnvironment.
                let env = agent.current_lexical_environment(gc.nogc());
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let private_env = agent.current_private_environment(gc.nogc());
                // Note: Non-constructor methods never have a function
                // prototype.
                // 4. If functionPrototype is present, then
                //     a. Let prototype be functionPrototype.
                // 5. Else,
                //     a. Let prototype be %Function.prototype%.
                let params = OrdinaryFunctionCreateParams {
                    function_prototype: None,
                    source_code: None,
                    // 4. Let sourceText be the source text matched by MethodDefinition.
                    source_text: function_expression.span,
                    parameters_list: &function_expression.params,
                    body: function_expression.body.as_ref().unwrap(),
                    is_concise_arrow_function: false,
                    is_async: function_expression.r#async,
                    is_generator: function_expression.generator,
                    lexical_this: false,
                    env,
                    private_env,
                };
                // 7. Let closure be OrdinaryFunctionCreate(
                //      prototype,
                //      sourceText,
                //      UniqueFormalParameters,
                //      FunctionBody,
                //      non-lexical-this,
                //      env,
                //      privateEnv
                //  ).
                let closure = ordinary_function_create(agent, params, gc.nogc());
                // 8. Perform MakeMethod(closure, object).
                make_method(agent, closure, object);
                // 2. Perform SetFunctionName(closure, propKey).
                set_function_name(agent, closure, prop_key, None, gc.nogc());
                // 3. Return ? DefineMethodProperty(
                //      object,
                //      methodDef.[[Key]],
                //      methodDef.[[Closure]],
                //      enumerable
                // ).
                // 2. If key is a Private Name, then
                // a. Return PrivateElement {
                //      [[Key]]: key,
                //      [[Kind]]: method,
                //      [[Value]]: closure
                // }.
                // 3. Else,
                // a. Let desc be the PropertyDescriptor {
                //      [[Value]]: closure,
                //      [[Writable]]: true,
                //      [[Enumerable]]: enumerable,
                //      [[Configurable]]: true
                // }.
                let desc = PropertyDescriptor {
                    value: Some(closure.into_value().unbind()),
                    writable: Some(true),
                    enumerable: Some(enumerable),
                    configurable: Some(true),
                    ..Default::default()
                };
                // b. Perform ? DefinePropertyOrThrow(homeObject, key, desc).
                // c. NOTE: DefinePropertyOrThrow only returns an abrupt
                // completion when attempting to define a class static method whose key is "prototype".

                let object = object.unbind();
                let prop_key = prop_key.unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| define_property_or_throw(agent, object, prop_key, desc, gc),
                    gc,
                )?;
                // c. Return unused.
            }
            Instruction::ObjectDefineGetter => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                let function_expression = expression.get();
                let enumerable = instr.get_second_bool();
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = vm.stack.pop().unwrap();
                let prop_key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, prop_key, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // 2. Let env be the running execution context's LexicalEnvironment.
                let env = agent.current_lexical_environment(gc.nogc());
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let private_env = agent.current_private_environment(gc.nogc());
                // 5. Let formalParameterList be an instance of the production FormalParameters : [empty] .
                // We have to create a temporary allocator to create the empty
                // items Vec. The allocator will never be asked to allocate
                // anything.
                static EMPTY_PARAMETERS: OnceLock<EmptyParametersList> = OnceLock::new();
                let empty_parameters = EMPTY_PARAMETERS.get_or_init(|| {
                    let allocator: &'static oxc_allocator::Allocator = Box::leak(Box::default());
                    EmptyParametersList(ast::FormalParameters {
                        span: Default::default(),
                        kind: ast::FormalParameterKind::FormalParameter,
                        items: oxc_allocator::Vec::new_in(allocator),
                        rest: None,
                    })
                });
                let params = OrdinaryFunctionCreateParams {
                    function_prototype: None,
                    source_code: None,
                    // 4. Let sourceText be the source text matched by MethodDefinition.
                    source_text: function_expression.span,
                    parameters_list: &empty_parameters.0,
                    body: function_expression.body.as_ref().unwrap(),
                    is_async: function_expression.r#async,
                    is_generator: function_expression.generator,
                    is_concise_arrow_function: false,
                    lexical_this: false,
                    env,
                    private_env,
                };
                // 6. Let closure be OrdinaryFunctionCreate(
                //      %Function.prototype%,
                //      sourceText,
                //      formalParameterList,
                //      FunctionBody,
                //      non-lexical-this,
                //      env,
                //      privateEnv
                //  ).
                let closure = ordinary_function_create(agent, params, gc.nogc());
                // 7. Perform MakeMethod(closure, object).
                let object = Object::try_from(*vm.stack.last().unwrap())
                    .unwrap()
                    .bind(gc.nogc());
                make_method(agent, closure, object.into_object());
                // 8. Perform SetFunctionName(closure, propKey, "get").
                set_function_name(
                    agent,
                    closure,
                    prop_key,
                    Some(SetFunctionNamePrefix::Get),
                    gc.nogc(),
                );
                // 9. If propKey is a Private Name, then
                // a. Return PrivateElement { [[Key]]: propKey, [[Kind]]: accessor, [[Get]]: closure, [[Set]]: undefined }.
                // 10. Else,
                // a. Let desc be the PropertyDescriptor { [[Get]]: closure, [[Enumerable]]: enumerable, [[Configurable]]: true }.
                let desc = PropertyDescriptor {
                    value: None,
                    writable: None,
                    get: Some(closure.into_function().unbind()),
                    set: None,
                    enumerable: Some(enumerable),
                    configurable: Some(true),
                };
                // b. Perform ? DefinePropertyOrThrow(object, propKey, desc).
                let object = object.unbind();
                let prop_key = prop_key.unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| define_property_or_throw(agent, object, prop_key, desc, gc),
                    gc,
                )?;
                // c. Return unused.
            }
            Instruction::ObjectDefineSetter => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                let function_expression = expression.get();
                let enumerable = instr.get_second_bool();
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = vm.stack.pop().unwrap();
                let prop_key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, prop_key, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // 2. Let env be the running execution context's LexicalEnvironment.
                let env = agent.current_lexical_environment(gc.nogc());
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let private_env = agent.current_private_environment(gc.nogc());
                let params = OrdinaryFunctionCreateParams {
                    function_prototype: None,
                    source_code: None,
                    // 4. Let sourceText be the source text matched by MethodDefinition.
                    source_text: function_expression.span,
                    parameters_list: &function_expression.params,
                    body: function_expression.body.as_ref().unwrap(),
                    is_concise_arrow_function: false,
                    is_async: function_expression.r#async,
                    is_generator: function_expression.generator,
                    lexical_this: false,
                    env,
                    private_env,
                };
                // 5. Let closure be OrdinaryFunctionCreate(
                //      %Function.prototype%,
                //      sourceText,
                //      PropertySetParameterList,
                //      FunctionBody,
                //      non-lexical-this,
                //      env,
                //      privateEnv
                //  ).
                let closure = ordinary_function_create(agent, params, gc.nogc());
                // 6. Perform MakeMethod(closure, object).
                let object = Object::try_from(*vm.stack.last().unwrap())
                    .unwrap()
                    .bind(gc.nogc());
                make_method(agent, closure, object.into_object());
                // 7. Perform SetFunctionName(closure, propKey, "set").
                set_function_name(
                    agent,
                    closure,
                    prop_key,
                    Some(SetFunctionNamePrefix::Set),
                    gc.nogc(),
                );
                // 8. If propKey is a Private Name, then
                // a. Return PrivateElement { [[Key]]: propKey, [[Kind]]: accessor, [[Get]]: undefined, [[Set]]: closure }.
                // 9. Else,
                // a. Let desc be the PropertyDescriptor { [[Set]]: closure, [[Enumerable]]: enumerable, [[Configurable]]: true }.
                let desc = PropertyDescriptor {
                    value: None,
                    writable: None,
                    get: None,
                    set: Some(closure.into_function().unbind()),
                    enumerable: Some(enumerable),
                    configurable: Some(true),
                };
                // b. Perform ? DefinePropertyOrThrow(object, propKey, desc).
                let object = object.unbind();
                let prop_key = prop_key.unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| define_property_or_throw(agent, object, prop_key, desc, gc),
                    gc,
                )?;
                // c. Return unused.
            }
            Instruction::ObjectSetPrototype => {
                let prop_value = vm.result.take().unwrap().bind(gc.nogc());
                // i. Perform ! object.[[SetPrototypeOf]](propValue).
                let object = Object::try_from(*vm.stack.last().unwrap())
                    .unwrap()
                    .bind(gc.nogc());

                // a. If propValue is an Object or propValue is null, then
                let prop_value = if prop_value.is_null() {
                    None
                } else if let Ok(prop_value) = Object::try_from(prop_value) {
                    Some(prop_value.unbind())
                } else {
                    // b. Return unused.
                    return Ok(ContinuationKind::Normal);
                };

                let object = object.unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| object.internal_set_prototype_of(agent, prop_value, gc),
                    gc,
                )?;
                // b. Return unused.
            }
            Instruction::PushReference => {
                vm.reference_stack.push(vm.reference.take().unwrap());
            }
            Instruction::PopReference => {
                vm.reference = Some(vm.reference_stack.pop().unwrap());
            }
            Instruction::PutValue => {
                let value = vm.result.take().unwrap();
                let reference = vm.reference.take().unwrap();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| put_value(agent, &reference, value, gc),
                    gc,
                )?;
            }
            Instruction::GetValue => {
                // 1. If V is not a Reference Record, return V.
                let reference = vm.reference.take().unwrap();

                let result = if let TryResult::Continue(result) =
                    try_get_value(agent, &reference, gc.nogc())
                {
                    result.unbind()?.bind(gc.into_nogc())
                } else {
                    with_vm_gc(agent, vm, |agent, gc| get_value(agent, &reference, gc), gc)?
                };

                vm.result = Some(result.unbind());
            }
            Instruction::GetValueKeepReference => {
                // 1. If V is not a Reference Record, return V.
                let reference = vm.reference.as_ref().unwrap();

                let reference = if is_property_reference(reference)
                    && !reference.is_static_property_reference()
                {
                    // Expression reference; we need to convert to PropertyKey
                    // first.
                    let referenced_name = reference.referenced_name_value();
                    let referenced_name = if let TryResult::Continue(referenced_name) =
                        to_property_key_simple(agent, referenced_name, gc.nogc())
                    {
                        referenced_name
                    } else {
                        let base = reference.base_value();
                        if base.is_undefined() || base.is_null() {
                            // Undefined and null should throw an error from
                            // ToObject before ToPropertyKey gets called.
                            return Err(throw_read_undefined_or_null_error(
                                agent,
                                referenced_name,
                                base,
                                gc.into_nogc(),
                            ));
                        }
                        let referenced_name = referenced_name.unbind();
                        with_vm_gc(
                            agent,
                            vm,
                            |agent, gc| to_property_key_complex(agent, referenced_name, gc),
                            gc.reborrow(),
                        )
                        .unbind()?
                        .bind(gc.nogc())
                    };
                    let reference = vm.reference.as_mut().unwrap();
                    reference.set_referenced_name_to_property_key(referenced_name);
                    reference.clone()
                } else {
                    reference.clone()
                };

                let result = if let TryResult::Continue(result) =
                    try_get_value(agent, &reference, gc.nogc())
                {
                    result.unbind()?.bind(gc.into_nogc())
                } else {
                    with_vm_gc(agent, vm, |agent, gc| get_value(agent, &reference, gc), gc)?
                };

                vm.result = Some(result.unbind());
            }
            Instruction::Typeof => {
                // 2. If val is a Reference Record, then
                let val = if let Some(reference) = vm.reference.take() {
                    // a. If IsUnresolvableReference(val) is true,
                    if is_unresolvable_reference(&reference) {
                        // return "undefined".
                        Value::Undefined
                    } else {
                        // 3. Set val to ? GetValue(val).
                        if let TryResult::Continue(result) =
                            try_get_value(agent, &reference, gc.nogc())
                        {
                            result.unbind()?.bind(gc.nogc())
                        } else {
                            with_vm_gc(
                                agent,
                                vm,
                                |agent, gc| get_value(agent, &reference, gc),
                                gc.reborrow(),
                            )
                            .unbind()?
                            .bind(gc.nogc())
                        }
                    }
                } else {
                    vm.result.unwrap().bind(gc.nogc())
                };
                vm.result = Some(typeof_operator(agent, val, gc.nogc()).into_value())
            }
            Instruction::ObjectCreate => {
                let object = ordinary_object_create_with_intrinsics(
                    agent,
                    Some(ProtoIntrinsics::Object),
                    None,
                    gc.nogc(),
                );
                vm.stack.push(object.into_value().unbind())
            }
            Instruction::CopyDataProperties => {
                let source = vm.result.take().unwrap();
                let Value::Object(target) = *vm.stack.last().unwrap() else {
                    unreachable!()
                };
                with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| copy_data_properties(agent, target, source, gc),
                    gc,
                )?;
            }
            Instruction::CopyDataPropertiesIntoObject => {
                let from = Object::try_from(vm.result.unwrap())
                    .unwrap()
                    .bind(gc.nogc());

                let num_excluded_items = instr.get_first_index();
                let mut excluded_items = PropertyKeySet::new(gc.nogc());
                assert!(vm.reference.is_none());
                for _ in 0..num_excluded_items {
                    let reference = vm.reference_stack.pop().unwrap();
                    debug_assert_eq!(reference.base_value(), from.into_value());
                    debug_assert!(!is_super_reference(&reference));
                    excluded_items.insert(agent, reference.referenced_name_property_key());
                }

                if let TryResult::Continue(result) =
                    try_copy_data_properties_into_object(agent, from, &excluded_items, gc.nogc())
                {
                    vm.result = Some(result.into_value().unbind());
                } else {
                    let from = from.unbind();
                    let excluded_items = excluded_items.scope(agent, gc.nogc());
                    let result = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            copy_data_properties_into_object(agent, from, excluded_items, gc)
                        },
                        gc,
                    )?;
                    vm.result = Some(result.into_value().unbind());
                }
            }
            Instruction::InstantiateArrowFunctionExpression => {
                // ArrowFunction : ArrowParameters => ConciseBody
                let ArrowFunctionExpression {
                    expression,
                    identifier,
                } = executable.fetch_arrow_function_expression(agent, instr.get_first_index());
                let function_expression = expression.get();
                let identifier = *identifier;
                // 2. Let env be the LexicalEnvironment of the running execution context.
                let env = agent.current_lexical_environment(gc.nogc());
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let private_env = agent.current_private_environment(gc.nogc());
                // 4. Let sourceText be the source text matched by ArrowFunction.
                // 5. Let closure be OrdinaryFunctionCreate(%Function.prototype%, sourceText, ArrowParameters, ConciseBody, LEXICAL-THIS, env, privateEnv).
                // 6. Perform SetFunctionName(closure, name).
                // 7. Return closure.
                // 1. If name is not present, set name to "".
                let params = OrdinaryFunctionCreateParams {
                    function_prototype: None,
                    source_code: None,
                    source_text: function_expression.span,
                    parameters_list: &function_expression.params,
                    body: &function_expression.body,
                    is_concise_arrow_function: function_expression.expression,
                    is_async: function_expression.r#async,
                    is_generator: false,
                    lexical_this: true,
                    env,
                    private_env,
                };
                let mut function = ordinary_function_create(agent, params, gc.nogc());
                let name = if let Some(parameter) = &identifier {
                    let pk_result = match parameter {
                        NamedEvaluationParameter::Result => {
                            let value = vm.result.take().unwrap().bind(gc.nogc());
                            if let TryResult::Continue(pk) =
                                to_property_key_simple(agent, value, gc.nogc())
                            {
                                Ok(pk)
                            } else {
                                Err(value)
                            }
                        }
                        NamedEvaluationParameter::Stack => {
                            let value = vm.stack.last().unwrap().bind(gc.nogc());
                            if let TryResult::Continue(pk) =
                                to_property_key_simple(agent, value, gc.nogc())
                            {
                                Ok(pk)
                            } else {
                                Err(value)
                            }
                        }
                    };

                    match pk_result {
                        Ok(pk) => pk.bind(gc.nogc()),
                        Err(pk_value) => {
                            let scoped_function = function.scope(agent, gc.nogc());
                            let pk_value = pk_value.unbind();
                            let pk = with_vm_gc(
                                agent,
                                vm,
                                |agent, gc| to_property_key_complex(agent, pk_value, gc),
                                gc.reborrow(),
                            )
                            .unbind()?
                            .bind(gc.nogc());
                            // SAFETY: not shared.
                            function = unsafe { scoped_function.take(agent).bind(gc.nogc()) };
                            pk
                        }
                    }
                } else {
                    let pk: PropertyKey = String::EMPTY_STRING.into();
                    pk.bind(gc.nogc())
                };
                set_function_name(agent, function, name, None, gc.nogc());
                vm.result = Some(function.into_value().unbind());
            }
            Instruction::InstantiateOrdinaryFunctionExpression => {
                let FunctionExpression {
                    expression,
                    identifier,
                    ..
                } = executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                let function_expression = expression.get();
                let identifier = *identifier;

                let (name, env, init_binding) = if let Some(parameter) = identifier {
                    debug_assert!(function_expression.id.is_none());
                    let pk = match parameter {
                        NamedEvaluationParameter::Result => vm.result.take().unwrap(),
                        NamedEvaluationParameter::Stack => *vm.stack.last().unwrap(),
                    };
                    let name = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| to_property_key(agent, pk, gc),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    (name, agent.current_lexical_environment(gc.nogc()), false)
                } else if let Some(binding_identifier) = &function_expression.id {
                    let name = String::from_str(agent, &binding_identifier.name, gc.nogc());
                    let func_env = new_declarative_environment(
                        agent,
                        Some(agent.current_lexical_environment(gc.nogc())),
                        gc.nogc(),
                    );
                    func_env.create_immutable_binding(agent, name, false);
                    (name.into(), Environment::Declarative(func_env), true)
                } else {
                    (
                        String::EMPTY_STRING.into(),
                        agent.current_lexical_environment(gc.nogc()),
                        false,
                    )
                };

                let private_env = agent.current_private_environment(gc.nogc());
                let params = OrdinaryFunctionCreateParams {
                    function_prototype: None,
                    source_code: None,
                    source_text: function_expression.span,
                    parameters_list: &function_expression.params,
                    body: function_expression.body.as_ref().unwrap(),
                    is_concise_arrow_function: false,
                    is_async: function_expression.r#async,
                    is_generator: function_expression.generator,
                    lexical_this: false,
                    env,
                    private_env,
                };
                let function = ordinary_function_create(agent, params, gc.nogc());
                let FunctionExpression {
                    compiled_bytecode, ..
                } = executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                if let Some(compiled_bytecode) = compiled_bytecode {
                    agent[function].compiled_bytecode = Some(compiled_bytecode.unbind());
                }
                set_function_name(agent, function, name, None, gc.nogc());
                if !function_expression.r#async && !function_expression.generator {
                    make_constructor(agent, function, None, None, gc.nogc());
                }

                if function_expression.generator {
                    // InstantiateGeneratorFunctionExpression
                    // 7. Let prototype be OrdinaryObjectCreate(%GeneratorFunction.prototype.prototype%).
                    // NOTE: Although `prototype` has the generator prototype, it doesn't have the generator
                    // internals slots, so it's created as an ordinary object.
                    let prototype = ordinary_object_create_with_intrinsics(
                        agent,
                        Some(ProtoIntrinsics::Object),
                        Some(if function_expression.r#async {
                            agent
                                .current_realm_record()
                                .intrinsics()
                                .async_generator_prototype()
                                .into_object()
                        } else {
                            agent
                                .current_realm_record()
                                .intrinsics()
                                .generator_prototype()
                                .into_object()
                        }),
                        gc.nogc(),
                    );
                    // 8. Perform ! DefinePropertyOrThrow(F, "prototype", PropertyDescriptor { [[Value]]: prototype, [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false }).
                    unwrap_try(try_define_property_or_throw(
                        agent,
                        function,
                        BUILTIN_STRING_MEMORY.prototype.to_property_key(),
                        PropertyDescriptor {
                            value: Some(prototype.into_value().unbind()),
                            writable: Some(true),
                            get: None,
                            set: None,
                            enumerable: Some(false),
                            configurable: Some(false),
                        },
                        gc.nogc(),
                    ))
                    .unwrap();
                }

                if init_binding {
                    let name = match name {
                        PropertyKey::SmallString(data) => data.into(),
                        PropertyKey::String(data) => data.unbind().into(),
                        _ => unreachable!("maybe?"),
                    };

                    unwrap_try(env.try_initialize_binding(
                        agent,
                        name,
                        function.into_value(),
                        gc.nogc(),
                    ))
                    .unwrap();
                }

                vm.result = Some(function.into_value().unbind());
            }
            Instruction::ClassDefineConstructor => {
                let name = vm.stack.pop().unwrap();
                let class_name = set_class_name(agent, vm, name, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());

                let FunctionExpression {
                    expression,
                    compiled_bytecode,
                    ..
                } = executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                let function_expression = expression.get();
                let compiled_bytecode = *compiled_bytecode;
                let has_constructor_parent = instr.get_second_bool();

                let function_prototype = if has_constructor_parent {
                    Some(Object::try_from(vm.stack.pop().unwrap()).unwrap())
                } else {
                    None
                };
                let proto = OrdinaryObject::try_from(*vm.stack.last().unwrap()).unwrap();

                let is_null_derived_class = !has_constructor_parent
                    && unwrap_try(proto.try_get_prototype_of(agent, gc.nogc())).is_none();

                let env = agent.current_lexical_environment(gc.nogc());
                let private_env = agent.current_private_environment(gc.nogc());

                let params = OrdinaryFunctionCreateParams {
                    function_prototype,
                    source_code: None,
                    source_text: function_expression.span,
                    parameters_list: &function_expression.params,
                    body: function_expression.body.as_ref().unwrap(),
                    is_concise_arrow_function: false,
                    is_async: function_expression.r#async,
                    is_generator: function_expression.generator,
                    lexical_this: false,
                    env,
                    private_env,
                };
                let function = ordinary_function_create(agent, params, gc.nogc());
                if let Some(compiled_bytecode) = compiled_bytecode {
                    agent[function].compiled_bytecode = Some(compiled_bytecode.unbind());
                }
                set_function_name(agent, function, class_name.into(), None, gc.nogc());
                make_constructor(agent, function, Some(false), Some(proto), gc.nogc());
                agent[function].ecmascript_function.home_object = Some(proto.into_object());
                agent[function].ecmascript_function.constructor_status =
                    if has_constructor_parent || is_null_derived_class {
                        ConstructorStatus::DerivedClass
                    } else {
                        ConstructorStatus::BaseClass
                    };

                unwrap_try(proto.try_define_own_property(
                    agent,
                    BUILTIN_STRING_MEMORY.constructor.into(),
                    PropertyDescriptor {
                        value: Some(function.into_value().unbind()),
                        writable: Some(true),
                        enumerable: Some(false),
                        configurable: Some(true),
                        ..Default::default()
                    },
                    gc.nogc(),
                ));

                vm.result = Some(function.into_value().unbind());
            }
            Instruction::ClassDefineDefaultConstructor => {
                let name = vm.stack.pop().unwrap();
                let class_name = set_class_name(agent, vm, name, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());

                let class_initializer_bytecode_index = instr.get_first_index();
                let (compiled_initializer_bytecode, has_constructor_parent) = executable
                    .fetch_class_initializer_bytecode(
                        agent,
                        class_initializer_bytecode_index,
                        gc.nogc(),
                    );
                let function_prototype = if has_constructor_parent {
                    Some(Object::try_from(vm.stack.pop().unwrap()).unwrap())
                } else {
                    Some(
                        agent
                            .current_realm_record()
                            .intrinsics()
                            .function_prototype()
                            .into_object(),
                    )
                };
                let proto = Object::try_from(*vm.stack.last().unwrap()).unwrap();

                let env = agent.current_lexical_environment(gc.nogc());
                let private_env = agent.current_private_environment(gc.nogc());
                let source_code = agent.current_source_code(gc.nogc());

                let function = create_builtin_constructor(
                    agent,
                    BuiltinConstructorArgs {
                        class_name,
                        is_derived: has_constructor_parent,
                        prototype: function_prototype,
                        prototype_property: proto,
                        compiled_initializer_bytecode,
                        env,
                        private_env,
                        source_code,
                        source_text: Span::new(0, 0),
                    },
                    gc.nogc(),
                );

                unwrap_try(proto.try_define_own_property(
                    agent,
                    BUILTIN_STRING_MEMORY.constructor.into(),
                    PropertyDescriptor {
                        value: Some(function.into_value().unbind()),
                        writable: Some(true),
                        enumerable: Some(false),
                        configurable: Some(true),
                        ..Default::default()
                    },
                    gc.nogc(),
                ));

                vm.result = Some(function.into_value().unbind());
            }
            Instruction::ClassDefinePrivateMethod => {
                let description =
                    String::try_from(vm.result.take().unwrap().bind(gc.nogc())).unwrap();
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.get_first_index(), gc.nogc());
                let bits = instr.get_second_index() as u8;
                let is_static = bits & 0b100 == 0b100;
                let is_setter = bits & 0b10 == 0b10;
                let is_getter = bits & 0b1 == 0b1;
                assert!(vm.stack.len() >= 2);
                let object = if is_static {
                    vm.stack[vm.stack.len() - 1]
                } else {
                    vm.stack[vm.stack.len() - 2]
                };
                let object = Object::try_from(object).unwrap().bind(gc.nogc());
                let function_expression = expression.get();
                // 2. Let env be the running execution context's LexicalEnvironment.
                let env = agent.current_lexical_environment(gc.nogc());
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let private_env = agent.current_private_environment(gc.nogc()).unwrap();
                // 1. Let propKey be ? Evaluation of ClassElementName.
                // 5. Let formalParameterList be ...
                let params = OrdinaryFunctionCreateParams {
                    function_prototype: None,
                    source_code: None,
                    // 4. Let sourceText be the source text matched by MethodDefinition.
                    source_text: function_expression.span,
                    parameters_list: &function_expression.params,
                    body: function_expression.body.as_ref().unwrap(),
                    is_async: function_expression.r#async,
                    is_generator: function_expression.generator,
                    is_concise_arrow_function: false,
                    lexical_this: false,
                    env,
                    private_env: Some(private_env),
                };
                // 6. Let closure be OrdinaryFunctionCreate(
                //      %Function.prototype%,
                //      sourceText,
                //      formalParameterList,
                //      FunctionBody,
                //      non-lexical-this,
                //      env,
                //      privateEnv
                //  ).
                let closure = ordinary_function_create(agent, params, gc.nogc());
                // 7. Perform MakeMethod(closure, object).
                make_method(agent, closure, object.into_object());
                // 8. Perform SetFunctionName(closure, propKey).
                let function_name = format!("#{}", description.to_string_lossy(agent));
                let function_name = String::from_string(agent, function_name, gc.nogc());
                set_function_name(
                    agent,
                    closure,
                    // Note: it should be guaranteed that description is a
                    // non-numeric PropertyKey.
                    function_name.into(),
                    if is_getter {
                        Some(SetFunctionNamePrefix::Get)
                    } else if is_setter {
                        Some(SetFunctionNamePrefix::Set)
                    } else {
                        None
                    },
                    gc.nogc(),
                );
                if is_static {
                    let desc = PropertyDescriptor {
                        value: if !is_getter && !is_setter {
                            Some(closure.into_value().unbind())
                        } else {
                            None
                        },
                        writable: if !is_getter && !is_setter {
                            Some(false)
                        } else {
                            None
                        },
                        get: if is_getter {
                            Some(closure.into_function().unbind())
                        } else {
                            None
                        },
                        set: if is_setter {
                            Some(closure.into_function().unbind())
                        } else {
                            None
                        },
                        enumerable: Some(false),
                        configurable: Some(true),
                    };
                    // b. Perform ? DefinePropertyOrThrow(object, propKey, desc).
                    let private_name = private_env.add_static_private_method(agent, description);
                    let object = object.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            define_property_or_throw(agent, object, private_name.into(), desc, gc)
                        },
                        gc,
                    )?;
                } else {
                    // a. Return PrivateElement {
                    //      [[Key]]: propKey,
                    //      [[Kind]]: ...,
                    //      [[Get]]: ...,
                    //      [[Set]]: ...
                    //    }.
                    let private_method = if is_getter {
                        PrivateMethod::Getter(closure)
                    } else if is_setter {
                        PrivateMethod::Setter(closure)
                    } else {
                        PrivateMethod::Method(closure)
                    };
                    private_env.add_instance_private_method(agent, description, private_method);
                }
                // c. Return unused.
            }
            Instruction::ClassDefinePrivateProperty => {
                let description =
                    executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                let is_static = instr.get_second_bool();
                let private_env = agent
                    .current_private_environment(gc.nogc())
                    .expect("Attempted to define private property with no PrivateEnvironment");
                if is_static {
                    let private_name = private_env.add_static_private_field(agent, description);
                    let object = vm.stack.last().unwrap().bind(gc.nogc());
                    let object = Object::try_from(object).unwrap();
                    object
                        .get_or_create_backing_object(agent)
                        .bind(gc.nogc())
                        .property_storage()
                        .add_private_field_slot(agent, private_name);
                } else {
                    private_env.add_instance_private_field(agent, description);
                }
            }
            Instruction::ClassInitializePrivateElements => {
                let gc = gc.into_nogc();
                let target = Object::try_from(vm.stack.last().unwrap().bind(gc)).unwrap();
                target
                    .get_or_create_backing_object(agent)
                    .property_storage()
                    .initialize_private_elements(agent, gc)?;
            }
            Instruction::ClassInitializePrivateValue => {
                let gc = gc.into_nogc();
                let target = Object::try_from(vm.stack.last().unwrap().bind(gc)).unwrap();
                let value = vm.result.take().unwrap().bind(gc);
                if target.is_proxy() {
                    return Err(throw_no_proxy_private_names(agent, gc));
                }
                let offset = instr.get_first_index();
                let private_env = agent
                    .current_private_environment(gc)
                    .expect("Attempted to define private property with no PrivateEnvironment");
                // SAFETY: Generated bytecode only uses this instruction when
                // it statically knows we're inside the correct
                // PrivateEnvironment.
                let private_name = unsafe { private_env.get_private_name(agent, offset) };
                assert!(
                    target
                        .get_or_create_backing_object(agent)
                        .property_storage()
                        .set_private_field_value(agent, private_name, offset, value)
                );
            }
            Instruction::Swap => {
                let a = vm.stack.pop().unwrap();
                let b = vm.stack.pop().unwrap();
                vm.stack.push(a);
                vm.stack.push(b);
            }
            Instruction::Empty => {
                vm.result = None;
            }
            Instruction::DirectEvalCall => {
                let func = with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let func_ref =
                            resolve_binding(agent, BUILTIN_STRING_MEMORY.eval, None, gc.reborrow())
                                .unbind()?
                                .bind(gc.nogc());
                        get_value(agent, &func_ref.unbind(), gc)
                    },
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                let args = vm.get_call_args(instr, gc.nogc());

                // a. If SameValue(func, %eval%) is true, then
                let result = if func
                    == agent
                        .current_realm_record()
                        .intrinsics()
                        .eval()
                        .into_value()
                {
                    // i. Let argList be ? ArgumentListEvaluation of arguments.
                    // ii. If argList has no elements, return undefined.
                    if args.is_empty() {
                        Value::Undefined
                    } else {
                        // iii. Let evalArg be the first element of argList.
                        let eval_arg = args[0];
                        // iv. If IsStrict(this CallExpression) is true, let
                        //     strictCaller be true. Otherwise let strictCaller
                        //     be false.
                        let strict_caller = agent
                            .running_execution_context()
                            .ecmascript_code
                            .unwrap()
                            .is_strict_mode;
                        // v. Return ? PerformEval(evalArg, strictCaller, true).
                        let eval_arg = eval_arg.unbind();
                        with_vm_gc(
                            agent,
                            vm,
                            |agent, gc| perform_eval(agent, eval_arg, true, strict_caller, gc),
                            gc,
                        )?
                    }
                } else {
                    let func = func.unbind();
                    let mut args = args.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            call(
                                agent,
                                func,
                                Value::Undefined,
                                Some(ArgumentsList::from_mut_slice(args.as_mut_slice())),
                                gc,
                            )
                        },
                        gc,
                    )?
                };
                vm.result = Some(result.unbind());
            }
            Instruction::EvaluateCall => {
                let reference = vm.reference.take();
                // 1. If ref is a Reference Record, then
                let this_value = if let Some(reference) = reference {
                    // a. If IsPropertyReference(ref) is true, then
                    if is_property_reference(&reference) {
                        // i. Let thisValue be GetThisValue(ref).
                        get_this_value(&reference).bind(gc.nogc())
                    } else {
                        // b. Else,
                        // i. Let refEnv be ref.[[Base]].
                        // ii. Assert: refEnv is an Environment Record.
                        let ref_env = reference.base_env();
                        // iii. Let thisValue be refEnv.WithBaseObject().
                        ref_env
                            .with_base_object(agent)
                            .map_or(Value::Undefined, |object| object.into_value())
                            .bind(gc.nogc())
                    }
                } else {
                    // 2. Else,
                    // a. Let thisValue be undefined.
                    Value::Undefined
                };
                let mut args = vm.get_call_args(instr, gc.nogc()).unbind();
                let func = vm.stack.pop().unwrap().unbind();
                let this_value = this_value.unbind();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        call(
                            agent,
                            func,
                            this_value,
                            Some(ArgumentsList::from_mut_slice(args.as_mut_slice())),
                            gc,
                        )
                    },
                    gc,
                )?;
                vm.result = Some(result.unbind());
            }
            Instruction::EvaluateNew => {
                let args = vm.get_call_args(instr, gc.nogc());
                let constructor = vm.stack.pop().unwrap().bind(gc.nogc());
                let Some(constructor) = is_constructor(agent, constructor) else {
                    let constructor_string = {
                        let constructor = constructor.unbind();
                        with_vm_gc(
                            agent,
                            vm,
                            |agent, gc| constructor.string_repr(agent, gc),
                            gc.reborrow(),
                        )
                    };
                    let error_message = format!(
                        "'{}' is not a constructor.",
                        constructor_string.to_string_lossy(agent)
                    );
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        error_message,
                        gc.into_nogc(),
                    ));
                };

                let constructor = constructor.unbind();
                let mut args = args.unbind();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        construct(
                            agent,
                            constructor,
                            Some(ArgumentsList::from_mut_slice(args.as_mut_slice())),
                            None,
                            gc,
                        )
                    },
                    gc,
                )?;
                vm.result = Some(result.unbind().into_value());
            }
            Instruction::EvaluateSuper => {
                let Environment::Function(this_env) = get_this_environment(agent, gc.nogc()) else {
                    unreachable!();
                };
                // 1. Let newTarget be GetNewTarget().
                // 2. Assert: newTarget is an Object.
                // 3. Let func be GetSuperConstructor().
                let (new_target, func) = {
                    let new_target = this_env.get_new_target(agent);
                    let function_object = this_env.get_function_object(agent);
                    (
                        Function::try_from(new_target.unwrap())
                            .unwrap()
                            .bind(gc.nogc()),
                        unwrap_try(function_object.try_get_prototype_of(agent, gc.nogc())),
                    )
                };
                // 4. Let argList be ? ArgumentListEvaluation of Arguments.
                let arg_list = vm.get_call_args(instr, gc.nogc());
                // 5. If IsConstructor(func) is false, throw a TypeError exception.
                let Some(func) = func.and_then(|func| is_constructor(agent, func)) else {
                    let constructor = func.map_or(Value::Null, |f| f.into_value().unbind());
                    let error_message = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            format!(
                                "'{}' is not a constructor.",
                                constructor
                                    .into_value()
                                    .string_repr(agent, gc)
                                    .to_string_lossy(agent)
                            )
                        },
                        gc.reborrow(),
                    );
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        error_message,
                        gc.into_nogc(),
                    ));
                };
                // 6. Let result be ? Construct(func, argList, newTarget).
                let result = {
                    let func = func.unbind();
                    let mut arg_list = arg_list.unbind();
                    let new_target = new_target.unbind();
                    let result = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            construct(
                                agent,
                                func,
                                Some(ArgumentsList::from_mut_slice(arg_list.as_mut_slice())),
                                Some(new_target),
                                gc,
                            )
                        },
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    result.unbind().bind(gc.nogc())
                };
                // 7. Let thisER be GetThisEnvironment().
                let Environment::Function(this_er) = get_this_environment(agent, gc.nogc()) else {
                    unreachable!();
                };
                // 8. Perform ? thisER.BindThisValue(result).
                this_er
                    .bind_this_value(agent, result.into_value(), gc.nogc())
                    .unbind()?
                    .bind(gc.nogc());
                // 9. Let F be thisER.[[FunctionObject]].
                // 10. Assert: F is an ECMAScript function object.
                let Function::ECMAScriptFunction(_f) = this_er.get_function_object(agent) else {
                    unreachable!();
                };
                // 11. Perform ? InitializeInstanceElements(result, F).
                // 12. Return result.
                vm.result = Some(result.into_value().unbind());
            }
            Instruction::EvaluatePropertyAccessWithExpressionKey => {
                let property_name_value = vm.result.take().unwrap().bind(gc.nogc());
                let base_value = vm.stack.pop().unwrap().bind(gc.nogc());
                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;

                vm.reference = Some(
                    Reference::new_property_expression_reference(
                        base_value,
                        property_name_value,
                        strict,
                    )
                    .unbind(),
                );
            }
            Instruction::EvaluatePropertyAccessWithIdentifierKey => {
                let property_name_string =
                    executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                let base_value = vm.result.take().unwrap().bind(gc.nogc());
                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;

                vm.reference = Some(
                    Reference::new_property_reference(
                        base_value,
                        // Note: identifiers cannot be numeric.
                        property_name_string.to_property_key(),
                        strict,
                    )
                    .unbind(),
                );
            }
            Instruction::MakePrivateReference => {
                let gc = gc.into_nogc();
                let private_identifier =
                    executable.fetch_identifier(agent, instr.get_first_index(), gc);
                let base_value = vm.result.take().unwrap().bind(gc);
                // 1. Let privateEnv be the running execution context's
                //    PrivateEnvironment.
                // 2. Assert: privateEnv is not null.
                let private_env = agent
                    .current_private_environment(gc)
                    .expect("Attempted to make private reference in non-class environment");
                // 3. Let privateName be ResolvePrivateIdentifier(privateEnv, privateIdentifier).
                let private_name =
                    resolve_private_identifier(agent, private_env, private_identifier);
                // 4. Return the Reference Record {
                //    [[Base]]: baseValue,
                //    [[ReferencedName]]: privateName,
                //    [[Strict]]: true,
                //    [[ThisValue]]: empty
                // }.
                vm.reference
                    .replace(Reference::new_private_reference(base_value, private_name).unbind());
            }
            Instruction::MakeSuperPropertyReferenceWithExpressionKey => {
                // ### SuperProperty : super [ Expression ]

                // 1. Let env be GetThisEnvironment().
                let env = get_this_environment(agent, gc.nogc());
                // 2. Let actualThis be ? env.GetThisBinding().
                let actual_this = env
                    .get_this_binding(agent, gc.nogc())
                    .unbind()?
                    .bind(gc.nogc());
                // 3. Let propertyNameReference be ? Evaluation of Expression.
                // 4. Let propertyNameValue be ? GetValue(propertyNameReference).
                let property_name_value = vm.result.take().unwrap().bind(gc.nogc());
                // 5. Let strict be IsStrict(this SuperProperty).
                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;
                // 6. NOTE: In most cases, ToPropertyKey will be performed on
                //    propertyNameValue immediately after this step. However,
                //    in the case of super[b] = c, it will not be performed
                //    until after evaluation of c.
                // 7. Return MakeSuperPropertyReference(
                //        actualThis,
                //        propertyNameValue,
                //        strict
                //    ).
                // 1. Let env be GetThisEnvironment().
                // 2. Assert: env.HasSuperBinding() is true.
                // 3. Assert: env is a Function Environment Record.
                debug_assert!(env.has_super_binding(agent));
                let Environment::Function(env) = env else {
                    unreachable!()
                };
                // 4. Let baseValue be GetSuperBase(env).
                let base_value = env.get_super_base(agent, gc.nogc());

                // 5. Return the Reference Record {
                // [[Base]]: baseValue,
                vm.reference = Some(
                    Reference::new_super_expression_reference(
                        base_value,
                        // [[ReferencedName]]: propertyKey,
                        property_name_value,
                        // [[ThisValue]]: actualThis
                        actual_this,
                        // [[Strict]]: strict,
                        strict,
                    )
                    .unbind(),
                );
                // }.
            }
            Instruction::MakeSuperPropertyReferenceWithIdentifierKey => {
                // ### SuperProperty : super . IdentifierName

                // 1. Let env be GetThisEnvironment().
                let env = get_this_environment(agent, gc.nogc());
                // 2. Let actualThis be ? env.GetThisBinding().
                let actual_this = env
                    .get_this_binding(agent, gc.nogc())
                    .unbind()?
                    .bind(gc.nogc());
                // 3. Let propertyKey be the StringValue of IdentifierName.
                let property_key =
                    executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                // 4. Let strict be IsStrict(this SuperProperty).
                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;
                // 5. Return MakeSuperPropertyReference(actualThis, propertyKey, strict).
                // 1. Let env be GetThisEnvironment().
                // 2. Assert: env.HasSuperBinding() is true.
                // 3. Assert: env is a Function Environment Record.
                assert!(env.has_super_binding(agent));
                let Environment::Function(env) = env else {
                    unreachable!()
                };
                // 4. Let baseValue be GetSuperBase(env).
                let base_value = env.get_super_base(agent, gc.nogc());
                // 4. Let baseValue be GetSuperBase(env).
                // 5. Return the Reference Record {
                vm.reference = Some(
                    Reference::new_super_reference(
                        // [[Base]]: baseValue,
                        base_value,
                        // [[ReferencedName]]: propertyKey,
                        // Note: identifiers cannot be numeric.
                        property_key.to_property_key(),
                        // [[ThisValue]]: actualThis
                        actual_this,
                        // [[Strict]]: strict,
                        strict,
                    )
                    .unbind(),
                );
                // }.
            }
            Instruction::Jump => {
                let ip = instr.get_jump_slot();
                if agent.options.print_internals {
                    eprintln!("Jumping to {ip}");
                }
                vm.ip = ip;
            }
            Instruction::JumpIfNot => {
                let result = vm.result.take().unwrap();
                let ip = instr.get_jump_slot();
                if !to_boolean(agent, result) {
                    if agent.options.print_internals {
                        eprintln!("Comparison failed, jumping to {ip}");
                    }
                    vm.ip = ip;
                }
            }
            Instruction::JumpIfTrue => {
                let result = vm.result.take().unwrap();
                let Value::Boolean(result) = result else {
                    unreachable!()
                };
                if result {
                    let ip = instr.get_jump_slot();
                    if agent.options.print_internals {
                        eprintln!("Comparison succeeded, jumping to {ip}");
                    }
                    vm.ip = ip;
                }
            }
            Instruction::Increment => {
                let lhs = vm.result.take().unwrap().bind(gc.nogc());
                // Note: This is done by the previous instruction.
                let old_value = Numeric::try_from(lhs).unwrap();
                let new_value = if let Ok(old_value) = Number::try_from(old_value) {
                    Number::add(agent, old_value, 1.into()).into_value()
                } else {
                    let old_value = BigInt::try_from(old_value).unwrap();
                    BigInt::add(agent, old_value, 1.into()).into_value()
                };
                vm.result = Some(new_value.unbind())
            }
            Instruction::Decrement => {
                let lhs = vm.result.take().unwrap();
                // Note: This is done by the previous instruction.
                let old_value = Numeric::try_from(lhs).unwrap();
                let new_value = if let Ok(old_value) = Number::try_from(old_value) {
                    Number::subtract(agent, old_value, 1.into()).into_value()
                } else {
                    let old_value = BigInt::try_from(old_value).unwrap();
                    BigInt::subtract(agent, old_value, 1.into()).into_value()
                };
                vm.result = Some(new_value.unbind());
            }
            Instruction::LessThan => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| is_less_than::<true>(agent, lval, rval, gc),
                    gc,
                )?;
                let result = result == Some(true);
                vm.result = Some(result.into());
            }
            Instruction::LessThanEquals => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| is_less_than::<false>(agent, rval, lval, gc),
                    gc,
                )?;
                let result = result == Some(false);
                vm.result = Some(result.into());
            }
            Instruction::GreaterThan => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| is_less_than::<false>(agent, rval, lval, gc),
                    gc,
                )?;
                let result = result == Some(true);
                vm.result = Some(result.into());
            }
            Instruction::GreaterThanEquals => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| is_less_than::<true>(agent, lval, rval, gc),
                    gc,
                )?;
                let result = result == Some(false);
                vm.result = Some(result.into());
            }
            Instruction::HasProperty => {
                let lval = vm.stack.pop().unwrap().bind(gc.nogc());
                let rval = vm.result.take().unwrap().bind(gc.nogc());
                // RelationalExpression : RelationalExpression in ShiftExpression
                // 5. If rval is not an Object, throw a TypeError exception.
                let Ok(mut rval) = Object::try_from(rval) else {
                    return Err(throw_error_in_target_not_object(
                        agent,
                        rval.unbind(),
                        gc.into_nogc(),
                    ));
                };
                // 6. Return ? HasProperty(rval, ? ToPropertyKey(lval)).
                let property_key = if lval.is_string() || lval.is_integer() {
                    unwrap_try(to_property_key_simple(agent, lval, gc.nogc()))
                } else {
                    let scoped_rval = rval.scope(agent, gc.nogc());
                    let lval = lval.unbind();
                    let property_key = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| to_property_key(agent, lval, gc),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    // SAFETY: not shared.
                    rval = unsafe { scoped_rval.take(agent).bind(gc.nogc()) };
                    property_key
                };
                let result = if let TryResult::Continue(result) =
                    try_has_property(agent, rval, property_key, gc.nogc())
                {
                    result
                } else {
                    let rval = rval.unbind();
                    let property_key = property_key.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| has_property(agent, rval, property_key, gc),
                        gc,
                    )?
                };
                vm.result = Some(result.into());
            }
            Instruction::HasPrivateElement => {
                let Some(reference) = vm.reference.take() else {
                    unreachable!()
                };
                let (r_val, private_name) = reference.into_private_reference_data();
                // 4. If rVal is not an Object,
                if let Ok(r_val) = Object::try_from(r_val) {
                    // 8. If PrivateElementFind(rVal, privateName) is not
                    //    empty, return true.
                    // 9. Return false.
                    let result = private_element_find(agent, r_val, private_name).is_some();
                    vm.result = Some(result.into_value());
                } else {
                    // throw a TypeError exception.
                    return Err(throw_error_in_target_not_object(
                        agent,
                        r_val,
                        gc.into_nogc(),
                    ));
                }
            }
            Instruction::IsStrictlyEqual => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_strictly_equal(agent, lval, rval);
                vm.result = Some(result.into());
            }
            Instruction::IsLooselyEqual => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| is_loosely_equal(agent, lval, rval, gc),
                    gc,
                )?;
                vm.result = Some(result.into());
            }
            Instruction::IsNullOrUndefined => {
                let val = vm.result.take().unwrap();
                let result = val.is_null() || val.is_undefined();
                vm.result = Some(result.into());
            }
            Instruction::IsUndefined => {
                let val = vm.result.take().unwrap();
                let result = val.is_undefined();
                vm.result = Some(result.into());
            }
            Instruction::IsNull => {
                let val = vm.result.take().unwrap();
                let result = val.is_null();
                vm.result = Some(result.into());
            }
            Instruction::IsObject => {
                let val = vm.result.take().unwrap();
                let result = val.is_object();
                vm.result = Some(result.into());
            }
            Instruction::IsConstructor => {
                let val = vm.result.take().unwrap();
                let result = if let Ok(val) = Function::try_from(val) {
                    val.is_constructor(agent)
                } else {
                    false
                };
                vm.result = Some(result.into());
            }
            Instruction::LogicalNot => {
                // 2. Let oldValue be ToBoolean(? GetValue(expr)).
                let old_value = to_boolean(agent, vm.result.take().unwrap());

                // 3. If oldValue is true, return false.
                // 4. Return true.
                vm.result = Some((!old_value).into());
            }
            Instruction::InitializeReferencedBinding => {
                let v = vm.reference.take().unwrap();
                let w = vm.result.take().unwrap();
                // Note: https://tc39.es/ecma262/#sec-initializereferencedbinding
                // suggests this cannot call user code, hence NoGC.
                unwrap_try(try_initialize_referenced_binding(agent, v, w, gc.nogc()))
                    .unbind()?
                    .bind(gc.nogc());
            }
            Instruction::InitializeVariableEnvironment => {
                let num_variables = instr.get_first_index();
                let strict = instr.get_second_bool();

                // 10.2.11 FunctionDeclarationInstantiation
                // 28.b. Let varEnv be NewDeclarativeEnvironment(env).
                let env = agent.current_lexical_environment(gc.nogc());
                let var_env = new_declarative_environment(agent, Some(env), gc.nogc());
                // c. Set the VariableEnvironment of calleeContext to varEnv.
                agent.set_current_variable_environment(var_env.into());

                // e. For each element n of varNames, do
                for _ in 0..num_variables {
                    let n = String::try_from(vm.stack.pop().unwrap()).unwrap();
                    let initial_value = vm.stack.pop().unwrap();
                    // 2. Perform ! varEnv.CreateMutableBinding(n, false).
                    var_env.create_mutable_binding(agent, n, false);
                    // 5. Perform ! varEnv.InitializeBinding(n, initialValue).
                    var_env.initialize_binding(agent, n, initial_value);
                }

                // 30. If strict is false, then
                let lex_env = if !strict {
                    // a. Let lexEnv be NewDeclarativeEnvironment(varEnv).
                    new_declarative_environment(
                        agent,
                        Some(Environment::Declarative(var_env)),
                        gc.nogc(),
                    )
                } else {
                    // 31. Else,
                    // a. Let lexEnv be varEnv.
                    var_env
                };

                // 32. Set the LexicalEnvironment of calleeContext to lexEnv.
                agent.set_current_lexical_environment(lex_env.into());
            }
            Instruction::EnterDeclarativeEnvironment => {
                let outer_env = agent.current_lexical_environment(gc.nogc());
                let new_env = new_declarative_environment(agent, Some(outer_env), gc.nogc());
                agent.set_current_lexical_environment(new_env.into());
            }
            Instruction::EnterClassStaticElementEnvironment => {
                let class_constructor = Function::try_from(*vm.stack.last().unwrap())
                    .unwrap()
                    .bind(gc.nogc());
                let local_env =
                    new_class_static_element_environment(agent, class_constructor, gc.nogc());
                let local_env = Environment::Function(local_env);

                agent.set_current_lexical_environment(local_env);
                agent.set_current_variable_environment(local_env);
            }
            Instruction::EnterPrivateEnvironment => {
                let outer_env = agent.current_private_environment(gc.nogc());
                let new_env =
                    new_private_environment(agent, outer_env, instr.get_first_index(), gc.nogc());
                agent.set_current_private_environment(new_env.into());
            }
            Instruction::ExitDeclarativeEnvironment => {
                let old_env = agent
                    .current_lexical_environment(gc.nogc())
                    .get_outer_env(agent)
                    .unwrap();
                agent.set_current_lexical_environment(old_env);
            }
            Instruction::ExitVariableEnvironment => {
                let old_env = agent
                    .current_variable_environment(gc.nogc())
                    .get_outer_env(agent)
                    .unwrap();
                agent.set_current_variable_environment(old_env);
            }
            Instruction::ExitPrivateEnvironment => {
                let old_env = agent
                    .current_private_environment(gc.nogc())
                    .unwrap()
                    .get_outer_env(agent);
                agent.set_current_private_environment(old_env);
            }
            Instruction::CreateMutableBinding => {
                let lex_env = agent.current_lexical_environment(gc.nogc());
                let name = executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());

                unwrap_try(lex_env.try_create_mutable_binding(
                    agent,
                    name.unbind(),
                    false,
                    gc.nogc(),
                ))
                .unwrap();
            }
            Instruction::CreateImmutableBinding => {
                let lex_env = agent.current_lexical_environment(gc.nogc());
                let name = executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                lex_env
                    .create_immutable_binding(agent, name, true, gc.nogc())
                    .unwrap();
            }
            Instruction::Throw => {
                let result = vm.result.take().unwrap();
                return Err(JsError::new(result));
            }
            Instruction::ThrowError => {
                let exception_type_immediate = instr.get_first_arg();
                let message = String::try_from(vm.result.take().unwrap()).unwrap();

                let exception_type = ExceptionType::try_from(exception_type_immediate).unwrap();

                return Err(agent.throw_exception_with_message(
                    exception_type,
                    message,
                    gc.into_nogc(),
                ));
            }
            Instruction::PushExceptionJumpTarget => {
                vm.exception_handler_stack
                    .push(ExceptionHandler::CatchBlock {
                        // Note: jump slots are passed to us in 32 bits, this
                        // conversion is lossless.
                        ip: instr.get_jump_slot() as u32,
                        lexical_environment: agent.current_lexical_environment(gc.nogc()).unbind(),
                    });
            }
            Instruction::PopExceptionJumpTarget => {
                vm.exception_handler_stack.pop().unwrap();
            }
            Instruction::InstanceofOperator => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| instanceof_operator(agent, lval, rval, gc),
                    gc,
                )?;
                vm.result = Some(result.into());
            }
            Instruction::BeginSimpleArrayBindingPattern => {
                let lexical = instr.get_second_bool();
                let env = if lexical {
                    // Lexical binding, const [] = a; or let [] = a;
                    Some(
                        agent
                            .current_lexical_environment(gc.nogc())
                            .scope(agent, gc.nogc()),
                    )
                } else {
                    // Var binding, var [] = a;
                    None
                };
                execute_simple_array_binding(agent, vm, executable, env, gc.reborrow()).unbind()?;
            }
            Instruction::BeginSimpleObjectBindingPattern => {
                let lexical = instr.get_first_bool();
                let env = if lexical {
                    // Lexical binding, const {} = a; or let {} = a;
                    Some(
                        agent
                            .current_lexical_environment(gc.nogc())
                            .scope(agent, gc.nogc()),
                    )
                } else {
                    // Var binding, var {} = a;
                    None
                };
                let object = to_object(agent, vm.result.take().unwrap(), gc.nogc())
                    .unbind()?
                    .bind(gc.nogc());
                execute_simple_object_binding(agent, vm, executable, object.unbind(), env, gc)?
            }
            Instruction::BindingPatternBind
            | Instruction::BindingPatternBindNamed
            | Instruction::BindingPatternBindRest
            | Instruction::BindingPatternSkip
            | Instruction::BindingPatternGetValue
            | Instruction::BindingPatternGetValueNamed
            | Instruction::BindingPatternGetRestValue
            | Instruction::FinishBindingPattern => {
                unreachable!("BeginArrayBindingPattern should take care of stepping over these");
            }
            Instruction::StringConcat => {
                let argument_count = instr.get_first_index();
                let first_arg_index = vm.stack.len() - argument_count;
                let mut length = 0;
                let all_easy = vm.stack[first_arg_index..]
                    .iter()
                    .all(|ele| ele.is_primitive() && !ele.is_symbol());
                let string = if all_easy {
                    let gc = gc.nogc();
                    let args = &mut vm.stack[first_arg_index..];
                    for arg in args.iter_mut() {
                        let string: String<'_> =
                            to_string_primitive(agent, Primitive::try_from(*arg).unwrap(), gc)
                                .unwrap();
                        length += string.len(agent);
                        // Note: We write String into each arg.
                        *arg = string.into_value().unbind();
                    }
                    let args = &*args;
                    // SAFETY: String is a sub-enum of Value and we've written
                    // a String into each of the args.
                    let args = unsafe { std::mem::transmute::<&[Value<'_>], &[String<'_>]>(args) };
                    concat_string_from_slice(agent, args, length, gc)
                } else {
                    let mut args = vm
                        .stack
                        .split_off(first_arg_index)
                        .iter_mut()
                        .map(|v| v.scope(agent, gc.nogc()))
                        .collect::<Vec<_>>();
                    with_vm_gc::<JsResult<()>>(
                        agent,
                        vm,
                        |agent, mut gc| {
                            for ele in args.iter_mut() {
                                let maybe_string = ele.get(agent).bind(gc.nogc());
                                if maybe_string.is_string() {
                                    continue;
                                }
                                let string = to_string(agent, maybe_string.unbind(), gc.reborrow())
                                    .unbind()?
                                    .bind(gc.nogc());
                                length += string.len(agent);
                                let string = string.into_value();
                                // SAFETY: args are never shared
                                unsafe { ele.replace(agent, string.unbind()) };
                            }
                            Ok(())
                        },
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                    let gc = gc.nogc();
                    let args = args
                        .into_iter()
                        .map(|v| String::try_from(v.get(agent)).unwrap().bind(gc))
                        .collect::<Vec<_>>();
                    concat_string_from_slice(agent, &args, length, gc)
                };
                vm.stack.truncate(first_arg_index);
                vm.result = Some(string.into_value().unbind());
            }
            Instruction::Delete => delete_evaluation(agent, vm, gc)?,
            Instruction::EnumerateObjectProperties => {
                let object = to_object(agent, vm.result.take().unwrap(), gc.nogc()).unwrap();
                vm.iterator_stack.push(
                    VmIteratorRecord::ObjectProperties(Box::new(
                        ObjectPropertiesIteratorRecord::new(object),
                    ))
                    .unbind(),
                )
            }
            Instruction::GetIteratorSync => {
                let expr_value = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| VmIteratorRecord::from_value(agent, expr_value, gc),
                    gc,
                )?;
                vm.iterator_stack.push(result.unbind());
            }
            Instruction::GetIteratorAsync => {
                let expr_value = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| VmIteratorRecord::async_from_value(agent, expr_value, gc),
                    gc,
                )?;
                vm.iterator_stack.push(result.unbind());
            }
            Instruction::IteratorStepValue => {
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).step_value(agent, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                vm.result = result.unbind();
                if result.is_none() {
                    // Iterator finished: jump to escape the iterator loop.
                    let ip = instr.get_jump_slot();
                    if agent.options.print_internals {
                        eprintln!("Iterator finished, jumping to {ip}");
                    }
                    vm.ip = ip;
                }
            }
            Instruction::IteratorStepValueOrUndefined => {
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).step_value(agent, gc),
                    gc,
                );
                if result.map_or(true, |r| r.is_none()) {
                    // We have exhausted the iterator or it threw an error;
                    // replace the top iterator with an empty slice iterator so
                    // further instructions aren't observable.
                    *vm.get_active_iterator_mut() = VmIteratorRecord::EmptySliceIterator;
                }
                vm.result = Some(result?.unwrap_or(Value::Undefined).unbind());
            }
            Instruction::IteratorCallNextMethod => {
                let result = vm.result.take();
                vm.result = Some(
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            ActiveIterator::new(agent, gc.nogc()).call_next(agent, result, gc)
                        },
                        gc,
                    )?
                    .unbind(),
                );
            }
            Instruction::IteratorComplete => {
                // 1. If innerResult is not an Object, throw a TypeError
                //    exception.
                let result = vm
                    .result
                    .expect("No iterator result object")
                    .bind(gc.nogc());
                let Ok(result) = Object::try_from(result) else {
                    return Err(throw_iterator_returned_non_object(agent, gc.into_nogc()));
                };
                // 2. Let done be ? IteratorComplete(innerResult).
                let done = {
                    let result = result.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| iterator_complete(agent, result, gc),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc())
                };
                // 3. If done is true, then
                if done {
                    // SAFETY: Result was checked to be an Object already.
                    let result = unsafe {
                        Object::try_from(vm.result.unwrap_unchecked())
                            .unwrap_unchecked()
                            .bind(gc.nogc())
                    };
                    // i. Return ? IteratorValue(innerResult).
                    let result = {
                        let result = result.unbind();
                        with_vm_gc(
                            agent,
                            vm,
                            // SAFETY: not shared.
                            |agent, gc| iterator_value(agent, result, gc),
                            gc,
                        )?
                    };
                    vm.result = Some(result.unbind());
                    let ip = instr.get_jump_slot();
                    if agent.options.print_internals {
                        eprintln!("IteratorComplete returned true, jumping to {ip}");
                    }
                    vm.ip = ip;
                }
            }
            Instruction::IteratorValue => {
                let result = vm
                    .result
                    .expect("No iterator result object")
                    .bind(gc.nogc());
                // NOTE: We crash here because this check should've been done
                // already.
                let result =
                    Object::try_from(result).expect("Iterator returned a non-object result");
                // 1. Return ? IteratorValue(innerResult).
                let result = {
                    let result = result.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        // SAFETY: not shared.
                        |agent, gc| iterator_value(agent, result, gc),
                        gc,
                    )?
                };
                vm.result = Some(result.unbind());
            }
            Instruction::IteratorThrow => {
                let result = vm.result.take().expect("IteratorThrow with no error");
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).throw(agent, result, gc),
                    gc,
                )?;
                if let Some(result) = result {
                    // Throw method was found and called successfully.
                    vm.result = Some(result.unbind());
                } else {
                    // No throw method was found, we should jump to provided
                    // instruction.
                    let ip = instr.get_jump_slot();
                    if agent.options.print_internals {
                        eprintln!("No iterator throw method found, jumping to {ip}");
                    }
                    vm.ip = ip;
                }
            }
            Instruction::IteratorReturn => {
                let result = vm.result.expect("IteratorReturn with no received value");
                let value = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        ActiveIterator::new(agent, gc.nogc()).r#return(agent, Some(result), gc)
                    },
                    gc,
                )?;
                if let Some(value) = value {
                    // Return method was found and called successfully.
                    vm.result = Some(value.unbind());
                } else {
                    // No return method was found, we should jump to provided
                    // instruction.
                    let ip = instr.get_jump_slot();
                    if agent.options.print_internals {
                        eprintln!("No iterator return method found, jumping to {ip}");
                    }
                    vm.ip = ip;
                }
            }
            Instruction::IteratorRestIntoArray => {
                let capacity = vm
                    .get_active_iterator()
                    .remaining_length_estimate(agent)
                    .unwrap_or(0);
                let array = array_create(agent, 0, capacity, None, gc.nogc())
                    .unbind()?
                    .scope(agent, gc.nogc());

                let result = with_vm_gc::<JsResult<()>>(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let mut idx: u32 = 0;
                        while let Some(value) = ActiveIterator::new(agent, gc.nogc())
                            .step_value(agent, gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc())
                        {
                            let key = PropertyKey::Integer(idx.into());
                            unwrap_try(try_create_data_property(
                                agent,
                                array.get(agent),
                                key,
                                value.unbind(),
                                gc.nogc(),
                            ));
                            idx += 1;
                        }
                        Ok(())
                    },
                    gc,
                );
                // We have exhausted the iterator or it threw an error; replace
                // the top iterator with an empty slice iterator so further
                // instructions aren't observable.
                *vm.get_active_iterator_mut() = VmIteratorRecord::EmptySliceIterator;
                // Now we're ready to throw the possible error.;
                result?;

                // Store the array as the result.
                // SAFETY: not shared.
                vm.result = Some(unsafe { array.take(agent).into_value() });
            }
            Instruction::IteratorClose => {
                if !vm
                    .get_active_iterator()
                    .requires_return_call(agent, gc.nogc())
                {
                    return Ok(ContinuationKind::Normal);
                }
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).r#return(agent, None, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                if let Some(result) = result {
                    // We did get innerResult from return method call: we have
                    // to check that it is an object.
                    verify_is_object(agent, result.unbind(), gc.into_nogc())?;
                }
            }
            Instruction::AsyncIteratorClose => {
                if !vm
                    .get_active_iterator()
                    .requires_return_call(agent, gc.nogc())
                {
                    // Skip over VerifyIsObject, message, and Store.
                    vm.ip += 4;
                    return Ok(ContinuationKind::Normal);
                }
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).r#return(agent, None, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                if let Some(result) = result {
                    // AsyncIteratorClose
                    // Iterator return method did return a value: we should
                    // put it into the result slot, place our original
                    // result into the stack, and perform an Await.
                    let result = vm.result.replace(result.unbind());
                    vm.stack.push(result.unwrap_or(Value::Undefined));
                    return Ok(ContinuationKind::Await);
                } else {
                    // Skip over VerifyIsObject, message, and Store.
                    vm.ip += 4;
                }
            }
            Instruction::IteratorCloseWithError => {
                // If our current active iterator requires a return call,
                // perform said call.
                if vm
                    .get_active_iterator()
                    .requires_return_call(agent, gc.nogc())
                {
                    // We don't care if the return call throws an error or not,
                    // nor if its returned value is an object or not: the
                    // original throw completion will be rethrown before that.
                    let _ = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| ActiveIterator::new(agent, gc.nogc()).r#return(agent, None, gc),
                        gc,
                    );
                }
                // Continue to the next instruction which should either be a
                // throw or an IteratorPop followed by a throw.
            }
            Instruction::AsyncIteratorCloseWithError => {
                if vm
                    .get_active_iterator()
                    .requires_return_call(agent, gc.nogc())
                {
                    let inner_result_value = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| ActiveIterator::new(agent, gc.nogc()).r#return(agent, None, gc),
                        gc.reborrow(),
                    )
                    .unbind()
                    .bind(gc.nogc());
                    if let Ok(Some(value)) = inner_result_value {
                        // ### 7.4.13 AsyncIteratorClose
                        // 4.d. If innerResult is a normal completion, set
                        //    innerResult to Completion(Await(innerResult.[[Value]])).
                        // 5. If completion is a throw completion, return ? completion.

                        // We need to await the value and ignore any errors it
                        // might throw, then rethrow the error. First, we need
                        // to load the error to the stack for later throwing.
                        vm.stack.push(vm.result.take().unwrap());
                        // Then we can put our value as the result.
                        vm.result = Some(value.unbind());
                        // Before we await we need to make sure that any error
                        // throw by the await gets ignored. This also ignores
                        // the next instruction coming after this, which would
                        // pop the exception handler stack: the ignore is
                        // necessary because handling the error pops the
                        // exception handler stack as well. Without the ignore
                        // we'd pop twice.
                        vm.exception_handler_stack
                            .push(ExceptionHandler::IgnoreErrorAndNextInstruction);
                        // Now we're ready to await: if the await succeeds then
                        // we'll continue execution which will pop the above
                        // exception handler, store the error as the result and
                        // then rethrow it. If the await throws an error, it
                        // will trigger our exception handler which will skip
                        // the exception handler pop instruction but otherwise
                        // continue as above. As a result, our error is always
                        // rethrown.
                        return Ok(ContinuationKind::Await);
                    }
                }
                // If we did not find a return method or get a value to await
                // then we'll skip the PopExceptionJumpTarget and Store
                // instructions, and go straight to rethrow handling. Note, we
                // do not manually rethrow as there may be more steps between
                // this and the final Throw instruction.
                vm.ip += 2;
            }
            Instruction::Yield => return Ok(ContinuationKind::Yield),
            Instruction::CreateUnmappedArgumentsObject => {
                let Some(VmIteratorRecord::SliceIterator(slice)) = vm.iterator_stack.last() else {
                    unreachable!()
                };
                vm.result = Some(
                    create_unmapped_arguments_object(agent, slice, gc.nogc())
                        .into_value()
                        .unbind(),
                );
            }
            Instruction::IteratorPop => {
                let _ = vm.pop_iterator(gc.nogc());
            }
            Instruction::GetNewTarget => {
                // 1. Let envRec be GetThisEnvironment().
                let env_rec = get_this_environment(agent, gc.nogc());
                // 2. Assert: envRec has a [[NewTarget]] field.
                let Environment::Function(env_rec) = env_rec else {
                    unreachable!()
                };
                // 3. Return envRec.[[NewTarget]].
                vm.result = Some(
                    env_rec
                        .get_new_target(agent)
                        .map_or(Value::Undefined, |v| v.into_value())
                        .unbind(),
                );
            }
            Instruction::ImportCall => {
                let specifier = vm.stack.pop().unwrap().bind(gc.nogc());
                let options = vm.result.take().bind(gc.nogc());
                vm.result = {
                    let specifier = specifier.unbind();
                    let options = options.unbind();
                    Some(
                        with_vm_gc(
                            agent,
                            vm,
                            |agent, gc| evaluate_import_call(agent, specifier, options, gc),
                            gc,
                        )
                        .into_value()
                        .unbind(),
                    )
                };
            }
            Instruction::ImportMeta => {
                let gc = gc.into_nogc();
                // 1. Let module be GetActiveScriptOrModule().
                let module = agent.get_active_script_or_module(gc);
                // 2. Assert: module is a Source Text Module Record.
                let Some(ScriptOrModule::SourceTextModule(module)) = module else {
                    unreachable!()
                };
                // 3. Let importMeta be module.[[ImportMeta]].
                let import_meta = module.get_import_meta(agent);
                // 4. If importMeta is empty, then
                let import_meta = match import_meta {
                    None => {
                        // b. Let importMetaValues be HostGetImportMetaProperties(module).
                        let import_meta_values = agent
                            .host_hooks
                            .get_import_meta_properties(agent, module, gc);
                        // a. Set importMeta to OrdinaryObjectCreate(null).
                        // c. For each Record { [[Key]], [[Value]] } p of importMetaValues, do
                        // i. Perform ! CreateDataPropertyOrThrow(importMeta, p.[[Key]], p.[[Value]]).
                        let import_meta = OrdinaryObject::create_object(
                            agent,
                            None,
                            &import_meta_values
                                .into_iter()
                                .map(|(key, value)| ObjectEntry::new_data_entry(key, value))
                                .collect::<Box<[ObjectEntry]>>(),
                        );
                        // d. Perform HostFinalizeImportMeta(importMeta, module).
                        agent
                            .host_hooks
                            .finalize_import_meta(agent, import_meta, module, gc);
                        // e. Set module.[[ImportMeta]] to importMeta.
                        module.set_import_meta(agent, import_meta);
                        // f. Return importMeta.
                        import_meta
                    }
                    Some(import_meta) => {
                        // 5. Else,
                        // a. Assert: importMeta is an Object.
                        // b. Return importMeta.
                        import_meta
                    }
                };
                vm.result = Some(import_meta.into_value().unbind());
            }
            Instruction::VerifyIsObject => {
                let result = vm.result.unwrap();
                if !result.is_object() {
                    let message =
                        executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                    return Err(agent.throw_exception_with_message(
                        ExceptionType::TypeError,
                        message.unbind(),
                        gc.into_nogc(),
                    ));
                }
            }
        }

        Ok(ContinuationKind::Normal)
    }

    fn get_call_args<'gc>(&mut self, instr: &Instr, _gc: NoGcScope<'gc, '_>) -> Vec<Value<'gc>> {
        let instr_arg0 = instr.get_first_arg();
        let arg_count = if instr_arg0 != IndexType::MAX {
            instr_arg0 as usize
        } else {
            // We parse the result as a SmallInteger.
            let Value::Integer(integer) = self.result.take().unwrap() else {
                panic!("Expected the number of function arguments to be an integer")
            };
            usize::try_from(integer.into_i64()).unwrap()
        };

        assert!(self.stack.len() >= arg_count);
        self.stack.split_off(self.stack.len() - arg_count)
    }

    /// Pop the active (top-most) iterator from the iterator stack.
    ///
    /// ### Panics
    ///
    /// Panics if the iterator stack is empty.
    pub(super) fn pop_iterator<'gc>(&mut self, gc: NoGcScope<'gc, '_>) -> VmIteratorRecord<'gc> {
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
        self.iterator_stack.last().expect("Iterator stack is empty")
    }

    /// Get the active (top-most) iterator from the iterator stack as mutable.
    ///
    /// ### Panics
    ///
    /// Panics if the iterator stack is empty.
    pub(super) fn get_active_iterator_mut(&mut self) -> &mut VmIteratorRecord<'static> {
        self.iterator_stack
            .last_mut()
            .expect("Iterator stack is empty")
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
        result_string.push_wtf8(string.as_wtf8(agent));
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
    op_text: BinaryOperator,
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
        bigint_binary_operator(agent, op_text, lnum, rnum, gc).map(|v| v.into_value())
    } else if let (Ok(lnum), Ok(rnum)) = (Number::try_from(lnum), Number::try_from(rnum)) {
        number_binary_operator(agent, op_text, lnum, rnum, gc).map(|v| v.into_value())
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
            return Ok(String::concat(agent, [lstr, rstr], gc).into_value());
        }
        (Ok(lstr), Err(_)) => {
            let lstr = lstr.scope(agent, gc);
            // ii. Let rstr be ? ToString(rprim).
            let rstr = to_string_primitive(agent, rprim, gc)?;
            // iii. Return the string-concatenation of lstr and rstr.
            return Ok(String::concat(agent, [lstr.get(agent).bind(gc), rstr], gc).into_value());
        }
        (Err(_), Ok(rstr)) => {
            let rstr = rstr.scope(agent, gc);
            // i. Let lstr be ? ToString(lprim).
            let lstr = to_string_primitive(agent, lprim, gc)?;
            // iii. Return the string-concatenation of lstr and rstr.
            return Ok(String::concat(agent, [lstr, rstr.get(agent).bind(gc)], gc).into_value());
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
        bigint_binary_operator(agent, BinaryOperator::Addition, lnum, rnum, gc)
            .map(|v| v.into_value())
    } else if let (Ok(lnum), Ok(rnum)) = (Number::try_from(lnum), Number::try_from(rnum)) {
        number_binary_operator(agent, BinaryOperator::Addition, lnum, rnum, gc)
            .map(|v| v.into_value())
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
    op_text: BinaryOperator,
    lnum: BigInt<'a>,
    rnum: BigInt<'a>,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, BigInt<'a>> {
    match op_text {
        // a. If opText is **, return ? BigInt::exponentiate(lnum, rnum).
        BinaryOperator::Exponential => BigInt::exponentiate(agent, lnum, rnum, gc),
        // b. If opText is /, return ? BigInt::divide(lnum, rnum).
        BinaryOperator::Division => BigInt::divide(agent, lnum, rnum, gc),
        // c. If opText is %, return ? BigInt::remainder(lnum, rnum).
        BinaryOperator::Remainder => BigInt::remainder(agent, lnum, rnum, gc),
        // d. If opText is >>>, return ? BigInt::unsignedRightShift(lnum, rnum).
        BinaryOperator::ShiftRightZeroFill => BigInt::unsigned_right_shift(agent, lnum, rnum, gc),
        // <<	BigInt	BigInt::leftShift
        BinaryOperator::ShiftLeft => BigInt::left_shift(agent, lnum, rnum, gc),
        // >>	BigInt	BigInt::signedRightShift
        BinaryOperator::ShiftRight => BigInt::signed_right_shift(agent, lnum, rnum, gc),
        // +	BigInt	BigInt::add
        BinaryOperator::Addition => Ok(BigInt::add(agent, lnum, rnum)),
        // -	BigInt	BigInt::subtract
        BinaryOperator::Subtraction => Ok(BigInt::subtract(agent, lnum, rnum)),
        // *	BigInt	BigInt::multiply
        BinaryOperator::Multiplication => Ok(BigInt::multiply(agent, lnum, rnum)),
        // |	BigInt	BigInt::bitwiseOR
        BinaryOperator::BitwiseOR => Ok(BigInt::bitwise_or(agent, lnum, rnum)),
        // ^	BigInt	BigInt::bitwiseXOR
        BinaryOperator::BitwiseXOR => Ok(BigInt::bitwise_xor(agent, lnum, rnum)),
        // &	BigInt	BigInt::bitwiseAND
        BinaryOperator::BitwiseAnd => Ok(BigInt::bitwise_and(agent, lnum, rnum)),
        _ => unreachable!(),
    }
}

fn number_binary_operator<'a>(
    agent: &mut Agent,
    op_text: BinaryOperator,
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
        BinaryOperator::Exponential => Number::exponentiate(agent, lnum, rnum),
        // *	Number	Number::multiply
        BinaryOperator::Multiplication => Number::multiply(agent, lnum, rnum, gc),
        // /	Number	Number::divide
        BinaryOperator::Division => Number::divide(agent, lnum, rnum, gc),
        // %	Number	Number::remainder
        BinaryOperator::Remainder => Number::remainder(agent, lnum, rnum, gc),
        // +	Number	Number::add
        BinaryOperator::Addition => Number::add(agent, lnum, rnum),
        // -	Number	Number::subtract
        BinaryOperator::Subtraction => Number::subtract(agent, lnum, rnum),
        // <<	Number	Number::leftShift
        BinaryOperator::ShiftLeft => Number::left_shift(agent, lnum, rnum),
        // >>	Number	Number::signedRightShift
        BinaryOperator::ShiftRight => Number::signed_right_shift(agent, lnum, rnum),
        // >>>	Number	Number::unsignedRightShift
        BinaryOperator::ShiftRightZeroFill => Number::unsigned_right_shift(agent, lnum, rnum),
        // |	Number	Number::bitwiseOR
        BinaryOperator::BitwiseOR => Number::bitwise_or(agent, lnum, rnum).into(),
        // ^	Number	Number::bitwiseXOR
        BinaryOperator::BitwiseXOR => Number::bitwise_xor(agent, lnum, rnum).into(),
        // &	Number	Number::bitwiseAND
        BinaryOperator::BitwiseAnd => Number::bitwise_and(agent, lnum, rnum).into(),
        _ => unreachable!(),
    })
}

/// ### [13.5.3 The typeof operator](https://tc39.es/ecma262/#sec-typeof-operator)
#[inline]
fn typeof_operator(agent: &Agent, val: Value, gc: NoGcScope) -> String<'static> {
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
        Value::AsyncFromSyncIterator |
        Value::AsyncGenerator(_) |
        Value::ArrayIterator(_) |
        Value::MapIterator(_) |
        Value::StringIterator(_) |
        Value::Generator(_) |
        Value::Module(_) |
        Value::EmbedderObject(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "regexp")]
        Value::RegExp(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "weak-refs")]
        Value::WeakMap(_) |
        Value::WeakRef(_) |
        Value::WeakSet(_)  => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "set")]
        Value::Set(_) |
        Value::SetIterator(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "shared-array-buffer")]
        Value::SharedArrayBuffer(_) => BUILTIN_STRING_MEMORY.object,
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
        #[cfg(feature = "date")]
        Value::Date(_)  => BUILTIN_STRING_MEMORY.object,
        // 13. If val has a [[Call]] internal slot, return "function".
        Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_) |
        Value::BuiltinGeneratorFunction |
        Value::BuiltinConstructorFunction(_) |
        Value::BuiltinPromiseResolvingFunction(_) |
        Value::BuiltinPromiseCollectorFunction |
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
    value: impl IntoValue<'b>,
    target: impl IntoValue<'b>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    // 1. If target is not an Object, throw a TypeError exception.
    let Ok(target) = Object::try_from(target.into_value()) else {
        let error_message = format!(
            "Invalid instanceof target {}.",
            target
                .into_value()
                .string_repr(agent, gc.reborrow())
                .to_string_lossy(agent)
        );
        return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.into_nogc()));
    };
    // 2. Let instOfHandler be ? GetMethod(target, @@hasInstance).
    let inst_of_handler = get_method(
        agent,
        target.into_value(),
        WellKnownSymbolIndexes::HasInstance.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. If instOfHandler is not undefined, then
    if let Some(inst_of_handler) = inst_of_handler {
        // a. Return ToBoolean(? Call(instOfHandler, target, « V »)).
        let result = call_function(
            agent,
            inst_of_handler.unbind(),
            target.into_value(),
            Some(ArgumentsList::from_mut_slice(&mut [value
                .into_value()
                .unbind()])),
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
                target
                    .into_value()
                    .string_repr(agent, gc.reborrow())
                    .to_string_lossy(agent)
            );
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                error_message,
                gc.into_nogc(),
            ));
        };
        // 5. Return ? OrdinaryHasInstance(target, V).
        Ok(ordinary_has_instance(agent, target.unbind(), value, gc)?)
    }
}

fn with_vm_gc<'a, 'b, R: 'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    work: impl FnOnce(&mut Agent, GcScope<'a, 'b>) -> R,
    gc: GcScope<'a, 'b>,
) -> R {
    let vm = NonNull::from(vm);
    agent.vm_stack.push(vm);
    let result = work(agent, gc);
    let return_vm = agent.vm_stack.pop().unwrap();
    assert_eq!(vm, return_vm, "VM Stack was misused");
    result
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
            reference_stack,
            iterator_stack,
            exception_handler_stack: exception_jump_target_stack,
            result,
            reference,
        } = self;
        stack.as_slice().mark_values(queues);
        reference_stack.as_slice().mark_values(queues);
        iterator_stack.as_slice().mark_values(queues);
        exception_jump_target_stack.as_slice().mark_values(queues);
        result.mark_values(queues);
        reference.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Vm {
            ip: _,
            stack,
            reference_stack,
            iterator_stack,
            exception_handler_stack: exception_jump_target_stack,
            result,
            reference,
        } = self;
        stack.as_mut_slice().sweep_values(compactions);
        reference_stack.as_mut_slice().sweep_values(compactions);
        iterator_stack.as_mut_slice().sweep_values(compactions);
        exception_jump_target_stack
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
            exception_jump_target_stack,
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
            exception_jump_target_stack,
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
    vm: &mut Vm,
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
            with_vm_gc(
                agent,
                vm,
                |agent, gc| to_property_key(agent, name, gc),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };

        let name = prop_key.convert_to_value(agent, gc.nogc());
        set_class_name(agent, vm, name.into_value().unbind(), gc)
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
        typeof_operator(agent, value, gc).to_string_lossy(agent)
    );
    agent.throw_exception(ExceptionType::TypeError, error_message, gc)
}

/// ### [13.5.1.2 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-delete-operator-runtime-semantics-evaluation)
fn delete_evaluation<'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let reference = vm.reference.take().unwrap().bind(gc.nogc());
    // 3. If IsUnresolvableReference(ref) is true, then
    if is_unresolvable_reference(&reference) {
        // a. Assert: ref.[[Strict]] is false.
        debug_assert!(!reference.strict());
        // b. Return true.
        vm.result = Some(true.into());
    } else if is_property_reference(&reference) {
        // 4. If IsPropertyReference(ref) is true, then

        // a. Assert: IsPrivateReference(ref) is false.
        debug_assert!(!is_private_reference(&reference));
        // b. If IsSuperReference(ref) is true, throw a ReferenceError exception.
        if is_super_reference(&reference) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::ReferenceError,
                "Invalid delete involving 'super'.",
                gc.into_nogc(),
            ));
        }
        // c. Let baseObj be ? ToObject(ref.[[Base]]).
        let base = reference.base_value();
        let mut base_obj = to_object(agent, base, gc.nogc()).unbind()?.bind(gc.nogc());
        let strict = reference.strict();
        // d. If ref.[[ReferencedName]] is not a property key, then
        let referenced_name = if !reference.is_static_property_reference() {
            // i. Set ref.[[ReferencedName]] to ? ToPropertyKey(ref.[[ReferencedName]]).
            let referenced_name = reference.referenced_name_value();
            if let TryResult::Continue(referenced_name) =
                to_property_key_simple(agent, referenced_name, gc.nogc())
            {
                referenced_name
            } else {
                let referenced_name = referenced_name.unbind();
                let scoped_base_obj = base_obj.scope(agent, gc.nogc());
                let referenced_name = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key_complex(agent, referenced_name, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared.
                base_obj = unsafe { scoped_base_obj.take(agent) }.bind(gc.nogc());
                referenced_name
            }
        } else {
            reference.referenced_name_property_key()
        };
        // e. Let deleteStatus be ? baseObj.[[Delete]](ref.[[ReferencedName]]).
        let delete_status = if let TryResult::Continue(delete_status) =
            base_obj.try_delete(agent, referenced_name, gc.nogc())
        {
            delete_status
        } else {
            let base_obj = base_obj.unbind();
            let referenced_name = referenced_name.unbind();
            with_vm_gc(
                agent,
                vm,
                |agent, gc| base_obj.internal_delete(agent, referenced_name, gc),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };
        // f. If deleteStatus is false and ref.[[Strict]] is true, throw a TypeError exception.
        if !delete_status && strict {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot delete property",
                gc.into_nogc(),
            ));
        }
        // g. Return deleteStatus.
        vm.result = Some(delete_status.into());
    } else {
        // 5. Else,

        // a. Let base be ref.[[Base]].
        // b. Assert: base is an Environment Record.
        let base = reference.base_env();
        let referenced_name = reference.referenced_name_string();
        // c. Return ? base.DeleteBinding(ref.[[ReferencedName]]).
        let result = if let TryResult::Continue(result) =
            base.try_delete_binding(agent, referenced_name, gc.nogc())
        {
            result.unbind()?
        } else {
            let referenced_name = referenced_name.unbind();
            let base = base.unbind();
            with_vm_gc(
                agent,
                vm,
                |agent, gc| base.delete_binding(agent, referenced_name, gc),
                gc,
            )?
        };
        vm.result = Some(result.into());
    }
    Ok(())

    // Note 1

    // When a delete operator occurs within strict mode code, a
    // SyntaxError exception is thrown if its UnaryExpression is a
    // direct reference to a variable, function argument, or
    // function name. In addition, if a delete operator occurs
    // within strict mode code and the property to be deleted has
    // the attribute { [[Configurable]]: false } (or otherwise
    // cannot be deleted), a TypeError exception is thrown.

    // Note 2

    // The object that may be created in step 4.c is not accessible
    // outside of the above abstract operation and the ordinary
    // object [[Delete]] internal method. An implementation might
    // choose to avoid the actual creation of that object.
}
