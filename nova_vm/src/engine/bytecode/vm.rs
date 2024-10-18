// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{ptr::NonNull, sync::OnceLock};

use ahash::AHashSet;
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
                copy_data_properties_into_object, create_data_property,
                create_data_property_or_throw, define_property_or_throw, get, get_method,
                has_property, ordinary_has_instance, set,
            },
            testing_and_comparison::{
                is_callable, is_constructor, is_less_than, is_loosely_equal, is_strictly_equal,
            },
            type_conversion::{
                to_boolean, to_number, to_numeric, to_object, to_primitive, to_property_key,
                to_string,
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
            is_super_reference, put_value, Base, BigInt, Function, InternalMethods, IntoFunction,
            IntoObject, IntoValue, Number, Numeric, Object, PropertyDescriptor, PropertyKey,
            Reference, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

use super::{
    executable::{ArrowFunctionExpression, SendableRef},
    instructions::Instr,
    iterator::{ObjectPropertiesIterator, VmIterator},
    Executable, FunctionExpression, IndexType, Instruction, InstructionIter,
    NamedEvaluationParameter,
};

struct EmptyParametersList(ast::FormalParameters<'static>);
unsafe impl Send for EmptyParametersList {}
unsafe impl Sync for EmptyParametersList {}

pub(crate) enum ExecutionResult {
    Return(Value),
    Throw(JsError),
    Await {
        vm: SuspendedVm,
        awaited_value: Value,
    },
    Yield {
        vm: SuspendedVm,
        yielded_value: Value,
    },
}
impl ExecutionResult {
    pub(crate) fn into_js_result(self) -> JsResult<Value> {
        match self {
            ExecutionResult::Return(value) => Ok(value),
            ExecutionResult::Throw(err) => Err(err),
            _ => panic!("Unexpected yield or await"),
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
    stack: Vec<Value>,
    reference_stack: Vec<Reference>,
    iterator_stack: Vec<VmIterator>,
    exception_jump_target_stack: Vec<ExceptionJumpTarget>,
    result: Option<Value>,
    exception: Option<Value>,
    reference: Option<Reference>,
}

#[derive(Debug)]
pub(crate) struct SuspendedVm {
    ip: usize,
    /// Note: Stack is non-empty only if the code awaits inside a call
    /// expression. This is reasonably rare that we can expect the stack to
    /// usually be empty. In this case this Box is an empty dangling pointer
    /// and no heap data clone is required.
    stack: Box<[Value]>,
    /// Note: Reference stack is non-empty only if the code awaits inside a
    /// call expression. This means that usually no heap data clone is
    /// required.
    reference_stack: Box<[Reference]>,
    /// Note: Iterator stack is non-empty only if the code awaits inside a
    /// for-in or for-of loop. This means that often no heap data clone is
    /// required.
    iterator_stack: Box<[VmIterator]>,
    /// Note: Exception jump stack is non-empty only if the code awaits inside
    /// a try block. This means that often no heap data clone is required.
    exception_jump_target_stack: Box<[ExceptionJumpTarget]>,
}

impl SuspendedVm {
    pub(crate) fn resume(
        self,
        agent: &mut Agent,
        executable: Executable,
        value: Value,
    ) -> ExecutionResult {
        let vm = Vm::from_suspended(self);
        vm.resume(agent, executable, value)
    }

    pub(crate) fn resume_throw(
        self,
        agent: &mut Agent,
        executable: Executable,
        err: Value,
    ) -> ExecutionResult {
        // Optimisation: Avoid unsuspending the Vm if we're just going to throw
        // out of it immediately.
        if self.exception_jump_target_stack.is_empty() {
            let err = JsError::new(err);
            return ExecutionResult::Throw(err);
        }
        let vm = Vm::from_suspended(self);
        vm.resume_throw(agent, executable, err)
    }
}

impl Vm {
    fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            iterator_stack: Vec::new(),
            exception_jump_target_stack: Vec::new(),
            result: None,
            exception: None,
            reference: None,
        }
    }

    fn suspend(self) -> SuspendedVm {
        SuspendedVm {
            ip: self.ip,
            stack: self.stack.into_boxed_slice(),
            reference_stack: self.reference_stack.into_boxed_slice(),
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
            exception: None,
            reference: None,
        }
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute(
        agent: &mut Agent,
        executable: Executable,
        arguments: Option<&[Value]>,
    ) -> ExecutionResult {
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
            eprintln!("Constants: {:?}", executable.get_constants(agent));
            eprintln!();

            eprintln!("Instructions:");
            let iter = InstructionIter::new(executable.get_instructions(agent));
            for (ip, instr) in iter {
                match instr.kind.argument_count() {
                    0 => {
                        eprintln!("  {}: {:?}()", ip, instr.kind);
                    }
                    1 => {
                        let arg0 = instr.args.first().unwrap().unwrap();
                        eprintln!("  {}: {:?}({})", ip, instr.kind, arg0);
                    }
                    2 => {
                        let arg0 = instr.args.first().unwrap().unwrap();
                        let arg1 = instr.args.last().unwrap();
                        eprintln!("  {}: {:?}({}, {:?})", ip, instr.kind, arg0, arg1);
                    }
                    _ => unreachable!(),
                }
            }
            eprintln!();
        }

        vm.inner_execute(agent, executable)
    }

    pub fn resume(
        mut self,
        agent: &mut Agent,
        executable: Executable,
        value: Value,
    ) -> ExecutionResult {
        self.result = Some(value);
        self.inner_execute(agent, executable)
    }

    pub fn resume_throw(
        mut self,
        agent: &mut Agent,
        executable: Executable,
        err: Value,
    ) -> ExecutionResult {
        let err = JsError::new(err);
        if !self.handle_error(agent, err) {
            return ExecutionResult::Throw(err);
        }
        self.inner_execute(agent, executable)
    }

    fn inner_execute(mut self, agent: &mut Agent, executable: Executable) -> ExecutionResult {
        #[cfg(feature = "interleaved-gc")]
        let do_gc = !agent.options.disable_gc;
        #[cfg(feature = "interleaved-gc")]
        let mut instr_count = 0u8;
        while let Some(instr) = executable.get_instruction(agent, &mut self.ip) {
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
                    let vm = unsafe { NonNull::new_unchecked(&mut self) };
                    agent
                        .vm_stack
                        // SAFETY: Pointer to self is never null.
                        .push(vm);
                    heap_gc(agent, &mut root_realms);
                    let return_vm = agent.vm_stack.pop().unwrap();
                    assert_eq!(vm, return_vm, "VM Stack was misused");
                }
            }
            match Self::execute_instruction(agent, &mut self, executable, &instr) {
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
            self.exception = Some(err.value());
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
    ) -> JsResult<ContinuationKind> {
        if agent.options.print_internals {
            eprintln!("Executing instruction {:?}", instr.kind);
        }
        match instr.kind {
            Instruction::ArrayCreate => {
                vm.stack.push(
                    array_create(agent, 0, instr.args[0].unwrap() as usize, None)?.into_value(),
                );
            }
            Instruction::ArrayPush => {
                let value = vm.result.take().unwrap();
                let array = *vm.stack.last().unwrap();
                let Ok(array) = Array::try_from(array) else {
                    unreachable!();
                };
                let len = array.len(agent);
                let key = PropertyKey::Integer(len.into());
                create_data_property_or_throw(agent, array, key, value)?
            }
            Instruction::ArrayElision => {
                let array = *vm.stack.last().unwrap();
                let Ok(array) = Array::try_from(array) else {
                    unreachable!();
                };
                set(
                    agent,
                    array.into_object(),
                    BUILTIN_STRING_MEMORY.length.into(),
                    (array.len(agent) + 1).into(),
                    true,
                )?;
            }
            Instruction::Await => return Ok(ContinuationKind::Await),
            Instruction::BitwiseNot => {
                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                let old_value = to_numeric(agent, vm.result.take().unwrap())?;

                // 3. If oldValue is a Number, then
                if let Ok(old_value) = Number::try_from(old_value) {
                    // a. Return Number::bitwiseNOT(oldValue).
                    vm.result = Some(Number::bitwise_not(agent, old_value)?.into_value());
                } else {
                    // 4. Else,
                    // a. Assert: oldValue is a BigInt.
                    let Ok(old_value) = BigInt::try_from(old_value) else {
                        unreachable!();
                    };

                    // b. Return BigInt::bitwiseNOT(oldValue).
                    vm.result = Some(BigInt::bitwise_not(agent, old_value).into_value());
                }
            }
            Instruction::Debug => {
                if agent.options.print_internals {
                    eprintln!("Debug: {:#?}", vm);
                }
            }
            Instruction::ResolveBinding => {
                let identifier =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);

                let reference = resolve_binding(agent, identifier, None)?;

                vm.reference = Some(reference);
            }
            Instruction::ResolveThisBinding => {
                // 1. Let envRec be GetThisEnvironment().
                let env_rec = get_this_environment(agent);
                // 2. Return ? envRec.GetThisBinding().
                vm.result = Some(match env_rec {
                    EnvironmentIndex::Declarative(_) => unreachable!(),
                    EnvironmentIndex::Function(idx) => idx.get_this_binding(agent)?,
                    EnvironmentIndex::Global(idx) => idx.get_this_binding(agent).into_value(),
                    EnvironmentIndex::Object(_) => unreachable!(),
                });
            }
            Instruction::LoadConstant => {
                let constant = executable.fetch_constant(agent, instr.args[0].unwrap() as usize);
                vm.stack.push(constant);
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
                let constant = executable.fetch_constant(agent, instr.args[0].unwrap() as usize);
                vm.result = Some(constant);
            }
            Instruction::UnaryMinus => {
                let old_value = vm.result.unwrap();

                // 3. If oldValue is a Number, then
                if let Ok(old_value) = Number::try_from(old_value) {
                    // a. Return Number::unaryMinus(oldValue).
                    vm.result = Some(Number::unary_minus(agent, old_value).into());
                }
                // 4. Else,
                else {
                    // a. Assert: oldValue is a BigInt.
                    let old_value = BigInt::try_from(old_value).unwrap();

                    // b. Return BigInt::unaryMinus(oldValue).
                    vm.result = Some(BigInt::unary_minus(agent, old_value).into());
                }
            }
            Instruction::ToNumber => {
                vm.result =
                    to_number(agent, vm.result.unwrap()).map(|number| Some(number.into()))?;
            }
            Instruction::ToNumeric => {
                vm.result =
                    Some(to_numeric(agent, vm.result.unwrap()).map(|result| result.into_value())?);
            }
            Instruction::ToObject => {
                vm.result = Some(to_object(agent, vm.result.unwrap())?.into_value());
            }
            Instruction::ApplyStringOrNumericBinaryOperator(op_text) => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                vm.result = Some(apply_string_or_numeric_binary_operator(
                    agent, lval, op_text, rval,
                )?);
            }
            Instruction::ObjectDefineProperty => {
                let value = vm.result.take().unwrap();
                let key = to_property_key(agent, vm.stack.pop().unwrap())?;
                let object = *vm.stack.last().unwrap();
                let object = Object::try_from(object).unwrap();
                create_data_property_or_throw(agent, object, key, value).unwrap()
            }
            Instruction::ObjectDefineMethod => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let enumerable = instr.args[1].unwrap() != 0;
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = to_property_key(agent, vm.stack.pop().unwrap())?;
                let object = Object::try_from(*vm.stack.last().unwrap()).unwrap();

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
                let closure = ordinary_function_create(agent, params);
                // 8. Perform MakeMethod(closure, object).
                make_method(agent, closure, object);
                // 2. Perform SetFunctionName(closure, propKey).
                set_function_name(agent, closure, prop_key, None);
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
                    value: Some(closure.into_value()),
                    writable: Some(true),
                    enumerable: Some(enumerable),
                    configurable: Some(true),
                    ..Default::default()
                };
                // b. Perform ? DefinePropertyOrThrow(homeObject, key, desc).
                // c. NOTE: DefinePropertyOrThrow only returns an abrupt
                // completion when attempting to define a class static method whose key is "prototype".
                define_property_or_throw(agent, object, prop_key, desc)?;
                // c. Return unused.
            }
            Instruction::ObjectDefineGetter => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let enumerable = instr.args[1].unwrap() != 0;
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = to_property_key(agent, vm.stack.pop().unwrap())?;
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
                    allocator.set_allocation_limit(Some(0));
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
                let closure = ordinary_function_create(agent, params);
                // 7. Perform MakeMethod(closure, object).
                let object = Object::try_from(*vm.stack.last().unwrap()).unwrap();
                make_method(agent, closure, object);
                // 8. Perform SetFunctionName(closure, propKey, "get").
                set_function_name(agent, closure, prop_key, Some(BUILTIN_STRING_MEMORY.get));
                // 9. If propKey is a Private Name, then
                // a. Return PrivateElement { [[Key]]: propKey, [[Kind]]: accessor, [[Get]]: closure, [[Set]]: undefined }.
                // 10. Else,
                // a. Let desc be the PropertyDescriptor { [[Get]]: closure, [[Enumerable]]: enumerable, [[Configurable]]: true }.
                let desc = PropertyDescriptor {
                    value: None,
                    writable: None,
                    get: Some(closure.into_function()),
                    set: None,
                    enumerable: Some(enumerable),
                    configurable: Some(true),
                };
                // b. Perform ? DefinePropertyOrThrow(object, propKey, desc).
                define_property_or_throw(agent, object, prop_key, desc)?;
                // c. Return unused.
            }
            Instruction::ObjectDefineSetter => {
                let FunctionExpression { expression, .. } =
                    executable.fetch_function_expression(agent, instr.args[0].unwrap() as usize);
                let function_expression = expression.get();
                let enumerable = instr.args[1].unwrap() != 0;
                // 1. Let propKey be ? Evaluation of ClassElementName.
                let prop_key = to_property_key(agent, vm.stack.pop().unwrap())?;
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
                let closure = ordinary_function_create(agent, params);
                // 6. Perform MakeMethod(closure, object).
                let object = Object::try_from(*vm.stack.last().unwrap()).unwrap();
                make_method(agent, closure, object);
                // 7. Perform SetFunctionName(closure, propKey, "set").
                set_function_name(agent, closure, prop_key, Some(BUILTIN_STRING_MEMORY.set));
                // 8. If propKey is a Private Name, then
                // a. Return PrivateElement { [[Key]]: propKey, [[Kind]]: accessor, [[Get]]: undefined, [[Set]]: closure }.
                // 9. Else,
                // a. Let desc be the PropertyDescriptor { [[Set]]: closure, [[Enumerable]]: enumerable, [[Configurable]]: true }.
                let desc = PropertyDescriptor {
                    value: None,
                    writable: None,
                    get: None,
                    set: Some(closure.into_function()),
                    enumerable: Some(enumerable),
                    configurable: Some(true),
                };
                // b. Perform ? DefinePropertyOrThrow(object, propKey, desc).
                define_property_or_throw(agent, object, prop_key, desc)?;
                // c. Return unused.
            }
            Instruction::ObjectSetPrototype => {
                let prop_value = vm.result.take().unwrap();

                // a. If propValue is an Object or propValue is null, then
                let prop_value = if prop_value.is_null() {
                    None
                } else if let Ok(prop_value) = Object::try_from(prop_value) {
                    Some(prop_value)
                } else {
                    // b. Return unused.
                    return Ok(ContinuationKind::Normal);
                };
                // i. Perform ! object.[[SetPrototypeOf]](propValue).
                let object = Object::try_from(*vm.stack.last().unwrap()).unwrap();
                object.internal_set_prototype_of(agent, prop_value)?;
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
                put_value(agent, &reference, value)?;
            }
            Instruction::GetValue => {
                // 1. If V is not a Reference Record, return V.
                let reference = vm.reference.take().unwrap();

                vm.result = Some(get_value(agent, &reference)?);
            }
            Instruction::GetValueKeepReference => {
                // 1. If V is not a Reference Record, return V.
                let reference = vm.reference.as_ref().unwrap();

                vm.result = Some(get_value(agent, reference)?);
            }
            Instruction::Typeof => {
                // 2. If val is a Reference Record, then
                let val = if let Some(reference) = vm.reference.take() {
                    if reference.base == Base::Unresolvable {
                        // a. If IsUnresolvableReference(val) is true, return "undefined".
                        Value::Undefined
                    } else {
                        // 3. Set val to ? GetValue(val).
                        get_value(agent, &reference)?
                    }
                } else {
                    vm.result.unwrap()
                };
                vm.result = Some(typeof_operator(agent, val).into())
            }
            Instruction::ObjectCreate => {
                let object = ordinary_object_create_with_intrinsics(
                    agent,
                    Some(ProtoIntrinsics::Object),
                    None,
                );
                vm.stack.push(object.into())
            }
            Instruction::CopyDataProperties => {
                let source = vm.result.take().unwrap();
                let Value::Object(target) = *vm.stack.last().unwrap() else {
                    unreachable!()
                };
                copy_data_properties(agent, target, source)?;
            }
            Instruction::CopyDataPropertiesIntoObject => {
                let from = Object::try_from(vm.result.unwrap()).unwrap();

                let num_excluded_items = usize::from(instr.args[0].unwrap());
                let mut excluded_items = AHashSet::with_capacity(num_excluded_items);
                assert!(vm.reference.is_none());
                for _ in 0..num_excluded_items {
                    let reference = vm.reference_stack.pop().unwrap();
                    assert_eq!(reference.base, Base::Value(from.into_value()));
                    assert!(reference.this_value.is_none());
                    excluded_items.insert(reference.referenced_name);
                }

                vm.result = Some(
                    copy_data_properties_into_object(agent, from, &excluded_items)?.into_value(),
                );
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
                let function = ordinary_function_create(agent, params);
                let name = if let Some(parameter) = &identifier {
                    match parameter {
                        NamedEvaluationParameter::Result => {
                            to_property_key(agent, vm.result.unwrap())?
                        }
                        NamedEvaluationParameter::Stack => {
                            to_property_key(agent, *vm.stack.last().unwrap())?
                        }
                        NamedEvaluationParameter::Reference => {
                            vm.reference.as_ref().unwrap().referenced_name
                        }
                        NamedEvaluationParameter::ReferenceStack => {
                            vm.reference_stack.last().unwrap().referenced_name
                        }
                    }
                } else {
                    String::EMPTY_STRING.into()
                };
                set_function_name(agent, function, name, None);
                vm.result = Some(function.into_value());
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
                    let name = match parameter {
                        NamedEvaluationParameter::Result => {
                            to_property_key(agent, vm.result.unwrap())?
                        }
                        NamedEvaluationParameter::Stack => {
                            to_property_key(agent, *vm.stack.last().unwrap())?
                        }
                        NamedEvaluationParameter::Reference => {
                            vm.reference.as_ref().unwrap().referenced_name
                        }
                        NamedEvaluationParameter::ReferenceStack => {
                            vm.reference_stack.last().unwrap().referenced_name
                        }
                    };
                    (name, lexical_environment, false)
                } else if let Some(binding_identifier) = &function_expression.id {
                    let name = String::from_str(agent, &binding_identifier.name);
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
                let function = ordinary_function_create(agent, params);
                if let Some(compiled_bytecode) = compiled_bytecode {
                    agent[function].compiled_bytecode = Some(compiled_bytecode);
                }
                set_function_name(agent, function, name, None);
                if !function_expression.r#async && !function_expression.generator {
                    make_constructor(agent, function, None, None);
                }

                if function_expression.generator {
                    // InstantiateGeneratorFunctionExpression
                    // 7. Let prototype be OrdinaryObjectCreate(%GeneratorFunction.prototype.prototype%).
                    // NOTE: Although `prototype` has the generator prototype, it doesn't have the generator
                    // internals slots, so it's created as an ordinary object.
                    let prototype = ordinary_object_create_with_intrinsics(
                        agent,
                        Some(ProtoIntrinsics::Object),
                        Some(
                            agent
                                .current_realm()
                                .intrinsics()
                                .generator_prototype()
                                .into_object(),
                        ),
                    );
                    // 8. Perform ! DefinePropertyOrThrow(F, "prototype", PropertyDescriptor { [[Value]]: prototype, [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false }).
                    define_property_or_throw(
                        agent,
                        function,
                        BUILTIN_STRING_MEMORY.prototype.to_property_key(),
                        PropertyDescriptor {
                            value: Some(prototype.into_value()),
                            writable: Some(true),
                            get: None,
                            set: None,
                            enumerable: Some(false),
                            configurable: Some(false),
                        },
                    )
                    .unwrap();
                }

                if init_binding {
                    let name = match name {
                        PropertyKey::SmallString(data) => data.into(),
                        PropertyKey::String(data) => data.into(),
                        _ => unreachable!("maybe?"),
                    };
                    env.initialize_binding(agent, name, function.into_value())
                        .unwrap();
                }
                vm.result = Some(function.into_value());
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
                let proto = Object::try_from(*vm.stack.last().unwrap()).unwrap();

                let is_null_derived_class = !has_constructor_parent
                    && proto.internal_get_prototype_of(agent).unwrap().is_none();

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
                let function = ordinary_function_create(agent, params);
                if let Some(compiled_bytecode) = compiled_bytecode {
                    agent[function].compiled_bytecode = Some(compiled_bytecode);
                }
                set_function_name(agent, function, class_name.into(), None);
                make_constructor(agent, function, Some(false), Some(proto));
                agent[function].ecmascript_function.home_object = Some(proto);
                agent[function].ecmascript_function.constructor_status =
                    if has_constructor_parent || is_null_derived_class {
                        ConstructorStatus::DerivedClass
                    } else {
                        ConstructorStatus::BaseClass
                    };

                proto
                    .internal_define_own_property(
                        agent,
                        BUILTIN_STRING_MEMORY.constructor.into(),
                        PropertyDescriptor {
                            value: Some(function.into_value()),
                            writable: Some(true),
                            enumerable: Some(false),
                            configurable: Some(true),
                            ..Default::default()
                        },
                    )
                    .unwrap();

                vm.result = Some(function.into_value());
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
                );

                proto
                    .internal_define_own_property(
                        agent,
                        BUILTIN_STRING_MEMORY.constructor.into(),
                        PropertyDescriptor {
                            value: Some(function.into_value()),
                            writable: Some(true),
                            enumerable: Some(false),
                            configurable: Some(true),
                            ..Default::default()
                        },
                    )
                    .unwrap();

                vm.result = Some(function.into_value());
            }
            Instruction::Swap => {
                let a = vm.stack.pop().unwrap();
                let b = vm.stack.pop().unwrap();
                vm.stack.push(a);
                vm.stack.push(b);
            }
            Instruction::DirectEvalCall => {
                let args = vm.get_call_args(instr);

                let func_reference = resolve_binding(agent, BUILTIN_STRING_MEMORY.eval, None)?;
                let func = get_value(agent, &func_reference)?;

                // a. If SameValue(func, %eval%) is true, then
                if func == agent.current_realm().intrinsics().eval().into_value() {
                    // i. Let argList be ? ArgumentListEvaluation of arguments.
                    // ii. If argList has no elements, return undefined.
                    if args.is_empty() {
                        vm.result = Some(Value::Undefined);
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
                        vm.result = Some(perform_eval(agent, eval_arg, true, strict_caller)?);
                    }
                } else if cfg!(feature = "interleaved-gc") {
                    let mut vm = NonNull::from(vm);
                    agent.vm_stack.push(vm);
                    let result = call(agent, func, Value::Undefined, Some(ArgumentsList(&args)));
                    let return_vm = agent.vm_stack.pop().unwrap();
                    assert_eq!(vm, return_vm, "VM Stack was misused");
                    // SAFETY: This is fairly bonkers-unsafe. We have an
                    // exclusive reference to `Vm` so turning that to a NonNull
                    // and making the `&mut Vm` unreachable here isn't wrong.
                    // Passing that NonNull into a stack isn't wrong.
                    // Popping from that stack isn't wrong.
                    // Turning that back into a `&mut Vm` is probably wrong.
                    // Even though we can't reach the `vm: &mut Vm` in this
                    // scope anymore, it's still there. Hence we have two
                    // exclusive references alive at the same time. That's not
                    // a good look. I'm sorry.
                    unsafe { vm.as_mut() }.result = Some(result?);
                } else {
                    vm.result = Some(call(
                        agent,
                        func,
                        Value::Undefined,
                        Some(ArgumentsList(&args)),
                    )?);
                }
            }
            Instruction::EvaluateCall => {
                let args = vm.get_call_args(instr);
                let reference = vm.reference.take();
                // 1. If ref is a Reference Record, then
                let this_value = if let Some(reference) = reference {
                    // a. If IsPropertyReference(ref) is true, then
                    match reference.base {
                        // i. Let thisValue be GetThisValue(ref).
                        Base::Value(_) => get_this_value(&reference),
                        // b. Else,
                        Base::Environment(ref_env) => {
                            // i. Let refEnv be ref.[[Base]].
                            // iii. Let thisValue be refEnv.WithBaseObject().
                            ref_env
                                .with_base_object(agent)
                                .map_or(Value::Undefined, |object| object.into_value())
                        }
                        // ii. Assert: refEnv is an Environment Record.
                        Base::Unresolvable => unreachable!(),
                    }
                } else {
                    // 2. Else,
                    // a. Let thisValue be undefined.
                    Value::Undefined
                };
                let func = vm.stack.pop().unwrap();
                if cfg!(feature = "interleaved-gc") {
                    let mut vm = NonNull::from(vm);
                    agent.vm_stack.push(vm);
                    let result = call(agent, func, this_value, Some(ArgumentsList(&args)));
                    let return_vm = agent.vm_stack.pop().unwrap();
                    assert_eq!(vm, return_vm, "VM Stack was misused");
                    // SAFETY: This is fairly bonkers-unsafe. I'm sorry.
                    unsafe { vm.as_mut() }.result = Some(result?);
                } else {
                    vm.result = Some(call(agent, func, this_value, Some(ArgumentsList(&args)))?);
                }
            }
            Instruction::EvaluateNew => {
                let args = vm.get_call_args(instr);
                let constructor = vm.stack.pop().unwrap();
                let Some(constructor) = is_constructor(agent, constructor) else {
                    let error_message = format!(
                        "'{}' is not a constructor.",
                        constructor.string_repr(agent).as_str(agent)
                    );
                    return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
                };

                if cfg!(feature = "interleaved-gc") {
                    let mut vm = NonNull::from(vm);
                    agent.vm_stack.push(vm);
                    let result = construct(agent, constructor, Some(ArgumentsList(&args)), None)
                        .map(|result| result.into_value());
                    let return_vm = agent.vm_stack.pop().unwrap();
                    assert_eq!(vm, return_vm, "VM Stack was misused");
                    // SAFETY: This is fairly bonkers-unsafe. I'm sorry.
                    unsafe { vm.as_mut() }.result = Some(result?);
                } else {
                    vm.result = Some(
                        construct(agent, constructor, Some(ArgumentsList(&args)), None)?
                            .into_value(),
                    );
                }
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
                        Function::try_from(data.new_target.unwrap()).unwrap(),
                        data.function_object
                            .internal_get_prototype_of(agent)
                            .unwrap(),
                    )
                };
                // 4. Let argList be ? ArgumentListEvaluation of Arguments.
                let arg_list = vm.get_call_args(instr);
                // 5. If IsConstructor(func) is false, throw a TypeError exception.
                let Some(func) = func.and_then(|func| is_constructor(agent, func)) else {
                    let error_message = format!(
                        "'{}' is not a constructor.",
                        func.map_or(Value::Null, |func| func.into_value())
                            .string_repr(agent)
                            .as_str(agent)
                    );
                    return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
                };
                // 6. Let result be ? Construct(func, argList, newTarget).
                let result = construct(
                    agent,
                    func,
                    Some(ArgumentsList(&arg_list)),
                    Some(new_target),
                )?;
                // 7. Let thisER be GetThisEnvironment().
                let EnvironmentIndex::Function(this_er) = get_this_environment(agent) else {
                    unreachable!();
                };
                // 8. Perform ? thisER.BindThisValue(result).
                this_er.bind_this_value(agent, result.into_value())?;
                // 9. Let F be thisER.[[FunctionObject]].
                // 10. Assert: F is an ECMAScript function object.
                let Function::ECMAScriptFunction(_f) = agent[this_er].function_object else {
                    unreachable!();
                };
                // 11. Perform ? InitializeInstanceElements(result, F).
                // 12. Return result.
                vm.result = Some(result.into_value());
            }
            Instruction::EvaluatePropertyAccessWithExpressionKey => {
                let property_name_value = vm.result.take().unwrap();
                let base_value = vm.stack.pop().unwrap();

                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;

                let property_key = to_property_key(agent, property_name_value)?;

                vm.reference = Some(Reference {
                    base: Base::Value(base_value),
                    referenced_name: property_key,
                    strict,
                    this_value: None,
                });
            }
            Instruction::EvaluatePropertyAccessWithIdentifierKey => {
                let property_name_string =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                let base_value = vm.result.take().unwrap();
                let strict = agent
                    .running_execution_context()
                    .ecmascript_code
                    .unwrap()
                    .is_strict_mode;

                vm.reference = Some(Reference {
                    base: Base::Value(base_value),
                    referenced_name: property_name_string.into(),
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
                let lhs = vm.result.take().unwrap();
                let old_value = to_numeric(agent, lhs)?;
                let new_value = if let Ok(old_value) = Number::try_from(old_value) {
                    Number::add(agent, old_value, 1.into())
                } else {
                    todo!();
                    // let old_value = BigInt::try_from(old_value).unwrap();
                    // BigInt::add(agent, old_value, 1.into());
                };
                vm.result = Some(new_value.into_value());
            }
            Instruction::Decrement => {
                let lhs = vm.result.take().unwrap();
                let old_value = to_numeric(agent, lhs)?;
                let new_value = if let Ok(old_value) = Number::try_from(old_value) {
                    Number::subtract(agent, old_value, 1.into())
                } else {
                    todo!();
                    // let old_value = BigInt::try_from(old_value).unwrap();
                    // BigInt::subtract(agent, old_value, 1.into());
                };
                vm.result = Some(new_value.into_value());
            }
            Instruction::LessThan => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_less_than::<true>(agent, lval, rval)? == Some(true);
                vm.result = Some(result.into());
            }
            Instruction::LessThanEquals => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_less_than::<false>(agent, rval, lval)? == Some(false);
                vm.result = Some(result.into());
            }
            Instruction::GreaterThan => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_less_than::<false>(agent, rval, lval)? == Some(true);
                vm.result = Some(result.into());
            }
            Instruction::GreaterThanEquals => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_less_than::<true>(agent, lval, rval)? == Some(false);
                vm.result = Some(result.into());
            }
            Instruction::HasProperty => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                // RelationalExpression : RelationalExpression in ShiftExpression
                // 5. If rval is not an Object, throw a TypeError exception.
                let Ok(rval) = Object::try_from(rval) else {
                    let error_message = format!(
                        "The right-hand side of an `in` expression must be an object, got '{}'.",
                        rval.string_repr(agent).as_str(agent)
                    );
                    return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
                };
                // 6. Return ? HasProperty(rval, ? ToPropertyKey(lval)).
                let property_key = to_property_key(agent, lval)?;
                vm.result = Some(Value::Boolean(has_property(agent, rval, property_key)?));
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
                let result = is_loosely_equal(agent, lval, rval)?;
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
                initialize_referenced_binding(agent, v, w)?;
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
                let name = executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                lex_env.create_mutable_binding(agent, name, false).unwrap();
            }
            Instruction::CreateImmutableBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name = executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                lex_env.create_immutable_binding(agent, name, true).unwrap();
            }
            Instruction::CreateCatchBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name = executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                lex_env.create_mutable_binding(agent, name, false).unwrap();
                lex_env
                    .initialize_binding(agent, name, vm.exception.unwrap())
                    .unwrap();
                vm.exception = None;
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
                if cfg!(feature = "interleaved-gc") {
                    let mut vm = NonNull::from(vm);
                    agent.vm_stack.push(vm);
                    let result = instanceof_operator(agent, lval, rval);
                    let return_vm = agent.vm_stack.pop().unwrap();
                    assert_eq!(vm, return_vm, "VM Stack was misused");
                    // SAFETY: This is fairly bonkers-unsafe. I'm sorry.
                    unsafe { vm.as_mut() }.result = Some(result?.into());
                } else {
                    vm.result = Some(instanceof_operator(agent, lval, rval)?.into());
                }
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
                let iterator = vm.iterator_stack.pop().unwrap();
                Self::execute_simple_array_binding(agent, vm, executable, iterator, env)?
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
                let object = to_object(agent, vm.stack.pop().unwrap())?;
                Self::execute_simple_object_binding(agent, vm, executable, object, env)?
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
                let argument_count = instr.args[0].unwrap();
                let last_item = vm.stack.len() - argument_count as usize;
                let mut length = 0;
                for ele in vm.stack[last_item..].iter_mut() {
                    if !ele.is_string() {
                        *ele = to_string(agent, *ele)?.into_value();
                    }
                    let string = String::try_from(*ele).unwrap();
                    length += string.len(agent);
                }
                let mut result_string = std::string::String::with_capacity(length);
                for ele in vm.stack[last_item..].iter() {
                    let string = String::try_from(*ele).unwrap();
                    result_string.push_str(string.as_str(agent));
                }
                vm.stack.truncate(last_item);
                vm.result = Some(String::from_string(agent, result_string).into_value());
            }
            Instruction::Delete => {
                let refer = vm.reference.take().unwrap();
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
                            ));
                        }
                        // c. Let baseObj be ? ToObject(ref.[[Base]]).
                        let base_obj = to_object(agent, base)?;
                        // d. If ref.[[ReferencedName]] is not a property key, then
                        // TODO: Is this relevant?
                        // i. Set ref.[[ReferencedName]] to ? ToPropertyKey(ref.[[ReferencedName]]).
                        // e. Let deleteStatus be ? baseObj.[[Delete]](ref.[[ReferencedName]]).
                        let delete_status =
                            base_obj.internal_delete(agent, refer.referenced_name)?;
                        // f. If deleteStatus is false and ref.[[Strict]] is true, throw a TypeError exception.
                        if !delete_status && refer.strict {
                            return Err(agent.throw_exception_with_static_message(
                                ExceptionType::TypeError,
                                "Cannot delete property",
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
                        vm.result = Some(base.delete_binding(agent, referenced_name)?.into());
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
                let object = to_object(agent, vm.result.take().unwrap()).unwrap();
                vm.iterator_stack
                    .push(VmIterator::ObjectProperties(ObjectPropertiesIterator::new(
                        object,
                    )))
            }
            Instruction::GetIteratorSync => {
                let expr_value = vm.result.take().unwrap();
                vm.iterator_stack
                    .push(VmIterator::from_value(agent, expr_value)?);
            }
            Instruction::GetIteratorAsync => {
                todo!();
            }
            Instruction::IteratorStepValue => {
                let result = vm.iterator_stack.last_mut().unwrap().step_value(agent);
                if let Ok(result) = result {
                    vm.result = result;
                    if result.is_none() {
                        // Iterator finished: Jump to escape iterator loop.
                        vm.iterator_stack.pop().unwrap();
                        vm.ip = instr.args[0].unwrap() as usize;
                    }
                } else {
                    vm.iterator_stack.pop();
                    result?;
                }
            }
            Instruction::IteratorStepValueOrUndefined => {
                let iterator = vm.iterator_stack.last_mut().unwrap();
                let result = iterator.step_value(agent);
                if let Ok(result) = result {
                    vm.result = Some(result.unwrap_or(Value::Undefined));
                    if result.is_none() {
                        // We have exhausted the iterator; replace it with an empty VmIterator so
                        // further instructions aren't observable.
                        *iterator = VmIterator::SliceIterator(SendableRef::new(&[]));
                    }
                } else {
                    vm.iterator_stack.pop();
                    result?;
                }
            }
            Instruction::IteratorRestIntoArray => {
                let mut iterator = vm.iterator_stack.pop().unwrap();
                let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                let array = array_create(agent, 0, capacity, None)?;

                let mut idx: u32 = 0;
                while let Some(value) = iterator.step_value(agent)? {
                    let key = PropertyKey::Integer(idx.into());
                    create_data_property(agent, array, key, value).unwrap();
                    idx += 1;
                }
                vm.result = Some(array.into_value());
            }
            Instruction::IteratorClose => {
                let iterator = vm.iterator_stack.pop().unwrap();
                if let VmIterator::GenericIterator(iterator_record) = iterator {
                    iterator_close(
                        agent,
                        &iterator_record,
                        Ok(vm.result.take().unwrap_or(Value::Undefined)),
                    )?;
                }
            }
            Instruction::Yield => return Ok(ContinuationKind::Yield),
            Instruction::CreateUnmappedArgumentsObject => {
                let Some(VmIterator::SliceIterator(slice)) = vm.iterator_stack.last() else {
                    unreachable!()
                };
                vm.result = Some(create_unmapped_arguments_object(agent, slice.get()).into_value());
            }
            other => todo!("{other:?}"),
        }

        Ok(ContinuationKind::Normal)
    }

    fn get_call_args(&mut self, instr: &Instr) -> Vec<Value> {
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

    fn execute_simple_array_binding(
        agent: &mut Agent,
        vm: &mut Vm,
        executable: Executable,
        mut iterator: VmIterator,
        environment: Option<EnvironmentIndex>,
    ) -> JsResult<()> {
        let mut iterator_is_done = false;

        loop {
            let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
            let mut break_after_bind = false;

            let value = match instr.kind {
                Instruction::BindingPatternBind
                | Instruction::BindingPatternGetValue
                | Instruction::BindingPatternSkip => {
                    let result = iterator.step_value(agent)?;
                    iterator_is_done = result.is_none();

                    if instr.kind == Instruction::BindingPatternSkip {
                        continue;
                    }
                    result.unwrap_or(Value::Undefined)
                }
                Instruction::BindingPatternBindRest | Instruction::BindingPatternGetRestValue => {
                    break_after_bind = true;
                    if iterator_is_done {
                        array_create(agent, 0, 0, None).unwrap().into_value()
                    } else {
                        let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                        let rest = array_create(agent, 0, capacity, None).unwrap();
                        let mut idx = 0u32;
                        while let Some(result) = iterator.step_value(agent)? {
                            create_data_property_or_throw(
                                agent,
                                rest,
                                PropertyKey::from(idx),
                                result,
                            )
                            .unwrap();
                            idx += 1;
                        }

                        iterator_is_done = true;
                        rest.into_value()
                    }
                }
                Instruction::FinishBindingPattern => break,
                _ => unreachable!(),
            };

            match instr.kind {
                Instruction::BindingPatternBind | Instruction::BindingPatternBindRest => {
                    let binding_id =
                        executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                    let lhs = resolve_binding(agent, binding_id, environment)?;
                    if environment.is_none() {
                        put_value(agent, &lhs, value)?;
                    } else {
                        initialize_referenced_binding(agent, lhs, value)?;
                    }
                }
                Instruction::BindingPatternGetValue | Instruction::BindingPatternGetRestValue => {
                    Self::execute_nested_simple_binding(agent, vm, executable, value, environment)?;
                }
                _ => unreachable!(),
            }

            if break_after_bind {
                break;
            }
        }

        // 8.6.2 Runtime Semantics: BindingInitialization
        // BindingPattern : ArrayBindingPattern
        // 3. If iteratorRecord.[[Done]] is false, return ? IteratorClose(iteratorRecord, result).
        // NOTE: `result` here seems to be UNUSED, which isn't a Value. This seems to be a spec bug.
        if !iterator_is_done {
            if let VmIterator::GenericIterator(iterator_record) = iterator {
                iterator_close(agent, &iterator_record, Ok(Value::Undefined))?;
            }
        }

        Ok(())
    }

    fn execute_simple_object_binding(
        agent: &mut Agent,
        vm: &mut Vm,
        executable: Executable,
        object: Object,
        environment: Option<EnvironmentIndex>,
    ) -> JsResult<()> {
        let mut excluded_names = AHashSet::new();

        loop {
            let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
            match instr.kind {
                Instruction::BindingPatternBind | Instruction::BindingPatternBindNamed => {
                    let binding_id =
                        executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                    let property_key = if instr.kind == Instruction::BindingPatternBind {
                        binding_id.into()
                    } else {
                        let key_value =
                            executable.fetch_constant(agent, instr.args[1].unwrap() as usize);
                        PropertyKey::try_from(key_value).unwrap()
                    };
                    excluded_names.insert(property_key);

                    let lhs = resolve_binding(agent, binding_id, environment)?;
                    let v = get(agent, object, property_key)?;
                    if environment.is_none() {
                        put_value(agent, &lhs, v)?;
                    } else {
                        initialize_referenced_binding(agent, lhs, v)?;
                    }
                }
                Instruction::BindingPatternGetValueNamed => {
                    let property_key = PropertyKey::from_value(
                        agent,
                        executable.fetch_constant(agent, instr.args[0].unwrap() as usize),
                    )
                    .unwrap();
                    excluded_names.insert(property_key);
                    let v = get(agent, object, property_key)?;
                    Self::execute_nested_simple_binding(agent, vm, executable, v, environment)?;
                }
                Instruction::BindingPatternBindRest => {
                    // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                    let binding_id =
                        executable.fetch_identifier(agent, instr.args[0].unwrap() as usize);
                    let lhs = resolve_binding(agent, binding_id, environment)?;
                    // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                    // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                    let rest_obj =
                        copy_data_properties_into_object(agent, object, &excluded_names)?
                            .into_value();
                    // 4. If environment is undefined, return ? PutValue(lhs, restObj).
                    // 5. Return ? InitializeReferencedBinding(lhs, restObj).
                    if environment.is_none() {
                        put_value(agent, &lhs, rest_obj)?;
                    } else {
                        initialize_referenced_binding(agent, lhs, rest_obj)?;
                    }
                    break;
                }
                Instruction::FinishBindingPattern => break,
                _ => unreachable!(),
            }
        }
        Ok(())
    }

    fn execute_nested_simple_binding(
        agent: &mut Agent,
        vm: &mut Vm,
        executable: Executable,
        value: Value,
        environment: Option<EnvironmentIndex>,
    ) -> JsResult<()> {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        match instr.kind {
            Instruction::BeginSimpleArrayBindingPattern => {
                let new_iterator = VmIterator::from_value(agent, value)?;
                Vm::execute_simple_array_binding(agent, vm, executable, new_iterator, environment)
            }
            Instruction::BeginSimpleObjectBindingPattern => {
                let object = to_object(agent, value)?;
                Vm::execute_simple_object_binding(agent, vm, executable, object, environment)
            }
            _ => unreachable!(),
        }
    }
}

/// ### [13.15.3 ApplyStringOrNumericBinaryOperator ( lval, opText, rval )](https://tc39.es/ecma262/#sec-applystringornumericbinaryoperator)
///
/// The abstract operation ApplyStringOrNumericBinaryOperator takes
/// arguments lval (an ECMAScript language value), opText (**, *, /, %, +,
/// -, <<, >>, >>>, &, ^, or |), and rval (an ECMAScript language value) and
/// returns either a normal completion containing either a String, a BigInt,
/// or a Number, or a throw completion.
#[inline]
fn apply_string_or_numeric_binary_operator(
    agent: &mut Agent,
    lval: Value,
    op_text: BinaryOperator,
    rval: Value,
) -> JsResult<Value> {
    let lnum: Numeric;
    let rnum: Numeric;
    // 1. If opText is +, then
    if op_text == BinaryOperator::Addition {
        // a. Let lprim be ? ToPrimitive(lval).
        let lprim = to_primitive(agent, lval, None)?;

        // b. Let rprim be ? ToPrimitive(rval).
        let rprim = to_primitive(agent, rval, None)?;

        // c. If lprim is a String or rprim is a String, then
        if lprim.is_string() || rprim.is_string() {
            // i. Let lstr be ? ToString(lprim).
            let lstr = to_string(agent, lprim)?;

            // ii. Let rstr be ? ToString(rprim).
            let rstr = to_string(agent, rprim)?;

            // iii. Return the string-concatenation of lstr and rstr.
            return Ok(String::concat(agent, [lstr, rstr]).into_value());
        }

        // d. Set lval to lprim.
        // e. Set rval to rprim.
        // 2. NOTE: At this point, it must be a numeric operation.
        // 3. Let lnum be ? ToNumeric(lval).
        lnum = to_numeric(agent, lprim)?;
        // 4. Let rnum be ? ToNumeric(rval).
        rnum = to_numeric(agent, rprim)?;
    } else {
        // 3. Let lnum be ? ToNumeric(lval).
        lnum = to_numeric(agent, lval)?;
        // 4. Let rnum be ? ToNumeric(rval).
        rnum = to_numeric(agent, rval)?;
    }

    // 6. If lnum is a BigInt, then
    if let (Ok(lnum), Ok(rnum)) = (BigInt::try_from(lnum), BigInt::try_from(rnum)) {
        Ok(match op_text {
            // a. If opText is **, return ? BigInt::exponentiate(lnum, rnum).
            BinaryOperator::Exponential => {
                BigInt::exponentiate(agent, lnum, rnum).map(|bigint| bigint.into_value())?
            }
            // b. If opText is /, return ? BigInt::divide(lnum, rnum).
            BinaryOperator::Division => {
                BigInt::divide(agent, lnum, rnum).map(|bigint| bigint.into_value())?
            }
            // c. If opText is %, return ? BigInt::remainder(lnum, rnum).
            BinaryOperator::Remainder => {
                BigInt::remainder(agent, lnum, rnum).map(|bigint| bigint.into_value())?
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
            BinaryOperator::Multiplication => Number::multiply(agent, lnum, rnum).into_value(),
            // /	Number	Number::divide
            BinaryOperator::Division => Number::divide(agent, lnum, rnum).into_value(),
            // %	Number	Number::remainder
            BinaryOperator::Remainder => Number::remainder(agent, lnum, rnum).into_value(),
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
            BinaryOperator::BitwiseOR => Number::bitwise_or(agent, lnum, rnum)?.into(),
            // ^	Number	Number::bitwiseXOR
            BinaryOperator::BitwiseXOR => Number::bitwise_xor(agent, lnum, rnum)?.into(),
            // &	Number	Number::bitwiseAND
            BinaryOperator::BitwiseAnd => Number::bitwise_and(agent, lnum, rnum)?.into(),
            _ => unreachable!(),
        })
    } else {
        // 5. If Type(lnum) is not Type(rnum), throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "The left and right-hand sides do not have the same type.",
        ))
    }
}

/// ### [13.5.3 The typeof operator](https://tc39.es/ecma262/#sec-typeof-operator)
#[inline]
fn typeof_operator(_: &mut Agent, val: Value) -> String {
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
        Value::RegExp(_) |
        Value::Arguments(_) |
        Value::FinalizationRegistry(_) |
        Value::Map(_) |
        Value::Promise(_) |
        Value::Set(_) |
        Value::AsyncFromSyncIterator |
        Value::AsyncIterator |
        Value::Iterator |
        Value::ArrayIterator(_) |
        Value::SetIterator(_) |
        Value::MapIterator(_) |
        Value::Generator(_) |
        Value::Module(_) |
        Value::EmbedderObject(_) => BUILTIN_STRING_MEMORY.object,
        #[cfg(feature = "weak-refs")]
        Value::WeakMap(_) |
        Value::WeakRef(_) |
        Value::WeakSet(_)  => BUILTIN_STRING_MEMORY.object,
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
pub(crate) fn instanceof_operator(
    agent: &mut Agent,
    value: impl IntoValue,
    target: impl IntoValue,
) -> JsResult<bool> {
    // 1. If target is not an Object, throw a TypeError exception.
    let Ok(target) = Object::try_from(target.into_value()) else {
        let error_message = format!(
            "Invalid instanceof target {}.",
            target.into_value().string_repr(agent).as_str(agent)
        );
        return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
    };
    // 2. Let instOfHandler be ? GetMethod(target, @@hasInstance).
    let inst_of_handler = get_method(
        agent,
        target.into_value(),
        WellKnownSymbolIndexes::HasInstance.into(),
    )?;
    // 3. If instOfHandler is not undefined, then
    if let Some(inst_of_handler) = inst_of_handler {
        // a. Return ToBoolean(? Call(instOfHandler, target, « V »)).
        let result = call_function(
            agent,
            inst_of_handler,
            target.into_value(),
            Some(ArgumentsList(&[value.into_value()])),
        )?;
        Ok(to_boolean(agent, result))
    } else {
        // 4. If IsCallable(target) is false, throw a TypeError exception.
        let Some(target) = is_callable(target) else {
            let error_message = format!(
                "Invalid instanceof target {} is not a function.",
                target.into_value().string_repr(agent).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
        };
        // 5. Return ? OrdinaryHasInstance(target, V).
        Ok(ordinary_has_instance(agent, target, value)?)
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
            exception,
            reference,
        } = self;
        stack.as_slice().mark_values(queues);
        reference_stack.as_slice().mark_values(queues);
        iterator_stack.as_slice().mark_values(queues);
        exception_jump_target_stack.as_slice().mark_values(queues);
        result.mark_values(queues);
        exception.mark_values(queues);
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
            exception,
            reference,
        } = self;
        stack.as_mut_slice().sweep_values(compactions);
        reference_stack.as_mut_slice().sweep_values(compactions);
        iterator_stack.as_mut_slice().sweep_values(compactions);
        exception_jump_target_stack
            .as_mut_slice()
            .sweep_values(compactions);
        result.sweep_values(compactions);
        exception.sweep_values(compactions);
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
