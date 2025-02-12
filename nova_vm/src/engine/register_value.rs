// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::{execution::Agent, types::IntoValue};

/// # Pass-by-register or stack JavaScript Value.
///
/// This is a wrapper type for passing Values in registers or on the stack
/// without binding them to the `&mut Agent` lifetime while making it harder to
/// forget about the need to immediately re-bind the parameters Values to it.
///
/// A Value passed through a register or on the stack is not visible to the
/// garbage collector. If a garbage collection safepoint is reached, all Values
/// contained in registers or on the stack must be rooted before the garbage
/// collection is performed and then reloaded after the garbage collection has
/// finished. Any Register Values that are not thus rooted and reloaded are
/// liable to become corrupted by the garbage collection, making them point to
/// different Values (of the same type) than before and possibly making them
/// point beyond the heap's limits. Using such corrupted Values will lead to
/// unpredictable JavaScript execution, or to an immediate crash from indexing
/// beyond a heap vector's bounds.
#[derive(Debug)]
#[repr(transparent)]
pub struct Register<T: IntoValue> {
    value: T,
}

impl<T: IntoValue> Register<T> {
    /// Extracts the Value from a passed-in register or from the stack and
    /// binds it to the passed in Agent's lifetime (@TODO).
    ///
    /// ## Safety
    ///
    /// The binding must be performed before any call taking `&mut Agent` has
    /// been called. It is recommended to bind all parameters at the very first
    /// line of a function:
    ///
    /// ```rs
    /// fn example(
    ///     agent: &mut Agent, mut gc: Gc<'_>,
    ///     this: Register<Value>,
    ///     args: ArgumentsList
    /// ) {
    ///     let (this, arg0, arg1) = unsafe { (
    ///         this.bind(agent),
    ///         args.get(0).bind(agent),
    ///         args.get(1).bind(agent)
    ///     ) };
    /// }
    /// ```
    /// This ensures that the Values cannot be accidentally misused.
    ///
    /// ### Invalid example
    ///
    /// The following code is **invalid**: Do not do this.
    ///
    /// ```rs
    /// fn example(
    ///     agent: &mut Agent, mut gc: Gc<'_>,
    ///     this: Register<Value>,
    ///     args: ArgumentsList
    /// ) {
    ///     let (this, arg0) = unsafe { (
    ///         this.bind(agent),
    ///         args.get(0).bind(agent)
    ///     ) };
    ///     example_2(agent, this, arg0); // Takes &mut Agent
    ///     let arg1 = unsafe { args.get(1).bind(agent) }; // Invalid!
    /// }
    /// ```
    /// Binding arg1 only after the call to `example_2` is invalid as calling
    /// `example_2` may have triggered GC, in which case `arg1` is invalid.
    ///
    /// See `Local<T>` for how rooting `arg1` should be done in this case.
    #[inline]
    unsafe fn bind(self, _agent: &Agent) -> T {
        self.value
    }
}
