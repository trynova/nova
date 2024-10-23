// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::{
                get_iterator, if_abrupt_close_iterator, iterator_close, iterator_step_value,
            },
            operations_on_objects::{
                create_array_from_list, create_data_property_or_throw, define_property_or_throw,
                enumerable_own_properties, enumerable_properties_kind, get, get_method,
                group_by_property, has_own_property,
                integrity::{Frozen, Sealed},
                set, set_integrity_level, test_integrity_level,
            },
            testing_and_comparison::{require_object_coercible, same_value},
            type_conversion::{to_object, to_property_key, to_property_key_simple},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            InternalMethods, IntoFunction, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyDescriptor, PropertyKey, String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::{IntrinsicConstructorIndexes, ObjectEntry, WellKnownSymbolIndexes},
};

pub(crate) struct ObjectConstructor;

impl Builtin for ObjectConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Object;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for ObjectConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Object;
}

struct ObjectAssign;

impl Builtin for ObjectAssign {
    const NAME: String = BUILTIN_STRING_MEMORY.assign;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::assign);
}

struct ObjectCreate;

impl Builtin for ObjectCreate {
    const NAME: String = BUILTIN_STRING_MEMORY.create;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::create);
}
struct ObjectDefineProperties;

impl Builtin for ObjectDefineProperties {
    const NAME: String = BUILTIN_STRING_MEMORY.defineProperties;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_properties);
}
struct ObjectDefineProperty;

impl Builtin for ObjectDefineProperty {
    const NAME: String = BUILTIN_STRING_MEMORY.defineProperty;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_property);
}
struct ObjectEntries;

impl Builtin for ObjectEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::entries);
}
struct ObjectFreeze;

impl Builtin for ObjectFreeze {
    const NAME: String = BUILTIN_STRING_MEMORY.freeze;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::freeze);
}
struct ObjectFromEntries;

impl Builtin for ObjectFromEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.fromEntries;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::from_entries);
}
struct ObjectGetOwnPropertyDescriptor;

impl Builtin for ObjectGetOwnPropertyDescriptor {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_descriptor);
}
struct ObjectGetOwnPropertyDescriptors;

impl Builtin for ObjectGetOwnPropertyDescriptors {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptors;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(ObjectConstructor::get_own_property_descriptors);
}
struct ObjectGetOwnPropertyNames;

impl Builtin for ObjectGetOwnPropertyNames {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyNames;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_names);
}
struct ObjectGetOwnPropertySymbols;

impl Builtin for ObjectGetOwnPropertySymbols {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertySymbols;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_symbols);
}
struct ObjectGetPrototypeOf;

impl Builtin for ObjectGetPrototypeOf {
    const NAME: String = BUILTIN_STRING_MEMORY.getPrototypeOf;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_prototype_of);
}
struct ObjectGroupBy;

impl Builtin for ObjectGroupBy {
    const NAME: String = BUILTIN_STRING_MEMORY.groupBy;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::group_by);
}
struct ObjectHasOwn;

impl Builtin for ObjectHasOwn {
    const NAME: String = BUILTIN_STRING_MEMORY.hasOwn;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::has_own);
}
struct ObjectIs;

impl Builtin for ObjectIs {
    const NAME: String = BUILTIN_STRING_MEMORY.is;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is);
}
struct ObjectIsExtensible;

impl Builtin for ObjectIsExtensible {
    const NAME: String = BUILTIN_STRING_MEMORY.isExtensible;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_extensible);
}
struct ObjectIsFrozen;

impl Builtin for ObjectIsFrozen {
    const NAME: String = BUILTIN_STRING_MEMORY.isFrozen;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_frozen);
}
struct ObjectIsSealed;

impl Builtin for ObjectIsSealed {
    const NAME: String = BUILTIN_STRING_MEMORY.isSealed;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_sealed);
}
struct ObjectKeys;

impl Builtin for ObjectKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::keys);
}
struct ObjectPreventExtensions;

impl Builtin for ObjectPreventExtensions {
    const NAME: String = BUILTIN_STRING_MEMORY.preventExtensions;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::prevent_extensions);
}

struct ObjectSeal;

impl Builtin for ObjectSeal {
    const NAME: String = BUILTIN_STRING_MEMORY.seal;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::seal);
}
struct ObjectSetPrototypeOf;

impl Builtin for ObjectSetPrototypeOf {
    const NAME: String = BUILTIN_STRING_MEMORY.setPrototypeOf;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::set_prototype_of);
}
struct ObjectValues;

impl Builtin for ObjectValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::values);
}

impl ObjectConstructor {
    /// ### [20.1.1.1 Object ( \[ value \] )](https://tc39.es/ecma262/#sec-object-value)
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let value = arguments.get(0);
        // 1. If NewTarget is neither undefined nor the active function object, then
        if new_target.is_some()
            && new_target
                != agent
                    .running_execution_context()
                    .function
                    .map(|obj| obj.into_object())
        {
            // a. Return ? OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%").
            ordinary_create_from_constructor(
                agent,
                // SAFETY: 'new_target' is checked to be is_some() above
                unsafe { new_target.unwrap_unchecked() }.try_into().unwrap(),
                ProtoIntrinsics::Object,
            )
            .map(|value| value.into_value())
        } else if value == Value::Undefined || value == Value::Null {
            // 2. If value is either undefined or null, return OrdinaryObjectCreate(%Object.prototype%).
            Ok(
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None)
                    .into_value(),
            )
        } else {
            // 3. Return ! ToObject(value).
            Ok(to_object(agent, value).unwrap().into_value())
        }
    }

    /// ### [20.1.2.1 Object.assign ( target, ...sources )](https://tc39.es/ecma262/#sec-object.assign)
    ///
    /// This function copies the values of all of the enumerable own properties
    /// from one or more source objects to a target object.
    fn assign(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let target = arguments.get(0);
        // 1. Let to be ? ToObject(target).
        let to = to_object(agent, target)?;
        // 2. If only one argument was passed, return to.
        if arguments.len() <= 1 {
            return Ok(to.into_value());
        }
        let sources = &arguments[1..];
        // 3. For each element nextSource of sources, do
        for next_source in sources {
            // a. If nextSource is neither undefined nor null, then
            if next_source.is_undefined() || next_source.is_null() {
                continue;
            }
            // i. Let from be ! ToObject(nextSource).
            let from = to_object(agent, *next_source)?;
            // ii. Let keys be ? from.[[OwnPropertyKeys]]().
            let keys = from.internal_own_property_keys(agent)?;
            // iii. For each element nextKey of keys, do
            for next_key in keys {
                // 1. Let desc be ? from.[[GetOwnProperty]](nextKey).
                let desc = from.internal_get_own_property(agent, next_key)?;
                // 2. If desc is not undefined and desc.[[Enumerable]] is true, then
                let Some(desc) = desc else {
                    continue;
                };
                if desc.enumerable != Some(true) {
                    continue;
                }
                // a. Let propValue be ? Get(from, nextKey).
                let prop_value = get(agent, from, next_key)?;
                // b. Perform ? Set(to, nextKey, propValue, true).
                set(agent, to, next_key, prop_value, true)?;
            }
        }
        // 4. Return to.
        Ok(to.into_value())
    }

    fn create(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let obj: OrdinaryObject = if o == Value::Null {
            agent.heap.create_null_object(&[])
        } else if let Ok(o) = Object::try_from(o) {
            agent.heap.create_object_with_prototype(o, &[])
        } else {
            let error_message = format!(
                "{} is not an object or null",
                o.string_repr(agent).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
        };
        let properties = arguments.get(1);
        if properties != Value::Undefined {
            object_define_properties(agent, obj, properties)?;
        }
        Ok(obj.into_value())
    }

    /// ### [20.1.2.3 Object.defineProperties ( O, Properties )](https://tc39.es/ecma262/#sec-object.defineproperties)
    ///
    /// This function adds own properties and/or updates the attributes of
    /// existing own properties of an object.
    fn define_properties(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let properties = arguments.get(1);
        // 1. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            let error_message = format!("{} is not an object", o.string_repr(agent).as_str(agent));
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
        };
        // 2. Return ? ObjectDefineProperties(O, Properties).
        let result = object_define_properties(agent, o, properties)?;
        Ok(result.into_value())
    }

    /// ### [20.1.2.4 Object.defineProperty ( O, P, Attributes )](https://tc39.es/ecma262/#sec-object.defineproperty)
    ///
    /// This function adds an own property and/or updates the attributes of an
    /// existing own property of an object.
    fn define_property(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let p = arguments.get(1);
        let attributes = arguments.get(2);
        // 1. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            let error_message = format!("{} is not an object", o.string_repr(agent).as_str(agent));
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
        };
        // 2. Let key be ? ToPropertyKey(P).
        let key = to_property_key(agent, p)?;
        // 3. Let desc be ? ToPropertyDescriptor(Attributes).
        let desc = PropertyDescriptor::to_property_descriptor(agent, attributes)?;
        // 4. Perform ? DefinePropertyOrThrow(O, key, desc).
        define_property_or_throw(agent, o, key, desc)?;
        // 5. Return O.
        Ok(o.into_value())
    }

    fn entries(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o)?;
        // 2. Let entryList be ? EnumerableOwnProperties(obj, KEY+VALUE).
        let entry_list = enumerable_own_properties::<
            enumerable_properties_kind::EnumerateKeysAndValues,
        >(agent, obj)?;
        // 3. Return CreateArrayFromList(entryList).
        Ok(create_array_from_list(agent, &entry_list).into_value())
    }

    /// ### [20.1.2.6 Object.freeze ( O )](https://tc39.es/ecma262/#sec-object.freeze)
    fn freeze(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 2. Let status be ? SetIntegrityLevel(O, FROZEN).
        let status = set_integrity_level::<Frozen>(agent, o)?;
        if !status {
            // 3. If status is false, throw a TypeError exception.
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not freeze object",
            ))
        } else {
            // 4. Return O.
            Ok(o.into_value())
        }
    }

    /// ### [20.1.2.7 Object.fromEntries ( iterable )](https://tc39.es/ecma262/#sec-object.fromentries)
    fn from_entries(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let iterable = arguments.get(0);
        // Fast path: Simple, dense array of N simple, dense arrays.
        if let Value::Array(entries_array) = iterable {
            let array_prototype = agent.current_realm().intrinsics().array_prototype();
            let intrinsic_array_iterator = agent
                .current_realm()
                .intrinsics()
                .array_prototype_values()
                .into_function();
            let array_iterator = get_method(
                agent,
                array_prototype.into_value(),
                WellKnownSymbolIndexes::Iterator.into(),
            )?;
            // SAFETY: If the iterator of the array is the intrinsic array
            // values iterator and the array is simple and dense, then we know
            // the behaviour of the iterator (access elements one by one) and
            // we know that accessing the elements will not trigger calls into
            // JavaScript. Hence, we can access the elements directly.
            if array_iterator == Some(intrinsic_array_iterator)
                && entries_array.is_simple(agent)
                && entries_array.is_dense(agent)
            {
                let entries_elements = &agent[agent[entries_array].elements];
                // Note: Separate vector for keys to detect duplicates.
                // This is optimal until ~20 keys, after which a HashMap would
                // be better.
                let mut entry_keys: Vec<PropertyKey> = Vec::with_capacity(entries_elements.len());
                let mut object_entries: Vec<ObjectEntry> =
                    Vec::with_capacity(entries_elements.len());
                // Fast path is valid if each entry in the array is itself a
                // simple and dense array that contains a valid property key
                // and value.
                // If these expectations are invalidated, we must go back to
                // the generic iterator path.
                let mut valid = true;
                for entry_element in entries_elements {
                    // SAFETY: Array is a simple, dense array. All values are
                    // defined.
                    let entry_element = entry_element.unwrap();
                    let entry_element_array =
                        if let Value::Array(entry_element_array) = entry_element {
                            // Note: We check length to equal 2 because it's
                            // the common case and it ensures simple and dense
                            // checking does not iterate a uselessly long
                            // array.
                            if entry_element_array.len(agent) != 2
                                || !entry_element_array.is_simple(agent)
                                || !entry_element_array.is_dense(agent)
                            {
                                valid = false;
                                break;
                            }
                            entry_element_array
                        } else {
                            valid = false;
                            break;
                        };
                    let key_value_elements = &agent[agent[entry_element_array].elements];
                    let key = key_value_elements.first().unwrap().unwrap();
                    let key = to_property_key_simple(agent, key);
                    let Some(key) = key else {
                        valid = false;
                        break;
                    };
                    let value = key_value_elements.last().unwrap().unwrap();
                    let entry = ObjectEntry::new_data_entry(key, value);
                    let existing = entry_keys
                        .iter()
                        .enumerate()
                        .find(|(_, entry)| **entry == key);
                    if let Some((index, _)) = existing {
                        object_entries[index] = entry;
                    } else {
                        object_entries.push(entry);
                        entry_keys.push(key);
                    }
                }
                if valid {
                    let object = agent.heap.create_object_with_prototype(
                        agent
                            .current_realm()
                            .intrinsics()
                            .object_prototype()
                            .into_object(),
                        &object_entries,
                    );
                    return Ok(object.into_value());
                }
            }
        }
        // 1. Perform ? RequireObjectCoercible(iterable).
        require_object_coercible(agent, iterable)?;
        // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        let obj =
            ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);
        // 3. Assert: obj is an extensible ordinary object with no own properties.
        let obj = OrdinaryObject::try_from(obj).unwrap();
        debug_assert!(obj.internal_own_property_keys(agent).unwrap().is_empty());
        // 4. Let closure be a new Abstract Closure with parameters (key,
        //    value) that captures obj and performs the following steps when
        //    called:
        // 5. Let adder be CreateBuiltinFunction(closure, 2, "", « »).
        // 6. Return ? AddEntriesFromIterable(obj, iterable, adder).
        add_entries_from_iterable_from_entries(agent, obj, iterable).map(|obj| obj.into_value())
    }

    /// ### [20.1.2.8 Object.getOwnPropertyDescriptor ( O, P )](https://tc39.es/ecma262/#sec-object.getownpropertydescriptor)
    fn get_own_property_descriptor(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = arguments.get(0);
        let p = arguments.get(1);
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o)?;
        // 2. Let key be ? ToPropertyKey(P).
        let key = to_property_key(agent, p)?;
        // 3. Let desc be ? obj.[[GetOwnProperty]](key).
        let desc = obj.internal_get_own_property(agent, key)?;
        // 4. Return FromPropertyDescriptor(desc).
        Ok(PropertyDescriptor::from_property_descriptor(desc, agent)
            .map_or(Value::Undefined, |obj| obj.into_value()))
    }

    /// ### [20.1.2.9 Object.getOwnPropertyDescriptors ( O )](https://tc39.es/ecma262/#sec-object.getownpropertydescriptors)
    fn get_own_property_descriptors(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = arguments.get(0);
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o)?;
        // 2. Let ownKeys be ? obj.[[OwnPropertyKeys]]().
        let own_keys = obj.internal_own_property_keys(agent)?;

        let mut descriptors = Vec::with_capacity(own_keys.len());
        // 4. For each element key of ownKeys, do
        for key in own_keys {
            // a. Let desc be ? obj.[[GetOwnProperty]](key).
            let desc = obj.internal_get_own_property(agent, key)?;
            // b. Let descriptor be FromPropertyDescriptor(desc).
            let descriptor = PropertyDescriptor::from_property_descriptor(desc, agent);
            // c. If descriptor is not undefined, perform ! CreateDataPropertyOrThrow(descriptors, key, descriptor).
            if let Some(descriptor) = descriptor {
                descriptors.push(ObjectEntry::new_data_entry(key, descriptor.into_value()));
            }
        }
        // 3. Let descriptors be OrdinaryObjectCreate(%Object.prototype%).
        let descriptors = agent.heap.create_object_with_prototype(
            agent
                .current_realm()
                .intrinsics()
                .object_prototype()
                .into_object(),
            &descriptors,
        );
        // 5. Return descriptors.
        Ok(descriptors.into_value())
    }

    /// ### [20.1.2.10 Object.getOwnPropertyNames ( O )](https://tc39.es/ecma262/#sec-object.getownpropertynames)
    fn get_own_property_names(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = arguments.get(0);
        // 1. Return CreateArrayFromList(? GetOwnPropertyKeys(O, STRING)).
        let keys = get_own_string_property_keys(agent, o)?;
        Ok(create_array_from_list(agent, &keys).into_value())
    }

    /// ### [20.1.2.11 Object.getOwnPropertySymbols ( O )](https://tc39.es/ecma262/#sec-object.getownpropertysymbols)
    fn get_own_property_symbols(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = arguments.get(0);
        // 1. Return CreateArrayFromList(? GetOwnPropertyKeys(O, SYMBOL)).
        let keys = get_own_symbol_property_keys(agent, o)?;
        Ok(create_array_from_list(agent, &keys).into_value())
    }

    /// ### [20.1.2.12 Object.getPrototypeOf ( O )](https://tc39.es/ecma262/#sec-object.getprototypeof)
    fn get_prototype_of(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let obj = to_object(agent, arguments.get(0))?;
        obj.internal_get_prototype_of(agent)
            .map(|proto| proto.map_or(Value::Null, |proto| proto.into_value()))
    }

    // ### [20.1.2.13 Object.groupBy ( items, callback )](https://tc39.es/ecma262/#sec-object.groupby)
    fn group_by(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let items = arguments.get(0);
        let callback_fn = arguments.get(1);

        // 1. Let groups be ? GroupBy(items, callback, property).
        let groups = group_by_property(agent, items, callback_fn)?;

        // 2. Let obj be OrdinaryObjectCreate(null).
        let object =
            ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None);

        // 3. For each Record { [[Key]], [[Elements]] } g of groups, do
        for g in groups {
            // a. Let elements be CreateArrayFromList(g.[[Elements]]).
            let elements = create_array_from_list(agent, &g.elements).into_value();

            // b. Perform ! CreateDataPropertyOrThrow(obj, g.[[Key]], elements).
            create_data_property_or_throw(agent, object, g.key, elements)?;
        }

        // 4. Return obj.
        Ok(object.into_value())
    }

    fn has_own(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let obj = to_object(agent, arguments.get(0))?;
        let key = to_property_key(agent, arguments.get(1))?;
        has_own_property(agent, obj, key).map(|result| result.into())
    }

    fn is(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(same_value(agent, arguments.get(0), arguments.get(1)).into())
    }

    fn is_extensible(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let result = match Object::try_from(o) {
            Ok(o) => o.internal_is_extensible(agent)?,
            Err(_) => false,
        };
        Ok(result.into())
    }

    fn is_frozen(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = arguments.get(0);
        let result = match Object::try_from(o) {
            Ok(o) => test_integrity_level::<Frozen>(agent, o)?,
            Err(_) => true,
        };
        Ok(result.into())
    }

    fn is_sealed(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = arguments.get(0);
        let result = match Object::try_from(o) {
            Ok(o) => test_integrity_level::<Sealed>(agent, o)?,
            Err(_) => true,
        };
        Ok(result.into())
    }

    /// ### [20.1.2.19 Object.keys ( O )](https://tc39.es/ecma262/#sec-object.keys)
    fn keys(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        println!("key target: {:?}", o);
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o)?;
        // 2. Let keyList be ? EnumerableOwnProperties(obj, KEY).
        let key_list =
            enumerable_own_properties::<enumerable_properties_kind::EnumerateKeys>(agent, obj)?;
        // 3. Return CreateArrayFromList(keyList).
        Ok(create_array_from_list(agent, &key_list).into_value())
    }

    /// ### [20.1.2.20 Object.preventExtensions ( O )](https://tc39.es/ecma262/#sec-object.preventextensions)
    fn prevent_extensions(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 2. Let status be ? O.[[PreventExtensions]]().
        let status = o.internal_prevent_extensions(agent)?;
        // 3. If status is false, throw a TypeError exception.
        if !status {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not prevent extensions",
            ))
        } else {
            // 4. Return O.
            Ok(o.into_value())
        }
    }

    /// ### [20.1.2.22 Object.seal ( O )](https://tc39.es/ecma262/#sec-object.seal)
    fn seal(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 2. Let status be ? SetIntegrityLevel(O, SEALED).
        let status = set_integrity_level::<Sealed>(agent, o)?;
        if !status {
            // 3. If status is false, throw a TypeError exception.
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not seal object",
            ))
        } else {
            // 4. Return O.
            Ok(o.into_value())
        }
    }

    /// ### [20.1.2.23 Object.setPrototypeOf ( O, proto )](https://tc39.es/ecma262/#sec-object.setprototypeof)
    fn set_prototype_of(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let proto = arguments.get(1);
        // 1. Set O to ? RequireObjectCoercible(O).
        let o = require_object_coercible(agent, o)?;
        // 2. If proto is not an Object and proto is not null, throw a TypeError exception.
        let proto = if let Ok(proto) = Object::try_from(proto) {
            Some(proto)
        } else if proto.is_null() {
            None
        } else {
            let error_message = format!(
                "{} is not an object or null",
                proto.string_repr(agent).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message));
        };
        // 3. If O is not an Object, return O.
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 4. Let status be ? O.[[SetPrototypeOf]](proto).
        let status = o.internal_set_prototype_of(agent, proto)?;
        // 5. If status is false, throw a TypeError exception.
        if !status {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not set prototype",
            ));
        }
        // 6. Return O.
        Ok(o.into_value())
    }

    fn values(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o)?;
        // 2. Let valueList be ? EnumerableOwnProperties(obj, VALUE).
        let value_list =
            enumerable_own_properties::<enumerable_properties_kind::EnumerateValues>(agent, obj)?;
        // 3. Return CreateArrayFromList(valueList).
        Ok(create_array_from_list(agent, &value_list).into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ObjectConstructor>(agent, realm)
            .with_property_capacity(24)
            .with_builtin_function_property::<ObjectAssign>()
            .with_builtin_function_property::<ObjectCreate>()
            .with_builtin_function_property::<ObjectDefineProperties>()
            .with_builtin_function_property::<ObjectDefineProperty>()
            .with_builtin_function_property::<ObjectEntries>()
            .with_builtin_function_property::<ObjectFreeze>()
            .with_builtin_function_property::<ObjectFromEntries>()
            .with_builtin_function_property::<ObjectGetOwnPropertyDescriptor>()
            .with_builtin_function_property::<ObjectGetOwnPropertyDescriptors>()
            .with_builtin_function_property::<ObjectGetOwnPropertyNames>()
            .with_builtin_function_property::<ObjectGetOwnPropertySymbols>()
            .with_builtin_function_property::<ObjectGetPrototypeOf>()
            .with_builtin_function_property::<ObjectGroupBy>()
            .with_builtin_function_property::<ObjectHasOwn>()
            .with_builtin_function_property::<ObjectIs>()
            .with_builtin_function_property::<ObjectIsExtensible>()
            .with_builtin_function_property::<ObjectIsFrozen>()
            .with_builtin_function_property::<ObjectIsSealed>()
            .with_builtin_function_property::<ObjectKeys>()
            .with_builtin_function_property::<ObjectPreventExtensions>()
            .with_prototype_property(object_prototype.into_object())
            .with_builtin_function_property::<ObjectSeal>()
            .with_builtin_function_property::<ObjectSetPrototypeOf>()
            .with_builtin_function_property::<ObjectValues>()
            .build();
    }
}

/// ### [20.1.2.3.1 ObjectDefineProperties ( O, Properties )](https://tc39.es/ecma262/#sec-objectdefineproperties)
///
/// The abstract operation ObjectDefineProperties takes arguments O (an Object)
/// and Properties (an ECMAScript language value) and returns either a normal
/// completion containing an Object or a throw completion.
fn object_define_properties<T: InternalMethods>(
    agent: &mut Agent,
    o: T,
    properties: Value,
) -> JsResult<T> {
    // 1. Let props be ? ToObject(Properties).
    let props = to_object(agent, properties)?;
    // 2. Let keys be ? props.[[OwnPropertyKeys]]().
    let keys = props.internal_own_property_keys(agent)?;
    // 3. Let descriptors be a new empty List.
    let mut descriptors = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. Let propDesc be ? props.[[GetOwnProperty]](nextKey).
        let prop_desc = props.internal_get_own_property(agent, next_key)?;
        // b. If propDesc is not undefined and propDesc.[[Enumerable]] is true, then
        let Some(prop_desc) = prop_desc else {
            continue;
        };
        if prop_desc.enumerable != Some(true) {
            continue;
        }
        // i. Let descObj be ? Get(props, nextKey).
        let desc_obj = get(agent, props, next_key)?;
        // ii. Let desc be ? ToPropertyDescriptor(descObj).
        let desc = PropertyDescriptor::to_property_descriptor(agent, desc_obj)?;
        // iii. Append the Record { [[Key]]: nextKey, [[Descriptor]]: desc } to descriptors.
        descriptors.push((next_key, desc));
    }
    // 5. For each element property of descriptors, do
    for (property_key, property_descriptor) in descriptors {
        // a. Perform ? DefinePropertyOrThrow(O, property.[[Key]], property.[[Descriptor]]).
        define_property_or_throw(agent, o, property_key, property_descriptor)?;
    }
    // 6. Return O.
    Ok(o)
}

/// ### [24.1.1.2 AddEntriesFromIterable ( target, iterable, adder )](https://tc39.es/ecma262/#sec-add-entries-from-iterable)
///
/// The abstract operation AddEntriesFromIterable takes arguments target (an
/// Object), iterable (an ECMAScript language value, but not undefined or
/// null), and adder (a function object) and returns either a normal completion
/// containing an ECMAScript language value or a throw completion. adder will
/// be invoked, with target as the receiver.
///
/// > NOTE: The parameter iterable is expected to be an object that implements
/// > an @@iterator method that returns an iterator object that produces a two
/// > element array-like object whose first element is a value that will be used
/// > as a Map key and whose second element is the value to associate with that
/// > key.
///
/// #### Unspecified specialization
///
/// This is a specialization for the `Object.fromEntries` use case where we
/// know what adder does and that it is never seen from JavaScript: As such it
/// does not need to be defined as a JavaScript function.
pub fn add_entries_from_iterable_from_entries(
    agent: &mut Agent,
    target: OrdinaryObject,
    iterable: Value,
) -> JsResult<OrdinaryObject> {
    // 1. Let iteratorRecord be ? GetIterator(iterable, SYNC).
    let mut iterator_record = get_iterator(agent, iterable, false)?;

    // 2. Repeat,
    loop {
        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, &mut iterator_record)?;
        // b. If next is DONE, return target.
        let Some(next) = next else {
            return Ok(target);
        };
        // c. If next is not an Object, then
        let Ok(next) = Object::try_from(next) else {
            // i. Let error be ThrowCompletion(a newly created TypeError object).
            let error_message = format!(
                "Invalid iterator next return value: {} is not an object",
                next.string_repr(agent).as_str(agent)
            );
            let error = agent.throw_exception(ExceptionType::TypeError, error_message);
            // ii. Return ? IteratorClose(iteratorRecord, error).
            iterator_close(agent, &iterator_record, Err(error))?;
            return Ok(target);
        };
        // d. Let k be Completion(Get(next, "0")).
        let k = get(agent, next, 0.into());
        // e. IfAbruptCloseIterator(k, iteratorRecord).
        let k = if_abrupt_close_iterator(agent, k, &iterator_record)?;
        // f. Let v be Completion(Get(next, "1")).
        let v = get(agent, next, 1.into());
        // g. IfAbruptCloseIterator(v, iteratorRecord).
        let v = if_abrupt_close_iterator(agent, v, &iterator_record)?;
        // h. Let status be Completion(Call(adder, target, « k, v »)).
        {
            // a. Let propertyKey be ? ToPropertyKey(key).
            let property_key = to_property_key(agent, k);
            // i. IfAbruptCloseIterator(status, iteratorRecord).
            let property_key = if_abrupt_close_iterator(agent, property_key, &iterator_record)?;
            // b. Perform ! CreateDataPropertyOrThrow(obj, propertyKey, value).
            target
                .internal_define_own_property(
                    agent,
                    property_key,
                    PropertyDescriptor::new_data_descriptor(v),
                )
                .unwrap();
            // c. Return undefined.
        }
    }
}

/// ### [20.1.2.11.1 GetOwnPropertyKeys ( O, type )](https://tc39.es/ecma262/#sec-getownpropertykeys)
///
/// The abstract operation GetOwnPropertyKeys takes arguments O (an ECMAScript
/// language value) and type (STRING or SYMBOL) and returns either a normal
/// completion containing a List of property keys or a throw completion.
fn get_own_string_property_keys(agent: &mut Agent, o: Value) -> JsResult<Vec<Value>> {
    // 1. Let obj be ? ToObject(O).
    let obj = to_object(agent, o)?;
    // 2. Let keys be ? obj.[[OwnPropertyKeys]]().
    let keys = obj.internal_own_property_keys(agent)?;
    // 3. Let nameList be a new empty List.
    let mut name_list = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. If nextKey is a String and type is STRING then
        match next_key {
            // i. Append nextKey to nameList.
            PropertyKey::Integer(next_key) => {
                let next_key = format!("{}", next_key.into_i64());
                name_list.push(Value::from_string(agent, next_key));
            }
            PropertyKey::SmallString(next_key) => name_list.push(Value::SmallString(next_key)),
            PropertyKey::String(next_key) => name_list.push(Value::String(next_key)),
            PropertyKey::Symbol(_) => {}
        }
    }
    // 5. Return nameList.
    Ok(name_list)
}

fn get_own_symbol_property_keys(agent: &mut Agent, o: Value) -> JsResult<Vec<Value>> {
    // 1. Let obj be ? ToObject(O).
    let obj = to_object(agent, o)?;
    // 2. Let keys be ? obj.[[OwnPropertyKeys]]().
    let keys = obj.internal_own_property_keys(agent)?;
    // 3. Let nameList be a new empty List.
    let mut name_list = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. If nextKey is a Symbol and type is SYMBOL then
        if let PropertyKey::Symbol(next_key) = next_key {
            name_list.push(next_key.into_value())
        }
    }
    // 5. Return nameList.
    Ok(name_list)
}
