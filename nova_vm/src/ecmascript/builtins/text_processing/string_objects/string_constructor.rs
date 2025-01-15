// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ordinary::get_prototype_from_constructor;
use crate::ecmascript::builtins::ordinary::ordinary_object_create_with_intrinsics;
use crate::ecmascript::builtins::primitive_objects::PrimitiveObject;
use crate::ecmascript::builtins::primitive_objects::PrimitiveObjectData;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::engine::context::GcScope;
use crate::heap::IntrinsicConstructorIndexes;
use crate::SmallString;

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
    fn constructor(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope,
    ) -> JsResult<Value> {
        // 1. If value is not present, then
        let s = if arguments.is_empty() {
            // a. Let s be the empty String.
            String::EMPTY_STRING
        } else {
            // 2. Else,
            let value = arguments.get(0);
            // a. If NewTarget is undefined and value is a Symbol, return SymbolDescriptiveString(value).
            if new_target.is_none() {
                if let Value::Symbol(value) = value {
                    return Ok(value.descriptive_string(agent, gc.nogc()).into_value());
                }
            }
            // b. Let s be ? ToString(value).
            to_string(agent, value, gc.reborrow())?
                .unbind()
                .bind(gc.nogc())
        };
        // 3. If NewTarget is undefined, return s.
        let Some(new_target) = new_target else {
            return Ok(s.into_value());
        };
        // 4. Return StringCreate(s, ? GetPrototypeFromConstructor(NewTarget, "%String.prototype%")).
        let value = s.scope(agent, gc.nogc());
        let prototype = get_prototype_from_constructor(
            agent,
            Function::try_from(new_target).unwrap(),
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
        Ok(s.into_value())
    }

    /// ### [22.1.2.1 String.fromCharCode ( ...`codeUnits` )](https://262.ecma-international.org/15.0/index.html#sec-string.fromcharcode)
    ///
    /// This function may be called with any number of arguments which form
    /// the rest parameter `codeUnits`.
    fn from_char_code(
        agent: &mut Agent,
        _this_value: Value,
        code_units: ArgumentsList,
        mut gc: GcScope,
    ) -> JsResult<Value> {
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

        let mut buf = Vec::with_capacity(code_units.len());

        for next in code_units.iter() {
            let code_unit = next.to_uint16(agent, gc.reborrow())?;
            buf.push(code_unit);
        }
        let result = std::string::String::from_utf16_lossy(&buf);

        Ok(String::from_string(agent, result, gc.nogc()).into())
    }

    /// ### [22.1.2.2 String.fromCodePoint ( ...`codePoints` ) ](https://262.ecma-international.org/15.0/index.html#sec-string.fromcodepoint)
    ///
    /// This function may be called with any number of arguments which form
    /// the rest parameter `codePoints`.
    fn from_code_point(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
        // 1. Let result be the empty String.
        // 2. For each element next of codePoints, do
        //     a. Let nextCP be ? ToNumber(next).
        //     b. If IsIntegralNumber(nextCP) is false, throw a RangeError exception.
        //     c. If ℝ(nextCP) < 0 or ℝ(nextCP) > 0x10FFFF, throw a RangeError exception.
        //     d. Set result to the string-concatenation of result and UTF16EncodeCodePoint(ℝ(nextCP)).
        // 3. Assert: If codePoints is empty, then result is the empty String.
        // 4. Return result.

        todo!()
    }

    fn raw(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _gc: GcScope,
    ) -> JsResult<Value> {
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
