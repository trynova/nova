// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sonic_rs::{JsonContainerTrait, JsonValueTrait};

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property, create_data_property_or_throw,
                enumerable_own_properties, enumerable_properties_kind, get, length_of_array_like,
            },
            testing_and_comparison::{is_array, is_callable},
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
    const NAME: String = BUILTIN_STRING_MEMORY.parse;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(JSONObject::parse);
}

struct JSONObjectStringify;
impl Builtin for JSONObjectStringify {
    const NAME: String = BUILTIN_STRING_MEMORY.stringify;

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
    fn parse(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let text = arguments.get(0);
        let reviver = arguments.get(1);

        // 1. Let jsonString be ? ToString(text).
        let json_string = to_string(agent, text)?;

        // 2. Parse StringToCodePoints(jsonString) as a JSON text as specified in ECMA-404. Throw a SyntaxError exception if it is not a valid JSON text as defined in that specification.
        let json_value = match sonic_rs::from_str::<sonic_rs::Value>(json_string.as_str(agent)) {
            Ok(value) => value,
            Err(error) => {
                return Err(agent.throw_exception(ExceptionType::SyntaxError, error.to_string()));
            }
        };

        // 3. Let scriptString be the string-concatenation of "(", jsonString, and ");".
        // 4. Let script be ParseText(scriptString, Script).
        // 5. NOTE: The early error rules defined in 13.2.5.1 have special handling for the above invocation of ParseText.
        // 6. Assert: script is a Parse Node.
        // 7. Let completion be Completion(Evaluation of script).
        let completion = value_from_json(agent, &json_value);

        // 8. NOTE: The PropertyDefinitionEvaluation semantics defined in 13.2.5.5 have special handling for the above evaluation.
        // 9. Let unfiltered be completion.[[Value]].
        let unfiltered = completion?;

        // 10. Assert: unfiltered is either a String, a Number, a Boolean, an Object that is defined by either an ArrayLiteral or an ObjectLiteral, or null.
        assert!(
            unfiltered.is_string()
                || unfiltered.is_number()
                || unfiltered.is_boolean()
                || unfiltered.is_object()
                || unfiltered.is_null()
        );

        // 11. If IsCallable(reviver) is true, then
        if let Some(reviver) = is_callable(reviver) {
            // a. Let root be OrdinaryObjectCreate(%Object.prototype%).
            let root =
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);

            // b. Let rootName be the empty String.
            let root_name = String::EMPTY_STRING.to_property_key();

            // c. Perform ! CreateDataPropertyOrThrow(root, rootName, unfiltered).
            create_data_property_or_throw(agent, root, root_name, unfiltered).unwrap();

            // d. Return ? InternalizeJSONProperty(root, rootName, reviver).
            return internalize_json_property(agent, root, root_name, reviver);
        }

        // 12. Else,
        // a. Return unfiltered.
        Ok(unfiltered)
    }

    fn stringify(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
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
pub(crate) fn internalize_json_property(
    agent: &mut Agent,
    holder: Object,
    name: PropertyKey,
    reviver: Function,
) -> JsResult<Value> {
    // 1. Let val be ? Get(holder, name).
    let val = get(agent, holder, name)?;
    // 2. If val is an Object, then
    if let Ok(val) = Object::try_from(val) {
        // a. Let isArray be ? IsArray(val).
        // b. If isArray is true, then
        if is_array(agent, val.into_value())? {
            // i. Let len be ? LengthOfArrayLike(val).
            let len = length_of_array_like(agent, val)?;
            // ii. Let I be 0.
            let mut i = 0;
            // iii. Repeat, while I < len,
            while i < len {
                // 1. Let prop be ! ToString(𝔽(I)).
                let prop = PropertyKey::from(SmallInteger::try_from(i).unwrap());

                // 2. Let newElement be ? InternalizeJSONProperty(val, prop, reviver).
                let new_element = internalize_json_property(agent, val, prop, reviver)?;

                // 3. If newElement is undefined, then
                if new_element.is_undefined() {
                    // a. Perform ? val.[[Delete]](prop).
                    val.internal_delete(agent, prop)?;
                } else {
                    // 4. Else,
                    // a. Perform ? CreateDataProperty(val, prop, newElement).
                    create_data_property(agent, val, prop, new_element)?;
                }

                // 5. Set I to I + 1.
                i += 1;
            }
        } else {
            // c. Else,
            // i. Let keys be ? EnumerableOwnProperties(val, key).
            let keys =
                enumerable_own_properties::<enumerable_properties_kind::EnumerateKeys>(agent, val)?;

            // ii. For each String P of keys, do
            for p in keys {
                let p = PropertyKey::try_from(p).unwrap();
                // 1. Let newElement be ? InternalizeJSONProperty(val, P, reviver).
                let new_element = internalize_json_property(agent, val, p, reviver)?;

                // 2. If newElement is undefined, then
                if new_element.is_undefined() {
                    // a. Perform ? val.[[Delete]](P).
                    val.internal_delete(agent, p)?;
                } else {
                    // 3. Else,
                    // a. Perform ? CreateDataProperty(val, P, newElement).
                    create_data_property(agent, val, p, new_element)?;
                }
            }
        }
    }

    // 3. Return ? Call(reviver, holder, « name, val »).
    call_function(
        agent,
        reviver,
        holder.into_value(),
        Some(ArgumentsList(&[name.into_value(), val])),
    )
}

pub(crate) fn value_from_json(agent: &mut Agent, json: &sonic_rs::Value) -> JsResult<Value> {
    match json.get_type() {
        sonic_rs::JsonType::Null => Ok(Value::Null),
        sonic_rs::JsonType::Boolean => Ok(Value::Boolean(json.is_true())),
        sonic_rs::JsonType::Number => Ok(Number::from_f64(agent, json.as_f64().unwrap()).into()),
        sonic_rs::JsonType::String => Ok(String::from_str(agent, json.as_str().unwrap()).into()),
        sonic_rs::JsonType::Array => {
            let json_array = json.as_array().unwrap();
            let len = json_array.len();
            let array_obj = array_create(agent, len, len, None)?;
            for (i, value) in json_array.iter().enumerate() {
                let prop = PropertyKey::from(SmallInteger::try_from(i as i64).unwrap());
                let js_value = value_from_json(agent, value)?;
                create_data_property(agent, array_obj, prop, js_value)?;
            }
            Ok(array_obj.into())
        }
        sonic_rs::JsonType::Object => {
            let json_object = json.as_object().unwrap();
            let object =
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);
            for (key, value) in json_object.iter() {
                let prop = PropertyKey::from_str(agent, key);
                let js_value = value_from_json(agent, value)?;
                create_data_property(agent, object, prop, js_value)?;
            }
            Ok(object.into())
        }
    }
}
