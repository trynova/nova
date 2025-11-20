// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod caches;
mod shape;

pub use caches::*;
pub use shape::*;

use std::{
    collections::{TryReserveError, hash_map::Entry},
    ops::ControlFlow,
    vec,
};

#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::SharedDataViewRecord;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::try_get_result_into_value;
#[cfg(feature = "temporal")]
use crate::ecmascript::{DurationHeapData, InstantRecord, PlainTimeHeapData};
use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, ExceptionType, Function, InternalMethods,
        InternalSlots, JsResult, Object, OrdinaryObject, PropertyDescriptor, PropertyKey,
        ProtoIntrinsics, Realm, SetAtOffsetProps, SetResult, String, Symbol, TryError,
        TryGetResult, TryHasResult, TryResult, Value, call_function, create_data_property,
        get_function_realm, handle_try_get_result, same_value, try_create_data_property, try_get,
        try_get_function_realm, try_result_into_js, unwrap_try,
    },
    engine::{Bindable, GcScope, NoGcScope, Scopable, Scoped},
    heap::{
        CreateHeapData, WellKnownSymbolIndexes, {ElementStorageRef, PropertyStorageRef},
    },
};

#[cfg(feature = "date")]
use crate::ecmascript::DateHeapData;
#[cfg(feature = "regexp")]
use crate::ecmascript::RegExpHeapData;
#[cfg(feature = "shared-array-buffer")]
use crate::ecmascript::SharedArrayBufferRecord;
#[cfg(feature = "array-buffer")]
use crate::ecmascript::{ArrayBufferHeapData, DataViewRecord, TypedArrayRecord};
use crate::ecmascript::{
    ArrayHeapData, ArrayIteratorHeapData, AsyncGeneratorHeapData, ErrorHeapData,
    FinalizationRegistryRecord, GeneratorHeapData, MapHeapData, MapIteratorHeapData, Module,
    PrimitiveObjectRecord, PromiseHeapData, StringIteratorHeapData,
};
#[cfg(feature = "set")]
use crate::ecmascript::{SetHeapData, SetIteratorHeapData};
#[cfg(feature = "weak-refs")]
use crate::ecmascript::{WeakMapRecord, WeakRefHeapData, WeakSetHeapData};

/// ## [10.1 Ordinary Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots)
impl<'a> InternalMethods<'a> for OrdinaryObject<'a> {
    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryGetResult<'gc> {
        let offset = offset.get_property_offset();
        let obj = self.bind(gc);
        let data = obj.get_elements_storage(agent);
        if let Some(v) = data.values[offset as usize] {
            v.into()
        } else {
            let d = data
                .descriptors
                .and_then(|d| d.get(&(offset as u32)))
                .unwrap();
            d.getter_function(gc)
                .map_or(TryGetResult::Value(Value::Undefined), TryGetResult::Get)
        }
    }

    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        props: &SetAtOffsetProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        ordinary_set_at_offset(agent, props, self.into(), Some(self), offset, gc)
    }
}

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
    gc: NoGcScope,
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
    // 7. Repeat, while done is false,
    // a. If p is null, then
    while let Some(p_inner) = p {
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
                break;
            } else {
                // ii. Else, set p to p.[[Prototype]].
                p = p_inner.internal_prototype(agent);
            }
        }
    }
    // i. Set done to true.

    // For caching reasons, prototype objects must always have "intrinsic"
    // shapes. We have to ensure the to-be prototype object has one now.
    if let Some(prototype) = prototype
        && !prototype.is_proxy()
        && !prototype.is_module()
    {
        prototype
            .get_or_create_backing_object(agent)
            .make_intrinsic(agent)
            .expect("Should perform GC here");
    }

    // 8. Set O.[[Prototype]] to V.
    let old_shape = object
        .get_backing_object(agent)
        .map(|o| o.object_shape(agent));
    object.internal_set_prototype(agent, prototype);

    if let Some(shape) = old_shape
        && shape.is_intrinsic(agent)
    {
        // We changed prototype of an intrinsic object and must invalidate
        // associated caches.
        Caches::invalidate_caches_on_intrinsic_shape_prototype_change(
            agent, object, shape, current, prototype, gc,
        );
    }

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
    agent: &mut Agent,
    object: Object,
    backing_object: OrdinaryObject,
    property_key: PropertyKey,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'a, '_>,
) -> Option<PropertyDescriptor<'a>> {
    let (value, x, offset) = if let Some(cache) = cache
        && let shape = backing_object.object_shape(agent)
        && let Some((offset, _)) = cache.find_cached_property_offset(agent, shape)
    {
        // A cache-based lookup on an ordinary object can fully rely on the
        // Object Shape and caches.
        // Found a cached result.
        // 1. If O does not have an own property with key P,
        if offset.is_unset() || offset.is_prototype_property() {
            // return undefined.
            return None;
        }
        let offset = offset.get_property_offset() as u32;
        let ElementStorageRef {
            values,
            descriptors,
        } = backing_object.get_elements_storage(agent);
        let value = &values[offset as usize];
        let x = descriptors.and_then(|d| d.get(&offset));
        (value, x, offset)
    } else {
        // 1. If O does not have an own property with key P, return undefined.
        // 3. Let X be O's own property whose key is P.
        backing_object.property_storage().get(agent, property_key)?
    };

    // 2. Let D be a newly created Property Descriptor with no fields.
    let mut descriptor = PropertyDescriptor::default();

    // 4. If X is a data property, then
    if let Some(value) = value {
        // a. Set D.[[Value]] to the value of X's [[Value]] attribute.
        descriptor.value = Some(value.bind(gc));

        // b. Set D.[[Writable]] to the value of X's [[Writable]] attribute.
        descriptor.writable = Some(x.is_none_or(|x| x.is_writable().unwrap()));
    } else {
        // 5. Else,
        // a. Assert: X is an accessor property.
        let x = x.unwrap();
        debug_assert!(x.is_accessor_descriptor());

        // b. Set D.[[Get]] to the value of X's [[Get]] attribute.
        descriptor.get = Some(x.getter_function(gc));

        // c. Set D.[[Set]] to the value of X's [[Set]] attribute.
        descriptor.set = Some(x.setter_function(gc));
    }

    // 6. Set D.[[Enumerable]] to the value of X's [[Enumerable]] attribute.
    descriptor.enumerable = Some(x.is_none_or(|x| x.is_enumerable()));

    // 7. Set D.[[Configurable]] to the value of X's [[Configurable]] attribute.
    descriptor.configurable = Some(x.is_none_or(|x| x.is_configurable()));

    if let Some(CacheToPopulate {
        receiver,
        cache,
        key: _,
        shape,
    }) = agent
        .heap
        .caches
        .take_current_cache_to_populate(property_key)
    {
        let ov: Value = object.into();
        let is_receiver = ov == receiver;
        if is_receiver {
            cache.insert_lookup_offset(agent, shape, offset);
        } else {
            cache.insert_prototype_lookup_offset(agent, shape, offset, object);
        }
    }

    // 8. Return D.
    Some(descriptor)
}

/// ### [10.1.6.1 OrdinaryDefineOwnProperty ( O, P, Desc )](https://tc39.es/ecma262/#sec-ordinarydefineownproperty)
pub(crate) fn ordinary_define_own_property<'gc>(
    agent: &mut Agent,
    o: Object,
    backing_object: OrdinaryObject,
    property_key: PropertyKey,
    descriptor: PropertyDescriptor,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    // Note: OrdinaryDefineOwnProperty is only used by the backing object type,
    // meaning that we know that this method cannot call into JavaScript.
    // 1. Let current be ! O.[[GetOwnProperty]](P).
    let current = ordinary_get_own_property(agent, o, backing_object, property_key, cache, gc);

    // 2. Let extensible be ! IsExtensible(O).
    let extensible = backing_object.internal_extensible(agent);

    // 3. Return ValidateAndApplyPropertyDescriptor(O, P, extensible, Desc, current).
    validate_and_apply_property_descriptor(
        agent,
        Some((o, backing_object)),
        property_key,
        extensible,
        descriptor,
        current,
        gc,
    )
    .map_err(|err| agent.throw_allocation_exception(err, gc))
}

/// ### [10.1.6.2 IsCompatiblePropertyDescriptor ( Extensible, Desc, Current )](https://tc39.es/ecma262/#sec-iscompatiblepropertydescriptor)
pub(crate) fn is_compatible_property_descriptor(
    agent: &mut Agent,
    extensible: bool,
    descriptor: PropertyDescriptor,
    current: Option<PropertyDescriptor>,
    gc: NoGcScope,
) -> Result<bool, TryReserveError> {
    let property_key = PropertyKey::from_str(agent, "", gc);
    validate_and_apply_property_descriptor(
        agent,
        None,
        property_key,
        extensible,
        descriptor,
        current,
        gc,
    )
}

/// ### [10.1.6.3 ValidateAndApplyPropertyDescriptor ( O, P, extensible, Desc, current )](https://tc39.es/ecma262/#sec-validateandapplypropertydescriptor)
fn validate_and_apply_property_descriptor(
    agent: &mut Agent,
    o: Option<(Object, OrdinaryObject)>,
    property_key: PropertyKey,
    extensible: bool,
    descriptor: PropertyDescriptor,
    current: Option<PropertyDescriptor>,
    gc: NoGcScope,
) -> Result<bool, TryReserveError> {
    // 1. Assert: IsPropertyKey(P) is true.

    // 2. If current is undefined, then
    let Some(current) = current else {
        // a. If extensible is false, return false.
        if !extensible {
            return Ok(false);
        }

        // b. If O is undefined, return true.
        let Some((o, backing_object)) = o else {
            return Ok(true);
        };

        // c. If IsAccessorDescriptor(Desc) is true, then
        if descriptor.is_accessor_descriptor() {
            // i. Create an own accessor property named P of object O whose [[Get]], [[Set]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            backing_object.property_storage().set(
                agent,
                o,
                property_key,
                PropertyDescriptor {
                    get: Some(descriptor.get.unwrap_or(None)),
                    set: Some(descriptor.set.unwrap_or(None)),
                    enumerable: Some(descriptor.enumerable.unwrap_or(false)),
                    configurable: Some(descriptor.configurable.unwrap_or(false)),
                    ..Default::default()
                },
                gc,
            )?;
        }
        // d. Else,
        else {
            // i. Create an own data property named P of object O whose [[Value]], [[Writable]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            backing_object.property_storage().set(
                agent,
                o,
                property_key,
                PropertyDescriptor {
                    value: Some(descriptor.value.unwrap_or(Value::Undefined)),
                    writable: Some(descriptor.writable.unwrap_or(false)),
                    enumerable: Some(descriptor.enumerable.unwrap_or(false)),
                    configurable: Some(descriptor.configurable.unwrap_or(false)),
                    ..Default::default()
                },
                gc,
            )?;
        }

        // e. Return true.
        return Ok(true);
    };

    // 3. Assert: current is a fully populated Property Descriptor.
    debug_assert!(current.is_fully_populated());

    // 4. If Desc does not have any fields, return true.
    if !descriptor.has_fields() {
        return Ok(true);
    }

    // 5. If current.[[Configurable]] is false, then
    if !current.configurable.unwrap() {
        // a. If Desc has a [[Configurable]] field and Desc.[[Configurable]] is true, return false.
        if descriptor.configurable == Some(true) {
            return Ok(false);
        }

        // b. If Desc has an [[Enumerable]] field and SameValue(Desc.[[Enumerable]], current.[[Enumerable]])
        //    is false, return false.
        if descriptor.enumerable.is_some() && descriptor.enumerable != current.enumerable {
            return Ok(false);
        }

        // c. If IsGenericDescriptor(Desc) is false and SameValue(IsAccessorDescriptor(Desc), IsAccessorDescriptor(current))
        //    is false, return false.
        if !descriptor.is_generic_descriptor()
            && descriptor.is_accessor_descriptor() != current.is_accessor_descriptor()
        {
            return Ok(false);
        }

        // d. If IsAccessorDescriptor(current) is true, then
        if current.is_accessor_descriptor() {
            // i. If Desc has a [[Get]] field and SameValue(Desc.[[Get]], current.[[Get]]) is false,
            //    return false.
            if let Some(desc_get) = descriptor.get {
                if let Some(cur_get) = current.get {
                    if desc_get != cur_get {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }

            // ii. If Desc has a [[Set]] field and SameValue(Desc.[[Set]], current.[[Set]]) is
            //     false, return false.
            if let Some(desc_set) = descriptor.set {
                if let Some(cur_set) = current.set {
                    if desc_set != cur_set {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
        }
        // e. Else if current.[[Writable]] is false, then
        else if current.writable == Some(false) {
            // i. If Desc has a [[Writable]] field and Desc.[[Writable]] is true, return false.
            if descriptor.writable == Some(true) {
                return Ok(false);
            }

            // ii. If Desc has a [[Value]] field and SameValue(Desc.[[Value]], current.[[Value]])
            //     is false, return false.
            if let Some(desc_value) = descriptor.value {
                if let Some(cur_value) = current.value {
                    if !same_value(agent, desc_value, cur_value) {
                        return Ok(false);
                    }
                } else {
                    return Ok(false);
                }
            }
        }
    }

    // 6. If O is not undefined, then
    if let Some((o, backing_object)) = o {
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
            backing_object.property_storage().set(
                agent,
                o,
                property_key,
                PropertyDescriptor {
                    get: Some(descriptor.get.unwrap_or(None)),
                    set: Some(descriptor.set.unwrap_or(None)),
                    enumerable: Some(enumerable),
                    configurable: Some(configurable),
                    ..Default::default()
                },
                gc,
            )?;
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
            backing_object.property_storage().set(
                agent,
                o,
                property_key,
                PropertyDescriptor {
                    value: Some(descriptor.value.unwrap_or(Value::Undefined)),
                    writable: Some(descriptor.writable.unwrap_or(false)),
                    enumerable: Some(enumerable),
                    configurable: Some(configurable),
                    ..Default::default()
                },
                gc,
            )?;
        }
        // c. Else,
        else {
            // i. For each field of Desc, set the corresponding attribute of the property named P
            //    of object O to the value of the field.
            backing_object.property_storage().set(
                agent,
                o,
                property_key,
                PropertyDescriptor {
                    value: descriptor.value.or(current.value),
                    writable: descriptor.writable.or(current.writable),
                    get: descriptor.get.or(current.get),
                    set: descriptor.set.or(current.set),
                    enumerable: descriptor.enumerable.or(current.enumerable),
                    configurable: descriptor.configurable.or(current.configurable),
                },
                gc,
            )?;
        }
    }

    // 7. Return true.
    Ok(true)
}

/// ### [10.1.7.1 OrdinaryHasProperty ( O, P )](https://tc39.es/ecma262/#sec-ordinaryhasproperty)
pub(crate) fn ordinary_try_has_property<'gc>(
    agent: &mut Agent,
    object: Object,
    backing_object: Option<OrdinaryObject>,
    property_key: PropertyKey,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, TryHasResult<'gc>> {
    if let Some(cache) = cache {
        // A cache-based lookup on an ordinary object can fully rely on the
        // Object Shape and caches.
        let shape = if let Some(bo) = backing_object {
            bo.object_shape(agent)
        } else {
            object.object_shape(agent)
        };
        if let Some((offset, prototype)) = cache.find_cached_property_offset(agent, shape) {
            // Found a cached result.
            return if offset.is_unset() {
                TryHasResult::Unset.into()
            } else {
                TryHasResult::Offset(
                    offset.get_property_offset() as u32,
                    prototype.unwrap_or(object).bind(gc),
                )
                .into()
            };
        }
    }

    // 1. Let hasOwn be ? O.[[GetOwnProperty]](P).
    let has_own = backing_object.and_then(|bo| {
        bo.object_shape(agent)
            .keys(&agent.heap.object_shapes, &agent.heap.elements)
            .iter()
            .enumerate()
            .find(|(_, p)| *p == &property_key)
            .map(|(i, _)| i as u32)
    });

    // 2. If hasOwn is not undefined, return true.
    if let Some(offset) = has_own {
        if let Some(CacheToPopulate {
            receiver,
            cache,
            key: _,
            shape,
        }) = agent
            .heap
            .caches
            .take_current_cache_to_populate(property_key)
        {
            let ov: Value = object.into();
            let is_receiver = ov == receiver;
            if is_receiver {
                cache.insert_lookup_offset(agent, shape, offset);
            } else {
                cache.insert_prototype_lookup_offset(agent, shape, offset, object);
            }
        }
        return TryHasResult::Offset(offset, object.bind(gc)).into();
    };

    // 3. Let parent be ? O.[[GetPrototypeOf]]().
    // Note: ? means that if we'd call a Proxy's GetPrototypeOf trap then we'll
    // instead return None.
    let parent = match backing_object
        .map_or(object, |bo| bo.into())
        .try_get_prototype_of(agent, gc)
    {
        ControlFlow::Continue(p) => p,
        ControlFlow::Break(_) => return TryError::GcError.into(),
    };

    // 4. If parent is not null, then
    if let Some(parent) = parent {
        // a. Return ? parent.[[HasProperty]](P).
        // Note: Here too, if we would call a Proxy's HasProperty trap then
        // we'll instead return None.
        return if cache.is_some() {
            let result = parent.try_has_property(agent, property_key, None, gc);
            agent.heap.caches.clear_current_cache_to_populate();
            result
        } else {
            parent.try_has_property(agent, property_key, None, gc)
        };
    }

    if let Some(CacheToPopulate {
        receiver: _,
        cache,
        key: _,
        shape,
    }) = agent
        .heap
        .caches
        .take_current_cache_to_populate(property_key)
    {
        cache.insert_unset(agent, shape);
    }

    // 5. Return false.
    TryHasResult::Unset.into()
}

/// ### [10.1.7.1 OrdinaryHasProperty ( O, P )](https://tc39.es/ecma262/#sec-ordinaryhasproperty)
pub(crate) fn ordinary_has_property<'a>(
    agent: &mut Agent,
    object: Object,
    backing_object: OrdinaryObject,
    property_key: PropertyKey,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let backing_object = backing_object.bind(gc.nogc());
    let property_key = property_key.bind(gc.nogc());
    // 1. Let hasOwn be ? O.[[GetOwnProperty]](P).

    let has_own = backing_object
        .object_shape(agent)
        .keys(&agent.heap.object_shapes, &agent.heap.elements)
        .iter()
        .enumerate()
        .find(|(_, p)| *p == &property_key)
        .map(|(i, _)| i as u32);

    // 2. If hasOwn is not undefined, return true.
    if let Some(offset) = has_own {
        if let Some(CacheToPopulate {
            receiver,
            cache,
            key: _,
            shape,
        }) = agent
            .heap
            .caches
            .take_current_cache_to_populate(property_key)
        {
            let ov: Value = object.into();
            let is_receiver = ov == receiver;
            if is_receiver {
                cache.insert_lookup_offset(agent, shape, offset);
            } else {
                cache.insert_prototype_lookup_offset(agent, shape, offset, object);
            }
        }
        return Ok(true);
    };

    // 3. Let parent be ? O.[[GetPrototypeOf]]().
    let parent = backing_object.internal_prototype(agent).bind(gc.nogc());

    // 4. If parent is not null, then
    if let Some(parent) = parent {
        // a. Return ? parent.[[HasProperty]](P).
        return parent
            .unbind()
            .internal_has_property(agent, property_key.unbind(), gc);
    }

    if let Some(CacheToPopulate {
        receiver: _,
        cache,
        key: _,
        shape,
    }) = agent
        .heap
        .caches
        .take_current_cache_to_populate(property_key)
    {
        cache.insert_unset(agent, shape);
    }

    // 5. Return false.
    Ok(false)
}

#[cfg(feature = "array-buffer")]
pub(crate) fn ordinary_has_property_entry<'a, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'a>,
    property_key: PropertyKey,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, bool> {
    let property_key = property_key.bind(gc.nogc());
    match object.get_backing_object(agent) {
        Some(backing_object) => ordinary_has_property(
            agent,
            object.into(),
            backing_object,
            property_key.unbind(),
            gc,
        ),
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
    object: Object,
    backing_object: Option<OrdinaryObject>,
    property_key: PropertyKey,
    receiver: Value,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, TryGetResult<'gc>> {
    let object = object.bind(gc);
    let backing_object = backing_object.bind(gc);
    let property_key = property_key.bind(gc);
    let receiver = receiver.bind(gc);

    if let Some(cache) = cache {
        // A cache-based lookup on an ordinary object can fully rely on the
        // Object Shape and caches.
        let shape = if let Some(bo) = backing_object {
            bo.object_shape(agent)
        } else {
            object.object_shape(agent)
        };
        if let Some(result) = shape.get_cached(agent, property_key, object.into(), cache, gc) {
            // Found a cached result.
            return result.into();
        }
    }

    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let Some(descriptor) = backing_object
        .and_then(|bo| ordinary_get_own_property(agent, object, bo, property_key, None, gc))
    else {
        // 2. If desc is undefined, then

        // a. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = object.internal_prototype(agent);

        // b. If parent is null, return undefined.
        let Some(parent) = parent else {
            if let Some(CacheToPopulate {
                receiver: _,
                cache,
                key: _,
                shape,
            }) = agent
                .heap
                .caches
                .take_current_cache_to_populate(property_key)
            {
                cache.insert_unset(agent, shape);
            }
            return TryGetResult::Unset.into();
        };
        // c. Return ? parent.[[Get]](P, Receiver).
        return if cache.is_some() {
            let result = parent.try_get(agent, property_key, receiver, None, gc);
            agent.heap.caches.clear_current_cache_to_populate();
            result
        } else {
            parent.try_get(agent, property_key, receiver, None, gc)
        };
    };

    // 3. If IsDataDescriptor(desc) is true, return desc.[[Value]].
    if let Some(value) = descriptor.value {
        debug_assert!(descriptor.is_data_descriptor());
        return TryGetResult::Value(value).into();
    }

    // 4. Assert: IsAccessorDescriptor(desc) is true.
    debug_assert!(descriptor.is_accessor_descriptor());

    // 5. Let getter be desc.[[Get]].
    // 6. If getter is undefined, return undefined.
    let Some(Some(getter)) = descriptor.get else {
        return TryGetResult::Unset.into();
    };

    // 7. Return ? Call(getter, Receiver).
    // call_function(agent, getter, receiver, None, gc)
    // Note: We cannot call a function without risking GC! There are future
    // options here:
    // 1. Special function types that are guaranteed to trigger no GC.
    // 2. Return a special value that tells which getter to call. Note that the
    //    receiver is statically known, so just returning the getter function
    //    should be enough.
    TryGetResult::Get(getter).into()
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
    let Some(Some(getter)) = descriptor.get else {
        return Ok(Value::Undefined);
    };

    // 7. Return ? Call(getter, Receiver).
    call_function(agent, getter.unbind(), receiver.unbind(), None, gc)
}

/// ### [10.1.9.1 OrdinarySet ( O, P, V, Receiver )](https://tc39.es/ecma262/#sec-ordinaryset)
pub(crate) fn ordinary_try_set<'o, 'gc>(
    agent: &mut Agent,
    object: impl InternalMethods<'o>,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    // 1. Let ownDesc be ! O.[[GetOwnProperty]](P).
    let own_descriptor = unwrap_try(object.try_get_own_property(agent, property_key, cache, gc));

    // 2. Return ? OrdinarySetWithOwnDescriptor(O, P, V, Receiver, ownDesc).
    ordinary_try_set_with_own_descriptor(
        agent,
        object,
        property_key,
        value,
        receiver,
        own_descriptor,
        cache,
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
#[allow(clippy::too_many_arguments)]
fn ordinary_try_set_with_own_descriptor<'gc, 'o>(
    agent: &mut Agent,
    object: impl InternalMethods<'o>,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    own_descriptor: Option<PropertyDescriptor>,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    let ov: Value = object.into();
    let is_receiver = ov == receiver;
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
            return parent.try_set(agent, property_key, value, receiver, cache, gc);
        }
        // c. Else,
        else {
            if is_receiver {
                // No property set and the receiver is the object itself; this
                // means that the property does not exist on object and the
                // property is not a custom property of the object type (eg.
                // "length" on Array). Hence, we can push the property directly
                // into the object's backing object property storage.
                // 1
                if !object.internal_extensible(agent) {
                    return SetResult::Unwritable.into();
                }
                if let Err(err) = object
                    .get_or_create_backing_object(agent)
                    .property_storage()
                    .push(agent, object.into(), property_key, Some(value), None, gc)
                {
                    return TryError::Err(agent.throw_allocation_exception(err, gc)).into();
                }
                return SetResult::Done.into();
            }
            // i. Set ownDesc to the PropertyDescriptor {
            //   [[Value]]: undefined,
            //   [[Writable]]: true,
            //   [[Enumerable]]: true,
            //   [[Configurable]]: true
            // }.
            PropertyDescriptor::new_data_descriptor(Value::Undefined)
        }
    };

    // 2. If IsDataDescriptor(ownDesc) is true, then
    if own_descriptor.is_data_descriptor() {
        // a. If ownDesc.[[Writable]] is false, return false.
        if own_descriptor.writable == Some(false) {
            return SetResult::Unwritable.into();
        }

        // b. If Receiver is not an Object, return false.
        let Ok(receiver) = Object::try_from(receiver) else {
            return SetResult::Unwritable.into();
        };

        // c. Let existingDescriptor be ? Receiver.[[GetOwnProperty]](P).
        // Note: Here again we do not have guarantees; the receiver could be a
        // Proxy.
        let existing_descriptor = if is_receiver {
            // Direct [[Set]] call on our receiver; we already know that the
            // existingDescriptor is going to equal ownDescriptor.
            Some(own_descriptor)
        } else {
            receiver.try_get_own_property(agent, property_key, cache, gc)?
        };

        // d. If existingDescriptor is not undefined, then
        let result = if let Some(existing_descriptor) = existing_descriptor {
            // i. If IsAccessorDescriptor(existingDescriptor) is true, return false.
            if existing_descriptor.is_accessor_descriptor() {
                return SetResult::Accessor.into();
            }

            // ii. If existingDescriptor.[[Writable]] is false, return false.
            if existing_descriptor.writable == Some(false) {
                return SetResult::Unwritable.into();
            }

            // iii. Let valueDesc be the PropertyDescriptor { [[Value]]: V }.
            let value_descriptor = PropertyDescriptor {
                value: Some(value.unbind()),
                ..Default::default()
            };

            // iv. Return ? Receiver.[[DefineOwnProperty]](P, valueDesc).
            // Again: Receiver could be a Proxy.
            receiver.try_define_own_property(agent, property_key, value_descriptor, cache, gc)
        }
        // e. Else,
        else {
            // i. Assert: Receiver does not currently have a property P.
            // ii. Return ? CreateDataProperty(Receiver, P, V).
            // Again: Receiver could be a Proxy.
            try_create_data_property(agent, receiver, property_key, value, cache, gc)
        };
        return result.map_continue(|result| {
            if result {
                SetResult::Done
            } else {
                SetResult::Unwritable
            }
        });
    }

    // 3. Assert: IsAccessorDescriptor(ownDesc) is true.
    debug_assert!(own_descriptor.is_accessor_descriptor());

    // 4. Let setter be ownDesc.[[Set]].
    let setter = own_descriptor.set.unwrap().bind(gc);
    // 5. If setter is undefined, return false.
    let Some(setter) = setter else {
        return SetResult::Accessor.into();
    };

    // 6. Perform ? Call(setter, Receiver, « V »).
    // Note: We cannot call a function as it may trigger GC. See above for
    // future options.
    // call_function(agent, setter, receiver, Some(ArgumentsList(&[value])), gc)?;

    // 7. Return true.
    // Some(true)
    SetResult::Set(setter).into()
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
            receiver.try_get_own_property(agent, property_key, None, gc.nogc())
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
    let Some(Some(setter)) = own_descriptor.set else {
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

/// ### [10.1.9.1 OrdinarySet ( O, P, V, Receiver )](https://tc39.es/ecma262/#sec-ordinaryset)
pub(crate) fn ordinary_set_at_offset<'a>(
    agent: &mut Agent,
    props: &SetAtOffsetProps,
    o: Object,
    bo: Option<OrdinaryObject>,
    offset: PropertyOffset,
    gc: NoGcScope<'a, '_>,
) -> TryResult<'a, SetResult<'a>> {
    let o = o.bind(gc);
    let bo = bo.bind(gc);
    let p = props.p.bind(gc);
    let v = props.value.bind(gc);
    let receiver = props.receiver.bind(gc);

    let ov: Value = o.into();
    let is_receiver = ov == receiver;

    if offset.is_unset() {
        // 1.c.i. Set ownDesc to PropertyDescriptor {
        //   [[Value]]: undefined,
        //   [[Writable]]: true,
        //   [[Enumerable]]: true,
        //   [[Configurable]]: true
        // }.
        if bo.is_some_and(|bo| !ordinary_is_extensible(agent, bo)) {
            return SetResult::Unwritable.into();
        }

        if is_receiver {
            // ## 2.e.
            // Fast path for growing an object when we know property does not
            // exist on its shape.
            // i. Assert. Receiver does not currently have a property P.
            // ii. Return ? CreateDataProperty(Receiver, P, V).
            let bo = bo.unwrap_or_else(|| o.get_or_create_backing_object(agent));
            if let Err(err) = bo.property_storage().push(agent, o, p, Some(v), None, gc) {
                return agent.throw_allocation_exception(err, gc).into();
            }
            let shape = bo.object_shape(agent);
            if !shape.is_intrinsic(agent) {
                // If we added a property to a non-intrinsic shape, add a
                // lookup cache to the newly added property for the new shape.
                // Note that it's possible this isn't the first time we're
                // doing this, in which case an old cache may already exist and
                // this is a noop.
                props
                    .cache
                    .insert_lookup_offset_if_not_found(agent, shape, bo.len(agent) - 1);
            }
            return SetResult::Done.into();
        } else {
            return handle_super_set_inner(agent, p, v, receiver, None, gc);
        }
    }

    // OrdinarySetWithOwnDescriptor
    let offset = offset.get_property_offset();
    let data = bo
        .expect("OrdinarySet at offset with valid offset but no backing object")
        .get_elements_storage_mut(agent);
    if let Some(slot) = &mut data.values[offset as usize] {
        // 2. If IsDataDescriptor(ownDesc) is true, then
        let writable = match data.descriptors {
            Entry::Occupied(e) => e
                .get()
                .get(&(offset as u32))
                .is_none_or(|d| d.is_writable().unwrap()),
            Entry::Vacant(_) => true,
        };
        if !writable {
            return SetResult::Unwritable.into();
        }
        if is_receiver {
            // ## 2.d.
            // iii. Let valueDesc be the PropertyDescriptor { [[Value]]: V }.
            // iv. Return ? Receiver.[[DefineOwnProperty]](P, valueDesc).
            *slot = v.unbind();
            SetResult::Done.into()
        } else {
            // b. If Receiver is not an Object, return false.
            handle_super_set_inner(agent, p, v, receiver, None, gc)
        }
    } else {
        let Entry::Occupied(e) = data.descriptors else {
            unreachable!()
        };
        let d = e.get().get(&(offset as u32)).unwrap();
        debug_assert!(d.is_accessor_descriptor());
        d.setter_function(gc)
            .map_or(SetResult::Accessor, SetResult::Set)
            .into()
    }
}

fn handle_super_set_inner<'gc>(
    agent: &mut Agent,
    p: PropertyKey,
    v: Value,
    receiver: Value,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    // b. If Receiver is not an Object, return false.
    let Ok(receiver) = Object::try_from(receiver) else {
        return SetResult::Unwritable.into();
    };
    // c. Let existingDescriptor be ? Receiver.[[GetOwnProperty]](P).
    // Note: Here again we do not have guarantees; the receiver could be a
    // Proxy.
    let existing_descriptor = receiver.try_get_own_property(agent, p, cache, gc)?;
    // d. If existingDescriptor is not undefined, then
    let result = if let Some(existing_descriptor) = existing_descriptor {
        // i. If IsAccessorDescriptor(existingDescriptor) is true, return false.
        if existing_descriptor.is_accessor_descriptor() {
            return SetResult::Accessor.into();
        }

        // ii. If existingDescriptor.[[Writable]] is false, return false.
        if existing_descriptor.writable == Some(false) {
            return SetResult::Unwritable.into();
        }

        // iii. Let valueDesc be the PropertyDescriptor { [[Value]]: V }.
        let value_desc = PropertyDescriptor {
            value: Some(v.unbind()),
            ..Default::default()
        };

        // iv. Return ? Receiver.[[DefineOwnProperty]](P, valueDesc).
        // Again: Receiver could be a Proxy.
        receiver.try_define_own_property(agent, p, value_desc, cache, gc)
    }
    // e. Else,
    else {
        // i. Assert: Receiver does not currently have a property P.
        // ii. Return ? CreateDataProperty(Receiver, P, V).
        // Again: Receiver could be a Proxy.
        try_create_data_property(agent, receiver, p, v, cache, gc)
    };
    result.map_continue(|result| {
        if result {
            SetResult::Done
        } else {
            SetResult::Unwritable
        }
    })
}

/// ### [10.1.10.1 OrdinaryDelete ( O, P )](https://tc39.es/ecma262/#sec-ordinarydelete)
pub(crate) fn ordinary_delete(
    agent: &mut Agent,
    o: Object,
    backing_object: OrdinaryObject,
    property_key: PropertyKey,
    gc: NoGcScope,
) -> bool {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let descriptor = ordinary_get_own_property(agent, o, backing_object, property_key, None, gc);

    // 2. If desc is undefined, return true.
    let Some(descriptor) = descriptor else {
        return true;
    };

    // 3. If desc.[[Configurable]] is true, then
    if let Some(true) = descriptor.configurable {
        // a. Remove the own property with name P from O.
        backing_object
            .property_storage()
            .remove(agent, o, property_key)
            .expect("Should perform GC here");

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
    gc: NoGcScope<'a, '_>,
) -> Vec<PropertyKey<'a>> {
    let PropertyStorageRef {
        keys,
        values: _,
        descriptors: _,
    } = object.get_property_storage(agent);
    // 1. Let keys be a new empty List.
    let mut integer_keys = vec![];
    let mut keys_vec = Vec::with_capacity(keys.len());
    let mut symbol_keys = vec![];

    // 3. For each own property key P of O such that P is a String and P is not an array index, in
    //    ascending chronological order of property creation, do
    for key in keys.iter() {
        // SAFETY: Keys are all PropertyKeys reinterpreted as Values without
        // conversion.
        match key {
            PropertyKey::Integer(integer_key) => {
                let key_value = integer_key.into_i64();
                if (0..u32::MAX as i64).contains(&key_value) {
                    // Integer property key! This requires sorting
                    integer_keys.push(key_value as u32);
                } else {
                    keys_vec.push(key.bind(gc));
                }
            }
            PropertyKey::Symbol(symbol) => symbol_keys.push(symbol.bind(gc)),
            // Note: PrivateName keys are always invisible.
            PropertyKey::PrivateName(_) => {}
            // a. Append P to keys.
            _ => keys_vec.push(key.bind(gc)),
        }
    }

    // 2. For each own property key P of O such that P is an array index,
    if !integer_keys.is_empty() {
        // in ascending numeric index order, do
        integer_keys.sort();
        // a. Append P to keys.
        keys_vec.splice(0..0, integer_keys.into_iter().map(|key| key.into()));
    }

    // 4. For each own property key P of O such that P is a Symbol,
    if !symbol_keys.is_empty() {
        // in ascending chronological order of property creation, do
        // a. Append P to keys.
        keys_vec.extend(symbol_keys.iter().map(|key| PropertyKey::Symbol(*key)));
    }

    // 5. Return keys.
    keys_vec
}

pub(crate) fn ordinary_object_create_null<'a>(
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
) -> OrdinaryObject<'a> {
    OrdinaryObject::create_object(agent, None, &[])
        .expect("Should perform GC here")
        .bind(gc)
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
    proto_intrinsics: ProtoIntrinsics,
    prototype: Option<Object<'a>>,
    gc: NoGcScope<'a, '_>,
) -> Object<'a> {
    let object = match proto_intrinsics {
        ProtoIntrinsics::Array => agent.heap.create(ArrayHeapData::default()).into(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::ArrayBuffer => agent.heap.create(ArrayBufferHeapData::default()).into(),
        ProtoIntrinsics::ArrayIterator => {
            agent.heap.create(ArrayIteratorHeapData::default()).into()
        }
        ProtoIntrinsics::BigInt => agent
            .heap
            .create(PrimitiveObjectRecord::new_big_int_object(0.into()))
            .into(),
        ProtoIntrinsics::Boolean => agent
            .heap
            .create(PrimitiveObjectRecord::new_boolean_object(false))
            .into(),
        ProtoIntrinsics::Error => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::Error, None, None))
            .into(),
        ProtoIntrinsics::EvalError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::EvalError, None, None))
            .into(),
        #[cfg(feature = "date")]
        ProtoIntrinsics::Date => agent.heap.create(DateHeapData::new_invalid()).into(),
        ProtoIntrinsics::Function => todo!(),
        ProtoIntrinsics::Number => agent
            .heap
            .create(PrimitiveObjectRecord::new_number_object(0.into()))
            .into(),
        ProtoIntrinsics::Object => OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into(),
            ),
            &[],
        )
        .expect("Should perform GC here")
        .into(),
        ProtoIntrinsics::RangeError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::RangeError, None, None))
            .into(),
        ProtoIntrinsics::ReferenceError => agent
            .heap
            .create(ErrorHeapData::new(
                ExceptionType::ReferenceError,
                None,
                None,
            ))
            .into(),
        ProtoIntrinsics::String => agent
            .heap
            .create(PrimitiveObjectRecord::new_string_object(
                String::EMPTY_STRING,
            ))
            .into(),
        ProtoIntrinsics::StringIterator => agent
            .heap
            .create(StringIteratorHeapData::new(String::EMPTY_STRING))
            .into(),
        ProtoIntrinsics::Symbol => agent
            .heap
            .create(PrimitiveObjectRecord::new_symbol_object(Symbol::from(
                WellKnownSymbolIndexes::AsyncIterator,
            )))
            .into(),
        ProtoIntrinsics::SyntaxError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::SyntaxError, None, None))
            .into(),
        #[cfg(feature = "temporal")]
        ProtoIntrinsics::TemporalInstant => agent.heap.create(InstantRecord::default()).into(),
        #[cfg(feature = "temporal")]
        ProtoIntrinsics::TemporalDuration => {
            agent.heap.create(DurationHeapData::default()).into_object()
        }
        #[cfg(feature = "temporal")]
        ProtoIntrinsics::TemporalPlainTime => agent
            .heap
            .create(PlainTimeHeapData::default())
            .into_object(),
        ProtoIntrinsics::TypeError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::TypeError, None, None))
            .into(),
        ProtoIntrinsics::URIError => agent
            .heap
            .create(ErrorHeapData::new(ExceptionType::UriError, None, None))
            .into(),
        ProtoIntrinsics::AggregateError => agent
            .heap
            .create(ErrorHeapData::new(
                ExceptionType::AggregateError,
                None,
                None,
            ))
            .into(),
        ProtoIntrinsics::AsyncFunction => todo!(),
        ProtoIntrinsics::AsyncGenerator => {
            agent.heap.create(AsyncGeneratorHeapData::default()).into()
        }
        ProtoIntrinsics::AsyncGeneratorFunction => todo!(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::BigInt64Array => {
            Object::BigInt64Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::BigUint64Array => {
            Object::BigUint64Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::DataView => agent.heap.create(DataViewRecord::default()).into(),
        #[cfg(feature = "shared-array-buffer")]
        ProtoIntrinsics::SharedDataView => {
            agent.heap.create(SharedDataViewRecord::default()).into()
        }
        ProtoIntrinsics::FinalizationRegistry => agent
            .heap
            .create(FinalizationRegistryRecord::default())
            .into(),
        #[cfg(feature = "proposal-float16array")]
        ProtoIntrinsics::Float16Array => {
            Object::Float16Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Float32Array => {
            Object::Float32Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Float64Array => {
            Object::Float64Array(agent.heap.create(TypedArrayRecord::default()))
        }
        ProtoIntrinsics::Generator => agent.heap.create(GeneratorHeapData::default()).into(),
        ProtoIntrinsics::GeneratorFunction => todo!(),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int16Array => {
            Object::Int16Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int32Array => {
            Object::Int32Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int8Array => {
            Object::Int8Array(agent.heap.create(TypedArrayRecord::default()))
        }
        ProtoIntrinsics::Iterator => OrdinaryObject::create_object(
            agent,
            Some(
                agent
                    .current_realm_record()
                    .intrinsics()
                    .iterator_prototype()
                    .into(),
            ),
            &[],
        )
        .expect("Should perform GC here")
        .into(),
        ProtoIntrinsics::Map => agent.heap.create(MapHeapData::default()).into(),
        ProtoIntrinsics::MapIterator => agent.heap.create(MapIteratorHeapData::default()).into(),
        ProtoIntrinsics::Promise => agent.heap.create(PromiseHeapData::default()).into(),
        #[cfg(feature = "regexp")]
        ProtoIntrinsics::RegExp => agent.heap.create(RegExpHeapData::default()).into(),
        #[cfg(feature = "regexp")]
        ProtoIntrinsics::RegExpStringIterator => unreachable!(),
        #[cfg(feature = "set")]
        ProtoIntrinsics::Set => agent.heap.create(SetHeapData::default()).into(),
        #[cfg(feature = "set")]
        ProtoIntrinsics::SetIterator => agent.heap.create(SetIteratorHeapData::default()).into(),
        #[cfg(feature = "shared-array-buffer")]
        ProtoIntrinsics::SharedArrayBuffer => {
            agent.heap.create(SharedArrayBufferRecord::default()).into()
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint16Array => {
            Object::Uint16Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint32Array => {
            Object::Uint32Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint8Array => {
            Object::Uint8Array(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint8ClampedArray => {
            Object::Uint8ClampedArray(agent.heap.create(TypedArrayRecord::default()))
        }
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakMap => agent.heap.create(WeakMapRecord::default()).into(),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakRef => agent.heap.create(WeakRefHeapData::default()).into(),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakSet => agent.heap.create(WeakSetHeapData::default()).into(),
    }
    .bind(gc);

    ordinary_object_populate_with_intrinsics(agent, object, prototype)
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
    .unbind()?;
    let gc = gc.into_nogc();
    let proto = proto.bind(gc.into_nogc());
    // 3. If internalSlotsList is present, let slotsList be internalSlotsList.
    // 4. Else, let slotsList be a new empty List.
    // 5. Return OrdinaryObjectCreate(proto, slotsList).
    Ok(ordinary_object_create_with_intrinsics(
        agent,
        intrinsic_default_proto,
        proto,
        gc,
    ))
}

fn ordinary_object_populate_with_intrinsics<'a>(
    agent: &mut Agent,
    object: Object<'a>,
    prototype: Option<Object<'a>>,
) -> Object<'a> {
    if let Some(prototype) = prototype {
        if !prototype.is_proxy() && !prototype.is_module() {
            prototype
                .get_or_create_backing_object(agent)
                .make_intrinsic(agent)
                .expect("Should perform GC here");
        }
        object.internal_set_prototype(agent, Some(prototype));
    }

    object
}

pub(crate) fn ordinary_populate_from_constructor<'gc>(
    agent: &mut Agent,
    object: Object,
    constructor: Function,
    intrinsic_default_proto: ProtoIntrinsics,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Object<'gc>> {
    let mut object = object.bind(gc.nogc());
    let constructor = constructor.bind(gc.nogc());

    // 1. Assert: intrinsicDefaultProto is this specification's name of an
    // intrinsic object. The corresponding object must be an intrinsic that is
    // intended to be used as the [[Prototype]] value of an object.

    // 2. Let proto be ? GetPrototypeFromConstructor(constructor, intrinsicDefaultProto).
    let proto = if let Some(proto) = try_result_into_js(try_get_prototype_from_constructor(
        agent,
        constructor,
        intrinsic_default_proto,
        gc.nogc(),
    ))
    .unbind()?
    .bind(gc.nogc())
    {
        proto
    } else {
        // Couldn't get proto without calling into JS. This is a very rare case.
        let scoped_object = object.scope(agent, gc.nogc());
        let proto = get_prototype_from_constructor(
            agent,
            constructor.unbind(),
            intrinsic_default_proto,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // SAFETY: not shared.
        object = unsafe { scoped_object.take(agent) }.bind(gc.nogc());
        proto
    };
    let object = object.unbind();
    let proto = proto.unbind();
    let gc = gc.into_nogc();
    let object = object.bind(gc);
    let proto = proto.bind(gc);
    // 3. If internalSlotsList is present, let slotsList be internalSlotsList.
    // 4. Else, let slotsList be a new empty List.
    // 5. Return OrdinaryObjectCreate(proto, slotsList).
    Ok(ordinary_object_populate_with_intrinsics(
        agent, object, proto,
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
    if let Some(function_realm) = function_realm {
        let intrinsic_constructor =
            get_intrinsic_constructor(agent, function_realm, intrinsic_default_proto);
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
    let key = BUILTIN_STRING_MEMORY.prototype.to_property_key();
    let proto = try_get(
        agent,
        constructor,
        key,
        PropertyLookupCache::get(agent, key),
        gc.nogc(),
    );
    let proto = match proto {
        ControlFlow::Continue(TryGetResult::Unset) => Value::Undefined,
        ControlFlow::Continue(TryGetResult::Value(v)) => v,
        ControlFlow::Break(TryError::Err(e)) => {
            return Err(e.unbind().bind(gc.into_nogc()));
        }
        _ => {
            let scoped_realm = function_realm.map(|r| r.scope(agent, gc.nogc()));
            let scoped_constructor = constructor.scope(agent, gc.nogc());
            let proto = handle_try_get_result(
                agent,
                constructor.unbind(),
                BUILTIN_STRING_MEMORY.prototype.to_property_key(),
                proto.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let gc = gc.nogc();
            // SAFETY: not shared.
            constructor = unsafe { scoped_constructor.take(agent) }.bind(gc);
            // SAFETY: not shared.
            function_realm = scoped_realm.map(|r| unsafe { r.take(agent) }.bind(gc));
            proto
        }
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

fn get_intrinsic_constructor<'a>(
    agent: &Agent,
    function_realm: Realm<'a>,
    intrinsic_default_proto: ProtoIntrinsics,
) -> Option<Function<'a>> {
    let intrinsics = agent.get_realm_record_by_id(function_realm).intrinsics();
    match intrinsic_default_proto {
        ProtoIntrinsics::AggregateError => Some(intrinsics.aggregate_error().into()),
        ProtoIntrinsics::Array => Some(intrinsics.array().into()),
        ProtoIntrinsics::ArrayIterator => None,
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::ArrayBuffer => Some(intrinsics.array_buffer().into()),
        ProtoIntrinsics::AsyncFunction => Some(intrinsics.async_function().into()),
        ProtoIntrinsics::AsyncGenerator => None,
        ProtoIntrinsics::AsyncGeneratorFunction => {
            Some(intrinsics.async_generator_function().into())
        }
        ProtoIntrinsics::BigInt => Some(intrinsics.big_int().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::BigInt64Array => Some(intrinsics.big_int64_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::BigUint64Array => Some(intrinsics.big_uint64_array().into()),
        ProtoIntrinsics::Boolean => Some(intrinsics.boolean().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::DataView => Some(intrinsics.data_view().into()),
        #[cfg(feature = "shared-array-buffer")]
        ProtoIntrinsics::SharedDataView => Some(intrinsics.data_view().into()),
        #[cfg(feature = "date")]
        ProtoIntrinsics::Date => Some(intrinsics.date().into()),
        ProtoIntrinsics::Error => Some(intrinsics.error().into()),
        ProtoIntrinsics::EvalError => Some(intrinsics.eval_error().into()),
        ProtoIntrinsics::FinalizationRegistry => Some(intrinsics.finalization_registry().into()),
        #[cfg(feature = "proposal-float16array")]
        ProtoIntrinsics::Float16Array => Some(intrinsics.float16_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Float32Array => Some(intrinsics.float32_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Float64Array => Some(intrinsics.float64_array().into()),
        ProtoIntrinsics::Function => Some(intrinsics.function().into()),
        ProtoIntrinsics::Generator => None,
        ProtoIntrinsics::GeneratorFunction => Some(intrinsics.generator_function().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int16Array => Some(intrinsics.int16_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int32Array => Some(intrinsics.int32_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Int8Array => Some(intrinsics.int8_array().into()),
        ProtoIntrinsics::Iterator => Some(intrinsics.iterator().into()),
        ProtoIntrinsics::Map => Some(intrinsics.map().into()),
        ProtoIntrinsics::MapIterator => None,
        ProtoIntrinsics::Number => Some(intrinsics.number().into()),
        ProtoIntrinsics::Object => Some(intrinsics.object().into()),
        ProtoIntrinsics::Promise => Some(intrinsics.promise().into()),
        ProtoIntrinsics::RangeError => Some(intrinsics.range_error().into()),
        ProtoIntrinsics::ReferenceError => Some(intrinsics.reference_error().into()),
        #[cfg(feature = "regexp")]
        ProtoIntrinsics::RegExp => Some(intrinsics.reg_exp().into()),
        #[cfg(feature = "set")]
        ProtoIntrinsics::Set => Some(intrinsics.set().into()),
        #[cfg(feature = "set")]
        ProtoIntrinsics::SetIterator => None,
        #[cfg(feature = "shared-array-buffer")]
        ProtoIntrinsics::SharedArrayBuffer => Some(intrinsics.shared_array_buffer().into()),
        ProtoIntrinsics::String => Some(intrinsics.string().into()),
        ProtoIntrinsics::StringIterator => None,
        #[cfg(feature = "regexp")]
        ProtoIntrinsics::RegExpStringIterator => None,
        ProtoIntrinsics::Symbol => Some(intrinsics.symbol().into()),
        ProtoIntrinsics::SyntaxError => Some(intrinsics.syntax_error().into()),
        ProtoIntrinsics::TypeError => Some(intrinsics.type_error().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint16Array => Some(intrinsics.uint16_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint32Array => Some(intrinsics.uint32_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint8Array => Some(intrinsics.uint8_array().into()),
        #[cfg(feature = "array-buffer")]
        ProtoIntrinsics::Uint8ClampedArray => Some(intrinsics.uint8_clamped_array().into()),
        ProtoIntrinsics::URIError => Some(intrinsics.uri_error().into()),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakMap => Some(intrinsics.weak_map().into()),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakRef => Some(intrinsics.weak_ref().into()),
        #[cfg(feature = "weak-refs")]
        ProtoIntrinsics::WeakSet => Some(intrinsics.weak_set().into()),
        #[cfg(feature = "temporal")]
        ProtoIntrinsics::TemporalInstant => Some(intrinsics.temporal_instant().into()),
        #[cfg(feature = "temporal")]
        ProtoIntrinsics::TemporalPlainTime => Some(intrinsics.temporal_plain_time().into()),
        #[cfg(feature = "temporal")]
        ProtoIntrinsics::TemporalDuration => Some(intrinsics.temporal_duration().into()),
    }
}

pub(crate) fn try_get_prototype_from_constructor<'a>(
    agent: &mut Agent,
    constructor: Function<'a>,
    intrinsic_default_proto: ProtoIntrinsics,
    gc: NoGcScope<'a, '_>,
) -> TryResult<'a, Option<Object<'a>>> {
    let function_realm = try_get_function_realm(agent, constructor, gc);
    // NOTE: %Constructor%.prototype is an immutable property; we can thus
    // check if we %Constructor% is the ProtoIntrinsic we expect and if it is,
    // return None because we know %Constructor%.prototype corresponds to the
    // ProtoIntrinsic.
    if let Some(function_realm) = function_realm {
        let intrinsic_constructor =
            get_intrinsic_constructor(agent, function_realm, intrinsic_default_proto);
        if Some(constructor) == intrinsic_constructor {
            // The ProtoIntrinsic's constructor matches the constructor we're
            // being called with, so the constructor's prototype matches the
            // ProtoIntrinsic.
            return TryResult::Continue(None);
        }
    }

    // 1. Assert: intrinsicDefaultProto is this specification's name of an
    // intrinsic object. The corresponding object must be an intrinsic that is
    // intended to be used as the [[Prototype]] value of an object.
    // 2. Let proto be ? Get(constructor, "prototype").
    let key = BUILTIN_STRING_MEMORY.prototype.to_property_key();
    let proto = try_get_result_into_value(try_get(
        agent,
        constructor,
        key,
        PropertyLookupCache::get(agent, key),
        gc,
    ))?;
    match Object::try_from(proto) {
        // 3. If proto is not an Object, then
        Err(_) => {
            // a. Let realm be ? GetFunctionRealm(constructor).
            // b. Set proto to realm's intrinsic object named intrinsicDefaultProto.
            // Note: We signify using the default proto by returning None.
            // We only need to call the get_function_realm function if it would
            // throw an error.
            if function_realm.is_none() {
                let err = get_function_realm(agent, constructor.unbind(), gc).unwrap_err();
                return err.into();
            }
            TryResult::Continue(None)
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
                    return TryResult::Continue(None);
                }
            }
            TryResult::Continue(Some(proto))
        }
    }
}

/// 10.4.7.2 SetImmutablePrototype ( O, V )
///
/// The abstract operation SetImmutablePrototype takes arguments O (an Object)
/// and V (an Object or null) and returns either a normal completion containing
/// a Boolean or a throw completion.
#[inline]
#[expect(dead_code)]
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

/// Fast path try-function for getting a Value from an OrdinaryObject.
///
/// Returns Ok(Some(Value)) if the property was found in the object or its
/// prototype chain, Ok(None) if the property did not exist. Returns Err if the
/// property was found but was a getter or setter, or a non-ordinary prototype
/// object was encountered.
pub(crate) fn try_get_ordinary_object_value<'a>(
    agent: &Agent,
    binding_object: OrdinaryObject<'a>,
    name: PropertyKey<'a>,
) -> Result<Option<Value<'a>>, ()> {
    let PropertyStorageRef {
        keys,
        values,
        descriptors,
    } = binding_object.get_property_storage(agent);
    let index = keys
        .iter()
        .enumerate()
        .find(|(_, k)| **k == name)
        .map(|(i, _)| i);
    if let Some(index) = index {
        // If value is None, it means that the slot is a getter or setter
        // and we cannot handle those on the fast path.
        let Some(value) = values[index] else {
            // Getter or setter, break the fast path.
            if descriptors.is_some_and(|d| {
                d.get(&(index as u32))
                    .is_some_and(|d| d.has_setter() && !d.has_getter())
            }) {
                // Setter-only, return undefined.
                return Ok(Some(Value::Undefined));
            }
            return Err(());
        };
        // Otherwise, we got a real Value for this property and can return
        // it.
        return Ok(Some(value));
    }
    let proto = binding_object.internal_prototype(agent);
    if let Some(Object::Object(proto)) = proto {
        // Prototype is also an ordinary object, we can take a look there
        // as well.
        try_get_ordinary_object_value(agent, proto, name)
    } else if proto.is_none() {
        // We never did find the property in the object or its prototype
        // chain.
        Ok(None)
    } else {
        // Interesting kind of prototype: we cannot handle it on the fast
        // path.
        Err(())
    }
}
