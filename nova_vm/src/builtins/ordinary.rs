use crate::{
    execution::{Agent, JsResult},
    types::{InternalMethods, Object, PropertyDescriptor, PropertyKey, Value},
};

use super::ArgumentsList;

/// 10.1 Ordinary Object Internal Methods and Internal Slots
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots
pub static METHODS: InternalMethods = InternalMethods {
    get_prototype_of,
    set_prototype_of,
    is_extensible,
    prevent_extensions,
    get_own_property,
    define_own_property,
    has_property,
    get,
    set,
    delete,
    own_property_keys,
    call: Some(call),
    construct: Some(construct),
};

/// 10.1.1 [[GetPrototypeOf]] ( )
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-getprototypeof
fn get_prototype_of(agent: &mut Agent, object: Object) -> Option<Object> {
    // 1. Return OrdinaryGetPrototypeOf(O).
    ordinary_get_prototype_of(agent, object)
}

/// 10.1.1.1 OrdinaryGetPrototypeOf ( O )
/// https://tc39.es/ecma262/#sec-ordinarygetprototypeof
pub fn ordinary_get_prototype_of(agent: &mut Agent, object: Object) -> Option<Object> {
    // 1. Return O.[[Prototype]].
    object.prototype(agent)
}

/// 10.1.2 [[SetPrototypeOf]] ( V )
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-setprototypeof-v
fn set_prototype_of(
    agent: &mut Agent,
    object: Object,
    prototype: Option<Object>,
) -> JsResult<bool> {
    // 1. Return OrdinarySetPrototypeOf(O, V).
    return ordinary_set_prototype_of(agent, object, prototype);
}

/// 10.1.2.1 OrdinarySetPrototypeOf ( O, V )
/// https://tc39.es/ecma262/#sec-ordinarysetprototypeof
pub fn ordinary_set_prototype_of(
    agent: &mut Agent,
    object: Object,
    prototype: Option<Object>,
) -> JsResult<bool> {
    // 1. Let current be O.[[Prototype]].
    let current = object.prototype(agent);

    // 2. If SameValue(V, current) is true, return true.
    match (prototype, current) {
        (Some(prototype), Some(current))
            if prototype
                .into_value()
                .same_value(agent, current.into_value()) =>
        {
            return Ok(true)
        }
        (None, None) => return Ok(true),
        _ => {}
    }

    // 3. Let extensible be O.[[Extensible]].
    let extensible = object.extensible(agent);

    // 4. If extensible is false, return false.
    if !extensible {
        return Ok(false);
    }

    // 5. Let p be V.
    let mut parent_prototype_outer = prototype;

    // 6. Let done be false.
    // 7. Repeat, while done is false,
    while let Some(parent_prototype) = parent_prototype_outer {
        // a. If p is null, then
        //     i. Set done to true.

        // b. Else if SameValue(p, O) is true, then
        if parent_prototype
            .into_value()
            .same_value(agent, object.into_value())
        {
            // i. Return false.
            return Ok(false);
        }

        // c. Else,
        // i. If p.[[GetPrototypeOf]] is not the ordinary object internal method defined in 10.1.1,
        //    set done to true.
        if parent_prototype.internal_methods(agent).get_prototype_of != get_prototype_of {
            break;
        }

        // ii. Else, set p to p.[[Prototype]].
        parent_prototype_outer = parent_prototype.prototype(agent);
    }

    // 8. Set O.[[Prototype]] to V.
    object.set_prototype(agent, parent_prototype_outer);

    // 9. Return true.
    Ok(true)
}

/// 10.1.3 [[IsExtensible]] ( )
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-isextensible
fn is_extensible(agent: &mut Agent, object: Object) -> JsResult<bool> {
    // 1. Return OrdinaryIsExtensible(O).
    Ok(ordinary_is_extensible(agent, object))
}

/// 10.1.3.1 OrdinaryIsExtensible ( O )
/// https://tc39.es/ecma262/#sec-ordinaryisextensible
pub fn ordinary_is_extensible(agent: &mut Agent, object: Object) -> bool {
    // 1. Return O.[[Extensible]].
    todo!()
}

/// 10.1.4 [[PreventExtensions]] ( )
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-preventextensions
fn prevent_extensions(agent: &mut Agent, object: Object) -> JsResult<bool> {
    // 1. Return OrdinaryPreventExtensions(O).
    Ok(ordinary_prevent_extensions(agent, object))
}

/// 10.1.4.1 OrdinaryPreventExtensions ( O )
/// https://tc39.es/ecma262/#sec-ordinarypreventextensions
pub fn ordinary_prevent_extensions(agent: &mut Agent, object: Object) -> bool {
    // 1. Set O.[[Extensible]] to false.
    todo!();

    // 2. Return true.
    return true;
}

pub fn get_own_property(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
) -> JsResult<()> {
    todo!()
}

pub fn define_own_property(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    property_descriptor: PropertyDescriptor,
) -> bool {
    todo!()
}

pub fn has_property(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
) -> JsResult<bool> {
    todo!()
}

pub fn get(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    receiver: Value,
) -> JsResult<Value> {
    todo!()
}

pub fn set(
    agent: &mut Agent,
    object: Object,
    property_key: PropertyKey,
    value: Value,
    receiver: Value,
) -> JsResult<bool> {
    todo!()
}

pub fn delete(agent: &mut Agent, object: Object, property_key: PropertyKey) -> JsResult<bool> {
    todo!()
}

pub fn own_property_keys(agent: &mut Agent, object: Object) -> JsResult<Vec<PropertyKey>> {
    todo!()
}

pub fn call(
    agent: &mut Agent,
    object: Object,
    this_value: Value,
    arguments_list: ArgumentsList,
) -> JsResult<Value> {
    todo!()
}

pub fn construct(
    agent: &mut Agent,
    object: Object,
    arguments_list: ArgumentsList,
) -> JsResult<Object> {
    todo!()
}
