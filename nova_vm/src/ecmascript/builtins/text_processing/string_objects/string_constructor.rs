// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::SmallString;
use crate::ecmascript::abstract_operations::testing_and_comparison::is_integral_number;
use crate::ecmascript::abstract_operations::type_conversion::to_number;
use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::abstract_operations::type_conversion::to_uint16_number;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::builtins::ordinary::get_prototype_from_constructor;
use crate::ecmascript::builtins::ordinary::ordinary_object_create_with_intrinsics;
use crate::ecmascript::builtins::primitive_objects::PrimitiveObject;
use crate::ecmascript::builtins::primitive_objects::PrimitiveObjectData;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Number;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::Scopable;
use crate::heap::IntrinsicConstructorIndexes;

pub struct StringConstructor;

impl Builtin for StringConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.String;
}
impl BuiltinIntrinsicConstructor for StringConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::String;
}

struct StringFromCharCode;
impl Builtin for StringFromCharCode {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringConstructor::from_char_code);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromCharCode;
}
struct StringFromCodePoint;
impl Builtin for StringFromCodePoint {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringConstructor::from_code_point);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromCodePoint;
}
struct StringRaw;
impl Builtin for StringRaw {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(StringConstructor::raw);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.raw;
}
impl StringConstructor {
    /// ### [22.1.1.1 String ( value )](https://tc39.es/ecma262/#sec-string-constructor-string-value)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let value = arguments.get(0).bind(nogc);
        let new_target = new_target.map(|n| n.bind(nogc));

        // 1. If value is not present, then
        let (s, new_target) = if arguments.is_empty() {
            // a. Let s be the empty String.
            (String::EMPTY_STRING, new_target)
        } else {
            // 2. Else,
            // a. If NewTarget is undefined and value is a Symbol, return SymbolDescriptiveString(value).
            if new_target.is_none() {
                if let Value::Symbol(value) = value {
                    return Ok(value
                        .unbind()
                        .descriptive_string(agent, gc.into_nogc())
                        .into_value());
                }
            }
            // b. Let s be ? ToString(value).
            if let Ok(s) = String::try_from(value) {
                (s, new_target)
            } else {
                let new_target = new_target.map(|n| n.scope(agent, gc.nogc()));
                let s = to_string(agent, value.unbind(), gc.reborrow())?.unbind();
                let nogc = gc.nogc();
                (s.bind(nogc), new_target.map(|n| n.get(agent).bind(nogc)))
            }
        };
        // 3. If NewTarget is undefined, return s.
        let Some(new_target) = new_target else {
            return Ok(s.into_value().unbind());
        };
        // 4. Return StringCreate(s, ? GetPrototypeFromConstructor(NewTarget, "%String.prototype%")).
        let value = s.scope(agent, gc.nogc());
        let prototype = get_prototype_from_constructor(
            agent,
            Function::try_from(new_target.unbind()).unwrap(),
            ProtoIntrinsics::String,
            gc.reborrow(),
        )?
        .map(|p| p.unbind())
        .map(|p| p.bind(gc.nogc()));
        // StringCreate: Returns a String exotic object.
        // 1. Let S be MakeBasicObject(« [[Prototype]], [[Extensible]], [[StringData]] »).
        let s = PrimitiveObject::try_from(ordinary_object_create_with_intrinsics(
            agent,
            Some(ProtoIntrinsics::String),
            prototype,
            gc.nogc(),
        ))
        .unwrap();

        // 2. Set S.[[Prototype]] to prototype.
        // 3. Set S.[[StringData]] to value.
        let value = value.get(agent).bind(gc.nogc());
        agent[s].data = match value {
            String::String(data) => PrimitiveObjectData::String(data.unbind()),
            String::SmallString(data) => PrimitiveObjectData::SmallString(data),
        };
        // 4. Set S.[[GetOwnProperty]] as specified in 10.4.3.1.
        // 5. Set S.[[DefineOwnProperty]] as specified in 10.4.3.2.
        // 6. Set S.[[OwnPropertyKeys]] as specified in 10.4.3.3.
        // 7. Let length be the length of value.
        // 8. Perform ! DefinePropertyOrThrow(S, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }).
        // 9. Return S.
        Ok(s.into_value().unbind())
    }

    /// ### [22.1.2.1 String.fromCharCode ( ...`codeUnits` )](https://262.ecma-international.org/15.0/index.html#sec-string.fromcharcode)
    ///
    /// This function may be called with any number of arguments which form
    /// the rest parameter `codeUnits`.
    fn from_char_code<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        code_units: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. Let result be the empty String.
        // 2. For each element next of codeUnits, do
        //   a. Let nextCU be the code unit whose numeric value is ℝ(? ToUint16(next)).
        //   b. Set result to the string-concatenation of result and nextCU.
        // 3. Return result.

        if code_units.is_empty() {
            return Ok(String::EMPTY_STRING.into_value());
        }

        // fast path: only a single valid code unit
        if code_units.len() == 1 {
            let cu = code_units.get(0).to_uint16(agent, gc.reborrow())?;
            if let Some(cu) = char::from_u32(cu as u32) {
                return Ok(SmallString::from(cu).into());
            }
        }

        let buf = if code_units.iter().all(|cu| cu.is_number()) {
            code_units
                .iter()
                .map(|&cu| to_uint16_number(agent, Number::try_from(cu).unwrap()))
                .collect::<Vec<_>>()
        } else {
            let scoped_code_units = code_units
                .iter()
                .map(|cu| cu.scope(agent, gc.nogc()))
                .collect::<Vec<_>>();
            scoped_code_units
                .iter()
                .map(|cu| {
                    let next = cu.get(agent);
                    next.to_uint16(agent, gc.reborrow())
                })
                .collect::<JsResult<Vec<_>>>()?
        };

        let result = std::string::String::from_utf16_lossy(&buf);

        Ok(String::from_string(agent, result, gc.into_nogc()).into())
    }

    /// ### [22.1.2.2 String.fromCodePoint ( ...`codePoints` )](https://tc39.es/ecma262/multipage/text-processing.html#sec-string.fromcodepoint)
    ///
    /// This function may be called with any number of arguments which form
    /// the rest parameter `codePoints`.
    fn from_code_point<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        code_points: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 3. Assert: If codePoints is empty, then result is the empty String.
        if code_points.is_empty() {
            return Ok(String::EMPTY_STRING.into_value());
        }
        // fast path: only a single valid code unit
        if code_points.len() == 1 && code_points.first().unwrap().is_integer() {
            // a. Let nextCP be ? ToNumber(next).
            // b. If IsIntegralNumber(nextCP) is false, throw a RangeError exception.
            // c. If ℝ(nextCP) < 0 or ℝ(nextCP) > 0x10FFFF, throw a RangeError exception.
            let Value::Integer(next_cp) = code_points.first().unwrap() else {
                unreachable!();
            };
            let next_cp = next_cp.into_i64();
            if !(0..=0x10FFFF).contains(&next_cp) {
                return Err(agent.throw_exception(
                    ExceptionType::RangeError,
                    format!("{:?} is not a valid code point", next_cp),
                    gc.nogc(),
                ));
            }
            // d. Set result to the string-concatenation of result and UTF16EncodeCodePoint(ℝ(nextCP)).
            if let Some(cu) = char::from_u32(next_cp as u32) {
                // 4. Return result.
                return Ok(SmallString::from(cu).into());
            }
        };
        // 1. Let result be the empty String.
        let mut result = std::string::String::with_capacity(code_points.len());
        if code_points.iter().all(|cp| cp.is_integer()) {
            // 2. For each element next of codePoints, do
            for next in code_points.iter() {
                let Value::Integer(next_cp) = next else {
                    unreachable!()
                };
                let next_cp = next_cp.into_i64();
                // c. If ℝ(nextCP) < 0 or ℝ(nextCP) > 0x10FFFF, throw a RangeError exception.
                if !(0..=0x10FFFF).contains(&next_cp) {
                    return Err(agent.throw_exception(
                        ExceptionType::RangeError,
                        format!("{:?} is not a valid code point", next_cp),
                        gc.nogc(),
                    ));
                }
                // d. Set result to the string-concatenation of result and UTF16EncodeCodePoint(ℝ(nextCP)).
                result.push(char::from_u32(next_cp as u32).unwrap());
            }
        } else {
            let code_points = code_points
                .iter()
                .map(|cp| cp.scope(agent, gc.nogc()))
                .collect::<Vec<_>>();
            // 2. For each element next of codePoints, do
            for next in code_points.into_iter() {
                // a. Let nextCP be ? ToNumber(next).
                let next_cp = to_number(agent, next.get(agent), gc.reborrow())?;
                // b. If IsIntegralNumber(nextCP) is false, throw a RangeError exception.
                if !is_integral_number(agent, next_cp) {
                    return Err(agent.throw_exception(
                        ExceptionType::RangeError,
                        format!("{:?} is not a valid code point", next_cp.to_real(agent)),
                        gc.nogc(),
                    ));
                }
                // c. If ℝ(nextCP) < 0 or ℝ(nextCP) > 0x10FFFF, throw a RangeError exception.
                let next_cp = next_cp.into_i64(agent);
                if !(0..=0x10FFFF).contains(&next_cp) {
                    return Err(agent.throw_exception(
                        ExceptionType::RangeError,
                        format!("{:?} is not a valid code point", next_cp),
                        gc.nogc(),
                    ));
                }
                // d. Set result to the string-concatenation of result and UTF16EncodeCodePoint(ℝ(nextCP)).
                result.push(char::from_u32(next_cp as u32).unwrap());
            }
        }
        // 4. Return result.
        Ok(String::from_string(agent, result, gc.into_nogc()).into())
    }

    fn raw<'gc>(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let string_prototype = intrinsics.string_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<StringConstructor>(agent, realm)
            .with_property_capacity(4)
            .with_builtin_function_property::<StringFromCharCode>()
            .with_builtin_function_property::<StringFromCodePoint>()
            .with_prototype_property(string_prototype.into_object())
            .with_builtin_function_property::<StringRaw>()
            .build();
    }
}
