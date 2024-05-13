use oxc_syntax::operator::BinaryOperator;

use crate::ecmascript::{
    abstract_operations::{
        operations_on_objects::{call, construct, create_data_property_or_throw},
        testing_and_comparison::{is_constructor, is_less_than, is_same_type, is_strictly_equal},
        type_conversion::{
            to_boolean, to_number, to_numeric, to_primitive, to_property_key, to_string,
        },
    },
    builtins::{
        array_create, ordinary::ordinary_object_create_with_intrinsics, ordinary_function_create,
        ArgumentsList, Array, OrdinaryFunctionCreateParams, ThisMode,
    },
    execution::{
        agent::{resolve_binding, ExceptionType, JsError},
        new_declarative_environment, Agent, ECMAScriptCodeEvaluationState, EnvironmentIndex,
        JsResult, ProtoIntrinsics,
    },
    types::{
        get_value, is_unresolvable_reference, put_value, Base, BigInt, Function, IntoValue, Number,
        Numeric, Object, PropertyKey, Reference, ReferencedName, String, Value,
        BUILTIN_STRING_MEMORY,
    },
};

use super::{instructions::Instr, Executable, Instruction, InstructionIter};

/// Indicates how the execution of an instruction should affect the remainder of
/// execution that contains it.
#[must_use]
enum ContinuationKind {
    Normal,
    Return,
    Yield,
    Await,
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
    exception_jump_target_stack: Vec<usize>,
    result: Option<Value>,
    exception: Option<Value>,
    reference: Option<Reference>,
}

impl Vm {
    fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            exception_jump_target_stack: Vec::new(),
            result: None,
            exception: None,
            reference: None,
        }
    }

    fn fetch_identifier(&self, exe: &Executable, index: usize) -> String {
        exe.identifiers[index]
    }

    fn fetch_constant(&self, exe: &Executable, index: usize) -> Value {
        exe.constants[index]
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute(agent: &mut Agent, executable: &Executable) -> JsResult<Option<Value>> {
        let mut vm = Vm::new();

        eprintln!();
        eprintln!("=== Executing Executable ===");
        eprintln!("Constants: {:?}", executable.constants);
        eprintln!("Identifiers: {:?}", executable.identifiers);
        eprintln!();

        eprintln!("Instructions:");
        let iter = InstructionIter::new(&executable.instructions);
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

        while let Some(instr) = executable.get_instruction(&mut vm.ip) {
            match Self::execute_instruction(agent, &mut vm, executable, &instr) {
                Ok(ContinuationKind::Normal) => {}
                Ok(ContinuationKind::Return) => return Ok(vm.result),
                Ok(ContinuationKind::Yield) => todo!(),
                Ok(ContinuationKind::Await) => todo!(),
                Err(err) => {
                    if let Some(catch_ip) = vm.exception_jump_target_stack.pop() {
                        vm.ip = catch_ip;
                        vm.exception = Some(err.value());
                    } else {
                        return Err(err);
                    }
                }
            }
        }

        Ok(vm.result)
    }

    fn execute_instruction(
        agent: &mut Agent,
        vm: &mut Vm,
        executable: &Executable,
        instr: &Instr,
    ) -> JsResult<ContinuationKind> {
        eprintln!("Executing instruction {:?}", instr.kind);
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
                create_data_property_or_throw(agent, array.into(), key, value)?
            }
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
                eprintln!("Debug: {:#?}", vm);
            }
            Instruction::ResolveBinding => {
                let identifier = vm.fetch_identifier(executable, instr.args[0].unwrap() as usize);

                let reference = resolve_binding(agent, identifier, None)?;

                vm.reference = Some(reference);
            }
            Instruction::LoadConstant => {
                let constant = vm.fetch_constant(executable, instr.args[0].unwrap() as usize);
                vm.stack.push(constant);
            }
            Instruction::Load => {
                vm.stack.push(vm.result.take().unwrap());
            }
            Instruction::Return => {
                return Ok(ContinuationKind::Return);
            }
            Instruction::Store => {
                vm.result = Some(vm.stack.pop().expect("Trying to pop from empty stack"));
            }
            Instruction::StoreConstant => {
                let constant = vm.fetch_constant(executable, instr.args[0].unwrap() as usize);
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
            Instruction::ApplyStringOrNumericBinaryOperator(op_text) => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.stack.pop().unwrap();
                vm.result = Some(apply_string_or_numeric_binary_operator(
                    agent, lval, op_text, rval,
                )?);
            }
            Instruction::ObjectSetProperty => {
                let value = vm.result.take().unwrap();
                let key = PropertyKey::try_from(vm.stack.pop().unwrap()).unwrap();
                let object = *vm.stack.last().unwrap();
                let object = Object::try_from(object).unwrap();
                create_data_property_or_throw(agent, object, key, value).unwrap()
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
                    get_value(agent, &reference)?
                } else {
                    vm.result.unwrap()
                };
                vm.result = Some(typeof_operator(agent, val).into())
            }
            Instruction::ObjectCreate => {
                let object =
                    ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object));
                vm.stack.push(object.into())
            }
            Instruction::InstantiateOrdinaryFunctionExpression => {
                let function_expression = executable
                    .function_expressions
                    .get(instr.args[0].unwrap() as usize)
                    .unwrap();
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
                    function_prototype: None,
                    source_text: function_expression.expression.span,
                    parameters_list: &function_expression.expression.params,
                    body: function_expression.expression.body.as_ref().unwrap(),
                    this_mode: ThisMode::Lexical,
                    env: lexical_environment,
                    private_env: private_environment,
                };
                let function = ordinary_function_create(agent, params).into_value();
                vm.result = Some(function);
            }
            Instruction::EvaluateCall => {
                let arg_count = instr.args[0].unwrap() as usize;
                let args = vm.stack.split_off(vm.stack.len() - arg_count);
                let reference = vm.reference.take();
                // 1. If ref is a Reference Record, then
                let this_value = if let Some(reference) = reference {
                    // a. If IsPropertyReference(ref) is true, then
                    match reference.base {
                        // i. Let thisValue be GetThisValue(ref).
                        Base::Value(value) => value,
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
                // let this_arg = vm.stack.pop();
                let func = vm.stack.pop().unwrap();
                vm.result = Some(call(agent, func, this_value, Some(ArgumentsList(&args)))?);
            }
            Instruction::EvaluateNew => {
                let arg_count = instr.args[0].unwrap() as usize;
                let args = vm.stack.split_off(vm.stack.len() - arg_count);
                let constructor = vm.stack.pop().unwrap();
                if !is_constructor(agent, constructor) {
                    return Err(
                        agent.throw_exception(ExceptionType::TypeError, "Not a constructor")
                    );
                }
                // SAFETY: Only Functions can be constructors
                let constructor = unsafe { Function::try_from(constructor).unwrap_unchecked() };
                vm.result = Some(
                    construct(agent, constructor, Some(ArgumentsList(&args)), None)
                        .map(|result| result.into_value())?,
                );
            }
            Instruction::EvaluatePropertyAccessWithExpressionKey => {
                let property_name_value = vm.result.take().unwrap();
                let base_value = vm.stack.pop().unwrap();

                let strict = true;

                let property_key = to_property_key(agent, property_name_value)?;

                vm.reference = Some(Reference {
                    base: Base::Value(base_value),
                    referenced_name: match property_key {
                        PropertyKey::SmallString(s) => ReferencedName::SmallString(s),
                        PropertyKey::String(s) => ReferencedName::String(s),
                        PropertyKey::Symbol(s) => ReferencedName::Symbol(s.into()),
                        _ => todo!("Index properties in ReferencedName"),
                    },
                    strict,
                    this_value: None,
                });
            }
            Instruction::EvaluatePropertyAccessWithIdentifierKey => {
                let property_name_string =
                    vm.fetch_identifier(executable, instr.args[0].unwrap() as usize);
                let base_value = vm.result.take().unwrap();
                let strict = true;

                vm.reference = Some(Reference {
                    base: Base::Value(base_value),
                    referenced_name: ReferencedName::from(property_name_string),
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
            Instruction::LessThan => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_less_than::<true>(agent, lval, rval)
                    .unwrap()
                    .unwrap_or_default();
                vm.result = Some(result.into());
            }
            Instruction::IsStrictlyEqual => {
                let lval = vm.stack.pop().unwrap();
                let rval = vm.result.take().unwrap();
                let result = is_strictly_equal(agent, lval, rval);
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
                // 1. Assert: IsUnresolvableReference(V) is false.
                debug_assert!(!is_unresolvable_reference(&v));
                // 2. Let base be V.[[Base]].
                let base = v.base;
                // 3. Assert: base is an Environment Record.
                let Base::Environment(base) = base else {
                    unreachable!()
                };
                let referenced_name = match v.referenced_name {
                    ReferencedName::String(data) => String::String(data),
                    ReferencedName::SmallString(data) => String::SmallString(data),
                    ReferencedName::Symbol(_) | ReferencedName::PrivateName => unreachable!(),
                };
                // 4. Return ? base.InitializeBinding(V.[[ReferencedName]], W).
                base.initialize_binding(agent, referenced_name, w).unwrap();
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
            Instruction::CreateMutableBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name = vm.fetch_identifier(executable, instr.args[0].unwrap() as usize);
                lex_env.create_mutable_binding(agent, name, false).unwrap();
            }
            Instruction::CreateImmutableBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name = vm.fetch_identifier(executable, instr.args[0].unwrap() as usize);
                lex_env.create_immutable_binding(agent, name, true).unwrap();
            }
            Instruction::CreateCatchBinding => {
                let lex_env = agent
                    .running_execution_context()
                    .ecmascript_code
                    .as_ref()
                    .unwrap()
                    .lexical_environment;
                let name = vm.fetch_identifier(executable, instr.args[0].unwrap() as usize);
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
            Instruction::PushExceptionJumpTarget => {
                let ip = instr.args[0].unwrap() as usize;
                vm.exception_jump_target_stack.push(ip);
            }
            Instruction::PopExceptionJumpTarget => {
                vm.exception_jump_target_stack.pop().unwrap();
            }
            other => todo!("{other:?}"),
        }

        Ok(ContinuationKind::Normal)
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
            let _lstr = to_string(agent, lprim)?;

            // ii. Let rstr be ? ToString(rprim).
            let _rstr = to_string(agent, rprim)?;

            // iii. Return the string-concatenation of lstr and rstr.
            todo!("Concatenate the strings.")
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

    // 5. If Type(lnum) is not Type(rnum), throw a TypeError exception.
    if !is_same_type(lnum, rnum) {
        return Err(agent.throw_exception(
            ExceptionType::TypeError,
            "The left and right-hand sides do not have the same type.",
        ));
    }

    // 6. If lnum is a BigInt, then
    if let (Ok(lnum), Ok(rnum)) = (BigInt::try_from(lnum), BigInt::try_from(rnum)) {
        match op_text {
            // a. If opText is **, return ? BigInt::exponentiate(lnum, rnum).
            BinaryOperator::Exponential => {
                return BigInt::exponentiate(agent, lnum, rnum).map(|bigint| bigint.into())
            }
            // b. If opText is /, return ? BigInt::divide(lnum, rnum).
            BinaryOperator::Division => todo!(),
            // c. If opText is %, return ? BigInt::remainder(lnum, rnum).
            BinaryOperator::Remainder => todo!(),
            // d. If opText is >>>, return ? BigInt::unsignedRightShift(lnum, rnum).
            BinaryOperator::ShiftRightZeroFill => todo!(),
            _ => unreachable!(),
        }
    }

    // 7. Let operation be the abstract operation associated with opText and
    // Type(lnum) in the following table:
    // 8. Return operation(lnum, rnum).
    // NOTE: We do step 8. explicitly in branch.
    Ok(match op_text {
        // opText	Type(lnum)	operation
        // **	Number	Number::exponentiate
        BinaryOperator::Exponential if lnum.is_number() => Number::exponentiate(
            agent,
            Number::try_from(lnum).unwrap(),
            rnum.try_into().unwrap(),
        )
        .into(),
        // *	Number	Number::multiply
        // *	BigInt	BigInt::multiply
        // /	Number	Number::divide
        // %	Number	Number::remainder
        // +	Number	Number::add
        BinaryOperator::Addition if lnum.is_number() => {
            Number::add(agent, lnum.try_into().unwrap(), rnum.try_into().unwrap()).into()
        }
        // +	BigInt	BigInt::add
        // -	Number	Number::subtract
        BinaryOperator::Subtraction if lnum.is_number() => {
            Number::subtract(agent, lnum.try_into().unwrap(), rnum.try_into().unwrap()).into()
        }
        // -	BigInt	BigInt::subtract
        // <<	Number	Number::leftShift
        // <<	BigInt	BigInt::leftShift
        // >>	Number	Number::signedRightShift
        // >>	BigInt	BigInt::signedRightShift
        // >>>	Number	Number::unsignedRightShift
        // &	Number	Number::bitwiseAND
        // &	BigInt	BigInt::bitwiseAND
        // ^	Number	Number::bitwiseXOR
        // ^	BigInt	BigInt::bitwiseXOR
        // |	Number	Number::bitwiseOR
        // |	BigInt	BigInt::bitwiseOR
        _ => todo!(),
    })
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
        Value::Float(_) => BUILTIN_STRING_MEMORY.number,
        // 10. If val is a BigInt, return "bigint".
        Value::BigInt(_) |
        Value::SmallBigInt(_) => BUILTIN_STRING_MEMORY.bigint,
        // 5. If val is null, return "object".
        Value::Null |
        // 11. Assert: val is an Object.
        // 12. NOTE: This step is replaced in section B.3.6.3.
        Value::Object(_)  |
        Value::Array(_)  |
        Value::ArrayBuffer(_)  |
        Value::Date(_)  |
        Value::Error(_)  |
        // 14. Return "object".
        Value::PrimitiveObject(_) |
        Value::RegExp(_) |
        Value::Arguments |
        Value::DataView(_) |
        Value::FinalizationRegistry(_) |
        Value::Map(_) |
        Value::Promise(_) |
        Value::Set(_) |
        Value::SharedArrayBuffer(_) |
        Value::WeakMap(_) |
        Value::WeakRef(_) |
        Value::WeakSet(_) |
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
        Value::AsyncFromSyncIterator |
        Value::AsyncIterator |
        Value::Iterator |
        Value::Module(_) |
        Value::EmbedderObject(_) => BUILTIN_STRING_MEMORY.object,
        // 13. If val has a [[Call]] internal slot, return "function".
        Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_) |
        Value::BuiltinGeneratorFunction |
        Value::BuiltinConstructorFunction |
        Value::BuiltinPromiseResolveFunction |
        Value::BuiltinPromiseRejectFunction |
        Value::BuiltinPromiseCollectorFunction |
        Value::BuiltinProxyRevokerFunction |
        Value::ECMAScriptAsyncFunction |
        Value::ECMAScriptAsyncGeneratorFunction |
        Value::ECMAScriptConstructorFunction |
        Value::ECMAScriptGeneratorFunction => BUILTIN_STRING_MEMORY.function,
        // TODO: Check [[Call]] slot for Proxy
        Value::Proxy(_) => todo!(),
    }
}
