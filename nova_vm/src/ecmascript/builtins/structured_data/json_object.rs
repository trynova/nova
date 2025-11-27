// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use sonic_rs::{JsonContainerTrait, JsonValueTrait};
use wtf8::{CodePoint, Wtf8Buf};

use crate::{
    SmallInteger,
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                call_function, create_data_property, enumerable_own_keys, get, get_v,
                length_of_array_like, scoped_enumerable_own_keys, try_create_data_property,
                try_create_data_property_or_throw,
            },
            testing_and_comparison::{is_array, is_callable},
            type_conversion::{
                to_integer_or_infinity_number, to_number, to_property_key, to_property_key_simple,
                to_string,
            },
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, array_create,
            ordinary::ordinary_object_create_with_intrinsics,
            primitive_objects::{PrimitiveObject, PrimitiveObjectData},
        },
        execution::{
            Agent, JsResult, ProtoIntrinsics, Realm,
            agent::{ExceptionType, unwrap_try},
        },
        types::{
            BUILTIN_STRING_MEMORY, Function, InternalMethods, IntoObject, IntoPrimitive, IntoValue,
            Number, Object, PropertyDescriptor, PropertyKey, ScopedPropertyKey, String, Value,
        },
    },
    engine::{
        ScopableCollection, Scoped, ScopedCollection,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct JSONObject;

struct JSONObjectParse;
impl Builtin for JSONObjectParse {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.parse;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(JSONObject::parse);
}

struct JSONObjectStringify;
impl Builtin for JSONObjectStringify {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.stringify;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(JSONObject::stringify);
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
    fn parse<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let text = arguments.get(0).bind(gc.nogc());
        let reviver = arguments.get(1).scope(agent, gc.nogc());

        // 1. Let jsonString be ? ToString(text).
        let json_string = to_string(agent, text.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

        // 2. Parse StringToCodePoints(jsonString) as a JSON text as specified in ECMA-404. Throw a SyntaxError exception if it is not a valid JSON text as defined in that specification.
        let json_value =
            match sonic_rs::from_str::<sonic_rs::Value>(&json_string.to_string_lossy(agent)) {
                Ok(value) => value,
                Err(error) => {
                    return Err(agent.throw_exception(
                        ExceptionType::SyntaxError,
                        error.to_string(),
                        gc.into_nogc(),
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
        let reviver = reviver.get(agent).bind(gc.nogc());
        if let Some(reviver) = is_callable(reviver, gc.nogc()) {
            let reviver = reviver.bind(gc.nogc());
            // a. Let root be OrdinaryObjectCreate(%Object.prototype%).
            let Object::Object(root) = ordinary_object_create_with_intrinsics(
                agent,
                Some(ProtoIntrinsics::Object),
                None,
                gc.nogc(),
            ) else {
                unreachable!()
            };

            // b. Let rootName be the empty String.
            let root_name = String::EMPTY_STRING
                .to_property_key()
                .unbind()
                .scope_static();

            // c. Perform ! CreateDataPropertyOrThrow(root, rootName, unfiltered).
            unwrap_try(try_create_data_property_or_throw(
                agent,
                root,
                root_name.unwrap(),
                unfiltered,
                None,
                gc.nogc(),
            ));

            // d. Return ? InternalizeJSONProperty(root, rootName, reviver).
            let root = root.unbind().into_object().scope(agent, gc.nogc());
            let reviver = reviver.unbind().scope(agent, gc.nogc());
            return internalize_json_property(agent, root, root_name, reviver, gc);
        }

        // 12. Else,
        // a. Return unfiltered.
        Ok(unfiltered.unbind())
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
    pub(crate) fn stringify<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let value = arguments.get(0).scope(agent, gc.nogc());
        let replacer = arguments.get(1).bind(gc.nogc());
        let space = arguments.get(2).scope(agent, gc.nogc());

        // 1. Let stack be a new empty List.
        let stack = Vec::<Value>::new().scope(agent, gc.nogc());
        // 3. Let PropertyList be undefined.
        let mut property_list: Option<ScopedCollection<Vec<PropertyKey>>> = None;

        // 4. Let ReplacerFunction be undefined.
        // a. If IsCallable(replacer) is true, then
        let replacer_function = if let Some(replacer) = is_callable(replacer, gc.nogc()) {
            // i. Set ReplacerFunction to replacer.
            Some(replacer.scope(agent, gc.nogc()))
        } else if let Ok(replacer) = Object::try_from(replacer) {
            // 5. If replacer is an Object, then
            // b. Else,
            // i. Let isArray be ? IsArray(replacer).
            if is_array(agent, replacer, gc.nogc()).unbind()? {
                let scoped_replacer = replacer.scope(agent, gc.nogc());
                // ii. If isArray is true, then
                // 2. Let len be ? LengthOfArrayLike(replacer).
                let len = length_of_array_like(agent, replacer.unbind(), gc.reborrow()).unbind()?;
                // 1. Set PropertyList to a new empty List.
                property_list =
                    Some(Vec::<PropertyKey>::with_capacity(len as usize).scope(agent, gc.nogc()));
                // 3. Let k be 0.
                // 4. Repeat, while k < len,
                // h. Set k to k + 1.
                for k in 0..len {
                    // a. Let prop be ! ToString(𝔽(k)).
                    let prop = PropertyKey::from(SmallInteger::try_from(k).unwrap());
                    // b. Let v be ? Get(replacer, prop).
                    let v = get(agent, scoped_replacer.get(agent), prop, gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                    // c. Let item be undefined.
                    let item = if let Ok(v) = String::try_from(v) {
                        // d. If v is a String, then
                        // i. Set item to v.
                        Some(to_property_key_simple(agent, v, gc.nogc()).unwrap())
                    } else if let Ok(v) = Number::try_from(v) {
                        // e. Else if v is a Number, then
                        // i. Set item to ! ToString(v).
                        Some(
                            to_property_key(agent, v.unbind(), gc.reborrow())
                                .unwrap()
                                .unbind()
                                .bind(gc.nogc()),
                        )
                    } else if let Ok(v) = PrimitiveObject::try_from(v) {
                        // f. Else if v is an Object, then
                        // i. If v has a [[StringData]] or [[NumberData]] internal slot, set item to ? ToString(v).
                        if v.is_string_object(agent) || v.is_number_object(agent) {
                            Some(
                                to_property_key(agent, v.unbind(), gc.reborrow())
                                    .unwrap()
                                    .unbind()
                                    .bind(gc.nogc()),
                            )
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    // g. If item is not undefined and PropertyList does not contain item, then
                    // i. Append item to PropertyList.
                    if let Some(item) = item {
                        let property_list = property_list.as_mut().unwrap();
                        if !property_list.iter(agent).any(|x| x.get(gc.nogc()) == item) {
                            property_list.push(agent, item);
                        }
                    }
                }
                // SAFETY: scoped_replacer is not shared.
                let _ = unsafe { scoped_replacer.take(agent) };
            }
            None
        } else {
            None
        };

        // SAFETY: space is not shared.
        let space = unsafe { space.take(agent) }.bind(gc.nogc());
        // 6. If space is an Object, then
        let space = if let Ok(space) = PrimitiveObject::try_from(space) {
            if space.is_number_object(agent) {
                // a. If space has a [[NumberData]] internal slot, then
                // i. Set space to ? ToNumber(space).
                Some(
                    to_number(agent, space.unbind(), gc.reborrow())
                        .unbind()?
                        .into_primitive()
                        .bind(gc.nogc()),
                )
            } else if space.is_string_object(agent) {
                // b. Else if space has a [[StringData]] internal slot, then
                // i. Set space to ? ToString(space).
                Some(
                    to_string(agent, space.unbind(), gc.reborrow())
                        .unbind()?
                        .into_primitive()
                        .bind(gc.nogc()),
                )
            } else {
                None
            }
        } else if let Ok(space) = Number::try_from(space) {
            Some(space.into_primitive())
        } else if let Ok(space) = String::try_from(space) {
            Some(space.into_primitive())
        } else {
            None
        };

        let gap: Box<str> = space.map_or("".into(), |space| {
            // 7. If space is a Number, then
            if let Ok(space) = Number::try_from(space) {
                // a. Let spaceMV be ! ToIntegerOrInfinity(space).
                let space_mv = to_integer_or_infinity_number(agent, space);
                // b. Set spaceMV to min(10, spaceMV).
                // c. If spaceMV < 1, let gap be the empty String; otherwise let gap be the String value containing spaceMV occurrences of the code unit 0x0020 (SPACE).
                let space_mv = space_mv.into_i64().clamp(0, 10) as usize;
                " ".repeat(space_mv as usize).into()
            } else if let Ok(space) = String::try_from(space) {
                // 8. Else if space is a String, then
                let space = space.to_string_lossy(agent);
                // a. If the length of space ≤ 10, let gap be space; otherwise let gap be the substring of space from 0 to 10.
                if space.len() <= 10 {
                    space.into()
                } else {
                    space[..10].into()
                }
            } else {
                // 9. Else,
                // a. Let gap be the empty String.
                "".into()
            }
        });

        // 10. Let wrapper be OrdinaryObjectCreate(%Object.prototype%).
        let Object::Object(mut wrapper) = ordinary_object_create_with_intrinsics(
            agent,
            Some(ProtoIntrinsics::Object),
            None,
            gc.nogc(),
        ) else {
            unreachable!()
        };
        // SAFETY: value is not shared.
        let mut value = unsafe { value.take(agent) }.bind(gc.nogc());
        // 11. Perform ! CreateDataPropertyOrThrow(wrapper, the empty String, value).
        if wrapper
            .property_storage()
            .set(
                agent,
                wrapper.into_object(),
                String::EMPTY_STRING.to_property_key(),
                PropertyDescriptor::new_data_descriptor(value),
                gc.nogc(),
            )
            .is_err()
        {
            let scoped_value = value.scope(agent, gc.nogc());
            let scoped_wrapper = wrapper.scope(agent, gc.nogc());
            agent.gc(gc.reborrow());
            // SAFETY: Not shared.
            unsafe {
                wrapper = scoped_wrapper.take(agent).bind(gc.nogc());
                value = scoped_value.take(agent).bind(gc.nogc());
            }
            if let Err(err) = wrapper.property_storage().set(
                agent,
                wrapper.into_object(),
                String::EMPTY_STRING.to_property_key(),
                PropertyDescriptor::new_data_descriptor(value),
                gc.nogc(),
            ) {
                return Err(agent.throw_allocation_exception(err, gc.into_nogc()));
            };
        }
        // 12. Let state be the JSON Serialization Record { [[ReplacerFunction]]: ReplacerFunction, [[Stack]]: stack, [[Indent]]: indent, [[Gap]]: gap, [[PropertyList]]: PropertyList }.
        let mut state = JSONSerializationRecord {
            result: Wtf8Buf::new(),
            replacer_function,
            stack,
            // 2. Let indent be the empty String.
            indent: Default::default(),
            gap,
            property_list,
        };
        // 13. Return ? SerializeJSONProperty(state, the empty String, wrapper).
        let key = String::EMPTY_STRING
            .to_property_key()
            .scope(agent, gc.nogc());
        let value_p = get_serializable_json_property_value(
            agent,
            state.replacer_function.clone(),
            key,
            wrapper.into_object().unbind(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        if let Some(value_p) = value_p {
            serialize_json_property_value(agent, &mut state, value_p.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            Ok(String::from_wtf8_buf(agent, state.result, gc.into_nogc()).into_value())
        } else {
            Ok(Value::Undefined)
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
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

///### [25.5.1.1 InternalizeJSONProperty ( holder, name, reviver )](https://tc39.es/ecma262/#sec-internalizejsonproperty)
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
    holder: Scoped<Object>,
    name: impl IndirectPropertyKey,
    reviver: Scoped<Function>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    // 1. Let val be ? Get(holder, name).
    let val = get(
        agent,
        holder.get(agent),
        name.get_key(agent, gc.nogc()).unbind(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 2. If val is an Object, then
    let val = if let Ok(val) = Object::try_from(val) {
        // a. Let isArray be ? IsArray(val).
        // b. If isArray is true, then
        let scoped_val = val.scope(agent, gc.nogc());
        if is_array(agent, val, gc.nogc()).unbind()? {
            // i. Let len be ? LengthOfArrayLike(val).
            let len = length_of_array_like(agent, val.unbind(), gc.reborrow()).unbind()?;
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
                )
                .unbind()?
                .bind(gc.nogc());

                // 3. If newElement is undefined, then
                if new_element.is_undefined() {
                    // a. Perform ? val.[[Delete]](prop).
                    // Note: Deleting from an Array never calls into JavaScript.
                    scoped_val
                        .get(agent)
                        .internal_delete(agent, prop.unwrap(), gc.reborrow())
                        .unbind()?;
                } else {
                    // 4. Else,
                    // a. Perform ? CreateDataProperty(val, prop, newElement).
                    // Note: Defining a property on an Array never calls into
                    // JavaScript.
                    create_data_property(
                        agent,
                        scoped_val.get(agent),
                        prop.unwrap(),
                        new_element.unbind(),
                        gc.reborrow(),
                    )
                    .unbind()?;
                }

                // 5. Set I to I + 1.
                i += 1;
            }
        } else {
            // c. Else,
            // i. Let keys be ? EnumerableOwnProperties(val, KEY).
            let keys = scoped_enumerable_own_keys(agent, scoped_val.clone(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());

            // ii. For each String P of keys, do
            for p in keys.iter(agent) {
                // 1. Let newElement be ? InternalizeJSONProperty(val, P, reviver).
                let new_element = internalize_json_property(
                    agent,
                    scoped_val.clone(),
                    p,
                    reviver.clone(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());

                // 2. If newElement is undefined, then
                if new_element.is_undefined() {
                    // a. Perform ? val.[[Delete]](P).
                    scoped_val
                        .get(agent)
                        .internal_delete(agent, p.get_key(agent, gc.nogc()).unbind(), gc.reborrow())
                        .unbind()?;
                } else {
                    // 3. Else,
                    // a. Perform ? CreateDataProperty(val, P, newElement).
                    create_data_property(
                        agent,
                        scoped_val.get(agent),
                        p.get_key(agent, gc.nogc()).unbind(),
                        new_element.unbind(),
                        gc.reborrow(),
                    )
                    .unbind()?;
                }
            }
        }
        // SAFETY: scoped_val was shared to other internalise calls as the
        // holder object but those calls have finished and do not store
        // scoped_val anywhere.
        unsafe { scoped_val.take(agent) }
            .into_value()
            .bind(gc.nogc())
    } else {
        val
    };

    // 3. Return ? Call(reviver, holder, « name, val »).
    // Note: Because this call gets holder as `this`, it can do dirty things to
    // it, such as `holder[other_key] = new Proxy()`.
    let name = name
        .get_key(agent, gc.nogc())
        .convert_to_value(agent, gc.nogc());
    call_function(
        agent,
        reviver.get(agent),
        holder.get(agent).into_value(),
        Some(ArgumentsList::from_mut_slice(&mut [
            name.into_value().unbind(),
            val.unbind(),
        ])),
        gc,
    )
}

struct JSONSerializationRecord<'a> {
    result: Wtf8Buf,
    replacer_function: Option<Scoped<'a, Function<'static>>>,
    stack: ScopedCollection<'a, Vec<Value<'static>>>,
    indent: Box<str>,
    gap: Box<str>,
    property_list: Option<ScopedCollection<'a, Vec<PropertyKey<'static>>>>,
}

trait IndirectPropertyKey {
    fn get_key<'a>(&self, agent: &Agent, gc: NoGcScope<'a, '_>) -> PropertyKey<'a>;
}

impl IndirectPropertyKey for ScopedPropertyKey<'_> {
    fn get_key<'a>(&self, _: &Agent, gc: NoGcScope<'a, '_>) -> PropertyKey<'a> {
        self.get(gc)
    }
}

impl IndirectPropertyKey for Scoped<'_, PropertyKey<'static>> {
    fn get_key<'a>(&self, agent: &Agent, gc: NoGcScope<'a, '_>) -> PropertyKey<'a> {
        self.get(agent).bind(gc)
    }
}

/// ### [25.5.2.2 SerializeJSONProperty ( state, key, holder )](https://tc39.es/ecma262/#sec-serializejsonproperty)
///
/// The abstract operation SerializeJSONProperty takes arguments state (a JSON
/// Serialization Record), key (a String), and holder (an Object) and returns
/// either a normal completion containing either a serializable Value or
/// undefined, or a throw completion.
///
/// > Note: This performs steps 1 through 4, and 10 and 12 of the
/// > SerializeJSONProperty abstract operation.
fn get_serializable_json_property_value<'a>(
    agent: &mut Agent,
    replacer_function: Option<Scoped<Function>>,
    key: impl IndirectPropertyKey,
    holder: Object,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Value<'a>>> {
    let holder = holder.bind(gc.nogc());
    let scoped_holder = replacer_function
        .as_ref()
        .map(|_| holder.scope(agent, gc.nogc()));
    // 1. Let value be ? Get(holder, key).
    let mut value = get(
        agent,
        holder.unbind(),
        key.get_key(agent, gc.nogc()).unbind(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 2. If value is an Object or value is a BigInt, then
    if value.is_object() || value.is_bigint() {
        let scoped_value = value.scope(agent, gc.nogc());
        // a. Let toJSON be ? GetV(value, "toJSON").
        let to_json = get_v(
            agent,
            value.unbind(),
            BUILTIN_STRING_MEMORY.toJSON.to_property_key(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // b. If IsCallable(toJSON) is true, then
        if let Some(to_json) = is_callable(to_json, gc.nogc()) {
            // i. Set value to ? Call(toJSON, value, « key »).
            let key = key
                .get_key(agent, gc.nogc())
                .convert_to_value(agent, gc.nogc());
            value = call_function(
                agent,
                to_json.unbind(),
                scoped_value.get(agent),
                Some(ArgumentsList::from_mut_value(
                    &mut key.into_value().unbind(),
                )),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // SAFETY: scoped_value is not shared.
            let _ = unsafe { scoped_value.take(agent) };
        } else {
            // Return the value back from scoping.
            // SAFETY: scoped_value is not shared.
            value = unsafe { scoped_value.take(agent) };
        }
    }
    // 3. If state.[[ReplacerFunction]] is not undefined, then
    if let Some(replacer_function) = replacer_function {
        // a. Set value to ? Call(state.[[ReplacerFunction]], holder, « key, value »).
        let key = key
            .get_key(agent, gc.nogc())
            .convert_to_value(agent, gc.nogc());
        value = call_function(
            agent,
            replacer_function.get(agent),
            // SAFETY: scoped_holder is not shared.
            unsafe { scoped_holder.unwrap().take(agent).into_value().unbind() },
            Some(ArgumentsList::from_mut_slice(&mut [
                key.into_value().unbind(),
                value.unbind(),
            ])),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
    }

    // 4. If value is an Object, then
    if let Ok(obj) = PrimitiveObject::try_from(value) {
        match agent[obj].data {
            // a. If value has a [[NumberData]] internal slot, then
            // i. Set value to ? ToNumber(value).
            PrimitiveObjectData::Number(_)
            | PrimitiveObjectData::Integer(_)
            | PrimitiveObjectData::SmallF64(_) => {
                value = to_number(agent, obj.unbind(), gc.reborrow())
                    .unbind()?
                    .into_value()
                    .bind(gc.nogc())
            }
            // b. Else if value has a [[StringData]] internal slot, then
            // i. Set value to ? ToString(value).
            PrimitiveObjectData::String(_) | PrimitiveObjectData::SmallString(_) => {
                value = to_string(agent, obj.unbind(), gc.reborrow())
                    .unbind()?
                    .into_value()
                    .bind(gc.nogc())
            }
            // c. Else if value has a [[BooleanData]] internal slot, then
            // i. Set value to value.[[BooleanData]].
            PrimitiveObjectData::Boolean(bool) => value = Value::Boolean(bool),
            // d. Else if value has a [[BigIntData]] internal slot, then
            // i. Set value to value.[[BigIntData]].
            PrimitiveObjectData::BigInt(bigint) => value = Value::BigInt(bigint),
            PrimitiveObjectData::SmallBigInt(bigint) => value = Value::SmallBigInt(bigint),
            _ => {}
        }
    }

    if value.is_bigint() {
        // 10. If value is a BigInt, throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cannot serialize BigInt to JSON",
            gc.into_nogc(),
        ))
    } else if value.is_undefined() || value.is_symbol() {
        Ok(None)
    } else if is_callable(value, gc.nogc()).is_some() {
        // 11. If value is an Object and IsCallable(value) is false, then
        Ok(None)
    } else {
        Ok(Some(value.unbind().bind(gc.into_nogc())))
    }
}

/// ### [25.5.2.2 SerializeJSONProperty ( state, key, holder )](https://tc39.es/ecma262/#sec-serializejsonproperty)
///
/// > Note: This performs steps 5 through 9, and 11 of the
/// > SerializeJSONProperty abstract operation.
fn serialize_json_property_value<'a, 'b>(
    agent: &mut Agent,
    state: &mut JSONSerializationRecord<'b>,
    value: Value,
    gc: GcScope<'a, 'b>,
) -> JsResult<'a, ()> {
    let value = value.bind(gc.nogc());
    match value {
        // 5. If value is null, return "null".
        Value::Null => {
            state.result.push_str("null");
        }
        // 6. If value is true, return "true".
        Value::Boolean(true) => {
            state.result.push_str("true");
        }
        // 7. If value is false, return "false".
        Value::Boolean(false) => {
            state.result.push_str("false");
        }
        // 8. If value is a String, return QuoteJSONString(value).
        Value::String(_) | Value::SmallString(_) => {
            let value = String::try_from(value).unwrap();
            quote_json_string(agent, &mut state.result, value);
        }
        // 9. If value is a Number, then
        Value::Number(_) | Value::SmallF64(_) | Value::Integer(_) => {
            let value = Number::try_from(value).unwrap();
            // a. If value is finite, return ! ToString(value).
            if value.is_finite(agent) {
                let mut buffer = ryu_js::Buffer::new();
                let value = buffer.format(value.into_f64(agent));
                state.result.push_str(value);
            } else {
                // b. Return "null".
                state.result.push_str("null");
            }
        }
        _ => {
            // 11. If value is an Object and IsCallable(value) is false, then
            // Note: All non-Object and callable values should've returned
            // None from get_serializable_json_property_value.
            let value = Object::try_from(value).unwrap();
            debug_assert!(is_callable(value, gc.nogc()).is_none());
            // a. Let isArray be ? IsArray(value).
            // b. If isArray is true, return ? SerializeJSONArray(state, value).
            if is_array(agent, value, gc.nogc()).unbind()? {
                serialize_json_array(agent, state, value.unbind(), gc)?;
            } else {
                // c. Return ? SerializeJSONObject(state, value).
                serialize_json_object(agent, state, value.unbind(), gc)?;
            }
        }
    }
    Ok(())
}

/// ### [25.5.2.3 QuoteJSONString ( value )](https://tc39.es/ecma262/#sec-quotejsonstring)
///
/// The abstract operation QuoteJSONString takes argument value (a String) and
/// returns a String. It wraps value in 0x0022 (QUOTATION MARK) code units and
/// escapes certain other code units within it. This operation interprets value
/// as a sequence of UTF-16 encoded code points, as described in 6.1.4.
fn quote_json_string(agent: &Agent, product: &mut Wtf8Buf, value: String) {
    product.reserve(value.len(agent) + 2);
    // 1. Let product be the String value consisting solely of the code unit
    //    0x0022 (QUOTATION MARK).
    product.push(CodePoint::from_char('"'));
    // 2. For each code point C of StringToCodePoints(value), do
    for c in value.as_wtf8(agent).code_points() {
        match c.to_u32() {
            // a. If C is listed in the “Code Point” column of Table 81, then
            // i. Set product to the string-concatenation of product and the
            //    escape sequence for C as specified in the “Escape Sequence”
            //    column of the corresponding row.

            // Table 81. JSON Single Character Escape Sequences
            // +------------+------------------------+-----------------+
            // | Code Point | Unicode Character Name | Escape Sequence |
            // +------------+------------------------+-----------------+
            // | U+0008     | Backspace              | \b              |
            0x0008 => product.push_str("\\b"),
            // | U+0009     | CHARACTER TABULATION   | \t              |
            0x0009 => product.push_str("\\t"),
            // | U+000A     | LINE FEED (LF)         | \n              |
            0x000A => product.push_str("\\n"),
            // | U+000C     | FORM FEED (FF)         | \f              |
            0x000C => product.push_str("\\f"),
            // | U+000D     | CARRIAGE RETURN (CR)   | \r              |
            0x000D => product.push_str("\\r"),
            // | U+0022     | QUOTATION MARK         | \"              |
            0x0022 => product.push_str("\\\""),
            // | U+005C     | REVERSE SOLIDUS        | \\              |
            0x005C => product.push_str("\\\\"),
            // +------------+------------------------+-----------------+
            // b. Else if C has a numeric value less than 0x0020 (SPACE) or C
            //    has the same numeric value as a leading surrogate or trailing
            //    surrogate, then
            _ if c.to_u32() < u32::from('\u{0020}') || c.to_char().is_none() => {
                // i. Let unit be the code unit whose numeric value is the
                //    numeric value of C.
                let unit = c.to_u32();
                // ii. Set product to the string-concatenation of product and
                //     UnicodeEscape(unit).
                debug_assert!(unit <= 0xFFFF);
                let unit = unit as u16;
                let mut buf = [0u8; 6];

                fn u16_to_char_code(v: u16) -> u8 {
                    if v < 10 { v as u8 + 48 } else { v as u8 + 87 }
                }

                buf[0] = b'\\';
                buf[1] = b'u';
                buf[5] = u16_to_char_code(unit % 16);
                buf[4] = u16_to_char_code((unit >> 4) % 16);
                buf[3] = u16_to_char_code((unit >> 8) % 16);
                buf[2] = u16_to_char_code(unit >> 12);
                // SAFETY: the buffer contains only valid UTF-8.
                let byte_buf = unsafe { str::from_utf8_unchecked_mut(&mut buf) };

                product.push_str(byte_buf);
            }
            // c. Else,
            // i. Set product to the string-concatenation of product and
            //    UTF16EncodeCodePoint(C).
            _ => product.push(c),
        }
    }
    // 3. Set product to the string-concatenation of product and the code unit
    //    0x0022 (QUOTATION MARK).
    product.push(CodePoint::from_char('"'));
    // 4. Return product.
}

fn quote_property_key(agent: &Agent, product: &mut Wtf8Buf, key: PropertyKey) {
    if let PropertyKey::Integer(key) = key {
        let key = key.into_i64();
        product.reserve(6);
        let string = format!("\"{key}\"");
        product.push_str(&string);
    } else {
        // Symbol keys do not get serialised into JSON.
        debug_assert!(key.is_string());
        // SAFETY: The key is guaranteed to be a non-integer string.
        let key = unsafe { key.into_value_unchecked() };
        quote_json_string(agent, product, String::try_from(key).unwrap())
    }
}

/// ### [25.5.2.5 SerializeJSONObject ( state, value )](https://tc39.es/ecma262/#sec-serializejsonobject)
///
/// The abstract operation SerializeJSONObject takes arguments state (a JSON
/// Serialization Record) and value (an Object) and returns either a normal
/// completion containing a String or a throw completion. It serializes an
/// object.
fn serialize_json_object<'a, 'b>(
    agent: &mut Agent,
    state: &mut JSONSerializationRecord<'b>,
    value: Object<'static>,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, ()> {
    let value = value.bind(gc.nogc());
    // 1. If state.[[Stack]] contains value, throw a TypeError exception
    //    because the structure is cyclical.
    if state
        .stack
        .iter(agent)
        // SAFETY: We only push objects into the stack.
        .any(|x| unsafe { Object::try_from(x.get(gc.nogc())).unwrap_unchecked() } == value)
    {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cyclical structure in JSON",
            gc.into_nogc(),
        ));
    }

    // 2. Append value to state.[[Stack]].
    state.stack.push(agent, value.into_value());

    // 5. If state.[[PropertyList]] is not undefined, then
    // a. Let K be state.[[PropertyList]].
    // 6. Else,
    // a. Let K be ? EnumerableOwnProperties(value, key).
    let k = if let Some(property_list) = &state.property_list {
        property_list.clone()
    } else {
        enumerable_own_keys(agent, value.unbind(), gc.reborrow())
            .unbind()?
            .scope(agent, gc.nogc())
    };

    if k.is_empty(agent) {
        // 9. If partial is empty, then
        // a. Let final be "{}".
        state.stack.pop(agent, gc.nogc());
        state.result.push_str("{}");
        return Ok(());
    }

    let open_string: Box<str>;
    let separator_string: Box<str>;
    let close_string: Box<str>;
    let step_back: Box<str>;

    // a. If state.[[Gap]] is the empty String, then
    let (open, separator, key_value_separator, close) = if state.gap.is_empty() {
        step_back = Default::default();
        // i. Let properties be the String value formed by concatenating
        //    all the element Strings of partial with each adjacent pair of
        //    Strings separated with the code unit 0x002C (COMMA). A comma
        //    is not inserted either before the first String or after the
        //    last String.
        // ii. Let final be the string-concatenation of "{", properties,
        //     and "}".
        ("{", ",", ":", "}")
    } else {
        // 3. Let stepBack be state.[[Indent]].
        // 4. Set state.[[Indent]] to the string-concatenation of state.[[Indent]] and state.[[Gap]].
        let mut new_ident =
            std::string::String::with_capacity(state.indent.len() + state.gap.len());
        new_ident.push_str(&state.indent);
        new_ident.push_str(&state.gap);
        step_back = core::mem::replace(&mut state.indent, new_ident.into_boxed_str());

        // b. Else,
        // i. Let separator be the string-concatenation of the code unit
        //    0x002C (COMMA), the code unit 0x000A (LINE FEED), and
        //    state.[[Indent]].
        separator_string = format!(",\n{}", &state.indent).into_boxed_str();
        // ii. Let properties be the String value formed by concatenating
        //     all the element Strings of partial with each adjacent pair
        //     of Strings separated with separator. The separator String is
        //     not inserted either before the first String or after the
        //     last String.
        // iii. Let final be the string-concatenation of "{", the code unit
        //      0x000A (LINE FEED), state.[[Indent]], properties, the code
        //      unit 0x000A (LINE FEED), stepBack, and "}".
        open_string = format!("{{\n{}", &state.indent).into_boxed_str();
        close_string = format!("\n{}}}", &step_back).into_boxed_str();
        (
            open_string.as_ref(),
            separator_string.as_ref(),
            ": ",
            close_string.as_ref(),
        )
    };

    let mut first_inserted = false;

    // Assume most properties will indeed get serialised properly.
    state.result.reserve(
        open.len()
            + close.len()
            + k.len(agent) * (separator.len() + 3 + key_value_separator.len() + 1),
    );
    // 7. Let partial be a new empty List.
    // 8. For each element P of K, do
    for p in k.iter(agent) {
        // a. Let strP be ? SerializeJSONProperty(state, P, value).
        let value_p = get_serializable_json_property_value(
            agent,
            state.replacer_function.clone(),
            p,
            // SAFETY: We only push the objects onto the stack.
            unsafe {
                Object::try_from(state.stack.last(agent, gc.nogc()).unwrap())
                    .unwrap_unchecked()
                    .unbind()
            },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // b. If strP is not undefined, then
        let Some(value_p) = value_p else {
            continue;
        };

        if !first_inserted {
            first_inserted = true;
            state.result.push_str(open);
        } else {
            state.result.push_str(separator);
        }

        // i. Let member be QuoteJSONString(P).
        quote_property_key(agent, &mut state.result, p.get(gc.nogc()));
        // ii. Set member to the string-concatenation of member and ":".
        // iii. If state.[[Gap]] is not the empty String, then
        // 1. Set member to the string-concatenation of member and the code unit 0x0020 (SPACE).
        state.result.push_str(key_value_separator);
        // iv. Set member to the string-concatenation of member and strP.
        serialize_json_property_value(agent, state, value_p.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // let member = String::concat(agent, member, gc.nogc()).unbind();
        // v. Append member to partial.
    }

    // 11. Remove the last element of state.[[Stack]].
    state.stack.pop(agent, gc.nogc());

    if state.gap.is_empty() {
        // 12. Set state.[[Indent]] to stepBack.
        state.indent = step_back;
        // 13. Return final.
    }

    // 9. If partial is empty, then
    if !first_inserted {
        // a. Let final be "{}".
        state.result.push_str("{}");
    } else {
        state.result.push_str(close);
    }
    Ok(())
}

/// ### [25.5.2.6 SerializeJSONArray ( state, value )](https://tc39.es/ecma262/#sec-serializejsonarray)
///
/// The abstract operation SerializeJSONArray takes arguments state (a JSON
/// Serialization Record) and value (an ECMAScript language value) and returns
/// either a normal completion containing a String or a throw completion. It
/// serializes an array.
fn serialize_json_array<'a, 'b>(
    agent: &mut Agent,
    state: &mut JSONSerializationRecord<'b>,
    value: Object<'static>,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, ()> {
    let value = value.bind(gc.nogc());
    // 1. If state.[[Stack]] contains value, throw a TypeError exception because the structure is cyclical.
    if state
        .stack
        .iter(agent)
        // SAFETY: We only push objects into the stack.
        .any(|x| unsafe { Object::try_from(x.get(gc.nogc())).unwrap_unchecked() } == value)
    {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Cyclical structure in JSON",
            gc.into_nogc(),
        ));
    }
    // 2. Append value to state.[[Stack]].
    state.stack.push(agent, value.into_value());
    // 6. Let len be ? LengthOfArrayLike(value).
    let len = length_of_array_like(agent, value.unbind(), gc.reborrow()).unbind()? as u64;

    // 9. If partial is empty, then
    // Note: We skip all the bookkeeping work when dealing with empty arrays.
    if len == 0 {
        // a. Let final be "[]".
        // 11. Remove the last element of state.[[Stack]].
        state.stack.pop(agent, gc.nogc());
        // 12. Set state.[[Indent]] to stepBack.
        // Note: We've not yet changed indent, so we shouldn't restore it
        // either.
        // 13. Return final.
        state.result.push_str("[]");
        return Ok(());
    }

    let open_string: Box<str>;
    let separator_string: Box<str>;
    let close_string: Box<str>;
    let step_back: Box<str>;

    // a. If state.[[Gap]] is the empty String, then
    let (open, separator, close) = if state.gap.is_empty() {
        step_back = Default::default();
        // i. Let properties be the String value formed by concatenating all
        //    the element Strings of partial with each adjacent pair of Strings
        //    separated with the code unit 0x002C (COMMA). A comma is not
        //    inserted either before the first String or after the last String.
        // ii. Let final be the string-concatenation of "[", properties, and
        //     "]".
        ("[", ",", "]")
    } else {
        // 3. Let stepBack be state.[[Indent]].
        // 4. Set state.[[Indent]] to the string-concatenation of state.[[Indent]] and state.[[Gap]].
        let mut new_ident =
            std::string::String::with_capacity(state.indent.len() + state.gap.len());
        new_ident.push_str(&state.indent);
        new_ident.push_str(&state.gap);
        step_back = core::mem::replace(&mut state.indent, new_ident.into_boxed_str());

        // b. Else,
        // i. Let separator be the string-concatenation of the code unit 0x002C
        //    (COMMA), the code unit 0x000A (LINE FEED), and state.[[Indent]].
        separator_string = format!(",\n{}", &state.indent).into_boxed_str();
        // ii. Let properties be the String value formed by concatenating all
        //     the element Strings of partial with each adjacent pair of
        //     Strings separated with separator. The separator String is not
        //     inserted either before the first String or after the last
        //     String.
        // iii. Let final be the string-concatenation of "[", the code unit
        //      0x000A (LINE FEED), state.[[Indent]], properties, the code unit
        //      0x000A (LINE FEED), stepBack, and "]".
        open_string = format!("[\n{}", &state.indent).into_boxed_str();
        close_string = format!("\n{}]", &step_back).into_boxed_str();
        (
            open_string.as_ref(),
            separator_string.as_ref(),
            close_string.as_ref(),
        )
    };

    // 5. Let partial be a new empty List.
    state
        .result
        .reserve(open.len() + close.len() + (len as usize) * (separator.len() + 1));
    state.result.push_str(open);
    // 7. Let index be 0.
    // 8. Repeat, while index < len,
    for index in 0..len {
        if index > 0 {
            state.result.push_str(separator);
        }
        let key = PropertyKey::try_from(index).unwrap().scope_static();
        // a. Let strP be ? SerializeJSONProperty(state, ! ToString(𝔽(index)), value).
        if let Some(value_p) = get_serializable_json_property_value(
            agent,
            state.replacer_function.clone(),
            key,
            unsafe {
                Object::try_from(state.stack.last(agent, gc.nogc()).unwrap())
                    .unwrap_unchecked()
                    .unbind()
            },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc())
        {
            // c. Else,
            // i. Append strP to partial.
            serialize_json_property_value(agent, state, value_p.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        } else {
            // b. If strP is undefined, then
            // i. Append "null" to partial.
            state.result.push_str("null");
        }
        // d. Set index to index + 1.
    }
    state.result.push_str(close);
    // 11. Remove the last element of state.[[Stack]].
    state.stack.pop(agent, gc.nogc());
    // 12. Set state.[[Indent]] to stepBack.
    state.indent = step_back;
    // 13. Return final.
    Ok(())
}

pub(crate) fn value_from_json<'gc>(
    agent: &mut Agent,
    json: &sonic_rs::Value,
    gc: NoGcScope<'gc, '_>,
) -> Value<'gc> {
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
                    agent, array_obj, prop, js_value, None, gc,
                ));
            }
            array_obj.into_value()
        }
        sonic_rs::JsonType::Object => {
            let json_object = json.as_object().unwrap();
            let Object::Object(object) = ordinary_object_create_with_intrinsics(
                agent,
                Some(ProtoIntrinsics::Object),
                None,
                gc,
            ) else {
                unreachable!()
            };
            for (key, value) in json_object.iter() {
                let prop = PropertyKey::from_str(agent, key, gc);
                let js_value = value_from_json(agent, value, gc);
                unwrap_try(try_create_data_property(
                    agent, object, prop, js_value, None, gc,
                ));
            }
            object.into()
        }
    }
}
