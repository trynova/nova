// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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
    pub unsafe fn bind(self, _: &GcScope<'gc, '_>) -> T {
        self.value
    }
}
