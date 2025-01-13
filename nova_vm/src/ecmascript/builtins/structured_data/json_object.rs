// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::iter::once;

use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::ecmascript::abstract_operations::operations_on_objects::enumerable_properties_kind::EnumerateKeys;
use crate::ecmascript::abstract_operations::operations_on_objects::{
    enumerable_own_properties, get_v, length_of_array_like, try_create_data_property,
    try_create_data_property_or_throw,
};
use crate::ecmascript::abstract_operations::testing_and_comparison::is_array;
use crate::ecmascript::abstract_operations::type_conversion::{
    to_integer_or_infinity_number, to_number,
};
use crate::ecmascript::builtins::primitive_objects::{PrimitiveObject, PrimitiveObjectData};
use crate::ecmascript::types::{IntoObject, IntoValue, PropertyDescriptor};
use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::{unwrap_try, Scoped};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property, get, scoped_enumerable_own_keys,
            },
            testing_and_comparison::is_callable,
            type_conversion::to_string,
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            array_create, ordinary::ordinary_object_create_with_intrinsics, ArgumentsList, Builtin,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            Function, InternalMethods, Number, Object, PropertyKey, String, Value,
            BUILTIN_STRING_MEMORY,
        },
    },
    heap::WellKnownSymbolIndexes,
    SmallInteger,
};

pub(crate) struct JSONObject;

struct JSONObjectParse;
impl Builtin for JSONObjectParse {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.parse;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(JSONObject::parse);
}

struct JSONObjectStringify;
impl Builtin for JSONObjectStringify {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.stringify;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(JSONObject::stringify);
}

impl JSONObject {
    /// ### [25.5.1 JSON.parse ( text \[ , reviver \] )](https://tc39.es/ecma262/#sec-json.parse)
    ///
    /// This function parses a JSON text (a JSON-formatted String) and produces
    /// an ECMAScript language value. The JSON format represents literals,
    /// arrays, and objects with a syntax similar to the syntax for ECMAScript
    /// literals, Array Initializers, and Object Initializers. After parsing,
    /// JSON objects are realized as ECMAScript objects. JSON arrays are
    /// realized as ECMAScript Array instances. JSON strings, numbers,
    /// booleans, and null are realized as ECMAScript Strings, Numbers,
    /// Booleans, and null.
    ///
    /// The optional reviver parameter is a function that takes two parameters,
    /// key and value. It can filter and transform the results. It is called
    /// with each of the key/value pairs produced by the parse, and its return
    /// value is used instead of the original value. If it returns what it
    /// received, the structure is not modified. If it returns undefined then
    /// the property is deleted from the result.
    ///
    /// > Note 1
    /// > Valid JSON text is a subset of the ECMAScript PrimaryExpression
    /// > syntax. Step 2 verifies that jsonString conforms to that subset, and
    /// > step 10 asserts that that parsing and evaluation returns a value of
    /// > an appropriate type.
    /// >
    /// > However, because 13.2.5.5 behaves differently during JSON.parse, the
    /// > same source text can produce different results when evaluated as a
    /// > PrimaryExpression rather than as JSON. Furthermore, the Early Error
    /// > for duplicate "__proto__" properties in object literals, which
    /// > likewise does not apply during JSON.parse, means that not all texts
    /// > accepted by JSON.parse are valid as a PrimaryExpression, despite
    /// > matching the grammar.
    fn parse(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        let text = arguments.get(0);
        let reviver = arguments.get(1);

        // 1. Let jsonString be ? ToString(text).
        let json_string = to_string(agent, text, gc.reborrow())?;

        // 2. Parse StringToCodePoints(jsonString) as a JSON text as specified in ECMA-404. Throw a SyntaxError exception if it is not a valid JSON text as defined in that specification.
        let json_value = match sonic_rs::from_str::<sonic_rs::Value>(json_string.as_str(agent)) {
            Ok(value) => value,
            Err(error) => {
                return Err(agent.throw_exception(
                    ExceptionType::SyntaxError,
                    error.to_string(),
                    gc.nogc(),
                ));
            }
        };

        // 3. Let scriptString be the string-concatenation of "(", jsonString, and ");".
        // 4. Let script be ParseText(scriptString, Script).
        // 5. NOTE: The early error rules defined in 13.2.5.1 have special handling for the above invocation of ParseText.
        // 6. Assert: script is a Parse Node.
        // 7. Let completion be Completion(Evaluation of script).
        let completion = value_from_json(agent, &json_value, gc.nogc());

        // 8. NOTE: The PropertyDefinitionEvaluation semantics defined in 13.2.5.5 have special handling for the above evaluation.
        // 9. Let unfiltered be completion.[[Value]].
        let unfiltered = completion;

        // 10. Assert: unfiltered is either a String, a Number, a Boolean, an Object that is defined by either an ArrayLiteral or an ObjectLiteral, or null.
        assert!(
            unfiltered.is_string()
                || unfiltered.is_number()
                || unfiltered.is_boolean()
                || unfiltered.is_object()
                || unfiltered.is_null()
        );

        // 11. If IsCallable(reviver) is true, then
        if let Some(reviver) = is_callable(reviver, gc.nogc()) {
            let reviver = reviver.bind(gc.nogc());
            // a. Let root be OrdinaryObjectCreate(%Object.prototype%).
            let Object::Object(root) =
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None)
            else {
                unreachable!()
            };

            // b. Let rootName be the empty String.
            let root_name = String::EMPTY_STRING.to_property_key().scope_static();

            // c. Perform ! CreateDataPropertyOrThrow(root, rootName, unfiltered).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                root,
                root_name.unwrap(),
                unfiltered,
                gc.nogc(),
            ))
            .unwrap();

            // d. Return ? InternalizeJSONProperty(root, rootName, reviver).
            let root = root.into_object().scope(agent, gc.nogc());
            let reviver = reviver.scope(agent, gc.nogc());
            return internalize_json_property(agent, root, root_name, reviver, gc.reborrow());
        }

        // 12. Else,
        // a. Return unfiltered.
        Ok(unfiltered)
    }

    /// ### [25.5.1 JSON.stringify ( value \[ , replacer \[ , space \] ] )](https://tc39.es/ecma262/#sec-json.stringify)
    ///
    /// This function returns a String in UTF-16 encoded JSON format
    /// representing an ECMAScript language value, or undefined. It can take
    /// three parameters. The value parameter is an ECMAScript language value,
    /// which is usually an object or array, although it can also be a String,
    /// Boolean, Number or null. The optional replacer parameter is either a
    /// function that alters the way objects and arrays are stringified, or an
    /// array of Strings and Numbers that acts as an inclusion list for
    /// selecting the object properties that will be stringified. The optional
    /// space parameter is a String or Number that allows the result to have
    /// white space injected into it to improve human readability.
    ///
    /// > Note 1
    /// >
    /// > JSON structures are allowed to be nested to any depth, but they must be acyclic. If value is or contains a cyclic structure, then this function must throw a TypeError exception. This is an example of a value that cannot be stringified:
    /// >
    /// > ```js
    /// > a = [];
    /// > a[0] = a;
    /// > my_text = JSON.stringify(a); // This must throw a TypeError.
    /// > ```
    ///
    /// > Note 2
    /// >
    /// > Symbolic primitive values are rendered as follows:
    /// >
    /// > - The null value is rendered in JSON text as the String value "null".
    /// > - The undefined value is not rendered.
    /// > - The true value is rendered in JSON text as the String value "true".
    /// > - The false value is rendered in JSON text as the String value "false".
    ///
    /// > Note 3
    /// >
    /// > String values are wrapped in QUOTATION MARK (`"``) code units. The code
    /// > units `"` and `\` are escaped with `\` prefixes. Control characters code
    /// > units are replaced with escape sequences `\uHHHH`, or with the shorter
    /// > forms, `\b` (BACKSPACE), `\f` (FORM FEED), `\n` (LINE FEED), `\r` (CARRIAGE
    /// > RETURN), `\t` (CHARACTER TABULATION).
    ///
    /// > Note 4
    /// >
    /// > Finite numbers are stringified as if by calling ToString(number). NaN
    /// > and Infinity regardless of sign are represented as the String value
    /// > "null".
    ///
    /// > Note 5
    /// >
    /// > Values that do not have a JSON representation (such as undefined and
    /// > functions) do not produce a String. Instead they produce the undefined
    /// > value. In arrays these values are represented as the String value
    /// > "null". In objects an unrepresentable value causes the property to be
    /// > excluded from stringification.
    ///
    /// > Note 6
    /// >
    /// > An object is rendered as U+007B (LEFT CURLY BRACKET) followed by zero
    /// > or more properties, separated with a U+002C (COMMA), closed with a
    /// > U+007D (RIGHT CURLY BRACKET). A property is a quoted String
    /// > representing the property name, a U+003A (COLON), and then the
    /// > stringified property value. An array is rendered as an opening U+005B
    /// > (LEFT SQUARE BRACKET) followed by zero or more values, separated with a
    /// > U+002C (COMMA), closed with a U+005D (RIGHT SQUARE BRACKET).
    fn stringify(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        let value = arguments.get(0);
        let replacer = arguments.get(1);
        let space = arguments.get(2);

        // 1. Let stack be a new empty List.
        let stack = Vec::new();
        // 3. Let PropertyList be undefined.
        let mut property_list: Option<Vec<String<'static>>> = None;

        // 4. Let ReplacerFunction be undefined.
        // a. If IsCallable(replacer) is true, then
        let replacer_function = if let Ok(replacer) = Function::try_from(replacer) {
            // i. Set ReplacerFunction to replacer.
            Some(replacer)
        } else if let Ok(replacer) = Object::try_from(replacer) {
            // 5. If replacer is an Object, then
            // b. Else,
            // i. Let isArray be ? IsArray(replacer).
            if is_array(agent, replacer, gc.nogc())? {
                // ii. If isArray is true, then
                // 2. Let len be ? LengthOfArrayLike(replacer).
                let len = length_of_array_like(agent, replacer, gc.reborrow())?;
                // 1. Set PropertyList to a new empty List.
                property_list = Some(Vec::with_capacity(len as usize));
                // 3. Let k be 0.
                // 4. Repeat, while k < len,
                // h. Set k to k + 1.
                for k in 0..len {
                    // a. Let prop be ! ToString(𝔽(k)).
                    let prop = PropertyKey::from(SmallInteger::try_from(k).unwrap());
                    // b. Let v be ? Get(replacer, prop).
                    let v = get(agent, replacer, prop, gc.reborrow())?;
                    // c. Let item be undefined.
                    let item = if let Ok(v) = String::try_from(v) {
                        // d. If v is a String, then
                        // i. Set item to v.
                        Some(v.unbind())
                    } else if v.is_number() {
                        // e. Else if v is a Number, then
                        // i. Set item to ! ToString(v).
                        Some(to_string(agent, v, gc.reborrow()).unwrap().unbind())
                    } else if let Ok(v) = PrimitiveObject::try_from(v) {
                        // f. Else if v is an Object, then
                        // i. If v has a [[StringData]] or [[NumberData]] internal slot, set item to ? ToString(v).
                        match agent[v].data {
                            PrimitiveObjectData::String(_)
                            | PrimitiveObjectData::SmallString(_)
                            | PrimitiveObjectData::Number(_)
                            | PrimitiveObjectData::Integer(_)
                            | PrimitiveObjectData::Float(_) => {
                                Some(to_string(agent, v, gc.reborrow())?.unbind())
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    // g. If item is not undefined and PropertyList does not contain item, then
                    // i. Append item to PropertyList.
                    if let Some(item) = item {
                        let property_list = property_list.as_mut().unwrap();
                        if !property_list.contains(&item) {
                            property_list.push(item);
                        }
                    }
                }
            }
            None
        } else {
            None
        };

        // 6. If space is an Object, then
        let space = if let Ok(space) = PrimitiveObject::try_from(space) {
            match agent[space].data {
                // a. If space has a [[NumberData]] internal slot, then
                // i. Set space to ? ToNumber(space).
                PrimitiveObjectData::Number(_)
                | PrimitiveObjectData::Integer(_)
                | PrimitiveObjectData::Float(_) => {
                    Some(to_number(agent, space, gc.reborrow())?.into_value())
                }
                // b. Else if space has a [[StringData]] internal slot, then
                // i. Set space to ? ToString(space).
                PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_) => {
                    Some(to_string(agent, space, gc.reborrow())?.into_value())
                }
                _ => None,
            }
        } else {
            None
        };

        let gap = space.map_or("".to_string(), |space| {
            // 7. If space is a Number, then
            if let Ok(space) = Number::try_from(space) {
                // a. Let spaceMV be ! ToIntegerOrInfinity(space).
                let space_mv = to_integer_or_infinity_number(agent, space, gc.nogc());
                // b. Set spaceMV to min(10, spaceMV).
                // c. If spaceMV < 1, let gap be the empty String; otherwise let gap be the String value containing spaceMV occurrences of the code unit 0x0020 (SPACE).
                let space_mv = space_mv.into_i64().clamp(0, 10) as usize;
                " ".repeat(space_mv as usize)
            } else if let Ok(space) = String::try_from(space) {
                // 8. Else if space is a String, then
                // a. If the length of space ≤ 10, let gap be space; otherwise let gap be the substring of space from 0 to 10.
                space.as_str(agent)[..10].to_string()
            } else {
                // 9. Else,
                // a. Let gap be the empty String.
                "".to_string()
            }
        });

        // 10. Let wrapper be OrdinaryObjectCreate(%Object.prototype%).
        let wrapper =
            ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);
        // 11. Perform ! CreateDataPropertyOrThrow(wrapper, the empty String, value).
        wrapper.property_storage().set(
            agent,
            String::EMPTY_STRING.to_property_key(),
            PropertyDescriptor::new_data_descriptor(value),
        );
        // 12. Let state be the JSON Serialization Record { [[ReplacerFunction]]: ReplacerFunction, [[Stack]]: stack, [[Indent]]: indent, [[Gap]]: gap, [[PropertyList]]: PropertyList }.
        let mut state = JSONSerializationRecord {
            replacer_function,
            stack,
            // 2. Let indent be the empty String.
            indent: "".to_owned(),
            gap,
            property_list,
        };
        // 13. Return ? SerializeJSONProperty(state, the empty String, wrapper).
        Ok(serialize_json_property(
            agent,
            &mut state,
            String::EMPTY_STRING,
            wrapper,
            gc.reborrow(),
        )?
        .map(|s| s.into_value())
        .unwrap_or(Value::Undefined))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.json();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<JSONObjectParse>()
            .with_builtin_function_property::<JSONObjectStringify>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.JSON.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}

/// [25.5.1.1 InternalizeJSONProperty ( holder, name, reviver )](https://tc39.es/ecma262/#sec-internalizejsonproperty)
///
/// The abstract operation InternalizeJSONProperty takes arguments holder (an
/// Object), name (a String), and reviver (a function object) and returns
/// either a normal completion containing an ECMAScript language value or a
/// throw completion.
///
/// > Note 1
/// > This algorithm intentionally does not throw an exception if either
/// > [[Delete]] or CreateDataProperty return false.
///
/// > Note 2
/// > In the case where there are duplicate name Strings within an object,
/// > lexically preceding values for the same key shall be overwritten.
fn internalize_json_property<'a>(
    agent: &mut Agent,
    holder: Scoped<'a, Object>,
    name: Scoped<'a, PropertyKey<'static>>,
    reviver: Scoped<'a, Function<'static>>,
    mut gc: GcScope<'_, 'a>,
) -> JsResult<Value> {
    // 1. Let val be ? Get(holder, name).
    let val = get(agent, holder.get(agent), name.get(agent), gc.reborrow())?;
    // 2. If val is an Object, then
    if let Ok(val) = Object::try_from(val) {
        // a. Let isArray be ? IsArray(val).
        // b. If isArray is true, then
        let scoped_val = val.scope(agent, gc.nogc());
        if is_array(agent, val, gc.nogc())? {
            // i. Let len be ? LengthOfArrayLike(val).
            let len = length_of_array_like(agent, val, gc.reborrow())?;
            // let val = val.scope(agent, gc.nogc());
            // ii. Let I be 0.
            let mut i = 0;
            // iii. Repeat, while I < len,
            while i < len {
                // 1. Let prop be ! ToString(𝔽(I)).
                let prop = PropertyKey::from(SmallInteger::try_from(i).unwrap()).scope_static();

                // 2. Let newElement be ? InternalizeJSONProperty(val, prop, reviver).
                let new_element = internalize_json_property(
                    agent,
                    scoped_val.clone(),
                    prop.clone(),
                    reviver.clone(),
                    gc.reborrow(),
                )?;

                // 3. If newElement is undefined, then
                if new_element.is_undefined() {
                    // a. Perform ? val.[[Delete]](prop).
                    // Note: Deleting from an Array never calls into JavaScript.
                    val.internal_delete(agent, prop.unwrap(), gc.reborrow())?;
                } else {
                    // 4. Else,
                    // a. Perform ? CreateDataProperty(val, prop, newElement).
                    // Note: Defining a property on an Array never calls into
                    // JavaScript.
                    create_data_property(
                        agent,
                        scoped_val.get(agent),
                        prop.unwrap(),
                        new_element,
                        gc.reborrow(),
                    )?;
                }

                // 5. Set I to I + 1.
                i += 1;
            }
        } else {
            // c. Else,
            // i. Let keys be ? EnumerableOwnProperties(val, KEY).
            let keys = scoped_enumerable_own_keys(agent, scoped_val.clone(), gc.reborrow())?;

            // ii. For each String P of keys, do
            for p in keys {
                // 1. Let newElement be ? InternalizeJSONProperty(val, P, reviver).
                let new_element = internalize_json_property(
                    agent,
                    scoped_val.clone(),
                    p.clone(),
                    reviver.clone(),
                    gc.reborrow(),
                )?;

                // 2. If newElement is undefined, then
                if new_element.is_undefined() {
                    // a. Perform ? val.[[Delete]](P).
                    scoped_val
                        .get(agent)
                        .internal_delete(agent, p.get(agent), gc.reborrow())?;
                } else {
                    // 3. Else,
                    // a. Perform ? CreateDataProperty(val, P, newElement).
                    create_data_property(
                        agent,
                        scoped_val.get(agent),
                        p.get(agent),
                        new_element,
                        gc.reborrow(),
                    )?;
                }
            }
        }
    }

    // 3. Return ? Call(reviver, holder, « name, val »).
    // Note: Because this call gets holder as `this`, it can do dirty things to
    // it, such as `holder[other_key] = new Proxy()`.
    let name = name.get(agent).convert_to_value(agent, gc.nogc());
    call_function(
        agent,
        reviver.get(agent),
        holder.get(agent).into_value(),
        Some(ArgumentsList(&[name, val])),
        gc.reborrow(),
    )
}

struct JSONSerializationRecord {
    replacer_function: Option<Function<'static>>,
    stack: Vec<Object>,
    indent: std::string::String,
    gap: std::string::String,
    property_list: Option<Vec<String<'static>>>,
}

/// ### [25.5.2.2 SerializeJSONProperty ( state, key, holder )](https://tc39.es/ecma262/#sec-serializejsonproperty)
///
/// The abstract operation SerializeJSONProperty takes arguments state (a JSON
/// Serialization Record), key (a String), and holder (an Object) and returns
/// either a normal completion containing either a String or undefined, or a
/// throw completion.
fn serialize_json_property(
    agent: &mut Agent,
    state: &mut JSONSerializationRecord,
    key: String<'static>,
    holder: Object,
    mut gc: GcScope<'_, '_>,
) -> JsResult<Option<String<'static>>> {
    // 1. Let value be ? Get(holder, key).
    let mut value = get(agent, holder, PropertyKey::from(key), gc.reborrow())?;
    // 2. If value is an Object or value is a BigInt, then
    if value.is_object() || value.is_bigint() {
        // a. Let toJSON be ? GetV(value, "toJSON").
        let to_json = get_v(
            agent,
            value,
            BUILTIN_STRING_MEMORY.toJSON.to_property_key(),
            gc.reborrow(),
        )?;
        // b. If IsCallable(toJSON) is true, then
        if let Some(to_json) = is_callable(to_json, gc.nogc()) {
            // i. Set value to ? Call(toJSON, value, « key »).
            value = call_function(
                agent,
                to_json.unbind(),
                value,
                Some(ArgumentsList(&[key.into()])),
                gc.reborrow(),
            )?;
        }
    }
    // 3. If state.[[ReplacerFunction]] is not undefined, then
    if let Some(replacer_function) = &state.replacer_function {
        // a. Set value to ? Call(state.[[ReplacerFunction]], holder, « key, value »).
        value = call_function(
            agent,
            *replacer_function,
            holder.into(),
            Some(ArgumentsList(&[key.into(), value])),
            gc.reborrow(),
        )?;
    }

    // 4. If value is an Object, then
    if let Ok(obj) = PrimitiveObject::try_from(value) {
        let data = agent[obj].data;
        match data {
            // a. If value has a [[NumberData]] internal slot, then
            // i. Set value to ? ToNumber(value).
            PrimitiveObjectData::Number(_)
            | PrimitiveObjectData::Integer(_)
            | PrimitiveObjectData::Float(_) => {
                value = to_number(agent, obj, gc.reborrow())?.into_value()
            }
            // b. Else if value has a [[StringData]] internal slot, then
            // i. Set value to ? ToString(value).
            PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_) => {
                value = to_string(agent, obj, gc.reborrow())?.into_value()
            }
            // c. Else if value has a [[BooleanData]] internal slot, then
            // i. Set value to value.[[BooleanData]].
            // d. Else if value has a [[BigIntData]] internal slot, then
            // i. Set value to value.[[BigIntData]].
            PrimitiveObjectData::Boolean(_)
            | PrimitiveObjectData::BigInt(_)
            | PrimitiveObjectData::SmallBigInt(_) => value = data.into_value(),
            _ => {}
        }
    }

    match value {
        // 5. If value is null, return "null".
        Value::Null => return Ok(Some(BUILTIN_STRING_MEMORY.null)),
        // 6. If value is true, return "true".
        Value::Boolean(true) => return Ok(Some(BUILTIN_STRING_MEMORY.r#true)),
        // 7. If value is false, return "false".
        Value::Boolean(false) => return Ok(Some(BUILTIN_STRING_MEMORY.r#false)),
        // 8. If value is a String, return QuoteJSONString(value).
        Value::String(_) | Value::SmallString(_) => {
            let value = value.to_string(agent, gc.reborrow()).unwrap().unbind();
            return Ok(Some(
                quote_json_string(agent, value, gc.into_nogc()).unbind(),
            ));
        }
        // 9. If value is a Number, then
        Value::Number(_) | Value::SmallF64(_) | Value::Integer(_) => {
            let Ok(value) = Number::try_from(value) else {
                unreachable!()
            };
            // a. If value is finite, return ! ToString(value).
            if value.is_finite(agent) {
                return Ok(Some(
                    to_string(agent, value, gc.reborrow()).unwrap().unbind(),
                ));
            }
            // b. Return "null".
            return Ok(Some(BUILTIN_STRING_MEMORY.null));
        }
        Value::BigInt(_) | Value::SmallBigInt(_) => {
            // 10. If value is a BigInt, throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Cannot serialize BigInt to JSON",
                gc.nogc(),
            ));
        }
        _ => {}
    }

    // 11. If value is an Object and IsCallable(value) is false, then
    if let Ok(value) = Object::try_from(value) {
        if is_callable(value, gc.nogc()).is_none() {
            // a. Let isArray be ? IsArray(value).
            // b. If isArray is true, return ? SerializeJSONArray(state, value).
            if is_array(agent, value, gc.nogc())? {
                return Ok(Some(serialize_json_array(agent, state, value, gc)?));
            }
            // c. Return ? SerializeJSONObject(state, value).
            return Ok(Some(serialize_json_object(agent, state, value, gc)?));
        }
    }
    // 12. Return undefined.
    Ok(None)
}

/// ### [25.5.2.3 QuoteJSONString ( value )](https://tc39.es/ecma262/#sec-quotejsonstring)
///
/// The abstract operation QuoteJSONString takes argument value (a String) and
/// returns a String. It wraps value in 0x0022 (QUOTATION MARK) code units and
/// escapes certain other code units within it. This operation interprets value
/// as a sequence of UTF-16 encoded code points, as described in 6.1.4.
pub(crate) fn quote_json_string<'a>(
    agent: &mut Agent,
    value: String,
    gc: NoGcScope<'a, '_>,
) -> String<'a> {
    // 1. Let product be the String value consisting solely of the code unit 0x0022 (QUOTATION MARK).
    let mut product = Vec::with_capacity(value.utf16_len(agent) + 2);
    product.push('"');
    // 2. For each code point C of StringToCodePoints(value), do
    for c in value.as_str(agent).chars() {
        match c {
            // a. If C is listed in the “Code Point” column of Table 75, then
            // i. Set product to the string-concatenation of product and the escape sequence for C as specified in the “Escape Sequence” column of the corresponding row.
            '\u{0008}' => product.extend_from_slice(&['\\', 'b']),
            '\u{0009}' => product.extend_from_slice(&['\\', 't']),
            '\u{000A}' => product.extend_from_slice(&['\\', 'n']),
            '\u{000C}' => product.extend_from_slice(&['\\', 'f']),
            '\u{000D}' => product.extend_from_slice(&['\\', 'r']),
            '\u{0022}' => product.extend_from_slice(&['\\', '"']),
            '\u{005C}' => product.extend_from_slice(&['\\', '\\']),
            // b. Else if C has a numeric value less than 0x0020 (SPACE) or C has the same numeric value as a leading surrogate or trailing surrogate, then
            // i. Let unit be the code unit whose numeric value is the numeric value of C.
            // ii. Set product to the string-concatenation of product and UnicodeEscape(unit).
            _ if c < '\u{0020}' => product.extend(format!("\\u{:04x}", c as u32).chars()),
            // c. Else,
            // i. Set product to the string-concatenation of product and UTF16EncodeCodePoint(C).
            _ => product.push(c),
        }
    }
    // 3. Set product to the string-concatenation of product and the code unit 0x0022 (QUOTATION MARK).
    product.push('"');
    // 4. Return product.
    String::from_string(agent, product.iter().collect(), gc)
}

/// ### [25.5.2.5 SerializeJSONObject ( state, value )](https://tc39.es/ecma262/#sec-serializejsonobject)
///
/// The abstract operation SerializeJSONObject takes arguments state (a JSON
/// Serialization Record) and value (an Object) and returns either a normal
/// completion containing a String or a throw completion. It serializes an
/// object.
fn serialize_json_object(
    agent: &mut Agent,
    state: &mut JSONSerializationRecord,
    value: Object,
    mut gc: GcScope<'_, '_>,
) -> JsResult<String<'static>> {
    // 1. If state.[[Stack]] contains value, throw a TypeError exception because the structure is cyclical.
    if state.stack.contains(&value) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cyclical structure in JSON",
            gc.nogc(),
        ));
    }
    // 2. Append value to state.[[Stack]].
    state.stack.push(value);
    // 3. Let stepBack be state.[[Indent]].
    let step_back = state.indent.clone();
    // 4. Set state.[[Indent]] to the string-concatenation of state.[[Indent]] and state.[[Gap]].
    state.indent.push_str(&state.gap);
    // 5. If state.[[PropertyList]] is not undefined, then
    // a. Let K be state.[[PropertyList]].
    // 6. Else,
    // a. Let K be ? EnumerableOwnProperties(value, key).
    let k = state.property_list.as_ref().map_or_else(
        || {
            enumerable_own_properties::<EnumerateKeys>(agent, value, gc.reborrow()).map(|k| {
                k.iter()
                    .map(|k| k.to_string(agent, gc.reborrow()).unwrap().unbind())
                    .collect()
            })
        },
        |k| Ok(k.to_vec()),
    )?;
    // 7. Let partial be a new empty List.
    let mut partial = Vec::with_capacity(k.len());
    // 8. For each element P of K, do
    for p in k {
        // a. Let strP be ? SerializeJSONProperty(state, P, value).
        // SAFETY: Because the enumerable type is key we are guaranteed that the property is a valid property key.
        let str_p = serialize_json_property(agent, state, p, value, gc.reborrow())?;
        // b. If strP is not undefined, then
        let Some(str_p) = str_p else {
            continue;
        };

        // i. Let member be QuoteJSONString(P).
        let member = quote_json_string(agent, p, gc.nogc()).unbind();
        // ii. Set member to the string-concatenation of member and ":".
        let mut member = vec![
            member,
            String::from_static_str(agent, ":", gc.nogc()).unbind(),
        ];
        // iii. If state.[[Gap]] is not the empty String, then
        if !state.gap.is_empty() {
            // 1. Set member to the string-concatenation of member and the code unit 0x0020 (SPACE).
            member.push(BUILTIN_STRING_MEMORY.__);
        };
        // iv. Set member to the string-concatenation of member and strP.
        member.push(str_p);
        // let member = String::concat(agent, member, gc.nogc()).unbind();
        // v. Append member to partial.
        partial.push(member);
    }

    // 9. If partial is empty, then
    let r#final = if partial.is_empty() {
        // a. Let final be "{}".
        String::from_static_str(agent, "{}", gc.nogc())
    } else {
        let newline = String::from_static_str(agent, "\n", gc.nogc()).unbind();
        let opening_brace = String::from_static_str(agent, "{", gc.nogc()).unbind();
        let closing_brace = String::from_static_str(agent, "}", gc.nogc()).unbind();
        let mut separator = vec![String::from_static_str(agent, ",", gc.nogc()).unbind()];
        // 10. Else,
        // a. If state.[[Gap]] is the empty String, then
        if state.gap.is_empty() {
            // i. Let properties be the String value formed by concatenating all the element Strings of partial with each adjacent pair of Strings separated with the code unit 0x002C (COMMA). A comma is not inserted either before the first String or after the last String.
            // ii. Let final be the string-concatenation of "{", properties, and "}".
            String::concat(
                agent,
                once(opening_brace)
                    .chain(partial.into_iter().intersperse(separator).flatten())
                    .chain(once(closing_brace))
                    .collect::<Vec<String<'static>>>(),
                gc.nogc(),
            )
        } else {
            let indent = String::from_string(agent, state.indent.clone(), gc.nogc()).unbind();
            let step_back = String::from_string(agent, step_back.clone(), gc.nogc()).unbind();
            separator.extend_from_slice(&[newline, indent]);
            // b. Else,
            // i. Let separator be the string-concatenation of the code unit 0x002C (COMMA), the code unit 0x000A (LINE FEED), and state.[[Indent]].
            // ii. Let properties be the String value formed by concatenating all the element Strings of partial with each adjacent pair of Strings separated with separator. The separator String is not inserted either before the first String or after the last String.
            // iii. Let final be the string-concatenation of "{", the code unit 0x000A (LINE FEED), state.[[Indent]], properties, the code unit 0x000A (LINE FEED), stepBack, and "}".
            String::concat(
                agent,
                once(opening_brace)
                    .chain(once(newline))
                    .chain(once(indent))
                    .chain(partial.into_iter().intersperse(separator).flatten())
                    .chain(once(newline))
                    .chain(once(step_back))
                    .chain(once(closing_brace))
                    .collect::<Vec<String<'static>>>(),
                gc.nogc(),
            )
        }
    };
    // 11. Remove the last element of state.[[Stack]].
    state.stack.pop();
    // 12. Set state.[[Indent]] to stepBack.
    state.indent = step_back;
    // 13. Return final.
    Ok(r#final.unbind())
}

/// ### [25.5.2.6 SerializeJSONArray ( state, value )](https://tc39.es/ecma262/#sec-serializejsonarray)
///
/// The abstract operation SerializeJSONArray takes arguments state (a JSON
/// Serialization Record) and value (an ECMAScript language value) and returns
/// either a normal completion containing a String or a throw completion. It
/// serializes an array.
fn serialize_json_array(
    agent: &mut Agent,
    state: &mut JSONSerializationRecord,
    value: Object,
    mut gc: GcScope<'_, '_>,
) -> JsResult<String<'static>> {
    // 1. If state.[[Stack]] contains value, throw a TypeError exception because the structure is cyclical.
    if state.stack.contains(&value) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cyclical structure in JSON",
            gc.nogc(),
        ));
    }
    // 2. Append value to state.[[Stack]].
    state.stack.push(value);
    // 3. Let stepBack be state.[[Indent]].
    let step_back = state.indent.clone();
    // 4. Set state.[[Indent]] to the string-concatenation of state.[[Indent]] and state.[[Gap]].
    state.indent.push_str(&state.gap);
    // 5. Let partial be a new empty List.
    let mut partial = Vec::new();
    // 6. Let len be ? LengthOfArrayLike(value).
    let len = length_of_array_like(agent, value, gc.reborrow())?;
    // 7. Let index be 0.
    let mut index = 0;
    // 8. Repeat, while index < len,
    while index < len {
        let key = to_string(agent, Number::try_from(index).unwrap(), gc.reborrow())
            .unwrap()
            .unbind();
        partial.push(
            // a. Let strP be ? SerializeJSONProperty(state, ! ToString(𝔽(index)), value).
            if let Some(str_p) = serialize_json_property(agent, state, key, value, gc.reborrow())? {
                // c. Else,
                // i. Append strP to partial.
                str_p.unbind()
            } else {
                // b. If strP is undefined, then
                // i. Append "null" to partial.
                BUILTIN_STRING_MEMORY.null.unbind()
            },
        );
        // d. Set index to index + 1.
        index += 1;
    }
    // 9. If partial is empty, then
    let r#final = if partial.is_empty() {
        // a. Let final be "[]".
        String::from_static_str(agent, "[]", gc.nogc())
    } else {
        let newline = String::from_static_str(agent, "\n", gc.nogc()).unbind();
        let opening_bracket = String::from_static_str(agent, "[", gc.nogc()).unbind();
        let closing_bracket = String::from_static_str(agent, "]", gc.nogc()).unbind();
        let mut separator = vec![String::from_static_str(agent, ",", gc.nogc()).unbind()];
        // 10. Else,
        // a. If state.[[Gap]] is the empty String, then
        if state.gap.is_empty() {
            // i. Let properties be the String value formed by concatenating all the element Strings of partial with each adjacent pair of Strings separated with the code unit 0x002C (COMMA). A comma is not inserted either before the first String or after the last String.
            // ii. Let final be the string-concatenation of "[", properties, and "]".
            String::concat(
                agent,
                once(opening_bracket)
                    .chain(
                        partial
                            .into_iter()
                            .map(|item| vec![item])
                            .intersperse(separator)
                            .flatten(),
                    )
                    .chain(once(closing_bracket))
                    .collect::<Vec<String<'static>>>(),
                gc.nogc(),
            )
        } else {
            let indent = String::from_string(agent, state.indent.clone(), gc.nogc()).unbind();
            let step_back = String::from_string(agent, step_back.clone(), gc.nogc()).unbind();
            separator.extend_from_slice(&[newline, indent]);
            // b. Else,
            // i. Let separator be the string-concatenation of the code unit 0x002C (COMMA), the code unit 0x000A (LINE FEED), and state.[[Indent]].
            // ii. Let properties be the String value formed by concatenating all the element Strings of partial with each adjacent pair of Strings separated with separator. The separator String is not inserted either before the first String or after the last String.
            // iii. Let final be the string-concatenation of "[", the code unit 0x000A (LINE FEED), state.[[Indent]], properties, the code unit 0x000A (LINE FEED), stepBack, and "]".
            String::concat(
                agent,
                once(opening_bracket)
                    .chain(once(newline))
                    .chain(once(indent))
                    .chain(
                        partial
                            .into_iter()
                            .map(|item| vec![item])
                            .intersperse(separator)
                            .flatten(),
                    )
                    .chain(once(newline))
                    .chain(once(step_back))
                    .chain(once(closing_bracket))
                    .collect::<Vec<String<'static>>>(),
                gc.nogc(),
            )
        }
    };
    // 11. Remove the last element of state.[[Stack]].
    state.stack.pop();
    // 12. Set state.[[Indent]] to stepBack.
    state.indent = step_back;
    // 13. Return final.
    Ok(r#final.unbind())
}

pub(crate) fn value_from_json(
    agent: &mut Agent,
    json: &sonic_rs::Value,
    gc: NoGcScope<'_, '_>,
) -> Value {
    match json.get_type() {
        sonic_rs::JsonType::Null => Value::Null,
        sonic_rs::JsonType::Boolean => Value::Boolean(json.is_true()),
        sonic_rs::JsonType::Number => Number::from_f64(agent, json.as_f64().unwrap(), gc).into(),
        sonic_rs::JsonType::String => String::from_str(agent, json.as_str().unwrap(), gc).into(),
        sonic_rs::JsonType::Array => {
            let json_array = json.as_array().unwrap();
            let len = json_array.len();
            let array_obj = array_create(agent, len, len, None, gc).unwrap();
            for (i, value) in json_array.iter().enumerate() {
                let prop = PropertyKey::from(SmallInteger::try_from(i as i64).unwrap());
                let js_value = value_from_json(agent, value, gc);
                unwrap_try(try_create_data_property(
                    agent, array_obj, prop, js_value, gc,
                ));
            }
            array_obj.into_value()
        }
        sonic_rs::JsonType::Object => {
            let json_object = json.as_object().unwrap();
            let Object::Object(object) =
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None)
            else {
                unreachable!()
            };
            for (key, value) in json_object.iter() {
                let prop = PropertyKey::from_str(agent, key, gc);
                let js_value = value_from_json(agent, value, gc);
                unwrap_try(try_create_data_property(agent, object, prop, js_value, gc));
            }
            object.into()
        }
    }
}
