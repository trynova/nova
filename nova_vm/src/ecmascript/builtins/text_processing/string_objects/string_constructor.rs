// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use wtf8::{CodePoint, Wtf8Buf};

use crate::{
    SmallString,
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, length_of_array_like},
            type_conversion::{to_number, to_object, to_string, to_uint16_number},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            ordinary::{get_prototype_from_constructor, ordinary_object_create_with_intrinsics},
            primitive_objects::{PrimitiveObject, PrimitiveObjectData},
        },
        execution::{Agent, JsResult, ProtoIntrinsics, Realm, agent::ExceptionType},
        types::{BUILTIN_STRING_MEMORY, Function, Number, Object, PropertyKey, String, Value},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{ArenaAccess, IntrinsicConstructorIndexes},
};

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
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let value = arguments.get(0).bind(nogc);
        let new_target = new_target.bind(nogc);

        // 1. If value is not present, then
        let (s, new_target) = if arguments.is_empty() {
            // a. Let s be the empty String.
            (String::EMPTY_STRING, new_target)
        } else
        // 2. Else,
        // a. If NewTarget is undefined and value is a Symbol,
        if new_target.is_none()
            && let Value::Symbol(value) = value
        {
            // return SymbolDescriptiveString(value).
            return Ok(value
                .unbind()
                .descriptive_string(agent, gc.into_nogc())
                .into());
        } else {
            // b. Let s be ? ToString(value).
            if let Ok(s) = String::try_from(value) {
                (s, new_target)
            } else {
                let new_target = new_target.map(|n| n.scope(agent, gc.nogc()));
                let s = to_string(agent, value.unbind(), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                (s, new_target.map(|n| n.get(agent).bind(gc.nogc())))
            }
        };
        // 3. If NewTarget is undefined, return s.
        let Some(new_target) = new_target else {
            return Ok(s.unbind().into());
        };
        // 4. Return StringCreate(s, ? GetPrototypeFromConstructor(NewTarget, "%String.prototype%")).
        let value = s.scope(agent, gc.nogc());
        let prototype = get_prototype_from_constructor(
            agent,
            Function::try_from(new_target.unbind()).unwrap(),
            ProtoIntrinsics::String,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // StringCreate: Returns a String exotic object.
        // 1. Let S be MakeBasicObject(« [[Prototype]], [[Extensible]], [[StringData]] »).
        let s = PrimitiveObject::try_from(ordinary_object_create_with_intrinsics(
            agent,
            ProtoIntrinsics::String,
            prototype,
            gc.nogc(),
        ))
        .unwrap();

        // 2. Set S.[[Prototype]] to prototype.
        // 3. Set S.[[StringData]] to value.
        let value = value.get(agent).bind(gc.nogc());
        s.get_mut(agent).data = match value {
            String::String(data) => PrimitiveObjectData::String(data.unbind()),
            String::SmallString(data) => PrimitiveObjectData::SmallString(data),
        };
        // 4. Set S.[[GetOwnProperty]] as specified in 10.4.3.1.
        // 5. Set S.[[DefineOwnProperty]] as specified in 10.4.3.2.
        // 6. Set S.[[OwnPropertyKeys]] as specified in 10.4.3.3.
        // 7. Let length be the length of value.
        // 8. Perform ! DefinePropertyOrThrow(S, "length", PropertyDescriptor { [[Value]]: 𝔽(length), [[Writable]]: false, [[Enumerable]]: false, [[Configurable]]: false }).
        // 9. Return S.
        Ok(s.unbind().into())
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
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let result be the empty String.
        // 2. For each element next of codeUnits, do
        //   a. Let nextCU be the code unit whose numeric value is ℝ(? ToUint16(next)).
        //   b. Set result to the string-concatenation of result and nextCU.
        // 3. Return result.

        if code_units.is_empty() {
            return Ok(String::EMPTY_STRING.into());
        }

        // fast path: only a single valid code unit
        if code_units.len() == 1 {
            let cu = code_units.get(0).to_uint16(agent, gc.reborrow()).unbind()?;
            // SAFETY: number within 0..0xFFFF.
            let cu = unsafe { CodePoint::from_u32_unchecked(cu as u32) };
            return Ok(SmallString::from_code_point(cu).into());
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
                    next.to_uint16(agent, gc.reborrow()).unbind()
                })
                .collect::<JsResult<Vec<_>>>()?
                .bind(gc.nogc())
        };

        let result = Wtf8Buf::from_ill_formed_utf16(&buf);

        Ok(String::from_wtf8_buf(agent, result, gc.into_nogc()).into())
    }

    /// ### [22.1.2.2 String.fromCodePoint ( ...`codePoints` )](https://tc39.es/ecma262/#sec-string.fromcodepoint)
    ///
    /// This function may be called with any number of arguments which form
    /// the rest parameter `codePoints`.
    fn from_code_point<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        code_points: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 3. Assert: If codePoints is empty, then result is the empty String.
        if code_points.is_empty() {
            return Ok(String::EMPTY_STRING.into());
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
                    format!("{next_cp:?} is not a valid code point"),
                    gc.into_nogc(),
                ));
            }
            // d. Set result to the string-concatenation of result and UTF16EncodeCodePoint(ℝ(nextCP)).
            if let Some(cu) = char::from_u32(next_cp as u32) {
                // 4. Return result.
                return Ok(SmallString::from(cu).into());
            }
        };
        fn handle_code_point<'a>(
            agent: &mut Agent,
            cp: i64,
            gc: NoGcScope<'a, '_>,
        ) -> JsResult<'a, CodePoint> {
            // c. If ℝ(nextCP) < 0 or ℝ(nextCP) > 0x10FFFF, throw a RangeError exception.
            if (0..=0x10FFFF).contains(&cp) {
                // SAFETY: checked to be less or equal to 0x10FFFF.
                let cp = unsafe { CodePoint::from_u32_unchecked(cp as u32) };
                return Ok(cp);
            }
            Err(agent.throw_exception(
                ExceptionType::RangeError,
                format!("{cp:#08X} is not a valid code point"),
                gc,
            ))
        }
        // 1. Let result be the empty String.
        let mut result = Wtf8Buf::with_capacity(code_points.len() * 3);
        if code_points.iter().all(|cp| cp.is_number()) {
            // 2. For each element next of codePoints, do
            for next in code_points.iter() {
                // SAFETY: checked in the iter().all().
                let next = unsafe { Number::try_from(*next).unwrap_unchecked() };
                let Number::Integer(next) = next else {
                    if next == Number::neg_zero() {
                        // Special case: -0 is an acceptable value here.
                        result.push_char('\0');
                        continue;
                    }
                    return Err(agent.throw_exception(
                        ExceptionType::RangeError,
                        format!("{} is not a valid code point", next.to_real(agent)),
                        gc.into_nogc(),
                    ));
                };
                let next_cp = next.into_i64();
                result.push(handle_code_point(agent, next_cp, gc.nogc()).unbind()?);
            }
        } else {
            let code_points = code_points
                .iter()
                .map(|cp| cp.scope(agent, gc.nogc()))
                .collect::<Vec<_>>();
            // 2. For each element next of codePoints, do
            for next in code_points.into_iter() {
                // a. Let nextCP be ? ToNumber(next).
                let next_cp = to_number(agent, next.get(agent), gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());
                // b. If IsIntegralNumber(nextCP) is false, throw a RangeError exception.
                let Number::Integer(next_cp) = next_cp else {
                    if next_cp == Number::neg_zero() {
                        // Special case: -0 is an acceptable value here.
                        result.push_char('\0');
                        continue;
                    }
                    return Err(agent.throw_exception(
                        ExceptionType::RangeError,
                        format!("{} is not a valid code point", next_cp.to_real(agent)),
                        gc.into_nogc(),
                    ));
                };
                let next_cp = next_cp.into_i64();
                result.push(handle_code_point(agent, next_cp, gc.nogc()).unbind()?);
            }
        }
        // 4. Return result.
        Ok(String::from_wtf8_buf(agent, result, gc.into_nogc()).into())
    }

    /// ### [22.1.2.4 String.raw ( template, ...substitutions )](https://tc39.es/ecma262/#sec-string.raw)
    ///
    /// This function may be called with a variable number of arguments. The
    /// first argument is template and the remainder of the arguments form the
    /// List substitutions.
    ///
    /// > NOTE: This function is intended for use as a tag function of a Tagged
    /// > Template (13.3.11). When called as such, the first argument will be a
    /// > well formed template object and the rest parameter will contain the
    /// > substitution values.
    fn raw<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        mut arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let template = arguments.get(0).bind(gc.nogc());
        let mut substitutions = if !arguments.is_empty() {
            ArgumentsList::from_mut_slice(&mut arguments.as_mut_slice()[1..])
        } else {
            ArgumentsList::from_mut_slice(&mut [])
        };

        // 1. Let substitutionCount be the number of elements in substitutions.
        let substitution_count = substitutions.len() as i64;

        // 2. Let cooked be ? ToObject(template).
        let cooked = to_object(agent, template, gc.nogc())
            .unbind()?
            .scope(agent, gc.nogc());

        substitutions.with_scoped(
            agent,
            |agent, substitutions, mut gc| {
                // 3. Let literals be ? ToObject(? Get(cooked, "raw")).
                let literals = get(
                    agent,
                    cooked.get(agent),
                    BUILTIN_STRING_MEMORY.raw.to_property_key(),
                    gc.reborrow(),
                )
                .unbind()?;
                let literals = to_object(agent, literals, gc.nogc())
                    .unbind()?
                    .scope(agent, gc.nogc());

                // 4. Let literalCount be ? LengthOfArrayLike(literals).
                let literal_count =
                    length_of_array_like(agent, literals.get(agent), gc.reborrow()).unbind()?;

                // 5. If literalCount ≤ 0, return the empty String.
                if literal_count <= 0 {
                    return Ok(String::EMPTY_STRING.into());
                }

                // 6. Let R be the empty String.
                let mut r = Wtf8Buf::with_capacity(literal_count as usize);

                // 7. Let nextIndex be 0.
                // 8. Repeat,
                for next_index in 0..literal_count {
                    // a. Let nextLiteralVal be ? Get(literals, ! ToString(𝔽(nextIndex))).
                    let next_literal_val = get(
                        agent,
                        literals.get(agent),
                        PropertyKey::try_from(next_index).unwrap(),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .scope(agent, gc.nogc());

                    // b. Let nextLiteral be ? ToString(nextLiteralVal).
                    let next_literal = to_string(agent, next_literal_val.get(agent), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());

                    // c. Set R to the string-concatenation of R and nextLiteral.
                    r.push_wtf8(next_literal.as_wtf8(agent));

                    // d. If nextIndex + 1 = literalCount, return R.
                    // Note: this branch is now below the loop.

                    // e. If nextIndex < substitutionCount, then
                    if next_index + 1 < literal_count && next_index < substitution_count {
                        // i. Let nextSubVal be substitutions[nextIndex].
                        let next_sub_val = substitutions.get(agent, next_index as u32, gc.nogc());

                        // ii. Let nextSub be ? ToString(nextSubVal).
                        let next_sub = to_string(agent, next_sub_val.unbind(), gc.reborrow())
                            .unbind()?
                            .bind(gc.nogc());

                        // iii. Set R to the string-concatenation of R and nextSub.
                        r.push_wtf8(next_sub.as_wtf8(agent));
                    }

                    // f. Set nextIndex to nextIndex + 1.
                }
                Ok(String::from_wtf8_buf(agent, r, gc.into_nogc()).into())
            },
            gc,
        )
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let string_prototype = intrinsics.string_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<StringConstructor>(agent, realm)
            .with_property_capacity(4)
            .with_builtin_function_property::<StringFromCharCode>()
            .with_builtin_function_property::<StringFromCodePoint>()
            .with_prototype_property(string_prototype.into())
            .with_builtin_function_property::<StringRaw>()
            .build();
    }
}
