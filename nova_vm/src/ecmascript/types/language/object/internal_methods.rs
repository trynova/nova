// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::ControlFlow;

use super::{InternalSlots, Object, PropertyKey};
use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::call_function,
        builtins::{
            ArgumentsList,
            ordinary::{
                caches::{PropertyLookupCache, PropertyOffset},
                ordinary_define_own_property, ordinary_delete, ordinary_get,
                ordinary_get_own_property, ordinary_get_prototype_of, ordinary_has_property,
                ordinary_is_extensible, ordinary_own_property_keys, ordinary_prevent_extensions,
                ordinary_set, ordinary_set_at_offset, ordinary_set_prototype_of, ordinary_try_get,
                ordinary_try_has_property, ordinary_try_set,
            },
            proxy::Proxy,
        },
        execution::{
            Agent, JsResult,
            agent::{TryError, TryResult, js_result_into_try, try_result_ok, unwrap_try},
        },
        types::{Function,  PropertyDescriptor, Value, throw_cannot_set_property},
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Scopable,
    },
};

/// ### [6.1.7.2 Object Internal Methods and Internal Slots](https://tc39.es/ecma262/#sec-object-internal-methods-and-internal-slots)
pub trait InternalMethods<'a>
where
    Self: InternalSlots<'a>,
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
    ) -> TryResult<'gc, Option<Object<'gc>>> {
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
    fn try_set_prototype_of<'gc>(
        self,
        agent: &mut Agent,
        prototype: Option<Object>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        TryResult::Continue(ordinary_set_prototype_of(agent, self.into(), prototype, gc))
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
    fn try_is_extensible<'gc>(
        self,
        agent: &mut Agent,
        // Note: Because of Proxies, this can call JS.
        _gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
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
    fn try_prevent_extensions<'gc>(
        self,
        agent: &mut Agent,
        _gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
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
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, Option<PropertyDescriptor<'gc>>> {
        // 1. Return OrdinaryGetOwnProperty(O, P).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => ordinary_get_own_property(
                agent,
                self.into().bind(gc),
                backing_object,
                property_key,
                cache,
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
            None,
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
    fn try_define_own_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        let backing_object = self
            .get_backing_object(agent)
            .unwrap_or_else(|| self.create_backing_object(agent));
        js_result_into_try(ordinary_define_own_property(
            agent,
            self.into(),
            backing_object,
            property_key,
            property_descriptor,
            cache,
            gc,
        ))
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
            None,
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
    fn try_has_property<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryHasResult<'gc>> {
        // 1. Return ? OrdinaryHasProperty(O, P).
        ordinary_try_has_property(
            agent,
            self.into(),
            self.get_backing_object(agent),
            property_key,
            cache,
            gc,
        )
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
            Some(backing_object) => ordinary_has_property(
                agent,
                self.into(),
                backing_object,
                property_key.unbind(),
                gc,
            ),
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

    /// ## Try \[\[Get\]\]
    ///
    /// This is a variant of the method that does not allow calling into
    /// JavaScript or triggering garbage collection. If the internal method
    /// cannot be completed without calling into JavaScript, then `TryBreak` is
    /// returned.
    fn try_get<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, TryGetResult<'gc>> {
        // 1. Return ? OrdinaryGet(O, P, Receiver).
        ordinary_try_get(
            agent,
            self.into(),
            self.get_backing_object(agent),
            property_key,
            receiver,
            cache,
            gc,
        )
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
    fn try_set<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        // 1. Return ? OrdinarySet(O, P, V, Receiver).
        ordinary_try_set(agent, self, property_key, value, receiver, cache, gc)
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
        ordinary_set(agent, self.into(), property_key, value, receiver, gc)
    }

    /// ## Infallible \[\[Delete\]\]
    ///
    /// This is an infallible variant of the method that does not allow calling
    /// into JavaScript or triggering garbage collection. If the internal
    /// method cannot be completed without calling into JavaScript, then `None`
    /// is returned. It is preferable to call this method first and only call
    /// the main method if this returns None.
    fn try_delete<'gc>(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        // 1. Return ? OrdinaryDelete(O, P).
        TryResult::Continue(match self.get_backing_object(agent) {
            Some(backing_object) => {
                ordinary_delete(agent, self.into(), backing_object, property_key, gc)
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
    ) -> TryResult<'gc, Vec<PropertyKey<'gc>>> {
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
    ) -> TryGetResult<'gc> {
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
        props: &SetCachedProps,
        offset: PropertyOffset,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        if offset.is_custom_property() {
            // We don't yet cache any of these accesses.
            todo!(
                "{} needs to implement custom property caching manually",
                core::any::type_name::<Self>()
            )
        } else {
            ordinary_set_at_offset(
                agent,
                props,
                self.into(),
                self.get_backing_object(agent),
                offset,
                gc,
            )
        }
    }

    /// ## \[\[Call\]\]
    #[allow(unused_variables)]
    fn internal_call<'gc>(
        self,
        agent: &mut Agent,
        this_value: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unreachable!()
    }

    /// ## \[\[Construct\]\]
    #[allow(unused_variables)]
    fn internal_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        unreachable!()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Continue results for the \[\[Get]] method's Try variant.
pub enum TryHasResult<'a> {
    /// Property was not found in the object or its prototype chain.
    Unset,
    /// The property was found at the provided offset in the provided object.
    Offset(u32, Object<'a>),
    /// The property was found in the provided object at a custom offset.
    Custom(u32, Object<'a>),
    /// A Proxy trap call is needed.
    ///
    /// This means that the method ran to completion but could not call the
    /// Proxy trap itself.
    Proxy(Proxy<'a>),
}
bindable_handle!(TryHasResult);
try_result_ok!(TryHasResult);

#[derive(Debug, Clone, Copy, PartialEq)]
/// Continue results for the \[\[Get]] method's Try variant.
pub enum TryGetResult<'a> {
    /// No property exists in the object or its prototype chain.
    Unset,
    /// A data property was found.
    Value(Value<'a>),
    /// A getter call is needed.
    ///
    /// This means that the method ran to completion but could not call the
    /// getter itself.
    Get(Function<'a>),
    /// A Proxy trap call is needed.
    ///
    /// This means that the method ran to completion but could not call the
    /// Proxy trap itself.
    Proxy(Proxy<'a>),
}
bindable_handle!(TryGetResult);
try_result_ok!(TryGetResult);

/// Handle TryGetResult within a GC scope.
///
/// It is recommended to handle at least `TryGetContinue::Unset` and
/// `TryGetContinue::Value`, and possibly `TryBreak::Error` outside of this
/// function as fast paths.
#[inline(never)]
pub fn handle_try_get_result<'gc>(
    agent: &mut Agent,
    o: impl InternalMethods<'gc>,
    p: PropertyKey,
    result: TryResult<TryGetResult>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let p = p.bind(gc.nogc());
    let result = result.bind(gc.nogc());
    match result {
        ControlFlow::Continue(c) => match c {
            TryGetResult::Unset => Ok(Value::Undefined),
            TryGetResult::Value(value) => Ok(value.unbind().bind(gc.into_nogc())),
            TryGetResult::Get(getter) => {
                call_function(agent, getter.unbind(), o.into().unbind(), None, gc)
            }
            TryGetResult::Proxy(proxy) => {
                proxy
                    .unbind()
                    .internal_get(agent, p.unbind(), o.into().unbind(), gc)
            }
        },
        ControlFlow::Break(b) => match b {
            TryError::Err(err) => Err(err.unbind().bind(gc.into_nogc())),
            TryError::GcError => o.internal_get(agent, p.unbind(), o.into().unbind(), gc),
        },
    }
}

pub fn try_get_result_into_value<'a>(
    v: TryResult<'a, TryGetResult<'a>>,
) -> TryResult<'a, Value<'a>> {
    match v? {
        TryGetResult::Unset => TryResult::Continue(Value::Undefined),
        TryGetResult::Value(value) => TryResult::Continue(value),
        _ => TryError::GcError.into(),
    }
}

#[inline(always)]
#[cfg(any(feature = "array-buffer", feature = "regexp"))]
pub(crate) fn unwrap_try_get_value<'a>(v: TryResult<'a, TryGetResult<'a>>) -> Value<'a> {
    match v {
        ControlFlow::Continue(TryGetResult::Value(v)) => v,
        _ => unreachable!(),
    }
}

#[inline(always)]
#[cfg(feature = "array-buffer")]
pub(crate) fn unwrap_try_get_value_or_unset<'a>(v: TryResult<'a, TryGetResult<'a>>) -> Value<'a> {
    match v {
        ControlFlow::Continue(TryGetResult::Value(v)) => v,
        ControlFlow::Continue(TryGetResult::Unset) => Value::Undefined,
        _ => unreachable!(),
    }
}

impl<'a> From<Value<'a>> for TryGetResult<'a> {
    fn from(value: Value<'a>) -> Self {
        Self::Value(value)
    }
}

impl<'a, T> From<Value<'a>> for ControlFlow<TryGetResult<'a>, T> {
    fn from(value: Value<'a>) -> Self {
        Self::Break(TryGetResult::Value(value))
    }
}

impl<'a> From<TryGetResult<'a>> for ControlFlow<TryGetResult<'a>, NoCache> {
    fn from(value: TryGetResult<'a>) -> Self {
        Self::Break(value)
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

pub struct SetCachedProps<'a> {
    pub p: PropertyKey<'a>,
    pub receiver: Value<'a>,
    pub cache: PropertyLookupCache<'a>,
    pub value: Value<'a>,
}

/// Early-return conditions for [[Set]] method's cached variant.
///
/// Early-return effectively means that a cached property lookup was found and
/// the normal \[\[Set]] method variant need not be entered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetResult<'a> {
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
bindable_handle!(SetResult);
try_result_ok!(SetResult);

impl SetResult<'_> {
    /// Returns true if the \[\[Set]] method finished successfully.
    pub fn succeeded(&self) -> bool {
        matches!(self, SetResult::Done)
    }

    /// Returns true if the \[\[Set]] method finished unsuccessfully.
    pub fn failed(&self) -> bool {
        matches!(self, SetResult::Accessor | SetResult::Unwritable)
    }

    /// Converts the \[\[Set]] result into a boolean.
    ///
    /// Returns None if the method did not run to finish.
    pub fn into_boolean(self) -> Option<bool> {
        match self {
            SetResult::Done => Some(true),
            SetResult::Unwritable | SetResult::Accessor => Some(false),
            SetResult::Set(_) | SetResult::Proxy(_) => None,
        }
    }
}

/// Helper function for calling a Proxy [[Set]] trap when triggered by finding
/// a Proxy used as a prototype.
pub fn call_proxy_set<'a>(
    agent: &mut Agent,
    proxy: Proxy,
    p: PropertyKey,
    value: Value,
    receiver: Value,
    strict: bool,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let proxy = proxy.unbind();
    let receiver = receiver.unbind();
    let p = p.unbind();
    let value = value.unbind();
    if strict {
        let scoped_p = p.scope(agent, gc.nogc());
        let scoped_o = receiver.scope(agent, gc.nogc());
        let succeeded = proxy.internal_set(agent, p, value, receiver.into(), gc.reborrow());
        // SAFETY: not shared.
        let o = unsafe { scoped_o.take(agent) };
        let succeeded = succeeded.unbind()?;
        if !succeeded {
            // d. If succeeded is false and V.[[Strict]] is true, throw a TypeError exception.
            let o = o
                .into()
                .string_repr(agent, gc.reborrow())
                .unbind()
                .bind(gc.nogc());
            // SAFETY: not shared.
            let p = unsafe { scoped_p.take(agent) }.bind(gc.nogc());
            return Err(throw_cannot_set_property(
                agent,
                o.into().unbind(),
                p.unbind(),
                gc.into_nogc(),
            ));
        }
    } else {
        // In sloppy mode we don't care about the result.
        let _ = proxy.internal_set(agent, p, value, receiver.into(), gc)?;
    }
    Ok(())
}
