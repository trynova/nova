// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{InternalSlots, IntoObject, Object, PropertyKey};
use crate::ecmascript::{
    abstract_operations::testing_and_comparison::same_value,
    builtins::{
        ordinary::{
            ordinary_define_own_property, ordinary_delete, ordinary_get, ordinary_get_own_property,
            ordinary_get_prototype_of, ordinary_has_property, ordinary_is_extensible,
            ordinary_own_property_keys, ordinary_prevent_extensions, ordinary_set,
            ordinary_set_prototype_of, ordinary_set_prototype_of_check_loop,
        },
        ArgumentsList,
    },
    execution::{Agent, JsResult},
    types::{Function, PropertyDescriptor, Value},
};

/// ### [6.1.7.2 Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots)
pub trait InternalMethods
where
    Self: Sized + Clone + Copy + Into<Object> + InternalSlots,
{
    /// \[\[GetPrototypeOf\]\]
    fn internal_get_prototype_of(self, agent: &mut Agent) -> JsResult<Option<Object>> {
        match self.get_backing_object(agent) {
            Some(backing_object) => Ok(ordinary_get_prototype_of(
                agent,
                backing_object.into_object(),
            )),
            None => Ok(self.internal_prototype(agent)),
        }
    }

    /// \[\[SetPrototypeOf\]\]
    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
    ) -> JsResult<bool> {
        match self.get_backing_object(agent) {
            Some(backing_object) => Ok(ordinary_set_prototype_of(
                agent,
                backing_object.into_object(),
                prototype,
            )),
            None => {
                // 1. Let current be O.[[Prototype]].
                let current = self.internal_prototype(agent);

                // 2. If SameValue(V, current) is true, return true.
                match (prototype, current) {
                    (Some(prototype), Some(current)) if same_value(agent, prototype, current) => {
                        return Ok(true)
                    }
                    (None, None) => return Ok(true),
                    _ => {}
                }

                // 3. Let extensible be O.[[Extensible]].
                let extensible = self.internal_extensible(agent);

                // 4. If extensible is false, return false.
                if !extensible {
                    // 7.b.i. Return false.
                    return Ok(false);
                }

                if !ordinary_set_prototype_of_check_loop(agent, self.into(), prototype) {
                    return Ok(false);
                }

                // 8. Set O.[[Prototype]] to V.
                self.internal_set_prototype(agent, prototype);

                // 9. Return true.
                Ok(true)
            }
        }
    }

    /// \[\[IsExtensible\]\]
    fn internal_is_extensible(self, agent: &mut Agent) -> JsResult<bool> {
        // 1. Return OrdinaryIsExtensible(O).
        match self.get_backing_object(agent) {
            Some(backing_object) => Ok(ordinary_is_extensible(agent, backing_object.into_object())),
            None => Ok(self.internal_extensible(agent)),
        }
    }

    /// \[\[PreventExtensions\]\]
    fn internal_prevent_extensions(self, agent: &mut Agent) -> JsResult<bool> {
        // 1. Return OrdinaryPreventExtensions(O).
        match self.get_backing_object(agent) {
            Some(backing_object) => Ok(ordinary_prevent_extensions(
                agent,
                backing_object.into_object(),
            )),
            None => {
                self.internal_set_extensible(agent, false);
                Ok(true)
            }
        }
    }

    /// \[\[GetOwnProperty\]\]
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
    ) -> JsResult<Option<PropertyDescriptor>> {
        // 1. Return OrdinaryGetOwnProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => Ok(ordinary_get_own_property(
                agent,
                backing_object.into_object(),
                property_key,
            )),
            None => Ok(None),
        }
    }

    /// \[\[DefineOwnProperty\]\]
    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
    ) -> JsResult<bool> {
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent))
            .into_object();
        ordinary_define_own_property(agent, backing_object, property_key, property_descriptor)
    }

    /// \[\[HasProperty\]\]
    fn internal_has_property(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_has_property(agent, backing_object.into_object(), property_key)
            }
            None => {
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                let parent = self.internal_get_prototype_of(agent)?;

                // 4. If parent is not null, then
                if let Some(parent) = parent {
                    // a. Return ? parent.[[HasProperty]](P).
                    parent.internal_has_property(agent, property_key)
                } else {
                    // 5. Return false.
                    Ok(false)
                }
            }
        }
    }

    /// \[\[Get\]\]
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
    ) -> JsResult<Value> {
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_get(agent, backing_object.into_object(), property_key, receiver)
            }
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = self.internal_get_prototype_of(agent)? else {
                    // b. If parent is null, return undefined.
                    return Ok(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.internal_get(agent, property_key, receiver)
            }
        }
    }

    /// \[\[Set\]\]
    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
    ) -> JsResult<bool> {
        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent))
            .into_object();
        ordinary_set(agent, backing_object, property_key, value, receiver)
    }

    /// \[\[Delete\]\]
    fn internal_delete(self, agent: &mut Agent, property_key: PropertyKey) -> JsResult<bool> {
        // 1. Return ? OrdinaryDelete(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_delete(agent, backing_object.into_object(), property_key)
            }
            None => Ok(true),
        }
    }

    /// \[\[OwnPropertyKeys\]\]
    fn internal_own_property_keys(self, agent: &mut Agent) -> JsResult<Vec<PropertyKey>> {
        // 1. Return OrdinaryOwnPropertyKeys(O).
        match self.get_backing_object(agent) {
            Some(backing_object) => Ok(ordinary_own_property_keys(agent, backing_object)),
            None => Ok(vec![]),
        }
    }

    /// \[\[Call\]\]
    fn internal_call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        unreachable!()
    }

    /// \[\[Construct\]\]
    fn internal_construct(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
    ) -> JsResult<Object> {
        unreachable!()
    }
}
