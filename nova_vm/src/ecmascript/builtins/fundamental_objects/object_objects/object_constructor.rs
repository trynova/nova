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
                create_array_from_list, create_array_from_scoped_list, define_property_or_throw,
                enumerable_own_properties, enumerable_properties_kind, get, get_method,
                group_by_property, has_own_property,
                integrity::{Frozen, Sealed},
                set, set_integrity_level, test_integrity_level, try_create_data_property,
                try_define_property_or_throw, try_get,
            },
            testing_and_comparison::{require_object_coercible, same_value},
            type_conversion::{to_object, to_property_key, to_property_key_simple},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
            ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
        },
        execution::{Agent, JsResult, ProtoIntrinsics, RealmIdentifier, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, IntoFunction, IntoObject, IntoValue, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value, scope_property_keys,
        },
    },
    engine::{
        Scoped, TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        unwrap_try,
    },
    heap::{IntrinsicConstructorIndexes, ObjectEntry, WellKnownSymbolIndexes},
};

pub(crate) struct ObjectConstructor;

impl Builtin for ObjectConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.Object;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for ObjectConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Object;
}

struct ObjectAssign;

impl Builtin for ObjectAssign {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.assign;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::assign);
}

struct ObjectCreate;

impl Builtin for ObjectCreate {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.create;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::create);
}
struct ObjectDefineProperties;

impl Builtin for ObjectDefineProperties {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.defineProperties;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_properties);
}
struct ObjectDefineProperty;

impl Builtin for ObjectDefineProperty {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.defineProperty;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_property);
}
struct ObjectEntries;

impl Builtin for ObjectEntries {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.entries;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::entries);
}
struct ObjectFreeze;

impl Builtin for ObjectFreeze {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.freeze;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::freeze);
}
struct ObjectFromEntries;

impl Builtin for ObjectFromEntries {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.fromEntries;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::from_entries);
}
struct ObjectGetOwnPropertyDescriptor;

impl Builtin for ObjectGetOwnPropertyDescriptor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_descriptor);
}
struct ObjectGetOwnPropertyDescriptors;

impl Builtin for ObjectGetOwnPropertyDescriptors {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptors;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(ObjectConstructor::get_own_property_descriptors);
}
struct ObjectGetOwnPropertyNames;

impl Builtin for ObjectGetOwnPropertyNames {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getOwnPropertyNames;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_names);
}
struct ObjectGetOwnPropertySymbols;

impl Builtin for ObjectGetOwnPropertySymbols {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getOwnPropertySymbols;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_symbols);
}
struct ObjectGetPrototypeOf;

impl Builtin for ObjectGetPrototypeOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.getPrototypeOf;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_prototype_of);
}
struct ObjectGroupBy;

impl Builtin for ObjectGroupBy {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.groupBy;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::group_by);
}
struct ObjectHasOwn;

impl Builtin for ObjectHasOwn {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.hasOwn;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::has_own);
}
struct ObjectIs;

impl Builtin for ObjectIs {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.is;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is);
}
struct ObjectIsExtensible;

impl Builtin for ObjectIsExtensible {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isExtensible;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_extensible);
}
struct ObjectIsFrozen;

impl Builtin for ObjectIsFrozen {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isFrozen;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_frozen);
}
struct ObjectIsSealed;

impl Builtin for ObjectIsSealed {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isSealed;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_sealed);
}
struct ObjectKeys;

impl Builtin for ObjectKeys {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.keys;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::keys);
}
struct ObjectPreventExtensions;

impl Builtin for ObjectPreventExtensions {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.preventExtensions;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::prevent_extensions);
}

struct ObjectSeal;

impl Builtin for ObjectSeal {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.seal;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::seal);
}
struct ObjectSetPrototypeOf;

impl Builtin for ObjectSetPrototypeOf {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.setPrototypeOf;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::set_prototype_of);
}
struct ObjectValues;

impl Builtin for ObjectValues {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.values;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::values);
}

impl ObjectConstructor {
    /// ### [20.1.1.1 Object ( \[ value \] )](https://tc39.es/ecma262/#sec-object-value)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let value = arguments.get(0).bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());
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
                unsafe { new_target.unwrap_unchecked().unbind() }
                    .try_into()
                    .unwrap(),
                ProtoIntrinsics::Object,
                gc.reborrow(),
            )
            .map(|value| value.into_value().unbind())
        } else if value == Value::Undefined || value == Value::Null {
            // 2. If value is either undefined or null, return OrdinaryObjectCreate(%Object.prototype%).
            Ok(ordinary_object_create_with_intrinsics(
                agent,
                Some(ProtoIntrinsics::Object),
                None,
                gc.into_nogc(),
            )
            .into_value())
        } else {
            // 3. Return ! ToObject(value).
            Ok(to_object(agent, value.unbind(), gc.into_nogc())
                .unwrap()
                .into_value())
        }
    }

    /// ### [20.1.2.1 Object.assign ( target, ...sources )](https://tc39.es/ecma262/#sec-object.assign)
    ///
    /// This function copies the values of all of the enumerable own properties
    /// from one or more source objects to a target object.
    fn assign<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let target = arguments.get(0).bind(nogc);
        // 1. Let to be ? ToObject(target).
        let to = to_object(agent, target, nogc)?;
        // 2. If only one argument was passed, return to.
        if arguments.len() <= 1 {
            return Ok(to.into_value().unbind());
        }
        let sources = arguments[1..]
            .iter()
            .map(|a| a.scope(agent, nogc))
            .collect::<Vec<_>>();
        // Note: We scope to twice: First for the from object and then for
        // itself. The scoped_from's heap data will be replaced on every loop
        // and thus cannot be used for storing the to object. We must not
        // use "clone" to duplicate scoped_from into to as that would reuse the
        // heap data slot.
        let mut scoped_from = to.scope(agent, nogc);
        let to = to.scope(agent, nogc);
        // 3. For each element nextSource of sources, do
        for scoped_next_source in sources {
            let next_source = scoped_next_source.get(agent).bind(gc.nogc());
            // a. If nextSource is neither undefined nor null, then
            if next_source.is_undefined() || next_source.is_null() {
                continue;
            }
            // i. Let from be ! ToObject(nextSource).
            let from = to_object(agent, next_source, gc.nogc()).unwrap();
            // SAFETY: scoped_from does not share its heap slot with anyone as
            // it is created separately (not a clone itself) and never cloned.
            unsafe { scoped_from.replace(agent, from.unbind()) };
            // ii. Let keys be ? from.[[OwnPropertyKeys]]().
            let keys = from
                .unbind()
                .internal_own_property_keys(agent, gc.reborrow())?;
            let keys = scope_property_keys(agent, keys.unbind(), gc.nogc());
            // iii. For each element nextKey of keys, do
            for next_key in keys {
                // 1. Let desc be ? from.[[GetOwnProperty]](nextKey).
                let desc = scoped_from.get(agent).internal_get_own_property(
                    agent,
                    next_key.get(agent),
                    gc.reborrow(),
                )?;
                // 2. If desc is not undefined and desc.[[Enumerable]] is true, then
                let Some(desc) = desc else {
                    continue;
                };
                if desc.enumerable != Some(true) {
                    continue;
                }
                // a. Let propValue be ? Get(from, nextKey).
                let prop_value = get(
                    agent,
                    scoped_from.get(agent),
                    next_key.get(agent),
                    gc.reborrow(),
                )?;
                // b. Perform ? Set(to, nextKey, propValue, true).
                set(
                    agent,
                    to.get(agent),
                    next_key.get(agent),
                    prop_value.unbind(),
                    true,
                    gc.reborrow(),
                )?;
            }
        }
        // 4. Return to.
        Ok(to.get(agent).into_value())
    }

    fn create<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let o = arguments.get(0).bind(nogc);
        let properties = arguments.get(1).bind(nogc);
        let obj: OrdinaryObject = if o == Value::Null {
            agent.heap.create_null_object(&[]).bind(nogc)
        } else if let Ok(o) = Object::try_from(o) {
            agent.heap.create_object_with_prototype(o, &[]).bind(nogc)
        } else {
            let error_message = format!(
                "{} is not an object or null",
                o.unbind().string_repr(agent, gc.reborrow()).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
        };
        if properties != Value::Undefined {
            let scoped_obj = obj.scope(agent, gc.nogc());
            object_define_properties(agent, obj.unbind(), properties.unbind(), gc.reborrow())?;
            Ok(scoped_obj.get(agent).into_value())
        } else {
            Ok(obj.into_value().unbind())
        }
    }

    /// ### [20.1.2.3 Object.defineProperties ( O, Properties )](https://tc39.es/ecma262/#sec-object.defineproperties)
    ///
    /// This function adds own properties and/or updates the attributes of
    /// existing own properties of an object.
    fn define_properties<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let o = arguments.get(0).bind(nogc);
        let properties = arguments.get(1).bind(nogc);
        // 1. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            let error_message = format!(
                "{} is not an object",
                o.unbind().string_repr(agent, gc.reborrow()).as_str(agent)
            );
            return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc()));
        };
        // 2. Return ? ObjectDefineProperties(O, Properties).
        let result = object_define_properties(agent, o.unbind(), properties.unbind(), gc)?;
        Ok(result.into_value())
    }

    /// ### [20.1.2.4 Object.defineProperty ( O, P, Attributes )](https://tc39.es/ecma262/#sec-object.defineproperty)
    ///
    /// This function adds an own property and/or updates the attributes of an
    /// existing own property of an object.
    fn define_property<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let o = arguments.get(0).bind(nogc);
        let p = arguments.get(1).bind(nogc);
        let mut attributes = arguments.get(2).bind(nogc);
        // 1. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            let error_message = format!(
                "{} is not an object",
                o.unbind().string_repr(agent, gc.reborrow()).as_str(agent)
            );
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                error_message,
                gc.into_nogc(),
            ));
        };
        let o = o.scope(agent, gc.nogc());
        // 2. Let key be ? ToPropertyKey(P).
        let mut key = if let TryResult::Continue(key) = to_property_key_simple(agent, p, nogc) {
            key
        } else {
            let scoped_attributes = attributes.scope(agent, nogc);
            let key = to_property_key(agent, p.unbind(), gc.reborrow())?.unbind();
            let gc = gc.nogc();
            attributes = scoped_attributes.get(agent).bind(gc);
            key.bind(gc)
        };
        // 3. Let desc be ? ToPropertyDescriptor(Attributes).
        let desc = if let TryResult::Continue(desc) =
            PropertyDescriptor::try_to_property_descriptor(agent, attributes, gc.nogc())
        {
            desc?
        } else {
            let scoped_key = key.scope(agent, gc.nogc());
            let desc = PropertyDescriptor::to_property_descriptor(
                agent,
                attributes.unbind(),
                gc.reborrow(),
            )?;
            key = scoped_key.get(agent).bind(gc.nogc());
            desc
        };
        // 4. Perform ? DefinePropertyOrThrow(O, key, desc).
        define_property_or_throw(agent, o.get(agent), key.unbind(), desc, gc.reborrow())?;
        // 5. Return O.
        Ok(o.get(agent).into_value())
    }

    fn entries<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o, gc.nogc())?;
        // 2. Let entryList be ? EnumerableOwnProperties(obj, KEY+VALUE).
        let entry_list = enumerable_own_properties::<
            enumerable_properties_kind::EnumerateKeysAndValues,
        >(agent, obj.unbind(), gc.reborrow())?;
        // 3. Return CreateArrayFromList(entryList).
        Ok(create_array_from_list(agent, &entry_list.unbind(), gc.into_nogc()).into_value())
    }

    /// ### [20.1.2.6 Object.freeze ( O )](https://tc39.es/ecma262/#sec-object.freeze)
    fn freeze<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0).bind(gc.nogc());
        let Ok(o) = Object::try_from(o) else {
            return Ok(o.unbind());
        };
        // 2. Let status be ? SetIntegrityLevel(O, FROZEN).
        let scoped_o = o.scope(agent, gc.nogc());
        let status = set_integrity_level::<Frozen>(agent, o.unbind(), gc.reborrow())?;
        if !status {
            // 3. If status is false, throw a TypeError exception.
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not freeze object",
                gc.nogc(),
            ))
        } else {
            // 4. Return O.
            Ok(scoped_o.get(agent).into_value())
        }
    }

    /// ### [20.1.2.7 Object.fromEntries ( iterable )](https://tc39.es/ecma262/#sec-object.fromentries)
    fn from_entries<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let mut iterable = arguments.get(0).bind(gc.nogc());
        // Fast path: Simple, dense array of N simple, dense arrays.
        if matches!(iterable, Value::Array(_)) {
            let array_prototype = agent.current_realm().intrinsics().array_prototype();
            let intrinsic_array_iterator = agent
                .current_realm()
                .intrinsics()
                .array_prototype_values()
                .into_function()
                .unbind();
            let scoped_iterable = iterable.scope(agent, gc.nogc());
            let array_iterator = get_method(
                agent,
                array_prototype.into_value(),
                WellKnownSymbolIndexes::Iterator.into(),
                gc.reborrow(),
            )?
            .unbind()
            .bind(gc.nogc());
            // SAFETY: Not shared.
            iterable = unsafe { scoped_iterable.take(agent).bind(gc.nogc()) };
            let Value::Array(entries_array) = iterable else {
                unreachable!()
            };
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
                    let key = to_property_key_simple(agent, key, gc.nogc());
                    let TryResult::Continue(key) = key else {
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
        require_object_coercible(agent, iterable, gc.nogc())?;
        // 2. Let obj be OrdinaryObjectCreate(%Object.prototype%).
        let obj = ordinary_object_create_with_intrinsics(
            agent,
            Some(ProtoIntrinsics::Object),
            None,
            gc.nogc(),
        );
        // 3. Assert: obj is an extensible ordinary object with no own properties.
        let obj = OrdinaryObject::try_from(obj).unwrap();
        debug_assert!(agent[obj].keys.is_empty());
        // 4. Let closure be a new Abstract Closure with parameters (key,
        //    value) that captures obj and performs the following steps when
        //    called:
        // 5. Let adder be CreateBuiltinFunction(closure, 2, "", « »).
        // 6. Return ? AddEntriesFromIterable(obj, iterable, adder).
        add_entries_from_iterable_from_entries(agent, obj.unbind(), iterable.unbind(), gc)
            .map(|obj| obj.into_value())
    }

    /// ### [20.1.2.8 Object.getOwnPropertyDescriptor ( O, P )](https://tc39.es/ecma262/#sec-object.getownpropertydescriptor)
    fn get_own_property_descriptor<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        let p = arguments.get(1).bind(gc.nogc());
        // 1. Let obj be ? ToObject(O).
        let mut obj = to_object(agent, o, gc.nogc())?;
        // 2. Let key be ? ToPropertyKey(P).
        let key = if let TryResult::Continue(key) = to_property_key_simple(agent, p, gc.nogc()) {
            key
        } else {
            let scoped_obj = obj.scope(agent, gc.nogc());
            let key = to_property_key(agent, p.unbind(), gc.reborrow())?
                .unbind()
                .bind(gc.nogc());
            obj = scoped_obj.get(agent).bind(gc.nogc());
            key
        };
        // 3. Let desc be ? obj.[[GetOwnProperty]](key).
        let desc = obj
            .unbind()
            .internal_get_own_property(agent, key.unbind(), gc.reborrow())?;
        // 4. Return FromPropertyDescriptor(desc).
        Ok(
            PropertyDescriptor::from_property_descriptor(desc, agent, gc.nogc())
                .map_or(Value::Undefined, |obj| obj.into_value().unbind()),
        )
    }

    /// ### [20.1.2.9 Object.getOwnPropertyDescriptors ( O )](https://tc39.es/ecma262/#sec-object.getownpropertydescriptors)
    fn get_own_property_descriptors<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        // 1. Let obj be ? ToObject(O).
        let mut obj = to_object(agent, o, gc.nogc())?;
        let mut scoped_obj = None;
        // 2. Let ownKeys be ? obj.[[OwnPropertyKeys]]().
        let mut own_keys =
            if let TryResult::Continue(own_keys) = obj.try_own_property_keys(agent, gc.nogc()) {
                own_keys
            } else {
                scoped_obj = Some(obj.scope(agent, gc.nogc()));
                let own_keys = obj
                    .unbind()
                    .internal_own_property_keys(agent, gc.reborrow())?
                    .unbind()
                    .bind(gc.nogc());
                obj = scoped_obj.as_ref().unwrap().get(agent).bind(gc.nogc());
                own_keys
            };

        let mut descriptors = Vec::with_capacity(own_keys.len());
        // 4. For each element key of ownKeys, do
        let mut i = 0;
        for &key in own_keys.iter() {
            // a. Let desc be ? obj.[[GetOwnProperty]](key).
            let TryResult::Continue(desc) = obj.try_get_own_property(agent, key, gc.nogc()) else {
                break;
            };
            // b. Let descriptor be FromPropertyDescriptor(desc).
            let descriptor = PropertyDescriptor::from_property_descriptor(desc, agent, gc.nogc());
            // c. If descriptor is not undefined, perform ! CreateDataPropertyOrThrow(descriptors, key, descriptor).
            if let Some(descriptor) = descriptor {
                descriptors.push(ObjectEntry::new_data_entry(key, descriptor.into_value()));
            }
            i += 1;
        }
        // 3. Let descriptors be OrdinaryObjectCreate(%Object.prototype%).
        let descriptors = agent
            .heap
            .create_object_with_prototype(
                agent
                    .current_realm()
                    .intrinsics()
                    .object_prototype()
                    .into_object(),
                &descriptors,
            )
            .bind(gc.nogc());
        if i < own_keys.len() {
            let _ = own_keys.drain(..i);
            let obj = scoped_obj.unwrap_or_else(|| obj.scope(agent, gc.nogc()));
            get_own_property_descriptors_slow(
                agent,
                obj,
                own_keys.unbind(),
                descriptors.unbind(),
                gc,
            )
        } else {
            // 5. Return descriptors.
            Ok(descriptors.into_value().unbind())
        }
    }

    /// ### [20.1.2.10 Object.getOwnPropertyNames ( O )](https://tc39.es/ecma262/#sec-object.getownpropertynames)
    fn get_own_property_names<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        // 1. Return CreateArrayFromList(? GetOwnPropertyKeys(O, STRING)).
        let keys = get_own_string_property_keys(agent, o.unbind(), gc.reborrow())?;
        Ok(create_array_from_list(agent, &keys.unbind(), gc.nogc())
            .into_value()
            .unbind())
    }

    /// ### [20.1.2.11 Object.getOwnPropertySymbols ( O )](https://tc39.es/ecma262/#sec-object.getownpropertysymbols)
    fn get_own_property_symbols<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        // 1. Return CreateArrayFromList(? GetOwnPropertyKeys(O, SYMBOL)).
        let keys = get_own_symbol_property_keys(agent, o.unbind(), gc.reborrow())?;
        Ok(create_array_from_list(agent, &keys.unbind(), gc.nogc())
            .into_value()
            .unbind())
    }

    /// ### [20.1.2.12 Object.getPrototypeOf ( O )](https://tc39.es/ecma262/#sec-object.getprototypeof)
    fn get_prototype_of<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let obj = to_object(agent, arguments.get(0), gc.nogc())?;
        // Note: We do not use try_get_prototype_of here as we don't need to
        // protect any on-stack values from GC. We're perfectly okay with
        // triggering GC here.
        obj.unbind()
            .internal_get_prototype_of(agent, gc)
            .map(|proto| proto.map_or(Value::Null, |proto| proto.into_value()))
    }

    // ### [20.1.2.13 Object.groupBy ( items, callback )](https://tc39.es/ecma262/#sec-object.groupby)
    fn group_by<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let items = arguments.get(0).bind(gc.nogc());
        let callback_fn = arguments.get(1).bind(gc.nogc());

        // 1. Let groups be ? GroupBy(items, callback, property).
        let groups = group_by_property(agent, items.unbind(), callback_fn.unbind(), gc.reborrow())?;

        // 2. Let obj be OrdinaryObjectCreate(null).
        // 3. For each Record { [[Key]], [[Elements]] } g of groups, do
        // a. Let elements be CreateArrayFromList(g.[[Elements]]).
        // b. Perform ! CreateDataPropertyOrThrow(obj, g.[[Key]], elements).
        let entries = groups
            .into_iter()
            .map(|g| {
                ObjectEntry::new_data_entry(
                    g.key.get(agent),
                    create_array_from_scoped_list(agent, g.elements, gc.nogc()).into_value(),
                )
            })
            .collect::<Vec<_>>();
        let object = agent.heap.create_null_object(&entries);

        // 4. Return obj.
        Ok(object.into_value())
    }

    fn has_own<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let p = arguments.get(1).bind(gc.nogc());
        let mut obj = to_object(agent, arguments.get(0), gc.nogc())?;
        let key = if let TryResult::Continue(key) = to_property_key_simple(agent, p, gc.nogc()) {
            key
        } else {
            let scoped_obj = obj.scope(agent, gc.nogc());
            let key = to_property_key(agent, p.unbind(), gc.reborrow())?
                .unbind()
                .bind(gc.nogc());
            obj = scoped_obj.get(agent).bind(gc.nogc());
            key
        };
        has_own_property(agent, obj.unbind(), key.unbind(), gc.reborrow())
            .map(|result| result.into())
    }

    fn is<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        Ok(same_value(agent, arguments.get(0), arguments.get(1)).into())
    }

    fn is_extensible<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        let result = match Object::try_from(o) {
            Ok(o) => o.unbind().internal_is_extensible(agent, gc.reborrow())?,
            Err(_) => false,
        };
        Ok(result.into())
    }

    fn is_frozen<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        let result = match Object::try_from(o) {
            Ok(o) => test_integrity_level::<Frozen>(agent, o.unbind(), gc.reborrow())?,
            Err(_) => true,
        };
        Ok(result.into())
    }

    fn is_sealed<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        let result = match Object::try_from(o) {
            Ok(o) => test_integrity_level::<Sealed>(agent, o.unbind(), gc.reborrow())?,
            Err(_) => true,
        };
        Ok(result.into())
    }

    /// ### [20.1.2.19 Object.keys ( O )](https://tc39.es/ecma262/#sec-object.keys)
    fn keys<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o, gc.nogc())?;
        // 2. Let keyList be ? EnumerableOwnProperties(obj, KEY).
        let key_list = enumerable_own_properties::<enumerable_properties_kind::EnumerateKeys>(
            agent,
            obj.unbind(),
            gc.reborrow(),
        )?;
        // 3. Return CreateArrayFromList(keyList).
        Ok(create_array_from_list(agent, &key_list.unbind(), gc.nogc())
            .into_value()
            .unbind())
    }

    /// ### [20.1.2.20 Object.preventExtensions ( O )](https://tc39.es/ecma262/#sec-object.preventextensions)
    fn prevent_extensions<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0).bind(gc.nogc());
        let Ok(o) = Object::try_from(o) else {
            return Ok(o.unbind());
        };
        let scoped_o = o.scope(agent, gc.nogc());
        // 2. Let status be ? O.[[PreventExtensions]]().
        let status = o
            .unbind()
            .internal_prevent_extensions(agent, gc.reborrow())?;
        // 3. If status is false, throw a TypeError exception.
        if !status {
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not prevent extensions",
                gc.nogc(),
            ))
        } else {
            // 4. Return O.
            Ok(scoped_o.get(agent).into_value())
        }
    }

    /// ### [20.1.2.22 Object.seal ( O )](https://tc39.es/ecma262/#sec-object.seal)
    fn seal<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0).bind(gc.nogc());
        let Ok(o) = Object::try_from(o) else {
            return Ok(o.unbind());
        };
        // 2. Let status be ? SetIntegrityLevel(O, SEALED).
        let scoped_o = o.scope(agent, gc.nogc());
        let status = set_integrity_level::<Sealed>(agent, o.unbind(), gc.reborrow())?;
        if !status {
            // 3. If status is false, throw a TypeError exception.
            Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not seal object",
                gc.nogc(),
            ))
        } else {
            // 4. Return O.
            Ok(scoped_o.get(agent).into_value())
        }
    }

    /// ### [20.1.2.23 Object.setPrototypeOf ( O, proto )](https://tc39.es/ecma262/#sec-object.setprototypeof)
    fn set_prototype_of<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        let proto = arguments.get(1).bind(gc.nogc());
        // 1. Set O to ? RequireObjectCoercible(O).
        let o = require_object_coercible(agent, o, gc.nogc())?;
        // 2. If proto is not an Object and proto is not null, throw a TypeError exception.
        let proto = if let Ok(proto) = Object::try_from(proto) {
            Some(proto)
        } else if proto.is_null() {
            None
        } else {
            let error_message = format!(
                "{} is not an object or null",
                proto
                    .unbind()
                    .string_repr(agent, gc.reborrow())
                    .as_str(agent)
            );
            return Err(agent.throw_exception(
                ExceptionType::TypeError,
                error_message,
                gc.into_nogc(),
            ));
        };
        // 3. If O is not an Object, return O.
        let Ok(o) = Object::try_from(o) else {
            return Ok(o.unbind());
        };
        // 4. Let status be ? O.[[SetPrototypeOf]](proto).
        let scoped_o = o.scope(agent, gc.nogc());
        let status = o
            .unbind()
            .internal_set_prototype_of(agent, proto.unbind(), gc.reborrow())?;
        // 5. If status is false, throw a TypeError exception.
        if !status {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Could not set prototype",
                gc.nogc(),
            ));
        }
        // 6. Return O.
        Ok(scoped_o.get(agent).into_value())
    }

    fn values<'gc>(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let o = arguments.get(0).bind(gc.nogc());
        // 1. Let obj be ? ToObject(O).
        let obj = to_object(agent, o, gc.nogc())?;
        // 2. Let valueList be ? EnumerableOwnProperties(obj, VALUE).
        let value_list = enumerable_own_properties::<enumerable_properties_kind::EnumerateValues>(
            agent,
            obj.unbind(),
            gc.reborrow(),
        )?;
        // 3. Return CreateArrayFromList(valueList).
        Ok(
            create_array_from_list(agent, &value_list.unbind(), gc.nogc())
                .into_value()
                .unbind(),
        )
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
fn object_define_properties<'a, T: InternalMethods<'a>>(
    agent: &mut Agent,
    o: T,
    properties: Value,
    mut gc: GcScope,
) -> JsResult<T> {
    // 1. Let props be ? ToObject(Properties).
    let props = to_object(agent, properties, gc.nogc())?.scope(agent, gc.nogc());
    // 2. Let keys be ? props.[[OwnPropertyKeys]]().
    let keys = props
        .get(agent)
        .internal_own_property_keys(agent, gc.reborrow())?
        .unbind()
        .bind(gc.nogc());
    let keys = scope_property_keys(agent, keys, gc.nogc());
    // 3. Let descriptors be a new empty List.
    let mut descriptors = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. Let propDesc be ? props.[[GetOwnProperty]](nextKey).
        let prop_desc = props.get(agent).internal_get_own_property(
            agent,
            next_key.get(agent),
            gc.reborrow(),
        )?;
        // b. If propDesc is not undefined and propDesc.[[Enumerable]] is true, then
        let Some(prop_desc) = prop_desc else {
            continue;
        };
        if prop_desc.enumerable != Some(true) {
            continue;
        }
        // i. Let descObj be ? Get(props, nextKey).
        let desc_obj = get(agent, props.get(agent), next_key.get(agent), gc.reborrow())?;
        // ii. Let desc be ? ToPropertyDescriptor(descObj).
        let desc =
            PropertyDescriptor::to_property_descriptor(agent, desc_obj.unbind(), gc.reborrow())?
                .scope(agent, gc.nogc());
        // iii. Append the Record { [[Key]]: nextKey, [[Descriptor]]: desc } to descriptors.
        descriptors.push((next_key, desc));
    }
    // 5. For each element property of descriptors, do
    for (property_key, property_descriptor) in descriptors {
        // a. Perform ? DefinePropertyOrThrow(O, property.[[Key]], property.[[Descriptor]]).
        define_property_or_throw(
            agent,
            o,
            property_key.get(agent),
            property_descriptor.into_property_descriptor(agent),
            gc.reborrow(),
        )?;
    }
    // 6. Return O.
    Ok(o)
}

fn try_object_define_properties<'a, T: InternalMethods<'a>>(
    agent: &mut Agent,
    o: T,
    properties: Value,
    gc: NoGcScope,
) -> TryResult<JsResult<T>> {
    // 1. Let props be ? ToObject(Properties).
    let props = match to_object(agent, properties, gc) {
        Ok(props) => props,
        Err(err) => {
            return TryResult::Continue(Err(err));
        }
    };
    // 2. Let keys be ? props.[[OwnPropertyKeys]]().
    let keys = props.try_own_property_keys(agent, gc)?;
    // 3. Let descriptors be a new empty List.
    let mut descriptors = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. Let propDesc be ? props.[[GetOwnProperty]](nextKey).
        let prop_desc = props.try_get_own_property(agent, next_key, gc)?;
        // b. If propDesc is not undefined and propDesc.[[Enumerable]] is true, then
        let Some(prop_desc) = prop_desc else {
            continue;
        };
        if prop_desc.enumerable != Some(true) {
            continue;
        }
        // i. Let descObj be ? Get(props, nextKey).
        let desc_obj = try_get(agent, props, next_key, gc)?;
        // ii. Let desc be ? ToPropertyDescriptor(descObj).
        let desc = PropertyDescriptor::try_to_property_descriptor(agent, desc_obj, gc)?;
        let desc = match desc {
            Ok(desc) => desc,
            Err(err) => {
                return TryResult::Continue(Err(err));
            }
        };
        // iii. Append the Record { [[Key]]: nextKey, [[Descriptor]]: desc } to descriptors.
        descriptors.push((next_key, desc));
    }
    // 5. For each element property of descriptors, do
    for (property_key, property_descriptor) in descriptors {
        // a. Perform ? DefinePropertyOrThrow(O, property.[[Key]], property.[[Descriptor]]).
        if let Err(err) =
            try_define_property_or_throw(agent, o, property_key, property_descriptor, gc)?
        {
            return TryResult::Continue(Err(err));
        }
    }
    // 6. Return O.
    TryResult::Continue(Ok(o))
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
pub fn add_entries_from_iterable_from_entries<'a>(
    agent: &mut Agent,
    target: OrdinaryObject,
    iterable: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<OrdinaryObject<'a>> {
    // Note: scoped_next is a slot for next value scoping that originally holds
    // the target but will later be reused for each repeat of the loop. We
    // cannot reuse the scoped target below for this, as the value held in the
    // scoped_next will change on each loop.
    let mut scoped_next = target.into_object().scope(agent, gc.nogc());
    let target = target.scope(agent, gc.nogc());
    let iterable = iterable.scope(agent, gc.nogc());
    // 1. Let iteratorRecord be ? GetIterator(iterable, SYNC).
    let mut iterator_record = get_iterator(agent, iterable.get(agent), false, gc.reborrow())?;

    // 2. Repeat,
    let mut scoped_k = Value::Undefined.scope_static();
    let mut scoped_v = Value::Undefined.scope_static();
    loop {
        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(agent, &mut iterator_record, gc.reborrow())?;
        // b. If next is DONE, return target.
        let Some(next) = next else {
            return Ok(target.get(agent).bind(gc.into_nogc()));
        };
        // c. If next is not an Object, then
        let Ok(next) = Object::try_from(next) else {
            // i. Let error be ThrowCompletion(a newly created TypeError object).
            let error_message = format!(
                "Invalid iterator next return value: {} is not an object",
                next.unbind()
                    .string_repr(agent, gc.reborrow())
                    .as_str(agent)
            );
            let error = agent.throw_exception(ExceptionType::TypeError, error_message, gc.nogc());
            // ii. Return ? IteratorClose(iteratorRecord, error).
            iterator_close(agent, &iterator_record, Err(error), gc.reborrow())?;
            return Ok(target.get(agent).bind(gc.into_nogc()));
        };
        // SAFETY: scoped_next is its own Scoped value, not a clone from target
        // or anything else. Hence we can change its held value freely.
        unsafe { scoped_next.replace(agent, next.unbind()) };
        // d. Let k be Completion(Get(next, "0")).
        let k = get(agent, next.unbind(), 0.into(), gc.reborrow());
        // e. IfAbruptCloseIterator(k, iteratorRecord).
        let k = if_abrupt_close_iterator!(agent, k, iterator_record, gc);
        // SAFETY: scoped_k is never shared.
        unsafe { scoped_k.replace(agent, k.unbind()) };
        // f. Let v be Completion(Get(next, "1")).
        let v = get(agent, scoped_next.get(agent), 1.into(), gc.reborrow());
        // g. IfAbruptCloseIterator(v, iteratorRecord).
        let v = if_abrupt_close_iterator!(agent, v, iterator_record, gc);
        // SAFETY: scoped_v is never shared.
        unsafe { scoped_v.replace(agent, v.unbind()) };
        // h. Let status be Completion(Call(adder, target, « k, v »)).
        {
            // a. Let propertyKey be ? ToPropertyKey(key).
            let property_key = to_property_key(agent, scoped_k.get(agent), gc.reborrow());
            // i. IfAbruptCloseIterator(status, iteratorRecord).
            let property_key = if_abrupt_close_iterator!(agent, property_key, iterator_record, gc);
            // b. Perform ! CreateDataPropertyOrThrow(obj, propertyKey, value).
            unwrap_try(target.get(agent).try_define_own_property(
                agent,
                property_key.unbind(),
                PropertyDescriptor::new_data_descriptor(scoped_v.get(agent)),
                gc.nogc(),
            ));
            // c. Return undefined.
        }
    }
}

/// ### [20.1.2.11.1 GetOwnPropertyKeys ( O, type )](https://tc39.es/ecma262/#sec-getownpropertykeys)
///
/// The abstract operation GetOwnPropertyKeys takes arguments O (an ECMAScript
/// language value) and type (STRING or SYMBOL) and returns either a normal
/// completion containing a List of property keys or a throw completion.
fn get_own_string_property_keys<'gc>(
    agent: &mut Agent,
    o: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Vec<Value<'gc>>> {
    // 1. Let obj be ? ToObject(O).
    let obj = to_object(agent, o, gc.nogc())?;
    // 2. Let keys be ? obj.[[OwnPropertyKeys]]().
    let keys = obj
        .unbind()
        .internal_own_property_keys(agent, gc.reborrow())?
        .unbind();
    let gc = gc.into_nogc();
    let keys = keys.bind(gc);
    // 3. Let nameList be a new empty List.
    let mut name_list = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. If nextKey is a String and type is STRING then
        match next_key {
            // i. Append nextKey to nameList.
            PropertyKey::Integer(next_key) => {
                let next_key = next_key.into_i64().to_string();
                name_list.push(Value::from_string(agent, next_key, gc));
            }
            PropertyKey::SmallString(next_key) => name_list.push(Value::SmallString(next_key)),
            PropertyKey::String(next_key) => name_list.push(Value::String(next_key.unbind())),
            PropertyKey::Symbol(_) => {}
        }
    }
    // 5. Return nameList.
    Ok(name_list)
}

fn get_own_symbol_property_keys<'gc>(
    agent: &mut Agent,
    o: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Vec<Value<'gc>>> {
    // 1. Let obj be ? ToObject(O).
    let obj = to_object(agent, o, gc.nogc())?;
    // 2. Let keys be ? obj.[[OwnPropertyKeys]]().
    let keys = obj
        .unbind()
        .internal_own_property_keys(agent, gc.reborrow())?;
    // 3. Let nameList be a new empty List.
    let mut name_list = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. If nextKey is a Symbol and type is SYMBOL then
        if let PropertyKey::Symbol(next_key) = next_key {
            name_list.push(next_key.into_value().unbind())
        }
    }
    // 5. Return nameList.
    Ok(name_list)
}

fn get_own_property_descriptors_slow<'gc>(
    agent: &mut Agent,
    obj: Scoped<'_, Object<'static>>,
    own_keys: Vec<PropertyKey>,
    descriptors: OrdinaryObject,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Value<'gc>> {
    let descriptors = descriptors.scope(agent, gc.nogc());
    let own_keys = scope_property_keys(agent, own_keys, gc.nogc());
    for key in own_keys {
        // a. Let desc be ? obj.[[GetOwnProperty]](key).
        let desc =
            obj.get(agent)
                .internal_get_own_property(agent, key.get(agent), gc.reborrow())?;
        // b. Let descriptor be FromPropertyDescriptor(desc).
        let descriptor = PropertyDescriptor::from_property_descriptor(desc, agent, gc.nogc());
        // c. If descriptor is not undefined, perform ! CreateDataPropertyOrThrow(descriptors, key, descriptor).
        if let Some(descriptor) = descriptor {
            let gc = gc.nogc();
            assert!(unwrap_try(try_create_data_property(
                agent,
                descriptors.get(agent).bind(gc),
                key.get(agent).bind(gc),
                descriptor.unbind().into_value(),
                gc,
            )));
        }
    }
    Ok(descriptors.get(agent).into_value())
}

fn object_define_properties_slow() {}
