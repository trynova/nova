// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use super::{InternalSlots, Object, PropertyKey};
use crate::{
    ecmascript::{
        builtins::{
            ordinary::{
                ordinary_delete, ordinary_get, ordinary_get_own_property,
                ordinary_get_prototype_of, ordinary_has_property, ordinary_is_extensible,
                ordinary_own_property_keys, ordinary_prevent_extensions, ordinary_set,
                ordinary_set_prototype_of, ordinary_try_define_own_property, ordinary_try_get,
                ordinary_try_has_property, ordinary_try_set,
            },
            ArgumentsList,
        },
        execution::{Agent, JsResult},
        types::{Function, PropertyDescriptor, Value},
    },
    engine::{
        context::{GcScope, NoGcScope},
        unwrap_try, TryResult,
    },
};

/// ### [6.1.7.2 Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots)
pub trait InternalMethods
where
    Self: std::fmt::Debug + Sized + Clone + Copy + Into<Object> + InternalSlots,
{
    /// ## Infallible \[\[GetPrototypeOf\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_get_prototype_of(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<Option<Object>> {
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get_prototype_of(agent, backing_object, gc),
            None => self.internal_prototype(agent),
        })
    }

    /// ## \[\[GetPrototypeOf\]\]
    fn internal_get_prototype_of(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can trigger GC.
        gc: GcScope<'_, '_>,
    ) -> JsResult<Option<Object>> {
        // Note: ordinary_get_prototype_of cannot call JS or trigger
        // GC: No object should ever have a try_proto method that can
        // return None while also using this default impl.
        Ok(unwrap_try(self.try_get_prototype_of(agent, gc.nogc())))
    }

    /// ## Infallible \[\[SetPrototypeOf\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_set_prototype_of(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        _gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        TryResult::Continue(ordinary_set_prototype_of(
            agent,
            self.into_object(),
            prototype,
        ))
    }

    /// ## \[\[SetPrototypeOf\]\]
    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can trigger GC.
        prototype: Option<Object>,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        match self.try_set_prototype_of(agent, prototype, gc.into_nogc()) {
            TryResult::Continue(t) => Ok(t),
            TryResult::Break(_) => unreachable!(),
        }
    }

    /// ## Infallible \[\[IsExtensible\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_is_extensible(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can call JS.
        _gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        // 1. Return OrdinaryIsExtensible(O).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_is_extensible(agent, backing_object),
            None => self.internal_extensible(agent),
        })
    }

    /// ## \[\[IsExtensible\]\]
    fn internal_is_extensible(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can call JS.
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        Ok(unwrap_try(self.try_is_extensible(agent, gc.into_nogc())))
    }

    /// ## Infallible \[\[PreventExtensions\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_prevent_extensions(self, agent: &mut Agent, _gc: NoGcScope<'_, '_>) -> TryResult<bool> {
        // 1. Return OrdinaryPreventExtensions(O).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_prevent_extensions(agent, backing_object),
            None => {
                self.internal_set_extensible(agent, false);
                true
            }
        })
    }

    /// ## \[\[PreventExtensions\]\]
    fn internal_prevent_extensions(self, agent: &mut Agent, gc: GcScope<'_, '_>) -> JsResult<bool> {
        Ok(unwrap_try(
            self.try_prevent_extensions(agent, gc.into_nogc()),
        ))
    }

    /// ## Infallible \[\[GetOwnProperty\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _gc: NoGcScope<'_, '_>,
    ) -> TryResult<Option<PropertyDescriptor>> {
        // 1. Return OrdinaryGetOwnProperty(O, P).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get_own_property(agent, backing_object, property_key),
            None => None,
        })
    }

    /// ## \[\[GetOwnProperty\]\]
    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Option<PropertyDescriptor>> {
        Ok(unwrap_try(self.try_get_own_property(
            agent,
            property_key,
            gc.into_nogc(),
        )))
    }

    /// ## Infallible \[\[DefineOwnProperty\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        TryResult::Continue(ordinary_try_define_own_property(
            agent,
            backing_object,
            property_key,
            property_descriptor,
            gc,
        ))
    }

    /// ## \[\[DefineOwnProperty\]\]
    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        Ok(unwrap_try(self.try_define_own_property(
            agent,
            property_key,
            property_descriptor,
            gc.into_nogc(),
        )))
    }

    /// ## Infallible \[\[HasProperty\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_try_has_property(agent, backing_object, property_key, gc)
            }
            None => {
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                let parent = self.try_get_prototype_of(agent, gc)?;

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

    /// ## \[\[HasProperty\]\]
    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_has_property(agent, backing_object, property_key, gc),
            None => {
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                let parent = self.internal_get_prototype_of(agent, gc.reborrow())?;

                // 4. If parent is not null, then
                if let Some(parent) = parent {
                    // a. Return ? parent.[[HasProperty]](P).
                    parent.internal_has_property(agent, property_key, gc)
                } else {
                    // 5. Return false.
                    Ok(false)
                }
            }
        }
    }

    /// ## Infallible \[\[Get\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<Value> {
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_try_get(agent, backing_object, property_key, receiver, gc)
            }
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = self.try_get_prototype_of(agent, gc)? else {
                    // b. If parent is null, return undefined.
                    return TryResult::Continue(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.try_get(agent, property_key, receiver, gc)
            }
        }
    }

    /// ## \[\[Get\]\]
    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get(agent, backing_object, property_key, receiver, gc),
            None => {
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = self.internal_get_prototype_of(agent, gc.reborrow())? else {
                    // b. If parent is null, return undefined.
                    return Ok(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent.internal_get(agent, property_key, receiver, gc)
            }
        }
    }

    /// ## Infallible \[\[Set\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    /// ## \[\[Set\]\]
    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    /// ## Infallible \[\[Delete\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        // 1. Return ? OrdinaryDelete(O, P).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_delete(agent, backing_object, property_key, gc),
            None => true,
        })
    }

    /// ## \[\[Delete\]\]
    fn internal_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        Ok(unwrap_try(self.try_delete(
            agent,
            property_key,
            gc.into_nogc(),
        )))
    }

    /// ## Infallible \[\[OwnPropertyKeys\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<Vec<PropertyKey<'a>>> {
        // 1. Return OrdinaryOwnPropertyKeys(O).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_own_property_keys(agent, backing_object, gc),
            None => vec![],
        })
    }

    /// ## \[\[OwnPropertyKeys\]\]
    fn internal_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: GcScope<'a, '_>,
    ) -> JsResult<Vec<PropertyKey<'a>>> {
        Ok(unwrap_try(
            self.try_own_property_keys(agent, gc.into_nogc()),
        ))
    }

    /// ## \[\[Call\]\]
    fn internal_call(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: ArgumentsList,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        unreachable!()
    }

    /// ## \[\[Construct\]\]
    fn internal_construct(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
        _gc: GcScope<'_, '_>,
    ) -> JsResult<Object> {
        unreachable!()
    }
}
