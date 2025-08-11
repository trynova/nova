// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::ControlFlow;

use super::{InternalSlots, Object, PropertyKey};
use crate::{
    ecmascript::{
        builtins::{
            ArgumentsList,
            ordinary::{
                caches::{PropertyLookupCache, PropertyOffset},
                ordinary_define_own_property, ordinary_delete, ordinary_get,
                ordinary_get_own_property, ordinary_get_prototype_of, ordinary_has_property,
                ordinary_is_extensible, ordinary_own_property_keys, ordinary_prevent_extensions,
                ordinary_set, ordinary_set_at_offset, ordinary_set_prototype_of, ordinary_try_get,
                ordinary_try_has_property, ordinary_try_set,
                shape::ShapeSetCachedProps,
            },
            proxy::Proxy,
        },
        execution::{Agent, JsResult},
        types::{Function, PropertyDescriptor, Value},
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
        unwrap_try,
    },
};

/// ### [6.1.7.2 Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots)
pub trait InternalMethods<'a>
where
    Self: 'a + core::fmt::Debug + Sized + Clone + Copy + Into<Object<'a>> + InternalSlots<'a>,
{
    /// ## Infallible \[\[GetPrototypeOf\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<Object<'gc>>> {
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get_prototype_of(agent, backing_object, gc),
            None => self.internal_prototype(agent),
        })
    }

    /// ## \[\[GetPrototypeOf\]\]
    #[inline(always)]
    fn internal_get_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can trigger GC.
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<Object<'gc>>> {
        // Note: ordinary_get_prototype_of cannot call JS or trigger
        // GC: No object should ever have a try_proto method that can
        // return None while also using this default impl.
        Ok(unwrap_try(self.try_get_prototype_of(agent, gc.into_nogc())))
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
        gc: NoGcScope,
    ) -> TryResult<bool> {
        TryResult::Continue(ordinary_set_prototype_of(
            agent,
            self.into_object(),
            prototype,
            gc,
        ))
    }

    /// ## \[\[SetPrototypeOf\]\]
    #[inline(always)]
    fn internal_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can trigger GC.
        prototype: Option<Object>,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
        _gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. Return OrdinaryIsExtensible(O).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_is_extensible(agent, backing_object),
            None => self.internal_extensible(agent),
        })
    }

    /// ## \[\[IsExtensible\]\]
    #[inline(always)]
    fn internal_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can call JS.
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        Ok(unwrap_try(self.try_is_extensible(agent, gc.into_nogc())))
    }

    /// ## Infallible \[\[PreventExtensions\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_prevent_extensions(self, agent: &mut Agent, _gc: NoGcScope) -> TryResult<bool> {
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
    #[inline(always)]
    fn internal_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
    fn try_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Option<PropertyDescriptor<'gc>>> {
        // 1. Return OrdinaryGetOwnProperty(O, P).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get_own_property(
                agent,
                self.into_object().bind(gc),
                backing_object,
                property_key,
                gc,
            ),
            None => None,
        })
    }

    /// ## \[\[GetOwnProperty\]\]
    #[inline(always)]
    fn internal_get_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Option<PropertyDescriptor<'gc>>> {
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
        gc: NoGcScope,
    ) -> TryResult<bool> {
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        match ordinary_define_own_property(
            agent,
            self.into_object(),
            backing_object,
            property_key,
            property_descriptor,
            gc,
        ) {
            Ok(b) => TryResult::Continue(b),
            Err(_) => TryResult::Break(()),
        }
    }

    /// ## \[\[DefineOwnProperty\]\]
    #[inline(always)]
    fn internal_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
        gc: NoGcScope,
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
    fn internal_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
        let property_key = property_key.bind(gc.nogc());
        // 1. Return ? OrdinaryHasProperty(O, P).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_has_property(agent, backing_object, property_key.unbind(), gc)
            }
            None => {
                let property_key = property_key.scope(agent, gc.nogc());
                // 3. Let parent be ? O.[[GetPrototypeOf]]().
                let parent = self
                    .internal_get_prototype_of(agent, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc());

                // 4. If parent is not null, then
                if let Some(parent) = parent {
                    // a. Return ? parent.[[HasProperty]](P).
                    parent
                        .unbind()
                        .internal_has_property(agent, property_key.get(agent), gc)
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
    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Value<'gc>> {
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_try_get(
                agent,
                self.into_object(),
                backing_object,
                property_key,
                receiver,
                gc,
            ),
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
    fn internal_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let property_key = property_key.bind(gc.nogc());
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_get(agent, backing_object, property_key.unbind(), receiver, gc)
            }
            None => {
                let property_key = property_key.scope(agent, gc.nogc());
                // a. Let parent be ? O.[[GetPrototypeOf]]().
                let Some(parent) = self
                    .internal_get_prototype_of(agent, gc.reborrow())
                    .unbind()?
                    .bind(gc.nogc())
                else {
                    // b. If parent is null, return undefined.
                    return Ok(Value::Undefined);
                };

                // c. Return ? parent.[[Get]](P, Receiver).
                parent
                    .unbind()
                    .internal_get(agent, property_key.get(agent), receiver, gc)
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
    #[inline(always)]
    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self.into_object(), property_key, value, receiver, gc)
    }

    /// ## \[\[Set\]\]
    #[inline(always)]
    fn internal_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
        gc: NoGcScope,
    ) -> TryResult<bool> {
        // 1. Return ? OrdinaryDelete(O, P).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_delete(agent, self.into_object(), backing_object, property_key, gc)
            }
            None => true,
        })
    }

    /// ## \[\[Delete\]\]
    #[inline(always)]
    fn internal_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, bool> {
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
    fn try_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<Vec<PropertyKey<'gc>>> {
        // 1. Return OrdinaryOwnPropertyKeys(O).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_own_property_keys(agent, backing_object, gc),
            None => vec![],
        })
    }

    /// ## \[\[OwnPropertyKeys\]\]
    fn internal_own_property_keys<'gc>(
        self,
        agent: &mut Agent,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Vec<PropertyKey<'gc>>> {
        Ok(unwrap_try(
            self.try_own_property_keys(agent, gc.into_nogc()),
        ))
    }

    /// ## \[\[Get]] method with caching.
    ///
    /// This method is a variant of the \[\[Get]] method which can never call
    /// into JavaScript and thus cannot trigger garbage collection. If the
    /// method would need to call a getter function or a Proxy trap, then the
    /// method explicit returns a result signifying that need. The caller is
    /// thus in charge of control flow.
    ///
    /// > NOTE: Because the method cannot call getters, the receiver parameter
    /// > is not part of the API.
    fn get_cached<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        cache: PropertyLookupCache,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<GetCachedResult<'gc>, NoCache> {
        // A cache-based lookup on an ordinary object can fully rely on the
        // Object Shape and caches.
        let shape = self.object_shape(agent);
        shape.get_cached(agent, p, self.into_value(), cache, gc)
    }

    /// ## \[\[Set]] method with caching.
    ///
    /// This method is a variant of the \[\[Set]] method which can never call
    /// into JavaScript and thus cannot trigger garbage collection. If the
    /// method would need to call a setter function or a Proxy trap, then the
    /// method explicit returns a result signifying that need. The caller is
    /// thus in charge of control flow.
    fn set_cached<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        value: Value,
        receiver: Value,
        cache: PropertyLookupCache,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        // A cache-based lookup on an ordinary object can fully rely on the
        // Object Shape and caches.
        let shape = self.object_shape(agent);
        shape.set_cached(
            agent,
            ShapeSetCachedProps {
                o: self.into_object(),
                p,
                receiver,
            },
            value,
            cache,
            gc,
        )
    }

    /// ## \[\[GetOwnProperty]] method with offset.
    ///
    /// This is a variant of the \[\[GetOwnProperty]] method that reads a
    /// property value (or getter function) at an offset based on cached data.
    /// The method cannot call into JavaScript and is used as part of the
    /// \[\[Get]] method's cached variant.
    fn get_own_property_at_offset<'gc>(
        self,
        agent: &Agent,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<GetCachedResult<'gc>, NoCache> {
        if offset.is_custom_property() {
            // We don't yet cache any of these accesses.
            todo!(
                "{} needs to implement custom property caching manually",
                core::any::type_name::<Self>()
            )
        } else {
            // It should be impossible for us to not have a backing store.
            self.get_backing_object(agent)
                .unwrap()
                .get_own_property_at_offset(agent, offset, gc)
        }
    }

    /// ## \[\[Set]] method with offset.
    ///
    /// This is a variant of the \[\[Set]] method that attempts to set the
    /// value of property at an offset. The method cannot call into JavaScript
    /// or trigger garbage collection, and any such needs must be interrupted
    /// and control returned to the caller. The method is used as part of the
    /// \[\[Set]] method's cached variant.
    fn set_at_offset<'gc>(
        self,
        agent: &mut Agent,
        p: PropertyKey,
        offset: PropertyOffset,
        value: Value,
        receiver: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<SetCachedResult<'gc>, NoCache> {
        if offset.is_custom_property() {
            // We don't yet cache any of these accesses.
            todo!(
                "{} needs to implement custom property caching manually",
                core::any::type_name::<Self>()
            )
        } else {
            let backing_object = self.get_backing_object(agent);
            ordinary_set_at_offset(
                agent,
                (self.into_object(), backing_object),
                (p, offset),
                value,
                receiver,
                gc,
            )
        }
    }

    /// ## \[\[Call\]\]
    fn internal_call<'gc>(
        self,
        _agent: &mut Agent,
        _this_value: Value,
        _arguments_list: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unreachable!()
    }

    /// ## \[\[Construct\]\]
    fn internal_construct<'gc>(
        self,
        _agent: &mut Agent,
        _arguments_list: ArgumentsList,
        _new_target: Function,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        unreachable!()
    }
}

/// Early-return conditions for [[Get]] method's cached variant.
///
/// Early-return effectively means that a cached property lookup was found
/// and the normal \[\[Get]] method variant need not be entered.
#[derive(Debug)]
pub enum GetCachedResult<'a> {
    /// A data property was found.
    Value(Value<'a>),
    /// A getter call is needed.
    Get(Function<'a>),
    /// A Proxy trap call is needed.
    Proxy(Proxy<'a>),
}

impl<'a, T> From<GetCachedResult<'a>> for ControlFlow<GetCachedResult<'a>, T> {
    fn from(value: GetCachedResult<'a>) -> Self {
        ControlFlow::Break(value)
    }
}

impl<'a, T> From<Value<'a>> for ControlFlow<GetCachedResult<'a>, T> {
    fn from(value: Value<'a>) -> Self {
        ControlFlow::Break(GetCachedResult::Value(value))
    }
}

/// No property cache was found.
///
/// The normal \[\[Get]] or \[\[Set]] method variant should be entered.
pub struct NoCache;

impl<T> From<NoCache> for ControlFlow<T, NoCache> {
    fn from(value: NoCache) -> Self {
        ControlFlow::Continue(value)
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for GetCachedResult<'_> {
    type Of<'a> = GetCachedResult<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

/// Early-return conditions for [[Set]] method's cached variant.
///
/// Early-return effectively means that a cached property lookup was found and
/// the normal \[\[Set]] method variant need not be entered.
pub enum SetCachedResult<'a> {
    /// Value was successfully set.
    Done,
    /// Value could not be set due to unwritable property or nonextensible
    /// object.
    Unwritable,
    /// Value could not be set due to being an accessor without a setter.
    Accessor,
    /// A setter call is needed.
    Set(Function<'a>),
    /// A Proxy trap call is needed.
    Proxy(Proxy<'a>),
}

impl<'a, T> From<SetCachedResult<'a>> for ControlFlow<SetCachedResult<'a>, T> {
    fn from(value: SetCachedResult<'a>) -> Self {
        ControlFlow::Break(value)
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for SetCachedResult<'_> {
    type Of<'a> = SetCachedResult<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}
