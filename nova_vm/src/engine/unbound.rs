// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::{
    builtins::ArgumentsList,
    execution::{Agent, JsResult},
    types::{
        Function, InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object,
        OrdinaryObject, PropertyDescriptor, PropertyKey, Value,
    },
};

use super::context::GcScope;

/// # Unbound JavaScript heap reference
///
/// This is a wrapper type for passing JavaScript heap references on the stack
/// without binding them to the `&'gc mut` lifetime while making it harder to
/// forget about the need to immediately re-bind the references to it. Binding
/// an unbound heap reference must happen before any garbage collection
/// safepoints are reached (meaning calls that take the `&'gc mut` lifetime).
///
/// A heap reference passed through a register or on the stack is not visible
/// to the garbage collector. If a garbage collection safepoint is reached, all
/// references not present in the Agent heap must be rooted before the garbage
/// collection is performed and then reloaded after the garbage collection has
/// finished. Any references that are not rooted and reloaded in this way are
/// liable to become corrupted by the garbage collection, making them point to
/// different heap data (of the same type) than before and possibly making them
/// point beyond the heap's limits. Using such corrupted references will lead
/// to unpredictable JavaScript execution, or to an immediate crash from
/// indexing beyond a heap vector's bounds.
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Unbound<T: Clone + Copy + 'static> {
    value: T,
}

impl<T: Clone + Copy + 'static> Unbound<T> {
    /// Create a new unbound heap reference from a static heap reference.
    ///
    /// ## Safety
    ///
    /// This method is not marked unsafe, as it requires the passed-in value to
    /// already have a static lifetime. If the caller has to do a lifetime
    /// transmute to call this method, then any safety guarantees related to
    /// the transmute are theirs to uphold.
    ///
    /// The `Unbound` struct itself guarantees that the value cannot be
    /// retrieved without binding it to the heap garbage collection lifetime.
    pub(crate) fn new(value: T) -> Self {
        Self { value }
    }
}

impl<T: IntoValue + 'static> Unbound<T> {
    pub fn into_value(self) -> Unbound<Value> {
        Unbound::new(self.value.into_value())
    }
}

impl<T: IntoObject + 'static> Unbound<T> {
    pub fn into_object(self) -> Unbound<Object> {
        Unbound::new(self.value.into_object())
    }
}

impl<T: IntoFunction + 'static> Unbound<T> {
    pub fn into_function(self) -> Unbound<Function> {
        Unbound::new(self.value.into_function())
    }
}

impl<'gc, T: Clone + Copy + 'gc> Unbound<T> {
    /// Binds an unbound heap reference to the passed in garbage collection
    /// lifetime.
    ///
    /// ## Safety
    ///
    /// The binding must be performed before any call taking `GcScope` has been
    /// called. All parameters should be bound at the very first line of a
    /// function:
    ///
    /// ```rs
    /// fn example(
    ///     agent: &mut Agent,
    ///     gc: GcScope<'_, '_>,
    ///     this: Unbound<Value>,
    ///     args: ArgumentsList
    /// ) {
    ///     let (this, arg0, arg1) = unsafe { (
    ///         this.bind(agent),
    ///         args.get(0).bind(agent),
    ///         args.get(1).bind(agent)
    ///     ) };
    ///     example_2(agent, gc this, arg0); // Takes GcScope
    /// }
    /// ```
    /// This ensures that the heap references cannot be accidentally used after
    /// garbage collection invalidates them.
    ///
    /// ### Invalid example
    ///
    /// The following code is **invalid**: Do not do this.
    ///
    /// ```rs
    /// fn example(
    ///     agent: &mut Agent,
    ///     gc: GcScope<'_, '_>,
    ///     this: Unbound<Value>,
    ///     args: ArgumentsList
    /// ) {
    ///     let (this, arg0) = unsafe { (
    ///         this.bind(agent),
    ///         args.get(0).bind(agent)
    ///     ) };
    ///     example_2(agent, gc, this, arg0); // Takes GcScope
    ///     let arg1 = unsafe { args.get(1).bind(agent) }; // Invalid!
    /// }
    /// ```
    /// Binding `arg1` after the call to `example_2` is invalid as calling
    /// `example_2` may have triggered garbage collection which invalidates the
    /// `arg1` heap reference.
    ///
    /// See `Scoped<T>` for how rooting `arg1` should be done in this case.
    #[inline]
    pub fn bind(self, _: &GcScope<'gc, '_>) -> T {
        self.value
    }
}

impl<T: InternalSlots> InternalSlots for Unbound<T> {
    #[inline]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject> {
        self.value.get_backing_object(agent)
    }

    #[inline]
    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject) {
        self.value.set_backing_object(agent, backing_object);
    }

    #[inline]
    fn internal_prototype(self, agent: &Agent) -> Option<Object> {
        self.value.internal_prototype(agent)
    }

    #[inline]
    fn internal_extensible(self, agent: &Agent) -> bool {
        self.value.internal_extensible(agent)
    }

    #[inline]
    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject {
        self.value.create_backing_object(agent)
    }

    #[inline]
    fn internal_set_prototype(self, agent: &mut Agent, prototype: Option<Object>) {
        self.value.internal_set_prototype(agent, prototype)
    }

    #[inline]
    fn internal_set_extensible(self, agent: &mut Agent, value: bool) {
        self.value.internal_set_extensible(agent, value)
    }
}

impl<T: InternalMethods> InternalMethods for Unbound<T> {
    fn internal_get_prototype_of(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Option<Object>> {
        self.value.internal_get_prototype_of(agent, gc)
    }

    fn internal_set_prototype_of(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        prototype: Option<Unbound<Object>>,
    ) -> JsResult<bool> {
        self.value.internal_set_prototype_of(agent, gc, prototype)
    }

    fn internal_is_extensible(self, agent: &mut Agent, gc: GcScope<'_, '_>) -> JsResult<bool> {
        self.value.internal_is_extensible(agent, gc)
    }

    fn internal_prevent_extensions(self, agent: &mut Agent, gc: GcScope<'_, '_>) -> JsResult<bool> {
        self.value.internal_prevent_extensions(agent, gc)
    }

    fn internal_get_own_property(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: Unbound<PropertyKey>,
    ) -> JsResult<Option<PropertyDescriptor>> {
        self.value
            .internal_get_own_property(agent, gc, property_key)
    }

    fn internal_define_own_property(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: Unbound<PropertyKey>,
        property_descriptor: Unbound<PropertyDescriptor>,
    ) -> JsResult<bool> {
        self.value
            .internal_define_own_property(agent, gc, property_key, property_descriptor)
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: Unbound<PropertyKey>,
    ) -> JsResult<bool> {
        self.value.internal_has_property(agent, gc, property_key)
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: Unbound<PropertyKey>,
        receiver: Unbound<Value>,
    ) -> JsResult<Value> {
        self.value.internal_get(agent, gc, property_key, receiver)
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: Unbound<PropertyKey>,
        value: Unbound<Value>,
        receiver: Unbound<Value>,
    ) -> JsResult<bool> {
        self.value
            .internal_set(agent, gc, property_key, value, receiver)
    }

    fn internal_delete(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        property_key: Unbound<PropertyKey>,
    ) -> JsResult<bool> {
        self.value.internal_delete(agent, gc, property_key)
    }

    fn internal_own_property_keys(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Vec<PropertyKey>> {
        self.value.internal_own_property_keys(agent, gc)
    }

    fn internal_call(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        this_value: Unbound<Value>,
        arguments_list: ArgumentsList,
    ) -> JsResult<Value> {
        self.value
            .internal_call(agent, gc, this_value, arguments_list)
    }

    fn internal_construct(
        self,
        agent: &mut Agent,
        gc: GcScope<'_, '_>,
        arguments_list: ArgumentsList,
        new_target: Function,
    ) -> JsResult<Object> {
        self.value
            .internal_construct(agent, gc, arguments_list, new_target)
    }
}
