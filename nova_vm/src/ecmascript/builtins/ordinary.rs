// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};
use std::vec;

use crate::{
    ecmascript::abstract_operations::operations_on_objects::{
        try_create_data_property, try_get, try_get_function_realm,
    },
    engine::{
        Scoped, TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        unwrap_try,
    },
};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, create_data_property, get, get_function_realm},
            testing_and_comparison::same_value,
        },
        builtins::ArgumentsList,
        execution::{Agent, JsResult, ProtoIntrinsics, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, Function, InternalMethods, InternalSlots, IntoFunction,
            IntoObject, Object, ObjectHeapData, OrdinaryObject, PropertyDescriptor, PropertyKey,
            String, Symbol, Value,
        },
    },
    heap::{CompactionLists, CreateHeapData, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

#[cfg(feature = "date")]
use super::date::data::DateHeapData;
#[cfg(feature = "regexp")]
use super::regexp::RegExpHeapData;
#[cfg(feature = "shared-array-buffer")]
use super::shared_array_buffer::data::SharedArrayBufferHeapData;
#[cfg(feature = "array-buffer")]
use super::{
    ArrayBufferHeapData, data_view::data::DataViewHeapData, typed_array::data::TypedArrayHeapData,
};
use super::{
    ArrayHeapData, async_generator_objects::AsyncGeneratorHeapData,
    control_abstraction_objects::generator_objects::GeneratorHeapData, error::ErrorHeapData,
    finalization_registry::data::FinalizationRegistryHeapData,
    indexed_collections::array_objects::array_iterator_objects::array_iterator::ArrayIteratorHeapData,
    keyed_collections::map_objects::map_iterator_objects::map_iterator::MapIteratorHeapData,
    map::data::MapHeapData, module::Module, primitive_objects::PrimitiveObjectHeapData,
    promise::data::PromiseHeapData,
};
#[cfg(feature = "set")]
use super::{
    keyed_collections::set_objects::set_iterator_objects::set_iterator::SetIteratorHeapData,
    set::data::SetHeapData,
};
#[cfg(feature = "weak-refs")]
use super::{
    weak_map::data::WeakMapHeapData, weak_ref::data::WeakRefHeapData,
    weak_set::data::WeakSetHeapData,
};

impl Index<OrdinaryObject<'_>> for Agent {
    type Output = ObjectHeapData<'static>;

    fn index(&self, index: OrdinaryObject<'_>) -> &Self::Output {
        &self.heap.objects[index]
    }
}

impl IndexMut<OrdinaryObject<'_>> for Agent {
    fn index_mut(&mut self, index: OrdinaryObject<'_>) -> &mut Self::Output {
        &mut self.heap.objects[index]
    }
}

impl Index<OrdinaryObject<'_>> for Vec<Option<ObjectHeapData<'static>>> {
    type Output = ObjectHeapData<'static>;

    fn index(&self, index: OrdinaryObject<'_>) -> &Self::Output {
        self.get(index.get_index())
            .expect("Object out of bounds")
            .as_ref()
            .expect("Object slot empty")
    }
}

impl IndexMut<OrdinaryObject<'_>> for Vec<Option<ObjectHeapData<'static>>> {
    fn index_mut(&mut self, index: OrdinaryObject<'_>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("Object out of bounds")
            .as_mut()
            .expect("Object slot empty")
    }
}

/// ### [10.1 Ordinary Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots)
impl<'a> InternalMethods<'a> for OrdinaryObject<'a> {}

/// ### [10.1.1.1 OrdinaryGetPrototypeOf ( O )](https://tc39.es/ecma262/#sec-ordinarygetprototypeof)
pub(crate) fn ordinary_get_prototype_of<'a>(
    agent: &mut Agent,
    object: OrdinaryObject,
    _: NoGcScope<'a, '_>,
) -> Option<Object<'a>> {
    // 1. Return O.[[Prototype]].
    object.internal_prototype(agent)
}

/// ### [10.1.2.1 OrdinarySetPrototypeOf ( O, V )](https://tc39.es/ecma262/#sec-ordinarysetprototypeof)
pub(crate) fn ordinary_set_prototype_of(
    agent: &mut Agent,
    object: Object,
    prototype: Option<Object>,
) -> bool {
    // 1. Let current be O.[[Prototype]].
    let current = object.internal_prototype(agent);

    // 2. If SameValue(V, current) is true, return true.
    if prototype == current {
        return true;
    }

    // 3. Let extensible be O.[[Extensible]].
    let extensible = object.internal_extensible(agent);

    // 4. If extensible is false, return false.
    if !extensible {
        // 7.b.i. Return false.
        return false;
    }

    // 5. Let p be V.
    let mut p = prototype;
    // 6. Let done be false.
    let mut done = false;

    // 7. Repeat, while done is false,
    while !done {
        if let Some(p_inner) = p {
            // b. Else if SameValue(p, O) is true, then
            if p_inner == object {
                // i. Return false.
                return false;
            } else {
                // c. Else,
                // i. If p.[[GetPrototypeOf]] is not the ordinary object internal method defined in 10.1.1,
                //    set done to true.
                // NOTE: At present there are two exotic objects that define their own [[GetPrototypeOf]]
                // methods. Those are Proxy and Module.
                if matches!(p_inner, Object::Module(_) | Object::Proxy(_)) {
                    done = true;
                } else {
                    // ii. Else, set p to p.[[Prototype]].
                    p = p_inner.internal_prototype(agent);
                }
            }
        } else {
            // a. If p is null, then
            // i. Set done to true.
            done = true;
        }
    }

    // 8. Set O.[[Prototype]] to V.
    object.internal_set_prototype(agent, prototype);

    // 9. Return true.
    true
}

/// ### [10.1.3.1 OrdinaryIsExtensible ( O )](https://tc39.es/ecma262/#sec-ordinaryisextensible)
pub(crate) fn ordinary_is_extensible(agent: &mut Agent, object: OrdinaryObject) -> bool {
    // 1. Return O.[[Extensible]].
    object.internal_extensible(agent)
}

/// ### [10.1.4.1 OrdinaryPreventExtensions ( O )](https://tc39.es/ecma262/#sec-ordinarypreventextensions)
pub(crate) fn ordinary_prevent_extensions(agent: &mut Agent, object: OrdinaryObject) -> bool {
    // 1. Set O.[[Extensible]] to false.
    object.internal_set_extensible(agent, false);

    // 2. Return true.
    true
}

/// ### [10.1.5.1 OrdinaryGetOwnProperty ( O, P )](https://tc39.es/ecma262/#sec-ordinarygetownproperty)
pub(crate) fn ordinary_get_own_property<'a>(
    agent: &Agent,
    object: OrdinaryObject<'a>,
    property_key: PropertyKey,
) -> Option<PropertyDescriptor<'a>> {
    // 1. If O does not have an own property with key P, return undefined.
    // 3. Let X be O's own property whose key is P.
    let x = object
        .into_object()
        .property_storage()
        .get(agent, property_key)?;

    // 2. Let D be a newly created Property Descriptor with no fields.
    let mut descriptor = PropertyDescriptor::default();

    // 4. If X is a data property, then
    if x.is_data_descriptor() {
        // a. Set D.[[Value]] to the value of X's [[Value]] attribute.
        descriptor.value = x.value;

        // b. Set D.[[Writable]] to the value of X's [[Writable]] attribute.
        descriptor.writable = x.writable;
    } else {
        // 5. Else,
        // a. Assert: X is an accessor property.
        debug_assert!(x.is_accessor_descriptor());

        // b. Set D.[[Get]] to the value of X's [[Get]] attribute.
        descriptor.get = x.get;

        // c. Set D.[[Set]] to the value of X's [[Set]] attribute.
        descriptor.set = x.set;
    }

    // 6. Set D.[[Enumerable]] to the value of X's [[Enumerable]] attribute.
    descriptor.enumerable = x.enumerable;

    // 7. Set D.[[Configurable]] to the value of X's [[Configurable]] attribute.
    descriptor.configurable = x.configurable;

    // 8. Return D.
    Some(descriptor)
}

/// ### [10.1.6.1 OrdinaryDefineOwnProperty ( O, P, Desc )](https://tc39.es/ecma262/#sec-ordinarydefineownproperty)
pub(crate) fn ordinary_define_own_property(
    agent: &mut Agent,
    object: OrdinaryObject,
    property_key: PropertyKey,
    descriptor: PropertyDescriptor,
    gc: NoGcScope,
) -> bool {
    // Note: OrdinaryDefineOwnProperty is only used by the backing object type,
    // meaning that we know that this method cannot call into JavaScript.
    // 1. Let current be ! O.[[GetOwnProperty]](P).
    let current = unwrap_try(object.try_get_own_property(agent, property_key, gc));

    // 2. Let extensible be ! IsExtensible(O).
    let extensible = object.internal_extensible(agent);

    // 3. Return ValidateAndApplyPropertyDescriptor(O, P, extensible, Desc, current).
    validate_and_apply_property_descriptor(
        agent,
        Some(object),
        property_key,
        extensible,
        descriptor,
        current,
    )
}

/// ### [10.1.6.2 IsCompatiblePropertyDescriptor ( Extensible, Desc, Current )](https://tc39.es/ecma262/#sec-iscompatiblepropertydescriptor)
pub(crate) fn is_compatible_property_descriptor(
    agent: &mut Agent,
    extensible: bool,
    descriptor: PropertyDescriptor,
    current: Option<PropertyDescriptor>,
    gc: NoGcScope,
) -> bool {
    let property_key = PropertyKey::from_str(agent, "", gc);
    validate_and_apply_property_descriptor(
        agent,
        None,
        property_key,
        extensible,
        descriptor,
        current,
    )
}

/// ### [10.1.6.3 ValidateAndApplyPropertyDescriptor ( O, P, extensible, Desc, current )](https://tc39.es/ecma262/#sec-validateandapplypropertydescriptor)
fn validate_and_apply_property_descriptor(
    agent: &mut Agent,
    object: Option<OrdinaryObject>,
    property_key: PropertyKey,
    extensible: bool,
    descriptor: PropertyDescriptor,
    current: Option<PropertyDescriptor>,
) -> bool {
    // 1. Assert: IsPropertyKey(P) is true.

    // 2. If current is undefined, then
    let Some(current) = current else {
        // a. If extensible is false, return false.
        if !extensible {
            return false;
        }

        // b. If O is undefined, return true.
        let Some(object) = object else {
            return true;
        };

        // c. If IsAccessorDescriptor(Desc) is true, then
        if descriptor.is_accessor_descriptor() {
            // i. Create an own accessor property named P of object O whose [[Get]], [[Set]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            object.into_object().property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    get: descriptor.get,
                    set: descriptor.set,
                    enumerable: Some(descriptor.enumerable.unwrap_or(false)),
                    configurable: Some(descriptor.configurable.unwrap_or(false)),
                    ..Default::default()
                },
            )
        }
        // d. Else,
        else {
            // i. Create an own data property named P of object O whose [[Value]], [[Writable]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            object.into_object().property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    value: Some(descriptor.value.unwrap_or(Value::Undefined)),
                    writable: Some(descriptor.writable.unwrap_or(false)),
                    enumerable: Some(descriptor.enumerable.unwrap_or(false)),
                    configurable: Some(descriptor.configurable.unwrap_or(false)),
                    ..Default::default()
                },
            )
        }

        // e. Return true.
        return true;
    };

    // 3. Assert: current is a fully populated Property Descriptor.
    debug_assert!(current.is_fully_populated());

    // 4. If Desc does not have any fields, return true.
    if !descriptor.has_fields() {
        return true;
    }

    // 5. If current.[[Configurable]] is false, then
    if !current.configurable.unwrap() {
        // a. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
        if let Some(true) = descriptor.configurable {
            return false;
        }

        // b. If Desc has an [[Enumerable]] field and SameValue(Desc.[[Enumerable]], current.[[Enumerable]])
        //    is false, return false.
        if descriptor.enumerable.is_some() && descriptor.enumerable != current.enumerable {
            return false;
        }

        // c. If IsGenericDescriptor(Desc) is false and SameValue(IsAccessorDescriptor(Desc), IsAccessorDescriptor(current))
        //    is false, return false.
        if !descriptor.is_generic_descriptor()
            && descriptor.is_accessor_descriptor() != current.is_accessor_descriptor()
        {
            return false;
        }

        // d. If IsAccessorDescriptor(current) is true, then
        if current.is_accessor_descriptor() {
            // i. If Desc has a [[Get]] field and SameValue(Desc.[[Get]], current.[[Get]]) is false,
            //    return false.
            if let Some(desc_get) = descriptor.get {
                if let Some(cur_get) = current.get {
                    if desc_get != cur_get {
                        return false;
                    }
                } else {
                    return false;
                }
            }

            // ii. If Desc has a [[Set]] field and SameValue(Desc.[[Set]], current.[[Set]]) is
            //     false, return false.
            if let Some(desc_set) = descriptor.set {
                if let Some(cur_set) = current.set {
                    if desc_set != cur_set {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
        // e. Else if current.[[Writable]] is false, then
        else if current.writable == Some(false) {
            // i. If Desc has a [[Writable]] field and Desc.[[Writable]] is true, return false.
            if let Some(true) = descriptor.writable {
                return false;
            }

            // ii. If Desc has a [[Value]] field and SameValue(Desc.[[Value]], current.[[Value]])
            //     is false, return false.
            if let Some(desc_value) = descriptor.value {
                if let Some(cur_value) = current.value {
                    if !same_value(agent, desc_value, cur_value) {
                        return false;
                    }
                } else {
                    return false;
                }
            }
        }
    }

    // 6. If O is not undefined, then
    if let Some(object) = object {
        // a. If IsDataDescriptor(current) is true and IsAccessorDescriptor(Desc) is true, then
        if current.is_data_descriptor() && descriptor.is_accessor_descriptor() {
            // i. If Desc has a [[Configurable]] field, let configurable be Desc.[[Configurable]];
            //    else let configurable be current.[[Configurable]].
            let configurable = descriptor
                .configurable
                .unwrap_or_else(|| current.configurable.unwrap());

            // ii. If Desc has a [[Enumerable]] field, let enumerable be Desc.[[Enumerable]]; else
            //     let enumerable be current.[[Enumerable]].
            let enumerable = descriptor
                .enumerable
                .unwrap_or_else(|| current.enumerable.unwrap());

            // iii. Replace the property named P of object O with an accessor property whose
            //      [[Configurable]] and [[Enumerable]] attributes are set to configurable and
            //      enumerable, respectively, and whose [[Get]] and [[Set]] attributes are set to
            //      the value of the corresponding field in Desc if Desc has that field, or to the
            //      attribute's default value otherwise.
            object.into_object().property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    get: descriptor.get,
                    set: descriptor.set,
                    enumerable: Some(enumerable),
                    configurable: Some(configurable),
                    ..Default::default()
                },
            );
        }
        // b. Else if IsAccessorDescriptor(current) is true and IsDataDescriptor(Desc) is true, then
        else if current.is_accessor_descriptor() && descriptor.is_data_descriptor() {
            // i. If Desc has a [[Configurable]] field, let configurable be Desc.[[Configurable]];
            //    else let configurable be current.[[Configurable]].
            let configurable = descriptor
                .configurable
                .unwrap_or_else(|| current.configurable.unwrap());

            // ii. If Desc has a [[Enumerable]] field, let enumerable be Desc.[[Enumerable]]; else
            //     let enumerable be current.[[Enumerable]].
            let enumerable = descriptor
                .enumerable
                .unwrap_or_else(|| current.enumerable.unwrap());

            // iii. Replace the property named P of object O with a data property whose
            //      [[Configurable]] and [[Enumerable]] attributes are set to configurable and
            //      enumerable, respectively, and whose [[Value]] and [[Writable]] attributes are
            //      set to the value of the corresponding field in Desc if Desc has that field, or
            //      to the attribute's default value otherwise.
            // try object.propertyStorage().set(property_key, PropertyDescriptor{
            //     .value = descriptor.value orelse .undefined,
            //     .writable = descriptor.writable orelse false,
            //     .enumerable = enumerable,
            //     .configurable = configurable,
            // });
            object.into_object().property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    value: Some(descriptor.value.unwrap_or(Value::Undefined)),
                    writable: Some(descriptor.writable.unwrap_or(false)),
                    enumerable: Some(enumerable),
                    configurable: Some(configurable),
                    ..Default::default()
                },
            );
        }
        // c. Else,
        else {
            // i. For each field of Desc, set the corresponding attribute of the property named P
            //    of object O to the value of the field.
            object.into_object().property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    value: descriptor.value.or(current.value),
                    writable: descriptor.writable.or(current.writable),
                    get: descriptor.get.or(current.get),
                    set: descriptor.set.or(current.set),
                    enumerable: descriptor.enumerable.or(current.enumerable),
                    configurable: descriptor.configurable.or(current.configurable),
                },
            );
        }
    }

    // 7. Return true.
    true
}

/// ### [10.1.7.1 OrdinaryHasProperty ( O, P )](https://tc39.es/ecma262/#sec-ordinaryhasproperty)
pub(crate) fn ordinary_try_has_property(
    agent: &mut Agent,
    object: OrdinaryObject,
    property_key: PropertyKey,
    gc: NoGcScope,
) -> TryResult<bool> {
    // 1. Let hasOwn be ? O.[[GetOwnProperty]](P).
    // Note: ? means that if we'd call a Proxy's GetOwnProperty trap then we'll
    // instead return None.
    let has_own = object.try_get_own_property(agent, property_key, gc)?;

    // 2. If hasOwn is not undefined, return true.
    if has_own.is_some() {
        return TryResult::Continue(true);
    }

    // 3. Let parent be ? O.[[GetPrototypeOf]]().
    // Note: ? means that if we'd call a Proxy's GetPrototypeOf trap then we'll
    // instead return None.
    let parent = object.try_get_prototype_of(agent, gc)?;

    // 4. If parent is not null, then
    if let Some(parent) = parent {
        // a. Return ? parent.[[HasProperty]](P).
        // Note: Here too, if we would call a Proxy's HasProperty trap then
        // we'll instead return None.
        return parent.try_has_property(agent, property_key, gc);
    }

    // 5. Return false.
    TryResult::Continue(false)
}

pub(crate) fn ordinary_try_has_property_entry<'a>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    gc: NoGcScope,
) -> TryResult<bool> {
    match object.get_backing_object(agent) {
        Some(backing_object) => ordinary_try_has_property(agent, backing_object, property_key, gc),
        None => {
            // 3. Let parent be ? O.[[GetPrototypeOf]]().
            let parent = unwrap_try(object.try_get_prototype_of(agent, gc));

            // 4. If parent is not null, then
            if let Some(parent) = parent {
                // a. Return ? parent.[[HasProperty]](P).
                parent.try_has_property(agent, property_key, gc)
            } else {
                // 5. Return false.
                TryResult::Continue(false)
            }
        }
    }
}

/// ### [10.1.7.1 OrdinaryHasProperty ( O, P )](https://tc39.es/ecma262/#sec-ordinaryhasproperty)
pub(crate) fn ordinary_has_property<'a>(
    agent: &mut Agent,
    object: OrdinaryObject,
    property_key: PropertyKey,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let object = object.bind(gc.nogc());
    let property_key = property_key.bind(gc.nogc());
    // Note: We scope here because it's likely we've already tried.
    let scoped_object = object.scope(agent, gc.nogc());
    let scoped_property_key = property_key.scope(agent, gc.nogc());
    // 1. Let hasOwn be ? O.[[GetOwnProperty]](P).

    let has_own = object
        .unbind()
        .internal_get_own_property(agent, property_key.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // 2. If hasOwn is not undefined, return true.
    if has_own.is_some() {
        return Ok(true);
    }

    // 3. Let parent be ? O.[[GetPrototypeOf]]().
    let object = scoped_object.get(agent).bind(gc.nogc());
    let (parent, property_key) =
        if let TryResult::Continue(parent) = object.try_get_prototype_of(agent, gc.nogc()) {
            (parent, scoped_property_key.get(agent).bind(gc.nogc()))
        } else {
            (
                object
                    .unbind()
                    .internal_get_prototype_of(agent, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc()),
                scoped_property_key.get(agent).bind(gc.nogc()),
            )
        };

    // 4. If parent is not null, then
    if let Some(parent) = parent {
        // a. Return ? parent.[[HasProperty]](P).
        return parent
            .unbind()
            .internal_has_property(agent, property_key.unbind(), gc);
    }

    // 5. Return false.
    Ok(false)
}

pub(crate) fn ordinary_has_property_entry<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    let property_key = property_key.bind(gc.nogc());
    match object.get_backing_object(agent) {
        Some(backing_object) => {
            ordinary_has_property(agent, backing_object, property_key.unbind(), gc)
        }
        None => {
            // 3. Let parent be ? O.[[GetPrototypeOf]]().
            let parent = unwrap_try(object.try_get_prototype_of(agent, gc.nogc()));

            // 4. If parent is not null, then
            if let Some(parent) = parent {
                // a. Return ? parent.[[HasProperty]](P).
                parent
                    .unbind()
                    .internal_has_property(agent, property_key.unbind(), gc)
            } else {
                // 5. Return false.
                Ok(false)
            }
        }
    }
}

/// ### [10.1.8.1 OrdinaryGet ( O, P, Receiver )](https://tc39.es/ecma262/#sec-ordinaryget)
pub(crate) fn ordinary_try_get<'gc>(
    agent: &mut Agent,
    object: OrdinaryObject,
    property_key: PropertyKey,
    receiver: Value,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<Value<'gc>> {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let Some(descriptor) = object.try_get_own_property(agent, property_key, gc)? else {
        // 2. If desc is undefined, then

        // a. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = object.try_get_prototype_of(agent, gc)?;

        // b. If parent is null, return undefined.
        let Some(parent) = parent else {
            return TryResult::Continue(Value::Undefined);
        };
        // c. Return ? parent.[[Get]](P, Receiver).
        return parent.try_get(agent, property_key, receiver, gc);
    };

    // 3. If IsDataDescriptor(desc) is true, return desc.[[Value]].
    if let Some(value) = descriptor.value {
        debug_assert!(descriptor.is_data_descriptor());
        return TryResult::Continue(value);
    }

    // 4. Assert: IsAccessorDescriptor(desc) is true.
    debug_assert!(descriptor.is_accessor_descriptor());

    // 5. Let getter be desc.[[Get]].
    // 6. If getter is undefined, return undefined.
    let Some(_getter) = descriptor.get else {
        return TryResult::Continue(Value::Undefined);
    };

    // 7. Return ? Call(getter, Receiver).
    // call_function(agent, getter, receiver, None, gc)
    // Note: We cannot call a function without risking GC! There are future
    // options here:
    // 1. Special function types that are guaranteed to trigger no GC.
    // 2. Return a special value that tells which getter to call. Note that the
    //    receiver is statically known, so just returning the getter function
    //    should be enough.
    TryResult::Break(())
}

/// ### [10.1.8.1 OrdinaryGet ( O, P, Receiver )](https://tc39.es/ecma262/#sec-ordinaryget)
pub(crate) fn ordinary_get<'gc>(
    agent: &mut Agent,
    object: OrdinaryObject,
    property_key: PropertyKey,
    receiver: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let object = object.bind(gc.nogc());
    let property_key = property_key.bind(gc.nogc());
    // Note: We scope here because it's likely we've already tried.
    let scoped_object = object.scope(agent, gc.nogc());
    let scoped_property_key = property_key.scope(agent, gc.nogc());
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let Some(descriptor) = object
        .unbind()
        .internal_get_own_property(agent, property_key.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc())
    else {
        // 2. If desc is undefined, then

        // a. Let parent be ? O.[[GetPrototypeOf]]().
        let object = scoped_object.get(agent).bind(gc.nogc());
        let (parent, property_key, receiver) =
            if let TryResult::Continue(parent) = object.try_get_prototype_of(agent, gc.nogc()) {
                let Some(parent) = parent else {
                    return Ok(Value::Undefined);
                };
                (
                    parent,
                    scoped_property_key.get(agent).bind(gc.nogc()),
                    receiver,
                )
            } else {
                // Note: We should root property_key and receiver here.
                let receiver = receiver.scope(agent, gc.nogc());
                let Some(parent) = object
                    .unbind()
                    .internal_get_prototype_of(agent, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc())
                else {
                    return Ok(Value::Undefined);
                };
                let parent = parent.unbind().bind(gc.nogc());
                let receiver = receiver.get(agent);
                (
                    parent,
                    scoped_property_key.get(agent).bind(gc.nogc()),
                    receiver,
                )
            };

        // c. Return ? parent.[[Get]](P, Receiver).
        return parent
            .unbind()
            .internal_get(agent, property_key.unbind(), receiver, gc);
    };

    // 3. If IsDataDescriptor(desc) is true, return desc.[[Value]].
    if let Some(value) = descriptor.value {
        debug_assert!(descriptor.is_data_descriptor());
        return Ok(value.unbind());
    }

    // 4. Assert: IsAccessorDescriptor(desc) is true.
    debug_assert!(descriptor.is_accessor_descriptor());

    // 5. Let getter be desc.[[Get]].
    // 6. If getter is undefined, return undefined.
    let Some(getter) = descriptor.get else {
        return Ok(Value::Undefined);
    };

    // 7. Return ? Call(getter, Receiver).
    call_function(agent, getter.unbind(), receiver.unbind(), None, gc)
}

/// ### [10.1.9.1 OrdinarySet ( O, P, V, Receiver )](https://tc39.es/ecma262/#sec-ordinaryset)
pub(crate) fn ordinary_try_set(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    gc: NoGcScope,
) -> TryResult<bool> {
    let property_key = property_key.bind(gc);

    // 1. Let ownDesc be ! O.[[GetOwnProperty]](P).
    // We're guaranteed to always get a result here.
    let own_descriptor = object.try_get_own_property(agent, property_key, gc)?;

    // 2. Return ? OrdinarySetWithOwnDescriptor(O, P, V, Receiver, ownDesc).
    ordinary_try_set_with_own_descriptor(
        agent,
        object,
        property_key,
        value,
        receiver,
        own_descriptor,
        gc,
    )
}

/// ### [10.1.9.1 OrdinarySet ( O, P, V, Receiver )](https://tc39.es/ecma262/#sec-ordinaryset)
pub(crate) fn ordinary_set<'a>(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let property_key = property_key.bind(gc.nogc());
    // Note: We scope here because it's likely we've already tried.
    let scoped_property_key = property_key.scope(agent, gc.nogc());

    // 1. Let ownDesc be ? O.[[GetOwnProperty]](P).
    let own_descriptor = object
        .internal_get_own_property(agent, property_key.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());

    // 2. Return ? OrdinarySetWithOwnDescriptor(O, P, V, Receiver, ownDesc).
    ordinary_set_with_own_descriptor(
        agent,
        object,
        scoped_property_key,
        value,
        receiver,
        own_descriptor.unbind(),
        gc,
    )
}

/// ### [10.1.9.2 OrdinarySetWithOwnDescriptor ( O, P, V, Receiver, ownDesc )](https://tc39.es/ecma262/#sec-ordinarysetwithowndescriptor)
fn ordinary_try_set_with_own_descriptor(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    own_descriptor: Option<PropertyDescriptor>,
    gc: NoGcScope,
) -> TryResult<bool> {
    let own_descriptor = if let Some(own_descriptor) = own_descriptor {
        own_descriptor
    } else {
        // 1. If ownDesc is undefined, then
        // a. Let parent be ! O.[[GetPrototypeOf]]().
        let parent = unwrap_try(object.try_get_prototype_of(agent, gc));

        // b. If parent is not null, then
        if let Some(parent) = parent {
            // i. Return ? parent.[[Set]](P, V, Receiver).
            // Note: Here we do not have guarantees: Parent could be a Proxy.
            return parent.try_set(agent, property_key, value, receiver, gc);
        }
        // c. Else,
        else {
            // i. Set ownDesc to the PropertyDescriptor {
            //      [[Value]]: undefined, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true
            //    }.
            PropertyDescriptor {
                value: Some(Value::Undefined),
                writable: Some(true),
                enumerable: Some(true),
                configurable: Some(true),
                ..Default::default()
            }
        }
    };

    // 2. If IsDataDescriptor(ownDesc) is true, then
    if own_descriptor.is_data_descriptor() {
        // a. If ownDesc.[[Writable]] is false, return false.
        if own_descriptor.writable == Some(false) {
            return TryResult::Continue(false);
        }

        // b. If Receiver is not an Object, return false.
        let Ok(receiver) = Object::try_from(receiver) else {
            return TryResult::Continue(false);
        };

        // c. Let existingDescriptor be ? Receiver.[[GetOwnProperty]](P).
        // Note: Here again we do not have guarantees; the receiver could be a
        // Proxy.
        let existing_descriptor = receiver.try_get_own_property(agent, property_key, gc)?;

        // d. If existingDescriptor is not undefined, then
        if let Some(existing_descriptor) = existing_descriptor {
            // i. If IsAccessorDescriptor(existingDescriptor) is true, return false.
            if existing_descriptor.is_accessor_descriptor() {
                return TryResult::Continue(false);
            }

            // ii. If existingDescriptor.[[Writable]] is false, return false.
            if existing_descriptor.writable == Some(false) {
                return TryResult::Continue(false);
            }

            // iii. Let valueDesc be the PropertyDescriptor { [[Value]]: V }.
            let value_descriptor = PropertyDescriptor {
                value: Some(value.unbind()),
                ..Default::default()
            };

            // iv. Return ? Receiver.[[DefineOwnProperty]](P, valueDesc).
            // Again: Receiver could be a Proxy.
            return receiver.try_define_own_property(agent, property_key, value_descriptor, gc);
        }
        // e. Else,
        else {
            // i. Assert: Receiver does not currently have a property P.
            let result = unwrap_try(receiver.try_get_own_property(agent, property_key, gc));
            debug_assert!(result.is_none());

            // ii. Return ? CreateDataProperty(Receiver, P, V).
            // Again: Receiver could be a Proxy.
            return try_create_data_property(agent, receiver, property_key, value, gc);
        }
    }

    // 3. Assert: IsAccessorDescriptor(ownDesc) is true.
    debug_assert!(own_descriptor.is_accessor_descriptor());

    // 4. Let setter be ownDesc.[[Set]].
    // 5. If setter is undefined, return false.
    let Some(_setter) = own_descriptor.set else {
        return TryResult::Continue(false);
    };

    // 6. Perform ? Call(setter, Receiver, « V »).
    // Note: We cannot call a function as it may trigger GC. See above for
    // future options.
    // call_function(agent, setter, receiver, Some(ArgumentsList(&[value])), gc)?;

    // 7. Return true.
    // Some(true)
    TryResult::Break(())
}

/// ### [10.1.9.2 OrdinarySetWithOwnDescriptor ( O, P, V, Receiver, ownDesc )](https://tc39.es/ecma262/#sec-ordinarysetwithowndescriptor)
fn ordinary_set_with_own_descriptor<'a>(
    agent: &mut Agent,
    object: Object,
    scoped_property_key: Scoped<PropertyKey>,
    value: Value,
    receiver: Value,
    own_descriptor: Option<PropertyDescriptor>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let mut value = value.bind(gc.nogc());
    let receiver = receiver.bind(gc.nogc());
    let property_key = scoped_property_key.get(agent).bind(gc.nogc());
    let own_descriptor = if let Some(own_descriptor) = own_descriptor {
        own_descriptor.bind(gc.nogc())
    } else {
        // 1. If ownDesc is undefined, then
        // a. Let parent be ? O.[[GetPrototypeOf]]().
        // Note: OrdinaryObject never fails to get prototype.
        let parent = unwrap_try(object.try_get_prototype_of(agent, gc.nogc()));

        // b. If parent is not null, then
        if let Some(parent) = parent {
            // i. Return ? parent.[[Set]](P, V, Receiver).
            // Note: Prototype might be a Proxy or contain a setter.
            return parent.unbind().internal_set(
                agent,
                property_key.unbind(),
                value.unbind(),
                receiver.unbind(),
                gc,
            );
        }
        // c. Else,
        else {
            // i. Set ownDesc to the PropertyDescriptor {
            //      [[Value]]: undefined, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: true
            //    }.
            PropertyDescriptor {
                value: Some(Value::Undefined),
                writable: Some(true),
                enumerable: Some(true),
                configurable: Some(true),
                ..Default::default()
            }
        }
    };

    // 2. If IsDataDescriptor(ownDesc) is true, then
    if own_descriptor.is_data_descriptor() {
        // a. If ownDesc.[[Writable]] is false, return false.
        if own_descriptor.writable == Some(false) {
            return Ok(false);
        }

        // b. If Receiver is not an Object, return false.
        let Ok(mut receiver) = Object::try_from(receiver) else {
            return Ok(false);
        };

        let property_key = scoped_property_key.get(agent).bind(gc.nogc());
        // c. Let existingDescriptor be ? Receiver.[[GetOwnProperty]](P).
        let existing_descriptor = if let TryResult::Continue(desc) =
            receiver.try_get_own_property(agent, property_key, gc.nogc())
        {
            desc
        } else {
            let scoped_receiver = receiver.scope(agent, gc.nogc());
            let scoped_value = value.scope(agent, gc.nogc());
            let desc = receiver
                .unbind()
                .internal_get_own_property(agent, scoped_property_key.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: Neither are shared.
            unsafe {
                value = scoped_value.take(agent).bind(gc.nogc());
                receiver = scoped_receiver.take(agent).bind(gc.nogc());
            }
            desc
        };

        // d. If existingDescriptor is not undefined, then
        if let Some(existing_descriptor) = existing_descriptor {
            // i. If IsAccessorDescriptor(existingDescriptor) is true, return false.
            if existing_descriptor.is_accessor_descriptor() {
                return Ok(false);
            }

            // ii. If existingDescriptor.[[Writable]] is false, return false.
            if existing_descriptor.writable == Some(false) {
                return Ok(false);
            }

            // iii. Let valueDesc be the PropertyDescriptor { [[Value]]: V }.
            let value_descriptor = PropertyDescriptor {
                value: Some(value),
                ..Default::default()
            };

            // iv. Return ? Receiver.[[DefineOwnProperty]](P, valueDesc).
            return receiver.unbind().internal_define_own_property(
                agent,
                scoped_property_key.get(agent).unbind(),
                value_descriptor.unbind(),
                gc,
            );
        }
        // e. Else,
        else {
            // i. Assert: Receiver does not currently have a property P.
            // Note: Kiesel and Ladybird check receiver object's property
            // storage here. Boa does nothing. We cannot test property storage
            // as only ordinary objects really have one, other objects at most
            // set up some partial property storage.

            // ii. Return ? CreateDataProperty(Receiver, P, V).
            return create_data_property(
                agent,
                receiver.unbind(),
                scoped_property_key.get(agent),
                value.unbind(),
                gc,
            );
        }
    }

    // 3. Assert: IsAccessorDescriptor(ownDesc) is true.
    debug_assert!(own_descriptor.is_accessor_descriptor());

    // 4. Let setter be ownDesc.[[Set]].
    // 5. If setter is undefined, return false.
    let Some(setter) = own_descriptor.set else {
        return Ok(false);
    };

    // 6. Perform ? Call(setter, Receiver, « V »).
    call_function(
        agent,
        setter.unbind(),
        receiver.unbind(),
        Some(ArgumentsList::from_mut_slice(&mut [value.unbind()])),
        gc,
    )?;

    // 7. Return true.
    Ok(true)
}

/// ### [10.1.10.1 OrdinaryDelete ( O, P )](https://tc39.es/ecma262/#sec-ordinarydelete)
pub(crate) fn ordinary_delete(
    agent: &mut Agent,
    object: OrdinaryObject,
    property_key: PropertyKey,
    gc: NoGcScope,
) -> bool {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    // We're guaranteed to always get a result here.
    let descriptor = unwrap_try(object.try_get_own_property(agent, property_key, gc));

    // 2. If desc is undefined, return true.
    let Some(descriptor) = descriptor else {
        return true;
    };

    // 3. If desc.[[Configurable]] is true, then
    if let Some(true) = descriptor.configurable {
        // a. Remove the own property with name P from O.
        object
            .into_object()
            .property_storage()
            .remove(agent, property_key);

        // b. Return true.
        return true;
    }

    // 4. Return false.
    false
}

/// ### [10.1.11.1 OrdinaryOwnPropertyKeys ( O )](https://tc39.es/ecma262/#sec-ordinaryownpropertykeys)
pub(crate) fn ordinary_own_property_keys<'a>(
    agent: &Agent,
    object: OrdinaryObject<'a>,
    _: NoGcScope<'a, '_>,
) -> Vec<PropertyKey<'a>> {
    let object_keys = agent[object].keys;
    // 1. Let keys be a new empty List.
    let mut integer_keys = vec![];
    let mut keys = Vec::with_capacity(object_keys.len() as usize);
    let mut symbol_keys = vec![];

    // 3. For each own property key P of O such that P is a String and P is not an array index, in
    //    ascending chronological order of property creation, do
    for key in agent[object_keys].iter() {
        // SAFETY: Keys are all PropertyKeys reinterpreted as Values without
        // conversion.
        let key = unsafe { PropertyKey::from_value_unchecked(key.unwrap()) };
        match key {
            PropertyKey::Integer(integer_key) => {
                let key_value = integer_key.into_i64();
                if (0..u32::MAX as i64).contains(&key_value) {
                    // Integer property key! This requires sorting
                    integer_keys.push(key_value as u32);
                } else {
                    keys.push(key);
                }
            }
            PropertyKey::Symbol(symbol) => symbol_keys.push(symbol),
            // a. Append P to keys.
            _ => keys.push(key),
        }
    }

    // 2. For each own property key P of O such that P is an array index,
    if !integer_keys.is_empty() {
        // in ascending numeric index order, do
        integer_keys.sort();
        // a. Append P to keys.
        keys.splice(0..0, integer_keys.into_iter().map(|key| key.into()));
    }

    // 4. For each own property key P of O such that P is a Symbol,
    if !symbol_keys.is_empty() {
        // in ascending chronological order of property creation, do
        // a. Append P to keys.
        keys.extend(symbol_keys.iter().map(|key| PropertyKey::Symbol(*key)));
    }

    debug_assert_eq!(keys.len() as u32, object_keys.len());

    // 5. Return keys.
    keys
}

/// ### [10.1.12 OrdinaryObjectCreate ( proto \[ , additionalInternalSlotsList \] )](https://tc39.es/ecma262/#sec-ordinaryobjectcreate)
///
/// The abstract operation OrdinaryObjectCreate takes argument proto (an Object
/// or null) and optional argument additionalInternalSlotsList (a List of names
/// of internal slots) and returns an Object. It is used to specify the runtime
/// creation of new ordinary objects. additionalInternalSlotsList contains the
/// names of additional internal slots that must be defined as part of the
/// object, beyond \[\[Prototype]] and \[\[Extensible]]. If
/// additionalInternalSlotsList is not provided, a new empty List is used.
///
/// > NOTE: Although OrdinaryObjectCreate does little more than call
/// > MakeBasicObject, its use communicates the intention to create an ordinary
/// > object, and not an exotic one. Thus, within this specification, it is not
/// > called by any algorithm that subsequently modifies the internal methods of
/// > the object in ways that would make the result non-ordinary. Operations that
/// > create exotic objects invoke MakeBasicObject directly.
///
/// NOTE: In this implementation, `proto_intrinsics` determines the heap in
/// which the object is placed, and therefore its heap data type and its
/// internal slots. If `prototype` is None, the object will be created with the
/// default prototype from the intrinsics, otherwise with the given prototype.
/// To create an object with null prototype, both `proto_intrinsics` and
/// `prototype` must be None.
pub(crate) fn ordinary_object_create_with_intrinsics<'a>(
    agent: &mut Agent,
    proto_intrinsics: Option<ProtoIntrinsics>,
    prototype: Option<Object>,
    gc: NoGcScope<'a, '_>,
) -> Object<'a> {
    let Some(proto_intrinsics) = proto_intrinsics else {
        assert!(prototype.is_none());
        return agent.heap.create_null_object(&[]).into();
    };

    let object = match proto_intrinsics {
        ProtoIntrinsics::Array => agent.heap.create(ArrayHeapData::default()).into_object(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::ArrayBuffer => agent
            .heap
            .create(ArrayBufferHeapData::default())
            .into_object(),
        ProtoIntrinsics::ArrayIterator => agent
            .heap
            .create(ArrayIteratorHeapData::default())
            .into_object(),
        ProtoIntrinsics::BigInt => agent
            .heap
            .create(PrimitiveObjectHeapData::new_big_int_object(0.into()))
            .into_object(),
        ProtoIntrinsics::Boolean => agent
            .heap
            .create(PrimitiveObjectHeapData::new_boolean_object(false))
            .into_object(),
        ProtoIntrinsics::Error => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::Error, None, None))
            .into_object(),
        ProtoIntrinsics::EvalError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::EvalError, None, None))
            .into_object(),
        #[cfg(feature = "date")]
        ProtoIntrinsics::Date => agent.heap.create(DateHeapData::new_invalid()).into_object(),
        ProtoIntrinsics::Function => todo!(),
        ProtoIntrinsics::Number => agent
            .heap
            .create(PrimitiveObjectHeapData::new_number_object(0.into()))
            .into_object(),
        ProtoIntrinsics::Object => agent
            .heap
            .create_object_with_prototype(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into_object(),
                &[],
            )
            .into(),
        ProtoIntrinsics::RangeError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::RangeError, None, None))
            .into_object(),
        ProtoIntrinsics::ReferenceError => agent
            .heap
            .create(ErrorHeapData::new(
                ExceptionType::ReferenceError,
                None,
                None,
            ))
            .into_object(),
        ProtoIntrinsics::String => agent
            .heap
            .create(PrimitiveObjectHeapData::new_string_object(
                String::EMPTY_STRING,
            ))
            .into_object(),
        ProtoIntrinsics::Symbol => agent
            .heap
            .create(PrimitiveObjectHeapData::new_symbol_object(Symbol::from(
                WellKnownSymbolIndexes::AsyncIterator,
            )))
            .into_object(),
        ProtoIntrinsics::SyntaxError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::SyntaxError, None, None))
            .into_object(),
        ProtoIntrinsics::TypeError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::TypeError, None, None))
            .into_object(),
        ProtoIntrinsics::UriError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::UriError, None, None))
            .into_object(),
        ProtoIntrinsics::AggregateError => agent
            .heap
            .create(ErrorHeapData::new(
                ExceptionType::AggregateError,
                None,
                None,
            ))
            .into_object(),
        ProtoIntrinsics::AsyncFunction => todo!(),
        ProtoIntrinsics::AsyncGenerator => agent
            .heap
            .create(AsyncGeneratorHeapData::default())
            .into_object(),
        ProtoIntrinsics::AsyncGeneratorFunction => todo!(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::BigInt64Array => {
            Object::BigInt64Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::BigUint64Array => {
            Object::BigUint64Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::DataView => agent.heap.create(DataViewHeapData::default()).into_object(),
        ProtoIntrinsics::FinalizationRegistry => agent
            .heap
            .create(FinalizationRegistryHeapData::default())
            .into_object(),
        #[cfg(feature = "proposal-float16array")]
        ProtoIntrinsics::Float16Array => {
            Object::Float16Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Float32Array => {
            Object::Float32Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Float64Array => {
            Object::Float64Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        ProtoIntrinsics::Generator => agent
            .heap
            .create(GeneratorHeapData::default())
            .into_object(),
        ProtoIntrinsics::GeneratorFunction => todo!(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int16Array => {
            Object::Int16Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int32Array => {
            Object::Int32Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int8Array => {
            Object::Int8Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        ProtoIntrinsics::Iterator => agent
            .heap
            .create_object_with_prototype(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .iterator_prototype()
                    .into_object(),
                &[],
            )
            .into_object(),
        ProtoIntrinsics::Map => agent.heap.create(MapHeapData::default()).into_object(),
        ProtoIntrinsics::MapIterator => agent
            .heap
            .create(MapIteratorHeapData::default())
            .into_object(),
        ProtoIntrinsics::Promise => agent.heap.create(PromiseHeapData::default()).into_object(),
        #[cfg(feature = "regexp")]
        ProtoIntrinsics::RegExp => agent.heap.create(RegExpHeapData::default()).into_object(),
        #[cfg(feature = "set")]
        ProtoIntrinsics::Set => agent.heap.create(SetHeapData::default()).into_object(),
        #[cfg(feature = "set")]
        ProtoIntrinsics::SetIterator => agent
            .heap
            .create(SetIteratorHeapData::default())
            .into_object(),
        #[cfg(feature = "shared-array-buffer")]
        ProtoIntrinsics::SharedArrayBuffer => agent
            .heap
            .create(SharedArrayBufferHeapData::default())
            .into_object(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint16Array => {
            Object::Uint16Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint32Array => {
            Object::Uint32Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint8Array => {
            Object::Uint8Array(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint8ClampedArray => {
            Object::Uint8ClampedArray(agent.heap.create(TypedArrayHeapData::default()))
        }
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakMap => agent.heap.create(WeakMapHeapData::default()).into_object(),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakRef => agent.heap.create(WeakRefHeapData::default()).into_object(),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakSet => agent.heap.create(WeakSetHeapData::default()).into_object(),
    };

    if let Some(prototype) = prototype {
        object.internal_set_prototype(agent, Some(prototype));
    }

    object.bind(gc)
}

/// ### [10.1.13 OrdinaryCreateFromConstructor ( constructor, intrinsicDefaultProto \[ , internalSlotsList \] )](https://tc39.es/ecma262/#sec-ordinarycreatefromconstructor)
///
/// The abstract operation OrdinaryCreateFromConstructor takes arguments
/// constructor (a constructor) and intrinsicDefaultProto (a String) and
/// optional argument internalSlotsList (a List of names of internal slots) and
/// returns either a normal completion containing an Object or a throw
/// completion. It creates an ordinary object whose \[\[Prototype]] value is
/// retrieved from a constructor's "prototype" property, if it exists.
/// Otherwise the intrinsic named by intrinsicDefaultProto is used for
/// \[\[Prototype]]. internalSlotsList contains the names of additional
/// internal slots that must be defined as part of the object. If
/// internalSlotsList is not provided, a new empty List is used.
///
/// NOTE: In this implementation, `intrinsic_default_proto` also defines which
/// kind of heap data type the created object uses, and therefore which internal
/// slots it has. Therefore the `internalSlotsList` property isn't present.
pub(crate) fn ordinary_create_from_constructor<'a>(
    agent: &mut Agent,
    constructor: Function,
    intrinsic_default_proto: ProtoIntrinsics,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    let constructor = constructor.bind(gc.nogc());
    // 1. Assert: intrinsicDefaultProto is this specification's name of an
    // intrinsic object. The corresponding object must be an intrinsic that is
    // intended to be used as the [[Prototype]] value of an object.

    // 2. Let proto be ? GetPrototypeFromConstructor(constructor, intrinsicDefaultProto).
    let proto = get_prototype_from_constructor(
        agent,
        constructor.unbind(),
        intrinsic_default_proto,
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. If internalSlotsList is present, let slotsList be internalSlotsList.
    // 4. Else, let slotsList be a new empty List.
    // 5. Return OrdinaryObjectCreate(proto, slotsList).
    Ok(ordinary_object_create_with_intrinsics(
        agent,
        Some(intrinsic_default_proto),
        proto.unbind(),
        gc.into_nogc(),
    ))
}

/// ### [10.1.14 GetPrototypeFromConstructor ( constructor, intrinsicDefaultProto )](https://tc39.es/ecma262/#sec-getprototypefromconstructor)
///
/// The abstract operation GetPrototypeFromConstructor takes arguments
/// constructor (a function object) and intrinsicDefaultProto (a String) and
/// returns either a normal completion containing an Object or a throw
/// completion. It determines the \[\[Prototype\]\] value that should be used
/// to create an object corresponding to a specific constructor. The value is
/// retrieved from the constructor's "prototype" property, if it exists.
/// Otherwise the intrinsic named by intrinsicDefaultProto is used for
/// \[\[Prototype\]\].
///
/// NOTE: In this implementation, the function returns None if the prototype it
/// would otherwise return is the prototype that corresponds to
/// `intrinsic_default_proto`.
pub(crate) fn get_prototype_from_constructor<'a>(
    agent: &mut Agent,
    constructor: Function,
    intrinsic_default_proto: ProtoIntrinsics,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Object<'a>>> {
    let mut constructor = constructor.bind(gc.nogc());
    let mut function_realm = try_get_function_realm(agent, constructor, gc.nogc());
    // NOTE: %Constructor%.prototype is an immutable property; we can thus
    // check if we %Constructor% is the ProtoIntrinsic we expect and if it is,
    // return None because we know %Constructor%.prototype corresponds to the
    // ProtoIntrinsic.
    if let Some(intrinsics) =
        function_realm.map(|realm| agent.get_realm_record_by_id(realm).intrinsics())
    {
        let intrinsic_constructor = match intrinsic_default_proto {
            ProtoIntrinsics::AggregateError => Some(intrinsics.aggregate_error().into_function()),
            ProtoIntrinsics::Array => Some(intrinsics.array().into_function()),
            ProtoIntrinsics::ArrayIterator => None,
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::ArrayBuffer => Some(intrinsics.array_buffer().into_function()),
            ProtoIntrinsics::AsyncFunction => Some(intrinsics.async_function().into_function()),
            ProtoIntrinsics::AsyncGenerator => None,
            ProtoIntrinsics::AsyncGeneratorFunction => {
                Some(intrinsics.async_generator_function().into_function())
            }
            ProtoIntrinsics::BigInt => Some(intrinsics.big_int().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigInt64Array => Some(intrinsics.big_int64_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::BigUint64Array => Some(intrinsics.big_uint64_array().into_function()),
            ProtoIntrinsics::Boolean => Some(intrinsics.boolean().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::DataView => Some(intrinsics.data_view().into_function()),
            #[cfg(feature = "date")]
            ProtoIntrinsics::Date => Some(intrinsics.date().into_function()),
            ProtoIntrinsics::Error => Some(intrinsics.error().into_function()),
            ProtoIntrinsics::EvalError => Some(intrinsics.eval_error().into_function()),
            ProtoIntrinsics::FinalizationRegistry => {
                Some(intrinsics.finalization_registry().into_function())
            }
            #[cfg(feature = "proposal-float16array")]
            ProtoIntrinsics::Float16Array => Some(intrinsics.float16_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Float32Array => Some(intrinsics.float32_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Float64Array => Some(intrinsics.float64_array().into_function()),
            ProtoIntrinsics::Function => Some(intrinsics.function().into_function()),
            ProtoIntrinsics::Generator => None,
            ProtoIntrinsics::GeneratorFunction => {
                Some(intrinsics.generator_function().into_function())
            }
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int16Array => Some(intrinsics.int16_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int32Array => Some(intrinsics.int32_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Int8Array => Some(intrinsics.int8_array().into_function()),
            ProtoIntrinsics::Iterator => Some(intrinsics.iterator().into_function()),
            ProtoIntrinsics::Map => Some(intrinsics.map().into_function()),
            ProtoIntrinsics::MapIterator => None,
            ProtoIntrinsics::Number => Some(intrinsics.number().into_function()),
            ProtoIntrinsics::Object => Some(intrinsics.object().into_function()),
            ProtoIntrinsics::Promise => Some(intrinsics.promise().into_function()),
            ProtoIntrinsics::RangeError => Some(intrinsics.range_error().into_function()),
            ProtoIntrinsics::ReferenceError => Some(intrinsics.reference_error().into_function()),
            #[cfg(feature = "regexp")]
            ProtoIntrinsics::RegExp => Some(intrinsics.reg_exp().into_function()),
            #[cfg(feature = "set")]
            ProtoIntrinsics::Set => Some(intrinsics.set().into_function()),
            #[cfg(feature = "set")]
            ProtoIntrinsics::SetIterator => None,
            #[cfg(feature = "shared-array-buffer")]
            ProtoIntrinsics::SharedArrayBuffer => {
                Some(intrinsics.shared_array_buffer().into_function())
            }
            ProtoIntrinsics::String => Some(intrinsics.string().into_function()),
            ProtoIntrinsics::Symbol => Some(intrinsics.symbol().into_function()),
            ProtoIntrinsics::SyntaxError => Some(intrinsics.syntax_error().into_function()),
            ProtoIntrinsics::TypeError => Some(intrinsics.type_error().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint16Array => Some(intrinsics.uint16_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint32Array => Some(intrinsics.uint32_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8Array => Some(intrinsics.uint8_array().into_function()),
            #[cfg(feature = "array-buffer")]
            ProtoIntrinsics::Uint8ClampedArray => {
                Some(intrinsics.uint8_clamped_array().into_function())
            }
            ProtoIntrinsics::UriError => Some(intrinsics.uri_error().into_function()),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakMap => Some(intrinsics.weak_map().into_function()),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakRef => Some(intrinsics.weak_ref().into_function()),
            #[cfg(feature = "weak-refs")]
            ProtoIntrinsics::WeakSet => Some(intrinsics.weak_set().into_function()),
        };
        if Some(constructor) == intrinsic_constructor {
            // The ProtoIntrinsic's constructor matches the constructor we're
            // being called with, so the constructor's prototype matches the
            // ProtoIntrinsic.
            return Ok(None);
        }
    }

    // 1. Assert: intrinsicDefaultProto is this specification's name of an
    // intrinsic object. The corresponding object must be an intrinsic that is
    // intended to be used as the [[Prototype]] value of an object.
    // 2. Let proto be ? Get(constructor, "prototype").
    let prototype_key = BUILTIN_STRING_MEMORY.prototype.into();
    let proto =
        if let TryResult::Continue(proto) = try_get(agent, constructor, prototype_key, gc.nogc()) {
            proto
        } else {
            let scoped_realm = function_realm.map(|r| r.scope(agent, gc.nogc()));
            let scoped_constructor = constructor.scope(agent, gc.nogc());
            let proto = get(agent, constructor.unbind(), prototype_key, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: scoped_constructor is not shared.
            constructor = unsafe { scoped_constructor.take(agent) }.bind(gc.nogc());
            // SAFETY: scoped_realm is not shared.
            function_realm = scoped_realm.map(|r| unsafe { r.take(agent) }.bind(gc.nogc()));
            proto
        };
    match Object::try_from(proto) {
        // 3. If proto is not an Object, then
        Err(_) => {
            // a. Let realm be ? GetFunctionRealm(constructor).
            // b. Set proto to realm's intrinsic object named intrinsicDefaultProto.
            // Note: We signify using the default proto by returning None.
            // We only need to call the get_function_realm function if it would
            // throw an error.
            if function_realm.is_none() {
                let err = get_function_realm(agent, constructor.unbind(), gc.nogc()).unwrap_err();
                return Err(err.unbind());
            }
            Ok(None)
        }
        Ok(proto) => {
            // 4. Return proto.
            // Note: We should still check if the proto is the default proto.
            // It's possible that a user's custom constructor object has
            // prototype property set to the default.
            if let Some(realm) = function_realm {
                let default_proto = agent
                    .get_realm_record_by_id(realm)
                    .intrinsics()
                    .get_intrinsic_default_proto(intrinsic_default_proto);
                if proto == default_proto {
                    return Ok(None);
                }
            }
            Ok(Some(proto.unbind().bind(gc.into_nogc())))
        }
    }
}

/// 10.4.7.2 SetImmutablePrototype ( O, V )
///
/// The abstract operation SetImmutablePrototype takes arguments O (an Object)
/// and V (an Object or null) and returns either a normal completion containing
/// a Boolean or a throw completion.
#[inline]
pub(crate) fn set_immutable_prototype(
    agent: &mut Agent,
    o: Module,
    v: Option<Object>,
    gc: NoGcScope,
) -> bool {
    // 1. Let current be ? O.[[GetPrototypeOf]]().
    let current = unwrap_try(o.try_get_prototype_of(agent, gc));
    // 2. If SameValue(V, current) is true, return true.
    // 3. Return false.
    v == current
}

impl HeapMarkAndSweep for OrdinaryObject<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.objects.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.objects.shift_index(&mut self.0);
    }
}
