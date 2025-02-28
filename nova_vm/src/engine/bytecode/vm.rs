// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod binding_methods;

use std::{ptr::NonNull, sync::OnceLock};

use ahash::AHashSet;
use binding_methods::{execute_simple_array_binding, execute_simple_object_binding};
use oxc_ast::ast;
use oxc_span::Span;
use oxc_syntax::operator::BinaryOperator;

#[cfg(feature = "interleaved-gc")]
use crate::{ecmascript::execution::RealmIdentifier, heap::heap_gc::heap_gc};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::iterator_close,
            operations_on_objects::{
                call, call_function, construct, copy_data_properties,
                copy_data_properties_into_object, create_data_property_or_throw,
                define_property_or_throw, get_method, has_property, ordinary_has_instance, set,
                try_copy_data_properties_into_object, try_create_data_property,
                try_create_data_property_or_throw, try_define_property_or_throw, try_has_property,
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
            array_create, create_builtin_constructor, create_unmapped_arguments_object,
            global_object::perform_eval, make_constructor, make_method,
            ordinary::ordinary_object_create_with_intrinsics, ordinary_function_create,
            set_function_name, ArgumentsList, Array, BuiltinConstructorArgs, ConstructorStatus,
            OrdinaryFunctionCreateParams,
        },
        execution::{
            agent::{resolve_binding, ExceptionType, JsError},
            get_this_environment, new_class_static_element_environment,
            new_declarative_environment, Agent, ECMAScriptCodeEvaluationState, EnvironmentIndex,
            JsResult, ProtoIntrinsics,
        },
        types::{
            get_this_value, get_value, initialize_referenced_binding, is_private_reference,
            is_super_reference, put_value, try_initialize_referenced_binding, Base, BigInt,
            Function, InternalMethods, IntoFunction, IntoObject, IntoValue, Number, Numeric,
            Object, OrdinaryObject, Primitive, PropertyDescriptor, PropertyKey, Reference, String,
            Value, BUILTIN_STRING_MEMORY,
        },
    },
    engine::{
        bytecode::{
            executable::{ArrowFunctionExpression, SendableRef},
            instructions::Instr,
            iterator::{ObjectPropertiesIterator, VmIterator},
            Executable, FunctionExpression, IndexType, Instruction, InstructionIter,
            NamedEvaluationParameter,
        },
        context::{Bindable, GcScope, NoGcScope},
        unwrap_try, TryResult,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

use super::executable::get_instruction;

struct EmptyParametersList(ast::FormalParameters<'static>);
unsafe impl Send for EmptyParametersList {}
unsafe impl Sync for EmptyParametersList {}

#[derive(Debug)]
pub(crate) enum ExecutionResult<'a> {
    Return(Value<'a>),
    Throw(JsError),
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
    pub(crate) fn into_js_result(self) -> JsResult<Value<'a>> {
        match self {
            ExecutionResult::Return(value) => Ok(value),
            ExecutionResult::Throw(err) => Err(err),
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
            Self::Throw(js_error) => ExecutionResult::Throw(js_error),
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
            Self::Throw(js_error) => ExecutionResult::Throw(js_error),
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

/// Indicates a place to jump after an exception is thrown.
#[derive(Debug)]
struct ExceptionJumpTarget {
    /// Instruction pointer.
    ip: usize,
    /// The lexical environment which contains this exception jump target.
    lexical_environment: EnvironmentIndex,
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
    iterator_stack: Vec<VmIterator>,
    exception_jump_target_stack: Vec<ExceptionJumpTarget>,
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
    iterator_stack: Box<[VmIterator]>,
    /// Note: Exception jump stack is non-empty only if the code awaits inside
    /// a try block. This means that often no heap data clone is required.
    exception_jump_target_stack: Box<[ExceptionJumpTarget]>,
}

impl SuspendedVm {
    pub(crate) fn resume<'gc>(
        self,
        agent: &mut Agent,
        executable: Executable,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let vm = Vm::from_suspended(self);
        vm.resume(agent, executable, value, gc)
    }

    pub(crate) fn resume_throw<'gc>(
        self,
        agent: &mut Agent,
        executable: Executable,
        err: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        // Optimisation: Avoid unsuspending the Vm if we're just going to throw
        // out of it immediately.
        if self.exception_jump_target_stack.is_empty() {
            let err = JsError::new(err.unbind());
            return ExecutionResult::Throw(err);
        }
        let vm = Vm::from_suspended(self);
        vm.resume_throw(agent, executable, err, gc)
    }
}

impl<'a> Vm {
    fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            iterator_stack: Vec::new(),
            exception_jump_target_stack: Vec::new(),
            result: None,
            reference: None,
        }
    }

    fn suspend(self) -> SuspendedVm {
        SuspendedVm {
            ip: self.ip,
            stack: self.stack.into_boxed_slice(),
            reference_stack: unsafe {
                core::mem::transmute::<Box<[Reference<'a>]>, Box<[Reference<'static>]>>(
                    self.reference_stack.into_boxed_slice(),
                )
            },
            iterator_stack: self.iterator_stack.into_boxed_slice(),
            exception_jump_target_stack: self.exception_jump_target_stack.into_boxed_slice(),
        }
    }

    fn from_suspended(suspended: SuspendedVm) -> Self {
        Self {
            ip: suspended.ip,
            stack: suspended.stack.into_vec(),
            reference_stack: suspended.reference_stack.into_vec(),
            iterator_stack: suspended.iterator_stack.into_vec(),
            exception_jump_target_stack: suspended.exception_jump_target_stack.into_vec(),
            result: None,
            reference: None,
        }
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute<'gc>(
        agent: &mut Agent,
        executable: Executable,
        arguments: Option<&[Value<'static>]>,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let mut vm = Vm::new();

        if let Some(arguments) = arguments {
            // SAFETY: awaits and yields are invalid syntax inside an arguments
            // list, so this reference shouldn't remain alive after this
            // function returns.
            let arguments = unsafe { SendableRef::new_as_static(arguments) };
            vm.iterator_stack.push(VmIterator::SliceIterator(arguments));
        }

        if agent.options.print_internals {
            eprintln!();
            eprintln!("=== Executing Executable ===");
            eprintln!(
                "Constants: {:?}",
                executable.get_constants(agent, gc.nogc())
            );
            eprintln!();

            eprintln!("Instructions:");
            let iter = InstructionIter::new(executable.get_instructions(agent));
            for (ip, instr) in iter {
                instr.debug_print(agent, ip, executable, gc.nogc());
            }
            eprintln!();
        }

        vm.inner_execute(agent, executable, gc)
    }

    pub fn resume<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Executable,
        value: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        self.result = Some(value.unbind());
        self.inner_execute(agent, executable, gc)
    }

    pub fn resume_throw<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Executable,
        err: Value,
        gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        let err = err.bind(gc.nogc());
        let err = JsError::new(err.unbind());
        if !self.handle_error(agent, err) {
            return ExecutionResult::Throw(err);
        }
        self.inner_execute(agent, executable, gc)
    }

    fn inner_execute<'gc>(
        mut self,
        agent: &mut Agent,
        executable: Executable,
        mut gc: GcScope<'gc, '_>,
    ) -> ExecutionResult<'gc> {
        #[cfg(feature = "interleaved-gc")]
        let do_gc = !agent.options.disable_gc;
        #[cfg(feature = "interleaved-gc")]
        let mut instr_count = 0u8;

        let instructions = executable.get_instructions(agent);
        while let Some(instr) = get_instruction(instructions, &mut self.ip) {
            #[cfg(feature = "interleaved-gc")]
            if do_gc {
                instr_count = instr_count.wrapping_add(1);
                if instr_count == 0 {
                    let mut root_realms = agent
                        .heap
                        .realms
                        .iter()
                        .enumerate()
                        .map(|(i, _)| Some(RealmIdentifier::from_index(i)))
                        .collect::<Vec<_>>();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| heap_gc(agent, &mut root_realms, gc),
                        gc.reborrow(),
                    );
                }
            }
            let temp = &mut self;
            let temp_self = unsafe { core::mem::transmute::<&mut Vm, &mut Vm>(temp) };
            match Self::execute_instruction(agent, temp_self, executable, &instr, gc.reborrow()) {
                Ok(ContinuationKind::Normal) => {}
                Ok(ContinuationKind::Return) => {
                    let result = self.result.unwrap_or(Value::Undefined);
                    return ExecutionResult::Return(result);
                }
                Ok(ContinuationKind::Yield) => {
                    let yielded_value = self.result.take().unwrap();
                    return ExecutionResult::Yield {
                        vm: self.suspend(),
                        yielded_value,
                    };
                }
                Ok(ContinuationKind::Await) => {
                    let awaited_value = self.result.take().unwrap();
                    return ExecutionResult::Await {
                        vm: self.suspend(),
                        awaited_value,
                    };
                }
                Err(err) => {
                    if !self.handle_error(agent, err) {
                        return ExecutionResult::Throw(err);
                    }
                }
            }
        }

        ExecutionResult::Return(Value::Undefined)
    }

    #[must_use]
    fn handle_error(&mut self, agent: &mut Agent, err: JsError) -> bool {
        if let Some(ejt) = self.exception_jump_target_stack.pop() {
            self.ip = ejt.ip;
            agent
                .running_execution_context_mut()
                .ecmascript_code
                .as_mut()
                .unwrap()
                .lexical_environment = ejt.lexical_environment;
            self.result = Some(err.value());
            true
        } else {
            false
        }
    }

    fn execute_instruction(
        agent: &mut Agent,
        vm: &mut Vm,
        executable: Executable,
        instr: &Instr,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<ContinuationKind> {
        if agent.options.print_internals {
            eprintln!("Executing instruction {:?}", instr.kind);
        }
        match instr.kind {
            Instruction::ArrayCreate => {
                let result =
                    array_create(agent, 0, instr.args[0].unwrap() as usize, None, gc.nogc())?
                        .into_value();
                vm.stack.push(result.unbind());
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
                    eprintln!("Debug: {:#?}", vm);
                }
            }
            Instruction::ResolveBinding => {
                let identifier =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());

                let reference = resolve_binding(agent, identifier.unbind(), None, gc)?;

                vm.reference = Some(reference.unbind());
            }
            Instruction::ResolveThisBinding => {
                // 1. Let envRec be GetThisEnvironment().
                let env_rec = get_this_environment(agent);
                // 2. Return ? envRec.GetThisBinding().
                let result = match env_rec {
                    EnvironmentIndex::Declarative(_) => unreachable!(),
                    EnvironmentIndex::Function(idx) => idx.get_this_binding(agent, gc.nogc())?,
                    EnvironmentIndex::Global(idx) => {
                        idx.get_this_binding(agent, gc.nogc()).into_value()
                    }
                    EnvironmentIndex::Object(_) => unreachable!(),
                };
                vm.result = Some(result.unbind());
            }
            Instruction::LoadConstant => {
                let constant =
                    executable.fetch_constant(agent, instr.args[0].unwrap() as usize, gc.nogc());
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
                let constant =
                    executable.fetch_constant(agent, instr.args[0].unwrap() as usize, gc.nogc());
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
                    to_object(agent, vm.result.unwrap(), gc.nogc())?
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
                        apply_string_or_numeric_binary_operator(agent, lval, op_text, rval, gc)
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
                )?;
                let key = key.unbind().bind(gc.nogc());
                let value = vm.result.take().unwrap().bind(gc.nogc());
                let object = vm.stack.last().unwrap().bind(gc.nogc());
                let object = Object::try_from(object).unwrap();

                unwrap_try(try_create_data_property_or_throw(
                    agent,
                    object,
                    key.unbind(),
                    value,
                    gc.nogc(),
                ))
                .unwrap();
            }
            Instruction::ObjectDefineMethod => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let enumerable = instr.args[1].unwrap() != 0;
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = vm.stack.pop().unwrap();
                let prop_key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, prop_key, gc),
                    gc.reborrow(),
                )?
                .unbind()
                .bind(gc.nogc());
                let object = Object::try_from(*vm.stack.last().unwrap())
                    .unwrap()
                    .bind(gc.nogc());

                // 2. Let env be the running execution context's LexicalEnvironment.
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let ECMAScriptCodeEvaluationState {
                    lexical_environment: env,
                    private_environment: private_env,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();
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
                    executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let enumerable = instr.args[1].unwrap() != 0;
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = vm.stack.pop().unwrap();
                let prop_key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, prop_key, gc),
                    gc.reborrow(),
                )?
                .unbind()
                .bind(gc.nogc());
                // 2. Let env be the running execution context's LexicalEnvironment.
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let ECMAScriptCodeEvaluationState {
                    lexical_environment: env,
                    private_environment: private_env,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();
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
                    Some(BUILTIN_STRING_MEMORY.get),
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
                    executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let enumerable = instr.args[1].unwrap() != 0;
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = vm.stack.pop().unwrap();
                let prop_key = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| to_property_key(agent, prop_key, gc),
                    gc.reborrow(),
                )?
                .unbind()
                .bind(gc.nogc());
                // 2. Let env be the running execution context's LexicalEnvironment.
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                let ECMAScriptCodeEvaluationState {
                    lexical_environment: env,
                    private_environment: private_env,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();
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
                    Some(BUILTIN_STRING_MEMORY.set),
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
                    gc.reborrow(),
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

                let result =
                    with_vm_gc(agent, vm, |agent, gc| get_value(agent, &reference, gc), gc)?;

                vm.result = Some(result.unbind());
            }
            Instruction::GetValueKeepReference => {
                // 1. If V is not a Reference Record, return V.
                let reference = vm.reference.as_ref().unwrap().clone();

                let result =
                    with_vm_gc(agent, vm, |agent, gc| get_value(agent, &reference, gc), gc)?;

                vm.result = Some(result.unbind());
            }
            Instruction::Typeof => {
                // 2. If val is a Reference Record, then
                let val = if let Some(reference) = vm.reference.take() {
                    if reference.base == Base::Unresolvable {
                        // a. If IsUnresolvableReference(val) is true, return "undefined".
                        Value::Undefined
                    } else {
                        // 3. Set val to ? GetValue(val).
                        with_vm_gc(agent, vm, |agent, gc| get_value(agent, &reference, gc), gc)?
                    }
                } else {
                    vm.result.unwrap().bind(gc.nogc())
                };
                vm.result = Some(typeof_operator(agent, val).into_value())
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

                let num_excluded_items = usize::from(instr.args[0].unwrap());
                let mut excluded_items = AHashSet::with_capacity(num_excluded_items);
                assert!(vm.reference.is_none());
                for _ in 0..num_excluded_items {
                    let reference = vm.reference_stack.pop().unwrap();
                    assert_eq!(reference.base, Base::Value(from.into_value()));
                    assert!(reference.this_value.is_none());
                    excluded_items.insert(reference.referenced_name);
                }

                if let TryResult::Continue(result) =
                    try_copy_data_properties_into_object(agent, from, &excluded_items, gc.nogc())
                {
                    vm.result = Some(result.into_value().unbind());
                } else {
                    let from = from.unbind();
                    let result = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            copy_data_properties_into_object(agent, from, &excluded_items, gc)
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
                } = executable
                    .fetch_arrow_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let identifier = *identifier;
                // 2. Let env be the LexicalEnvironment of the running execution context.
                // 3. Let privateEnv be the running execution context's PrivateEnvironment.
                // 4. Let sourceText be the source text matched by ArrowFunction.
                // 5. Let closure be OrdinaryFunctionCreate(%Function.prototype%, sourceText, ArrowParameters, ConciseBody, LEXICAL-THIS, env, privateEnv).
                // 6. Perform SetFunctionName(closure, name).
                // 7. Return closure.
                let ECMAScriptCodeEvaluationState {
                    lexical_environment,
                    private_environment,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();
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
                    env: lexical_environment,
                    private_env: private_environment,
                };
                let mut function = ordinary_function_create(agent, params, gc.nogc());
                let name = if let Some(parameter) = &identifier {
                    let pk_result = match parameter {
                        NamedEvaluationParameter::Result => {
                            let value = vm.result.unwrap().bind(gc.nogc());
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
                        NamedEvaluationParameter::Reference => Ok(vm
                            .reference
                            .as_ref()
                            .unwrap()
                            .referenced_name
                            .bind(gc.nogc())),
                        NamedEvaluationParameter::ReferenceStack => Ok(vm
                            .reference_stack
                            .last()
                            .unwrap()
                            .referenced_name
                            .bind(gc.nogc())),
                    };
                    let pk = match pk_result {
                        Ok(pk) => pk.bind(gc.nogc()),
                        Err(pk_value) => {
                            let scoped_function = function.scope(agent, gc.nogc());
                            let pk_value = pk_value.unbind();
                            let pk = with_vm_gc(
                                agent,
                                vm,
                                |agent, gc| to_property_key_complex(agent, pk_value, gc),
                                gc.reborrow(),
                            )?
                            .unbind()
                            .bind(gc.nogc());
                            function = scoped_function.get(agent).bind(gc.nogc());
                            pk
                        }
                    };
                    pk
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
                    compiled_bytecode,
                } = executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let identifier = *identifier;
                let compiled_bytecode = *compiled_bytecode;
                let ECMAScriptCodeEvaluationState {
                    lexical_environment,
                    private_environment,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();

                let (name, env, init_binding) = if let Some(parameter) = identifier {
                    debug_assert!(function_expression.id.is_none());
                    let pk = match parameter {
                        NamedEvaluationParameter::Result => Ok(vm.result.unwrap()),
                        NamedEvaluationParameter::Stack => Ok(*vm.stack.last().unwrap()),
                        NamedEvaluationParameter::Reference => {
                            Err(vm.reference.as_ref().unwrap().referenced_name)
                        }
                        NamedEvaluationParameter::ReferenceStack => {
                            Err(vm.reference_stack.last().unwrap().referenced_name)
                        }
                    };
                    let name = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| match pk {
                            Ok(value) => to_property_key(agent, value, gc),
                            Err(pk) => Ok(pk.bind(gc.into_nogc())),
                        },
                        gc.reborrow(),
                    )?
                    .unbind()
                    .bind(gc.nogc());
                    (name, lexical_environment, false)
                } else if let Some(binding_identifier) = &function_expression.id {
                    let name = String::from_str(agent, &binding_identifier.name, gc.nogc());
                    let func_env = new_declarative_environment(agent, Some(lexical_environment));
                    func_env.create_immutable_binding(agent, name, false);
                    (name.into(), EnvironmentIndex::Declarative(func_env), true)
                } else {
                    (String::EMPTY_STRING.into(), lexical_environment, false)
                };
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
                    private_env: private_environment,
                };
                let function = ordinary_function_create(agent, params, gc.nogc());
                if let Some(compiled_bytecode) = compiled_bytecode {
                    agent[function].compiled_bytecode = Some(compiled_bytecode);
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
                                .current_realm()
                                .intrinsics()
                                .async_generator_prototype()
                                .into_object()
                        } else {
                            agent
                                .current_realm()
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
                let FunctionExpression {
                    expression,
                    compiled_bytecode,
                    ..
                } = executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let compiled_bytecode = *compiled_bytecode;
                let has_constructor_parent = instr.args[1].unwrap();
                assert!(has_constructor_parent <= 1);
                let has_constructor_parent = has_constructor_parent == 1;

                let class_name = String::try_from(vm.stack.pop().unwrap()).unwrap();
                let function_prototype = if has_constructor_parent {
                    Some(Object::try_from(vm.stack.pop().unwrap()).unwrap())
                } else {
                    None
                };
                let proto = OrdinaryObject::try_from(*vm.stack.last().unwrap()).unwrap();

                let is_null_derived_class = !has_constructor_parent
                    && unwrap_try(proto.try_get_prototype_of(agent, gc.nogc())).is_none();

                let ECMAScriptCodeEvaluationState {
                    lexical_environment,
                    private_environment,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();

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
                    env: lexical_environment,
                    private_env: private_environment,
                };
                let function = ordinary_function_create(agent, params, gc.nogc());
                if let Some(compiled_bytecode) = compiled_bytecode {
                    agent[function].compiled_bytecode = Some(compiled_bytecode);
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
                let class_initializer_bytecode_index = instr.args[0].unwrap();
                let (compiled_initializer_bytecode, has_constructor_parent) = executable
                    .fetch_class_initializer_bytecode(
                        agent,
                        class_initializer_bytecode_index as usize,
                    );

                let class_name = String::try_from(vm.stack.pop().unwrap()).unwrap();
                let function_prototype = if has_constructor_parent {
                    Some(Object::try_from(vm.stack.pop().unwrap()).unwrap())
                } else {
                    Some(
                        agent
                            .current_realm()
                            .intrinsics()
                            .function_prototype()
                            .into_object(),
                    )
                };
                let proto = Object::try_from(*vm.stack.last().unwrap()).unwrap();

                let ECMAScriptCodeEvaluationState {
                    lexical_environment,
                    private_environment,
                    source_code,
                    ..
                } = *agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap();

                let function = create_builtin_constructor(
                    agent,
                    BuiltinConstructorArgs {
                        class_name,
                        is_derived: has_constructor_parent,
                        prototype: function_prototype,
                        prototype_property: proto,
                        compiled_initializer_bytecode,
                        env: lexical_environment,
                        private_env: private_environment,
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
            Instruction::Swap => {
                let a = vm.stack.pop().unwrap();
                let b = vm.stack.pop().unwrap();
                vm.stack.push(a);
                vm.stack.push(b);
            }
            Instruction::DirectEvalCall => {
                let func = with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let func_ref = resolve_binding(
                            agent,
                            BUILTIN_STRING_MEMORY.eval,
                            None,
                            gc.reborrow(),
                        )?
                        .unbind();
                        get_value(agent, &func_ref, gc)
                    },
                    gc.reborrow(),
                )?
                .unbind()
                .bind(gc.nogc());
                let args = vm.get_call_args(instr, gc.nogc());

                // a. If SameValue(func, %eval%) is true, then
                let result = if func == agent.current_realm().intrinsics().eval().into_value() {
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
                    let args = args.unbind();
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            call(
                                agent,
                                func,
                                Value::Undefined,
                                Some(ArgumentsList(&args)),
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
                    match reference.base {
                        // i. Let thisValue be GetThisValue(ref).
                        Base::Value(_) => get_this_value(&reference).bind(gc.nogc()),
                        // b. Else,
                        Base::Environment(ref_env) => {
                            // i. Let refEnv be ref.[[Base]].
                            // iii. Let thisValue be refEnv.WithBaseObject().
                            ref_env
                                .with_base_object(agent)
                                .map_or(Value::Undefined, |object| object.into_value())
                                .bind(gc.nogc())
                        }
                        // ii. Assert: refEnv is an Environment Record.
                        Base::Unresolvable => unreachable!(),
                    }
                } else {
                    // 2. Else,
                    // a. Let thisValue be undefined.
                    Value::Undefined
                };
                let args = vm.get_call_args(instr, gc.nogc()).unbind();
                let func = vm.stack.pop().unwrap().unbind();
                let this_value = this_value.unbind();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| call(agent, func, this_value, Some(ArgumentsList(&args)), gc),
                    gc.reborrow(),
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
                        constructor_string.as_str(agent)
                    );
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        error_message,
                        gc.nogc(),
                    ));
                };

                let constructor = constructor.unbind();
                let args = args.unbind();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| construct(agent, constructor, Some(ArgumentsList(&args)), None, gc),
                    gc,
                )?;
                vm.result = Some(result.unbind().into_value());
            }
            Instruction::EvaluateSuper => {
                let EnvironmentIndex::Function(this_env) = get_this_environment(agent) else {
                    unreachable!();
                };
                // 1. Let newTarget be GetNewTarget().
                // 2. Assert: newTarget is an Object.
                // 3. Let func be GetSuperConstructor().
                let (new_target, func) = {
                    let data = &agent[this_env];
                    (
                        Function::try_from(data.new_target.unwrap())
                            .unwrap()
                            .bind(gc.nogc()),
                        unwrap_try(data.function_object.try_get_prototype_of(agent, gc.nogc())),
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
                                    .as_str(agent)
                            )
                        },
                        gc.reborrow(),
                    );
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        error_message,
                        gc.nogc(),
                    ));
                };
                // 6. Let result be ? Construct(func, argList, newTarget).
                let result = {
                    let func = func.unbind();
                    let arg_list = arg_list.unbind();
                    let new_target = new_target.unbind();
                    let result = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            construct(
                                agent,
                                func,
                                Some(ArgumentsList(&arg_list)),
                                Some(new_target),
                                gc,
                            )
                        },
                        gc.reborrow(),
                    )?;
                    result.unbind().bind(gc.nogc())
                };
                // 7. Let thisER be GetThisEnvironment().
                let EnvironmentIndex::Function(this_er) = get_this_environment(agent) else {
                    unreachable!();
                };
                // 8. Perform ? thisER.BindThisValue(result).
                this_er.bind_this_value(agent, result.into_value(), gc.nogc())?;
                // 9. Let F be thisER.[[FunctionObject]].
                // 10. Assert: F is an ECMAScript function object.
                let Function::ECMAScriptFunction(_f) = agent[this_er].function_object else {
                    unreachable!();
                };
                // 11. Perform ? InitializeInstanceElements(result, F).
                // 12. Return result.
                vm.result = Some(result.into_value().unbind());
            }
            Instruction::EvaluatePropertyAccessWithExpressionKey => {
                let property_name_value = vm.result.take().unwrap().bind(gc.nogc());

                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;

                let property_key =
                    if property_name_value.is_string() || property_name_value.is_integer() {
                        unwrap_try(to_property_key_simple(
                            agent,
                            property_name_value,
                            gc.nogc(),
                        ))
                    } else {
                        let property_name_value = property_name_value.unbind();
                        with_vm_gc(
                            agent,
                            vm,
                            |agent, gc| to_property_key(agent, property_name_value, gc),
                            gc.reborrow(),
                        )?
                        .unbind()
                        .bind(gc.nogc())
                    };
                let base_value = vm.stack.pop().unwrap().bind(gc.nogc());

                vm.reference = Some(Reference {
                    base: Base::Value(base_value.unbind()),
                    referenced_name: property_key.unbind(),
                    strict,
                    this_value: None,
                });
            }
            Instruction::EvaluatePropertyAccessWithIdentifierKey => {
                let property_name_string =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
                let base_value = vm.result.take().unwrap().bind(gc.nogc());
                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;

                vm.reference = Some(Reference {
                    base: Base::Value(base_value.unbind()),
                    referenced_name: property_name_string.unbind().into(),
                    strict,
                    this_value: None,
                });
            }
            Instruction::Jump => {
                let ip = instr.args[0].unwrap() as usize;
                vm.ip = ip;
            }
            Instruction::JumpIfNot => {
                let result = vm.result.take().unwrap();
                let ip = instr.args[0].unwrap() as usize;
                if !to_boolean(agent, result) {
                    vm.ip = ip;
                }
            }
            Instruction::JumpIfTrue => {
                let result = vm.result.take().unwrap();
                let Value::Boolean(result) = result else {
                    unreachable!()
                };
                if result {
                    let ip = instr.args[0].unwrap() as usize;
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
                    let rval = rval.unbind();
                    let error_message = with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| {
                            format!(
                            "The right-hand side of an `in` expression must be an object, got '{}'.",
                            rval.string_repr(agent, gc).as_str(agent)
                        )
                        },
                        gc.reborrow(),
                    );
                    return Err(agent.throw_exception(
                        ExceptionType::TypeError,
                        error_message,
                        gc.nogc(),
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
                    )?
                    .unbind()
                    .bind(gc.nogc());
                    rval = scoped_rval.get(agent).bind(gc.nogc());
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
                unwrap_try(try_initialize_referenced_binding(agent, v, w, gc.nogc()))?;
            }
            Instruction::InitializeVariableEnvironment => {
                let num_variables = instr.args[0].unwrap();
                assert!(instr.args[1].unwrap() <= 1);
                let strict = instr.args[1].unwrap() == 1;

                // 10.2.11 FunctionDeclarationInstantiation
                // 28.b. Let varEnv be NewDeclarativeEnvironment(env).
                let env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let var_env = new_declarative_environment(agent, Some(env));
                // c. Set the VariableEnvironment of calleeContext to varEnv.
                agent
                    .running_execution_context_mut()
                    .ecmascript_code
                    .as_mut()
                    .unwrap()
                    .variable_environment = EnvironmentIndex::Declarative(var_env);

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
                    new_declarative_environment(agent, Some(EnvironmentIndex::Declarative(var_env)))
                } else {
                    // 31. Else,
                    // a. Let lexEnv be varEnv.
                    var_env
                };

                // 32. Set the LexicalEnvironment of calleeContext to lexEnv.
                agent
                    .running_execution_context_mut()
                    .ecmascript_code
                    .as_mut()
                    .unwrap()
                    .lexical_environment = EnvironmentIndex::Declarative(lex_env);
            }
            Instruction::EnterDeclarativeEnvironment => {
                let outer_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let new_env = new_declarative_environment(agent, Some(outer_env));
                agent
                    .running_execution_context_mut()
                    .ecmascript_code
                    .as_mut()
                    .unwrap()
                    .lexical_environment = EnvironmentIndex::Declarative(new_env);
            }
            Instruction::EnterClassStaticElementEnvironment => {
                let class_constructor = Function::try_from(*vm.stack.last().unwrap()).unwrap();
                let local_env = new_class_static_element_environment(agent, class_constructor);
                let local_env = EnvironmentIndex::Function(local_env);

                let current_context = agent
                    .running_execution_context_mut()
                    .ecmascript_code
                    .as_mut()
                    .unwrap();
                current_context.lexical_environment = local_env;
                current_context.variable_environment = local_env;
            }
            Instruction::ExitDeclarativeEnvironment => {
                let old_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment
                    .get_outer_env(agent)
                    .unwrap();
                agent
                    .running_execution_context_mut()
                    .ecmascript_code
                    .as_mut()
                    .unwrap()
                    .lexical_environment = old_env;
            }
            Instruction::ExitVariableEnvironment => {
                let old_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .variable_environment
                    .get_outer_env(agent)
                    .unwrap();
                agent
                    .running_execution_context_mut()
                    .ecmascript_code
                    .as_mut()
                    .unwrap()
                    .variable_environment = old_env;
            }
            Instruction::CreateMutableBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());

                unwrap_try(lex_env.try_create_mutable_binding(
                    agent,
                    name.unbind(),
                    false,
                    gc.nogc(),
                ))
                .unwrap();
            }
            Instruction::CreateImmutableBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
                lex_env
                    .create_immutable_binding(agent, name, true, gc.nogc())
                    .unwrap();
            }
            Instruction::Throw => {
                let result = vm.result.take().unwrap();
                return Err(JsError::new(result));
            }
            Instruction::ThrowError => {
                let exception_type_immediate = instr.args[0].unwrap();
                let message = String::try_from(vm.result.take().unwrap()).unwrap();

                let exception_type = ExceptionType::try_from(exception_type_immediate).unwrap();

                return Err(agent.throw_exception_with_message(exception_type, message));
            }
            Instruction::PushExceptionJumpTarget => {
                vm.exception_jump_target_stack.push(ExceptionJumpTarget {
                    ip: instr.args[0].unwrap() as usize,
                    lexical_environment: agent
                        .running_execution_context()
                        .ecmascript_code
                        .as_ref()
                        .unwrap()
                        .lexical_environment,
                });
            }
            Instruction::PopExceptionJumpTarget => {
                vm.exception_jump_target_stack.pop().unwrap();
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
                let lexical = instr.args[1].unwrap() == 1;
                let env = if lexical {
                    // Lexical binding, const [] = a; or let [] = a;
                    Some(
                        agent
                            .running_execution_context()
                            .ecmascript_code
                            .as_ref()
                            .unwrap()
                            .lexical_environment,
                    )
                } else {
                    // Var binding, var [] = a;
                    None
                };
                let iterator = vm.iterator_stack.pop().unwrap().bind(gc.nogc());
                execute_simple_array_binding(agent, vm, executable, iterator, env, gc.reborrow())?
            }
            Instruction::BeginSimpleObjectBindingPattern => {
                let lexical = instr.args[0].unwrap() == 1;
                let env = if lexical {
                    // Lexical binding, const {} = a; or let {} = a;
                    Some(
                        agent
                            .running_execution_context()
                            .ecmascript_code
                            .as_ref()
                            .unwrap()
                            .lexical_environment,
                    )
                } else {
                    // Var binding, var {} = a;
                    None
                };
                let object = to_object(agent, vm.stack.pop().unwrap(), gc.nogc())?;
                execute_simple_object_binding(
                    agent,
                    vm,
                    executable,
                    object.unbind(),
                    env,
                    gc.reborrow(),
                )?
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
                let argument_count = instr.args[0].unwrap() as usize;
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
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, mut gc| {
                            for ele in args.iter_mut() {
                                let maybe_string = ele.get(agent).bind(gc.nogc());
                                if maybe_string.is_string() {
                                    continue;
                                }
                                let string =
                                    to_string(agent, maybe_string.unbind(), gc.reborrow())?;
                                length += string.len(agent);
                                let string = string.into_value();
                                // SAFETY: args are never shared
                                unsafe { ele.replace(agent, string.unbind()) };
                            }
                            Ok(())
                        },
                        gc.reborrow(),
                    )?;
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
            Instruction::Delete => {
                let refer = vm.reference.take().unwrap().bind(gc.nogc());
                match refer.base {
                    // 3. If IsUnresolvableReference(ref) is true, then
                    Base::Unresolvable => {
                        // a. Assert: ref.[[Strict]] is false.
                        debug_assert!(!refer.strict);
                        // b. Return true.
                        vm.result = Some(true.into());
                    }
                    // 4. If IsPropertyReference(ref) is true, then
                    Base::Value(base) => {
                        // a. Assert: IsPrivateReference(ref) is false.
                        debug_assert!(!is_private_reference(&refer));
                        // b. If IsSuperReference(ref) is true, throw a ReferenceError exception.
                        if is_super_reference(&refer) {
                            return Err(agent.throw_exception_with_static_message(
                                ExceptionType::ReferenceError,
                                "Invalid delete involving 'super'.",
                                gc.nogc(),
                            ));
                        }
                        // c. Let baseObj be ? ToObject(ref.[[Base]]).
                        let base_obj = to_object(agent, base, gc.nogc())?;
                        // d. If ref.[[ReferencedName]] is not a property key, then
                        // TODO: Is this relevant?
                        // i. Set ref.[[ReferencedName]] to ? ToPropertyKey(ref.[[ReferencedName]]).
                        // e. Let deleteStatus be ? baseObj.[[Delete]](ref.[[ReferencedName]]).
                        let strict = refer.strict;
                        let delete_status = if let TryResult::Continue(delete_status) =
                            base_obj.try_delete(agent, refer.referenced_name, gc.nogc())
                        {
                            delete_status
                        } else {
                            let base_obj = base_obj.unbind();
                            let referenced_name = refer.referenced_name.unbind();
                            with_vm_gc(
                                agent,
                                vm,
                                |agent, gc| base_obj.internal_delete(agent, referenced_name, gc),
                                gc.reborrow(),
                            )?
                        };
                        // f. If deleteStatus is false and ref.[[Strict]] is true, throw a TypeError exception.
                        if !delete_status && strict {
                            return Err(agent.throw_exception_with_static_message(
                                ExceptionType::TypeError,
                                "Cannot delete property",
                                gc.nogc(),
                            ));
                        }
                        // g. Return deleteStatus.
                        vm.result = Some(delete_status.into());
                    }
                    // 5. Else,
                    Base::Environment(base) => {
                        // a. Let base be ref.[[Base]].
                        // b. Assert: base is an Environment Record.
                        let referenced_name = match refer.referenced_name {
                            PropertyKey::SmallString(data) => String::SmallString(data),
                            PropertyKey::String(data) => String::String(data),
                            _ => unreachable!(),
                        };
                        // c. Return ? base.DeleteBinding(ref.[[ReferencedName]]).
                        let result = if let TryResult::Continue(result) =
                            base.try_delete_binding(agent, referenced_name, gc.nogc())
                        {
                            result?
                        } else {
                            let referenced_name = referenced_name.unbind();
                            with_vm_gc(
                                agent,
                                vm,
                                |agent, gc| base.delete_binding(agent, referenced_name, gc),
                                gc,
                            )?
                        };
                        vm.result = Some(result.into());
                    }
                }

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
            Instruction::EnumerateObjectProperties => {
                let object = to_object(agent, vm.result.take().unwrap(), gc.nogc()).unwrap();
                vm.iterator_stack
                    .push(VmIterator::ObjectProperties(ObjectPropertiesIterator::new(
                        object,
                    )))
            }
            Instruction::GetIteratorSync => {
                let expr_value = vm.result.take().unwrap();
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| VmIterator::from_value(agent, expr_value, gc),
                    gc,
                )?;
                vm.iterator_stack.push(result);
            }
            Instruction::GetIteratorAsync => {
                todo!();
            }
            Instruction::IteratorStepValue => {
                let mut iterator = vm.iterator_stack.pop().unwrap();
                let result = with_vm_gc(agent, vm, |agent, gc| iterator.step_value(agent, gc), gc);
                if let Ok(result) = result {
                    vm.result = result.map(Value::unbind);
                    if result.is_none() {
                        // Iterator finished: No need to return the iterator
                        // onto the stack. Jump to escape the iterator loop.
                        vm.ip = instr.args[0].unwrap() as usize;
                    } else {
                        // Return our iterator into the iterator stack.
                        vm.iterator_stack.push(iterator);
                    }
                } else {
                    // No need to return the iterator into the stack.
                    result?;
                }
            }
            Instruction::IteratorStepValueOrUndefined => {
                let mut iterator = vm.iterator_stack.pop().unwrap();
                let result = with_vm_gc(agent, vm, |agent, gc| iterator.step_value(agent, gc), gc);
                if let Ok(result) = result {
                    vm.result = Some(result.unwrap_or(Value::Undefined).unbind());
                    if result.is_none() {
                        // We have exhausted the iterator; push in an empty
                        // VmIterator so further instructions aren't
                        // observable.
                        vm.iterator_stack
                            .push(VmIterator::SliceIterator(SendableRef::new(&[])));
                    } else {
                        // Return our iterator into the iterator stack.
                        vm.iterator_stack.push(iterator);
                    }
                } else {
                    // No need to return the iterator into the stack.
                    result?;
                }
            }
            Instruction::IteratorRestIntoArray => {
                let mut iterator = vm.iterator_stack.pop().unwrap();
                let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                let array =
                    array_create(agent, 0, capacity, None, gc.nogc())?.scope(agent, gc.nogc());

                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let mut idx: u32 = 0;
                        while let Some(value) = iterator.step_value(agent, gc.reborrow())? {
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
                )?;
                vm.result = Some(array.get(agent).into_value());
            }
            Instruction::IteratorClose => {
                let iterator = vm.iterator_stack.pop().unwrap();
                if let VmIterator::GenericIterator(iterator_record) = iterator {
                    let result = vm.result.take().unwrap_or(Value::Undefined);
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, gc| iterator_close(agent, &iterator_record, Ok(result), gc),
                        gc,
                    )?;
                }
            }
            Instruction::Yield => return Ok(ContinuationKind::Yield),
            Instruction::CreateUnmappedArgumentsObject => {
                let Some(VmIterator::SliceIterator(slice)) = vm.iterator_stack.last() else {
                    unreachable!()
                };
                vm.result = Some(
                    create_unmapped_arguments_object(agent, slice.get(), gc.nogc())
                        .into_value()
                        .unbind(),
                );
            }
            other => todo!("{other:?}"),
        }

        Ok(ContinuationKind::Normal)
    }

    fn get_call_args<'gc>(&mut self, instr: &Instr, _gc: NoGcScope<'gc, '_>) -> Vec<Value<'gc>> {
        let instr_arg0 = instr.args[0].unwrap();
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
}

fn concat_string_from_slice<'gc>(
    agent: &mut Agent,
    slice: &[String],
    string_length: usize,
    gc: NoGcScope<'gc, '_>,
) -> String<'gc> {
    let mut result_string = std::string::String::with_capacity(string_length);
    for string in slice.iter() {
        result_string.push_str(string.as_str(agent));
    }
    String::from_string(agent, result_string, gc)
}

/// ### [13.15.3 ApplyStringOrNumericBinaryOperator ( lval, opText, rval )](https://tc39.es/ecma262/#sec-applystringornumericbinaryoperator)
///
/// The abstract operation ApplyStringOrNumericBinaryOperator takes
/// arguments lval (an ECMAScript language value), opText (**, *, /, %, +,
/// -, <<, >>, >>>, &, ^, or |), and rval (an ECMAScript language value) and
/// returns either a normal completion containing either a String, a BigInt,
/// or a Number, or a throw completion.
#[inline]
fn apply_string_or_numeric_binary_operator<'gc>(
    agent: &mut Agent,
    lval: Value,
    op_text: BinaryOperator,
    rval: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Value<'gc>> {
    let lval = lval.bind(gc.nogc());
    let rval = rval.bind(gc.nogc());
    let lnum: Numeric<'gc>;
    let rnum: Numeric<'gc>;
    // 1. If opText is +, then
    let gc = if op_text == BinaryOperator::Addition {
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
                let rprim = to_primitive(agent, rval.unbind(), None, gc.reborrow())?.unbind();
                let gc = gc.into_nogc();
                let lprim = lprim.get(agent);
                (lprim.bind(gc), rprim.bind(gc), gc)
            }
            (Err(_), Ok(rprim)) => {
                let rprim = rprim.scope(agent, gc.nogc());
                let lprim = to_primitive(agent, lval.unbind(), None, gc.reborrow())?.unbind();
                let gc = gc.into_nogc();
                let rprim = rprim.get(agent);
                (lprim.bind(gc), rprim.bind(gc), gc)
            }
            (Err(_), Err(_)) => {
                let rval = rval.scope(agent, gc.nogc());
                let lprim = to_primitive(agent, lval.unbind(), None, gc.reborrow())?
                    .unbind()
                    .scope(agent, gc.nogc());
                let rprim = to_primitive(agent, rval.get(agent), None, gc.reborrow())?.unbind();
                let gc = gc.into_nogc();
                let lprim = lprim.get(agent);
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
                let lstr = lstr.bind(gc).scope(agent, gc);
                // ii. Let rstr be ? ToString(rprim).
                let rstr = to_string_primitive(agent, rprim, gc)?;
                // iii. Return the string-concatenation of lstr and rstr.
                return Ok(String::concat(agent, [lstr.get(agent).bind(gc), rstr], gc).into_value());
            }
            (Err(_), Ok(rstr)) => {
                let rstr = rstr.bind(gc).scope(agent, gc);
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
        lnum = to_numeric_primitive(agent, lprim, gc)?;
        // 4. Let rnum be ? ToNumeric(rval).
        rnum = to_numeric_primitive(agent, rprim, gc)?;
        gc
    } else {
        let rval = rval.scope(agent, gc.nogc());
        // 3. Let lnum be ? ToNumeric(lval).
        let scoped_lnum = to_numeric(agent, lval.unbind(), gc.reborrow())?
            .unbind()
            .scope(agent, gc.nogc());
        // 4. Let rnum be ? ToNumeric(rval).
        rnum = to_numeric(agent, rval.get(agent), gc.reborrow())?.unbind();
        let gc = gc.into_nogc();
        lnum = scoped_lnum.get(agent).bind(gc);
        gc
    };

    // 6. If lnum is a BigInt, then
    if let (Ok(lnum), Ok(rnum)) = (BigInt::try_from(lnum), BigInt::try_from(rnum)) {
        Ok(match op_text {
            // a. If opText is **, return ? BigInt::exponentiate(lnum, rnum).
            BinaryOperator::Exponential => {
                BigInt::exponentiate(agent, lnum, rnum, gc).map(|bigint| bigint.into_value())?
            }
            // b. If opText is /, return ? BigInt::divide(lnum, rnum).
            BinaryOperator::Division => {
                BigInt::divide(agent, lnum, rnum, gc).map(|bigint| bigint.into_value())?
            }
            // c. If opText is %, return ? BigInt::remainder(lnum, rnum).
            BinaryOperator::Remainder => {
                BigInt::remainder(agent, lnum, rnum, gc).map(|bigint| bigint.into_value())?
            }
            // d. If opText is >>>, return ? BigInt::unsignedRightShift(lnum, rnum).
            BinaryOperator::ShiftRightZeroFill => todo!(),
            // <<	BigInt	BigInt::leftShift
            BinaryOperator::ShiftLeft => todo!(),
            // >>	BigInt	BigInt::signedRightShift
            BinaryOperator::ShiftRight => todo!(),
            // +	BigInt	BigInt::add
            BinaryOperator::Addition => BigInt::add(agent, lnum, rnum).into_value(),
            // -	BigInt	BigInt::subtract
            BinaryOperator::Subtraction => BigInt::subtract(agent, lnum, rnum).into_value(),
            // *	BigInt	BigInt::multiply
            BinaryOperator::Multiplication => BigInt::multiply(agent, lnum, rnum).into_value(),
            // |	BigInt	BigInt::bitwiseOR
            BinaryOperator::BitwiseOR => todo!(),
            // ^	BigInt	BigInt::bitwiseXOR
            BinaryOperator::BitwiseXOR => todo!(),
            // &	BigInt	BigInt::bitwiseAND
            BinaryOperator::BitwiseAnd => todo!(),
            _ => unreachable!(),
        })
    } else if let (Ok(lnum), Ok(rnum)) = (Number::try_from(lnum), Number::try_from(rnum)) {
        // 7. Let operation be the abstract operation associated with opText and
        // Type(lnum) in the following table:
        // 8. Return operation(lnum, rnum).
        // NOTE: We do step 8. explicitly in branch.
        Ok(match op_text {
            // opText	Type(lnum)	operation
            // **	Number	Number::exponentiate
            BinaryOperator::Exponential => Number::exponentiate(agent, lnum, rnum).into_value(),
            // *	Number	Number::multiply
            BinaryOperator::Multiplication => Number::multiply(agent, lnum, rnum, gc).into_value(),
            // /	Number	Number::divide
            BinaryOperator::Division => Number::divide(agent, lnum, rnum, gc).into_value(),
            // %	Number	Number::remainder
            BinaryOperator::Remainder => Number::remainder(agent, lnum, rnum, gc).into_value(),
            // +	Number	Number::add
            BinaryOperator::Addition => Number::add(agent, lnum, rnum).into_value(),
            // -	Number	Number::subtract
            BinaryOperator::Subtraction => Number::subtract(agent, lnum, rnum).into_value(),
            // <<	Number	Number::leftShift
            BinaryOperator::ShiftLeft => Number::left_shift(agent, lnum, rnum).into_value(),
            // >>	Number	Number::signedRightShift
            BinaryOperator::ShiftRight => {
                Number::signed_right_shift(agent, lnum, rnum).into_value()
            }
            // >>>	Number	Number::unsignedRightShift
            BinaryOperator::ShiftRightZeroFill => {
                Number::unsigned_right_shift(agent, lnum, rnum).into_value()
            }
            // |	Number	Number::bitwiseOR
            BinaryOperator::BitwiseOR => Number::bitwise_or(agent, lnum, rnum).into(),
            // ^	Number	Number::bitwiseXOR
            BinaryOperator::BitwiseXOR => Number::bitwise_xor(agent, lnum, rnum).into(),
            // &	Number	Number::bitwiseAND
            BinaryOperator::BitwiseAnd => Number::bitwise_and(agent, lnum, rnum).into(),
            _ => unreachable!(),
        })
    } else {
        // 5. If Type(lnum) is not Type(rnum), throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "The left and right-hand sides do not have the same type.",
            gc,
        ))
    }
}

/// ### [13.5.3 The typeof operator](https://tc39.es/ecma262/#sec-typeof-operator)
#[inline]
fn typeof_operator(_: &mut Agent, val: Value) -> String<'static> {
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
        Value::Iterator |
        Value::ArrayIterator(_) |
        Value::MapIterator(_) |
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
        // TODO: Check [[Call]] slot for Proxy
        Value::Proxy(_) => todo!(),
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
pub(crate) fn instanceof_operator<'a>(
    agent: &mut Agent,
    value: impl IntoValue<'a>,
    target: impl IntoValue<'a>,
    mut gc: GcScope,
) -> JsResult<bool> {
    // 1. If target is not an Object, throw a TypeError exception.
    let Ok(target) = Object::try_from(target.into_value()) else {
        let error_message = format!(
            "Invalid instanceof target {}.",
            target
                .into_value()
                .string_repr(agent, gc.reborrow())
                .as_str(agent)
        );
        return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
    };
    // 2. Let instOfHandler be ? GetMethod(target, @@hasInstance).
    let inst_of_handler = get_method(
        agent,
        target.into_value(),
        WellKnownSymbolIndexes::HasInstance.into(),
        gc.reborrow(),
    )?;
    // 3. If instOfHandler is not undefined, then
    if let Some(inst_of_handler) = inst_of_handler {
        // a. Return ToBoolean(? Call(instOfHandler, target, « V »)).
        let result = call_function(
            agent,
            inst_of_handler.unbind(),
            target.into_value(),
            Some(ArgumentsList(&[value.into_value().unbind()])),
            gc.reborrow(),
        )?;
        Ok(to_boolean(agent, result))
    } else {
        // 4. If IsCallable(target) is false, throw a TypeError exception.
        let Some(target) = is_callable(target, gc.nogc()) else {
            let error_message = format!(
                "Invalid instanceof target {} is not a function.",
                target
                    .into_value()
                    .string_repr(agent, gc.reborrow())
                    .as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
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
    if cfg!(feature = "interleaved-gc") {
        let vm = NonNull::from(vm);
        agent.vm_stack.push(vm);
        let result = work(agent, gc);
        let return_vm = agent.vm_stack.pop().unwrap();
        assert_eq!(vm, return_vm, "VM Stack was misused");
        result
    } else {
        work(agent, gc)
    }
}

impl HeapMarkAndSweep for ExceptionJumpTarget {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            ip: _,
            lexical_environment,
        } = self;
        lexical_environment.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            ip: _,
            lexical_environment,
        } = self;
        lexical_environment.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for Vm {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Vm {
            ip: _,
            stack,
            reference_stack,
            iterator_stack,
            exception_jump_target_stack,
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
            exception_jump_target_stack,
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
