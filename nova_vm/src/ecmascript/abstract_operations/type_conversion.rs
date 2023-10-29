//! ## [7.1 Type Conversion](https://tc39.es/ecma262/#sec-type-conversion)
//!
//! The ECMAScript language implicitly performs automatic type conversion as
//! needed. To clarify the semantics of certain constructs it is useful to
//! define a set of conversion abstract operations. The conversion abstract
//! operations are polymorphic; they can accept a value of any ECMAScript
//! language type. But no other specification types are used with these
//! operations.
//!
//! The BigInt type has no implicit conversions in the ECMAScript language;
//! programmers must call BigInt explicitly to convert values from other types.

use crate::{
    ecmascript::{
        execution::{agent::JsError, Agent, JsResult},
        types::{Object, PropertyKey, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

use super::{
    operations_on_objects::{call, call_function, get, get_method},
    testing_and_comparison::is_callable,
};

#[derive(Debug, Clone, Copy)]
pub enum PreferredType {
    String = 1,
    Number,
}

/// ### [7.1.1 ToPrimitive ( input \[ , preferredType \] )](https://tc39.es/ecma262/#sec-toprimitive)
///
/// The abstract operation ToPrimitive takes argument input (an ECMAScript
/// language value) and optional argument preferredType (STRING or NUMBER) and
/// returns either a normal completion containing an ECMAScript language value
/// or a throw completion. It converts its input argument to a non-Object type.
/// If an object is capable of converting to more than one primitive type, it
/// may use the optional hint preferredType to favour that type. It performs
/// the following steps when called:
///
/// > NOTE: When ToPrimitive is called without a hint, then it generally
/// behaves as if the hint were NUMBER. However, objects may over-ride this
/// behaviour by defining a @@toPrimitive method. Of the objects defined in
/// this specification only Dates (see 21.4.4.45) and Symbol objects (see
/// 20.4.3.5) over-ride the default ToPrimitive behaviour. Dates treat the
/// absence of a hint as if the hint were STRING.
pub(crate) fn to_primitive(
    agent: &mut Agent,
    input: Value,
    preferred_type: Option<PreferredType>,
) -> JsResult<Value> {
    // 1. If input is an Object, then
    if let Ok(input) = Object::try_from(input) {
        // a. Let exoticToPrim be ? GetMethod(input, @@toPrimitive).
        let exotic_to_prim = get_method(
            agent,
            input.into_value(),
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToPrimitive.into()),
        )?;
        // b. If exoticToPrim is not undefined, then
        if let Some(exotic_to_prim) = exotic_to_prim {
            let hint = match preferred_type {
                // i. If preferredType is not present, then
                // 1. Let hint be "default".
                None => String::from_small_string("default"),
                // ii. Else if preferredType is STRING, then
                // 1. Let hint be "string".
                Some(PreferredType::String) => String::from_small_string("string"),
                // iii. Else,
                // 1. Assert: preferredType is NUMBER.
                // 2. Let hint be "number".
                Some(PreferredType::Number) => String::from_small_string("number"),
            };
            // iv. Let result be ? Call(exoticToPrim, input, « hint »).
            let result: Value =
                call_function(agent, exotic_to_prim, input.into(), Some(&[hint.into()]))?;
            if !result.is_object() {
                // v. If result is not an Object, return result.
                Ok(result)
            } else {
                // vi. Throw a TypeError exception.
                Err(JsError {})
            }
        } else {
            // c. If preferredType is not present, let preferredType be NUMBER.
            // d. Return ? OrdinaryToPrimitive(input, preferredType).
            ordinary_to_primitive(
                agent,
                input,
                preferred_type.unwrap_or(PreferredType::Number),
            )
        }
    } else {
        // 2. Return input.
        Ok(input)
    }
}

/// #### [7.1.1.1 OrdinaryToPrimitive ( O, hint )](https://tc39.es/ecma262/#sec-ordinarytoprimitive)
///
/// The abstract operation OrdinaryToPrimitive takes arguments O (an Object)
/// and hint (STRING or NUMBER) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion.
pub(crate) fn ordinary_to_primitive(
    agent: &mut Agent,
    o: Object,
    hint: PreferredType,
) -> JsResult<Value> {
    let to_string_key = PropertyKey::from(String::from_str(agent, "toString"));
    let value_of_key = PropertyKey::from(String::from_small_string("valueOf"));
    let method_names = match hint {
        PreferredType::String => {
            // 1. If hint is STRING, then
            // a. Let methodNames be « "toString", "valueOf" ».
            [to_string_key, value_of_key]
        }
        PreferredType::Number => {
            // 2. Else,
            // a. Let methodNames be « "valueOf", "toString" ».
            [value_of_key, to_string_key]
        }
    };
    // 3. For each element name of methodNames, do
    for name in method_names {
        // a. Let method be ? Get(O, name).
        let method = get(agent, o, name)?;
        // b. If IsCallable(method) is true, then
        if is_callable(method) {
            // i. Let result be ? Call(method, O).
            let result: Value = call(agent, method, o.into(), None)?;
            // ii. If result is not an Object, return result.
            if !result.is_object() {
                return Ok(result);
            }
        }
    }
    // 4. Throw a TypeError exception.
    Err(JsError {})
}

/// ### [7.1.18 ToObject ( argument )](https://tc39.es/ecma262/#sec-toobject)
///
/// The abstract operation ToObject takes argument argument (an ECMAScript
/// language value) and returns either a normal completion containing an Object
/// or a throw completion. It converts argument to a value of type Object
/// according to [Table 13](https://tc39.es/ecma262/#table-toobject-conversions):
pub(crate) fn to_object(agent: &mut Agent, argument: Value) -> JsResult<Object> {
    match argument {
        Value::Undefined | Value::Null => Err(JsError {}),
        // Return a new Boolean object whose [[BooleanData]] internal slot is set to argument.
        Value::Boolean(_) => todo!("BooleanObject"),
        // Return a new String object whose [[StringData]] internal slot is set to argument.
        Value::String(_) => todo!("StringObject"),
        Value::SmallString(_) => todo!("StringObject"),
        // Return a new Symbol object whose [[SymbolnData]] internal slot is set to argument.
        Value::Symbol(_) => todo!("SymbolObject"),
        // Return a new Number object whose [[NumberData]] internal slot is set to argument.
        Value::Number(_) => todo!("NumberObject"),
        Value::Integer(_) => todo!("NumberObject"),
        Value::Float(_) => todo!("NumberObject"),
        // Return a new BigInt object whose [[BigIntData]] internal slot is set to argument.
        Value::BigInt(_) => todo!("BigIntObject"),
        Value::SmallBigInt(_) => todo!("BigIntObject"),
        _ => Ok(Object::try_from(argument).unwrap()),
    }
}
