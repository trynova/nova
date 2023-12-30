use crate::ecmascript::{
    abstract_operations::{
        operations_on_objects::{get, get_function_realm},
        testing_and_comparison::same_value,
    },
    execution::{Agent, JsResult, ProtoIntrinsics},
    types::{
        Function, InternalMethods, Object, OrdinaryObject, OrdinaryObjectInternalSlots,
        PropertyDescriptor, PropertyKey, String, Value,
    },
};

/// 10.1 Ordinary Object Internal Methods and Internal Slots
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots
impl InternalMethods for OrdinaryObject {
    /// ### [10.1.1 \[\[GetPrototypeOf\]\] ( )](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-getprototypeof)
    fn get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        Ok(ordinary_get_prototype_of(agent, self.into()))
    }

    /// ### [10.1.2 \[\[SetPrototypeOf\]\] ( V )](https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-setprototypeof-v)
    fn set_prototype_of(self, agent: &mut Agent, prototype: Option<Object>) -> JsResult<bool> {
        Ok(ordinary_set_prototype_of(agent, self.into(), prototype))
    }

    /// 10.1.3 [[IsExtensible]] ( )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-isextensible
    fn is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        // 1. Return OrdinaryIsExtensible(O).
        Ok(ordinary_is_extensible(agent, self.into()))
    }

    /// 10.1.4 [[PreventExtensions]] ( )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-preventextensions
    fn prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        // 1. Return OrdinaryPreventExtensions(O).
        Ok(ordinary_prevent_extensions(agent, self.into()))
    }

    /// 10.1.5 [[GetOwnProperty]] ( P )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-getownproperty-p
    fn get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        // 1. Return OrdinaryGetOwnProperty(O, P).
        Ok(ordinary_get_own_property(agent, self.into(), property_key))
    }

    /// 10.1.6 [[DefineOwnProperty]] ( P, Desc )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-defineownproperty-p-desc
    fn define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        ordinary_define_own_property(agent, self.into(), property_key, descriptor)
    }

    /// 10.1.7 [[HasProperty]] ( P )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-hasproperty-p
    fn has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        // 1. Return ? OrdinaryHasProperty(O, P).
        ordinary_has_property(agent, self.into(), property_key)
    }

    /// 10.1.8 [[Get]] ( P, Receiver )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-get-p-receiver
    fn get(self, agent: &mut Agent, property_key: PropertyKey, receiver: Value) -> JsResult<Value> {
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        ordinary_get(agent, self.into(), property_key, receiver)
    }

    /// 10.1.9 [[Set]] ( P, V, Receiver )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-set-p-v-receiver
    fn set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(agent, self.into(), property_key, value, receiver)
    }

    /// 10.1.10 [[Delete]] ( P )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-delete-p
    fn delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        // 1. Return ? OrdinaryDelete(O, P).
        ordinary_delete(agent, self.into(), property_key)
    }

    /// 10.1.11 [[OwnPropertyKeys]] ( )
    /// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-ownpropertykeys
    fn own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        // 1. Return OrdinaryOwnPropertyKeys(O).
        ordinary_own_property_keys(agent, self.into())
    }
}

/// ### [10.1.1.1 OrdinaryGetPrototypeOf ( O )](https://tc39.es/ecma262/#sec-ordinarygetprototypeof)
pub(crate) fn ordinary_get_prototype_of(agent: &mut Agent, object: Object) -> Option<Object> {
    // 1. Return O.[[Prototype]].
    object.prototype(agent)
}

/// Implements steps 5 through 7 of OrdinarySetPrototypeOf
///
/// Returns true if a loop is detected, corresponding to substep 7.b.i. of the
/// abstract operation.
pub(crate) fn ordinary_set_prototype_of_check_loop(
    agent: &mut Agent,
    o: Object,
    v: Option<Object>,
) -> bool {
    // 5. Let p be V.
    let mut p = v;
    // 6. Let done be false.
    // 7. Repeat, while done is false,
    while let Some(p_inner) = p {
        // a. If p is null, then
        //     i. Set done to true.

        // b. Else if SameValue(p, O) is true, then
        if same_value(agent, p_inner, o) {
            // i. Return false.
            return false;
        }

        // c. Else,
        // i. If p.[[GetPrototypeOf]] is not the ordinary object internal method defined in 10.1.1,
        //    set done to true.
        // NOTE: At present there are two exotic objects that define their own [[GetPrototypeOf]]
        // methods. Those are Proxy and Module.

        // if parent_prototype.get_prototype_of != get_prototype_of {
        //     break;
        // }

        // ii. Else, set p to p.[[Prototype]].
        p = p_inner.prototype(agent);
    }
    true
}

/// 10.1.2.1 OrdinarySetPrototypeOf ( O, V )
/// https://tc39.es/ecma262/#sec-ordinarysetprototypeof
pub(crate) fn ordinary_set_prototype_of(
    agent: &mut Agent,
    object: Object,
    prototype: Option<Object>,
) -> bool {
    // 1. Let current be O.[[Prototype]].
    let current = object.prototype(agent);

    // 2. If SameValue(V, current) is true, return true.
    match (prototype, current) {
        (Some(prototype), Some(current)) if same_value(agent, prototype, current) => return true,
        (None, None) => return true,
        _ => {}
    }

    // 3. Let extensible be O.[[Extensible]].
    let extensible = object.extensible(agent);

    // 4. If extensible is false, return false.
    if !extensible {
        // 7.b.i. Return false.
        return false;
    }

    if ordinary_set_prototype_of_check_loop(agent, object, prototype) {
        return false;
    }

    // 8. Set O.[[Prototype]] to V.
    object.set_prototype(agent, prototype);

    // 9. Return true.
    true
}

/// 10.1.3.1 OrdinaryIsExtensible ( O )
/// https://tc39.es/ecma262/#sec-ordinaryisextensible
pub(crate) fn ordinary_is_extensible(agent: &mut Agent, object: Object) -> bool {
    // 1. Return O.[[Extensible]].
    object.extensible(agent)
}

/// 10.1.4.1 OrdinaryPreventExtensions ( O )
/// https://tc39.es/ecma262/#sec-ordinarypreventextensions
pub(crate) fn ordinary_prevent_extensions(agent: &mut Agent, object: Object) -> bool {
    // 1. Set O.[[Extensible]] to false.
    object.set_extensible(agent, false);

    // 2. Return true.
    true
}

/// 10.1.5.1 OrdinaryGetOwnProperty ( O, P )
/// https://tc39.es/ecma262/#sec-ordinarygetownproperty
pub(crate) fn ordinary_get_own_property(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
) -> Option<PropertyDescriptor> {
    // 1. If O does not have an own property with key P, return undefined.
    // 3. Let X be O's own property whose key is P.
    let x = object.property_storage().get(agent, property_key)?;

    // 2. Let D be a newly created Property Descriptor with no fields.
    let mut descriptor = PropertyDescriptor::default();

    // 4. If X is a data property, then
    if x.is_data_descriptor() {
        // a. Set D.[[Value]] to the value of X's [[Value]] attribute.
        descriptor.value = x.value;

        // b. Set D.[[Writable]] to the value of X's [[Writable]] attribute.
        descriptor.writable = x.writable;
    }
    // 5. Else,
    else {
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

/// 10.1.6.1 OrdinaryDefineOwnProperty ( O, P, Desc )
/// https://tc39.es/ecma262/#sec-ordinarydefineownproperty
pub(crate) fn ordinary_define_own_property(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    descriptor: PropertyDescriptor,
) -> JsResult<bool> {
    // 1. Let current be ? O.[[GetOwnProperty]](P).
    let current = object.get_own_property(agent, property_key)?;

    // 2. Let extensible be ? IsExtensible(O).
    let extensible = object.extensible(agent);

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

/// 10.1.6.3 ValidateAndApplyPropertyDescriptor ( O, P, extensible, Desc, current )
/// https://tc39.es/ecma262/#sec-validateandapplypropertydescriptor
fn validate_and_apply_property_descriptor(
    agent: &mut Agent,
    object: Option<Object>,
    property_key: PropertyKey,
    extensible: bool,
    descriptor: PropertyDescriptor,
    current: Option<PropertyDescriptor>,
) -> JsResult<bool> {
    // 1. Assert: IsPropertyKey(P) is true.

    // 2. If current is undefined, then
    let Some(current) = current else {
        // a. If extensible is false, return false.
        if !extensible {
            return Ok(false);
        }

        // b. If O is undefined, return true.
        let Some(object) = object else {
            return Ok(true);
        };

        // c. If IsAccessorDescriptor(Desc) is true, then
        if descriptor.is_accessor_descriptor() {
            // i. Create an own accessor property named P of object O whose [[Get]], [[Set]],
            //    [[Enumerable]], and [[Configurable]] attributes are set to the value of the
            //    corresponding field in Desc if Desc has that field, or to the attribute's default
            //    value otherwise.
            object.property_storage().set(
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
            // try object.propertyStorage().set(property_key, PropertyDescriptor{
            //     .value = descriptor.value orelse .undefined,
            //     .writable = descriptor.writable orelse false,
            //     .enumerable = descriptor.enumerable orelse false,
            //     .configurable = descriptor.configurable orelse false,
            // });
            object.property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    value: Some(descriptor.value.unwrap_or(Value::Undefined)),
                    enumerable: Some(descriptor.enumerable.unwrap_or(false)),
                    configurable: Some(descriptor.configurable.unwrap_or(false)),
                    ..Default::default()
                },
            )
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
        if let Some(true) = descriptor.configurable {
            return Ok(false);
        }

        // b. If Desc has an [[Enumerable]] field and SameValue(Desc.[[Enumerable]], current.[[Enumerable]])
        //    is false, return false.
        if let Some(true) = descriptor.enumerable {
            if descriptor.enumerable != current.enumerable {
                return Ok(false);
            }
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
        else if let Some(true) = current.writable {
            // i. If Desc has a [[Writable]] field and Desc.[[Writable]] is true, return false.
            if let Some(true) = descriptor.writable {
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
            object.property_storage().set(
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
            object.property_storage().set(
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
            object.property_storage().set(
                agent,
                property_key,
                PropertyDescriptor {
                    value: descriptor.value.or(current.value),
                    writable: Some(descriptor.writable.unwrap_or(false)),
                    get: descriptor.get.or(current.get),
                    set: descriptor.set.or(current.set),
                    enumerable: descriptor.enumerable.or(current.enumerable),
                    configurable: descriptor.configurable.or(current.configurable),
                },
            );
        }
    }

    // 7. Return true.
    Ok(true)
}

/// 10.1.7.1 OrdinaryHasProperty ( O, P )
/// https://tc39.es/ecma262/#sec-ordinaryhasproperty
pub(crate) fn ordinary_has_property(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
) -> JsResult<bool> {
    // 1. Let hasOwn be ? O.[[GetOwnProperty]](P).
    let has_own = object.get_own_property(agent, property_key)?;

    // 2. If hasOwn is not undefined, return true.
    if has_own.is_some() {
        return Ok(true);
    }

    // 3. Let parent be ? O.[[GetPrototypeOf]]().
    let parent = object.get_prototype_of(agent)?;

    // 4. If parent is not null, then
    if let Some(parent) = parent {
        // a. Return ? parent.[[HasProperty]](P).
        return parent.has_property(agent, property_key);
    }

    // 5. Return false.
    Ok(false)
}

/// 10.1.8.1 OrdinaryGet ( O, P, Receiver )
/// https://tc39.es/ecma262/#sec-ordinaryget
pub(crate) fn ordinary_get(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    receiver: Value,
) -> JsResult<Value> {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let Some(descriptor) = object.get_own_property(agent, property_key)? else {
        // 2. If desc is undefined, then

        // a. Let parent be ? O.[[GetPrototypeOf]]().
        let Some(parent) = object.get_prototype_of(agent)? else {
            return Ok(Value::Undefined);
        };

        // c. Return ? parent.[[Get]](P, Receiver).
        return parent.get(agent, property_key, receiver);
    };

    // 3. If IsDataDescriptor(desc) is true, return desc.[[Value]].
    if let Some(value) = descriptor.value {
        debug_assert!(descriptor.is_data_descriptor());
        return Ok(value);
    }

    // 4. Assert: IsAccessorDescriptor(desc) is true.
    debug_assert!(descriptor.is_accessor_descriptor());

    // 5. Let getter be desc.[[Get]].
    // 6. If getter is undefined, return undefined.
    let Some(_getter) = descriptor.get else {
        return Ok(Value::Undefined);
    };

    // 7. Return ? Call(getter, Receiver).
    todo!()
}

/// 10.1.9.1 OrdinarySet ( O, P, V, Receiver )
/// https://tc39.es/ecma262/#sec-ordinaryset
pub(crate) fn ordinary_set(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
) -> JsResult<bool> {
    // 1. Let ownDesc be ? O.[[GetOwnProperty]](P).
    let own_descriptor = object.get_own_property(agent, property_key)?;

    // 2. Return ? OrdinarySetWithOwnDescriptor(O, P, V, Receiver, ownDesc).
    ordinary_set_with_own_descriptor(agent, object, property_key, value, receiver, own_descriptor)
}

/// 10.1.9.2 OrdinarySetWithOwnDescriptor ( O, P, V, Receiver, ownDesc )
/// https://tc39.es/ecma262/#sec-ordinarysetwithowndescriptor
pub(crate) fn ordinary_set_with_own_descriptor(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
    own_descriptor: Option<PropertyDescriptor>,
) -> JsResult<bool> {
    let own_descriptor = if let Some(own_descriptor) = own_descriptor {
        own_descriptor
    } else {
        // 1. If ownDesc is undefined, then
        // a. Let parent be ? O.[[GetPrototypeOf]]().
        let parent = object.get_prototype_of(agent)?;

        // b. If parent is not null, then
        if let Some(parent) = parent {
            // i. Return ? parent.[[Set]](P, V, Receiver).
            return parent.set(agent, property_key, value, receiver);
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
        let Ok(receiver) = Object::try_from(receiver) else {
            return Ok(false);
        };

        // c. Let existingDescriptor be ? Receiver.[[GetOwnProperty]](P).
        let existing_descriptor = receiver.get_own_property(agent, property_key)?;

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
            return receiver.define_own_property(agent, property_key, value_descriptor);
        }
        // e. Else,
        else {
            // i. Assert: Receiver does not currently have a property P.
            debug_assert!(!receiver.property_storage().has(agent, property_key));

            // ii. Return ? CreateDataProperty(Receiver, P, V).
            return receiver.create_data_property(agent, property_key, value);
        }
    }

    // 3. Assert: IsAccessorDescriptor(ownDesc) is true.
    debug_assert!(own_descriptor.is_accessor_descriptor());

    // 4. Let setter be ownDesc.[[Set]].
    // 5. If setter is undefined, return false.
    let Some(_setter) = own_descriptor.set else {
        return Ok(false);
    };

    // 6. Perform ? Call(setter, Receiver, « V »).
    todo!();
    // 7. Return true.
}

/// 10.1.10.1 OrdinaryDelete ( O, P )
/// https://tc39.es/ecma262/#sec-ordinarydelete
pub(crate) fn ordinary_delete(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
) -> JsResult<bool> {
    // 1. Let desc be ? O.[[GetOwnProperty]](P).
    let descriptor = object.get_own_property(agent, property_key)?;

    // 2. If desc is undefined, return true.
    let Some(descriptor) = descriptor else {
        return Ok(true);
    };

    // 3. If desc.[[Configurable]] is true, then
    if let Some(true) = descriptor.configurable {
        // a. Remove the own property with name P from O.
        object.property_storage().remove(agent, property_key);

        // b. Return true.
        return Ok(true);
    }

    // 4. Return false.
    Ok(false)
}

/// 10.1.11.1 OrdinaryOwnPropertyKeys ( O )
/// https://tc39.es/ecma262/#sec-ordinaryownpropertykeys
pub(crate) fn ordinary_own_property_keys(
    _agent: &mut Agent,
    _object: Object,
) -> JsResult<Vec<PropertyKey>> {
    // 1. Let keys be a new empty List.
    let keys = Vec::new();

    // 2. For each own property key P of O such that P is an array index, in ascending numeric
    //    index order, do
    // for entry in object.property_storage().entries(agent) {
    // 	if entry.key.is_array_index() {
    // 		// a. Append P to keys.
    // 		keys.push(entry.key);
    // 	}
    // }

    // for (object.property_storage().hash_map.keys()) |property_key| {
    //     if (property_key.is_array_index()) {
    //         // a. Append P to keys.
    //         keys.appendAssumeCapacity(property_key);
    //     }
    // }

    // 3. For each own property key P of O such that P is a String and P is not an array index, in
    //    ascending chronological order of property creation, do
    // for (object.propertyStorage().hash_map.keys()) |property_key| {
    //     if (property_key == .string or (property_key == .integer_index and !property_key.isArrayIndex())) {
    //         // a. Append P to keys.
    //         keys.appendAssumeCapacity(property_key);
    //     }
    // }

    // 4. For each own property key P of O such that P is a Symbol, in ascending chronological
    //    order of property creation, do
    // for (object.propertyStorage().hash_map.keys()) |property_key| {
    //     if (property_key == .symbol) {
    //         // a. Append P to keys.
    //         keys.appendAssumeCapacity(property_key);
    //     }
    // }

    // 5. Return keys.
    Ok(keys)
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
pub(crate) fn get_prototype_from_constructor(
    agent: &mut Agent,
    constructor: Function,
    intrinsic_default_proto: ProtoIntrinsics,
) -> JsResult<Object> {
    // 1. Assert: intrinsicDefaultProto is this specification's name of an
    // intrinsic object. The corresponding object must be an intrinsic that is
    // intended to be used as the [[Prototype]] value of an object.
    // 2. Let proto be ? Get(constructor, "prototype").
    let prototype_key = String::from_str(agent, "prototype").into();
    let proto = get(agent, constructor.into(), prototype_key)?;
    // 3. If proto is not an Object, then
    match Object::try_from(proto) {
        Err(_) => {
            // a. Let realm be ? GetFunctionRealm(constructor).
            // SAFETY: Lifetimes are currently just badly guessed, meaningless lumps.
            let realm = get_function_realm(unsafe { std::mem::transmute(agent) }, constructor)?;
            // b. Set proto to realm's intrinsic object named intrinsicDefaultProto.
            Ok(realm
                .intrinsics()
                .get_intrinsic_default_proto(intrinsic_default_proto))
        }
        Ok(proto) => {
            // 4. Return proto.
            Ok(proto)
        }
    }
}
