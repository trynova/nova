// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::marker::PhantomData;

/// # ZST type representing access to the garbage collector.
///
/// Access to a garbage collected type's heap data should mainly require
/// holding a `ContextRef<'gc, GcToken>`. Borrowing the heap data should bind
/// to the `'gc` lifetime.
// Note: non-exhaustive to make sure this is not constructable on the outside.
#[non_exhaustive]
#[derive(Debug)]
pub(crate) struct GcToken;

/// # ZST type representing a JavaScript call scope
///
/// Access to scoped root values should mainly require holding a
/// `ContextRef<'scope, ScopeToken>`. In limited cases, borrowing heap data can
/// bind to the `'scope` lifetime.
// Note: non-exhaustive to make sure this is not constructable on the outside.
#[non_exhaustive]
#[derive(Debug)]
pub(crate) struct ScopeToken;

/// # JavaScript call scope that may trigger garbage collection
///
/// This marker represents access to the JavaScript call stack and specifically
/// gives the call stack the possibility of performing garbage collection.
/// In the engine, most values are by-default not rooted during operations
/// which means that garbage collection invalidates them. This GcScope marker
/// is a way for the borrow checker to ensure that all values are rooted /
/// returned to the heap / registered with the heap before the garbage
/// collector potentially runs.
///
/// In essence, this is a compile-time method of ensuring safepoint garbage
/// collection safety.
#[derive(Debug)]
pub struct GcScope<'a, 'b> {
    /// A GcScope "owns" the GC access: There is only ever one "active" GcToken
    /// in the world at a time. Reborrowing a GcScope binds the previous one,
    /// so its contained GcToken is "inactive" during the lifetime of the
    /// reborrowed one.
    gc: GcToken,
    /// A GcScope also "owns" the scope access: This is the access to the
    /// Scoped roots stack. This is not yet well-defined but probably only
    /// GC scopes are allowed to shrink the Scoped roots stack.
    scope: ScopeToken,
    /// We must also keep an exclusive borrow on a GcToken. This enables
    /// various engine values to reborrow this lifetime as shared and that way
    /// have the borrow checker check that those values are not used while
    /// garbage collection may run.
    _gc_marker: PhantomData<&'a mut GcToken>,
    /// We keep a shared borrow on the ScopeToken. This is not yet well-defined
    /// but probably we'll create new ScopeToken borrow lifetimes using the
    /// for<'a> closure trick.
    _scope_marker: PhantomData<&'b ScopeToken>,
}

/// # JavaScript call scope that may not trigger garbage collection
///
/// This marker represents access to the JavaScript call stack in a way that
/// cannot trigger garbage collection. Actions like working with primitive
/// JavaScript Values and accessing non-Proxy object prototypes are examples of
/// actions that can never trigger garbage collection.
///
/// This marker allows performing these sort of actions without rooting other
/// values held on the stack.
#[derive(Debug, Clone, Copy)]
pub struct NoGcScope<'a, 'b> {
    /// A NoGcScope does not own the GC access, and naturally cannot trigger
    /// garbage collection. We keep a shared borrow on this lifetime to ensure
    /// that the GcScope we derive from cannot be used concurrently.
    _gc_marker: PhantomData<&'a GcToken>,
    /// We also don't own scope access. This is not yet well-defined.
    _scope_marker: PhantomData<&'b ScopeToken>,
}

impl GcToken {
    unsafe fn new() -> Self {
        Self
    }
}

impl ScopeToken {
    unsafe fn new() -> Self {
        Self
    }
}

impl<'a, 'b> GcScope<'a, 'b> {
    /// SAFETY: Only one GcScope root should exist at any point in time.
    ///
    /// The caller must make sure to only create a new root when a new
    /// JavaScript call stack is initialized.
    #[inline]
    pub(crate) unsafe fn create_root() -> (GcToken, ScopeToken) {
        (GcToken::new(), ScopeToken::new())
    }

    #[inline]
    pub(crate) fn new(_: &'a mut GcToken, _: &'b mut ScopeToken) -> Self {
        Self {
            gc: GcToken,
            scope: ScopeToken,
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }

    /// Create a GcScope marker that inherits the current GcScope's lifetimes.
    /// This reborrowing is necessary to ensure that only one GcScope is active
    /// at any point in time, and the existence of the active GcScope binds any
    /// "parent" GcScopes from being used concurrently.
    #[inline]
    pub fn reborrow(&mut self) -> GcScope<'_, 'b> {
        Self {
            gc: GcToken,
            scope: ScopeToken,
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }

    /// Create a NoGcScope marker that is used to bind the garbage collector
    /// lifetime to various engine values. Existence of the NoGcScope is a
    /// build-time proof that garbage collection cannot happen.
    ///
    /// When a garbage collection can happen, the borrow checker will ensure
    /// that all engine values that were boudn to the NoGcScope are dropped or
    /// are registered with the heap using Scoped or Global roots.
    #[inline]
    pub fn nogc(&self) -> NoGcScope<'_, 'b> {
        NoGcScope::from_gc(self)
    }

    /// Turn a GcScope marker into a NoGcScope. This is otherwise equivalent to
    /// [the `nogc()` method](Self::nogc) with the exception that this consumes
    /// the parent GcScope.
    ///
    /// This is useful when a method ends in a NoGC scope based return within
    /// an if/else branch while another branch still uses the GcScope. The
    /// borrow checker does not like this with the `nogc()` method but allows
    /// it with this method.
    #[inline]
    pub fn into_nogc(self) -> NoGcScope<'a, 'b> {
        NoGcScope {
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }
}

impl<'a, 'b> NoGcScope<'a, 'b> {
    #[allow(unknown_lints, gc_scope_is_only_passed_by_value)]
    #[inline]
    pub(crate) fn from_gc(_: &GcScope<'a, 'b>) -> Self {
        Self {
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }
}

/// Method for binding and unbinding garbage collectable values from the
/// garbage collector lifetime. This is a necessary evil for calling and
/// entering functions that contain garbage collector safepoints.
///
/// ## Why is this needed?
///
/// From the borrow checker's point of view, bindable values all alias the
/// "garbage collector reference" contained in `GcScope`. Any function that
/// can trigger garbage collection takes an exclusive garbage collector
/// reference, which then means that passing any bound values would be an
/// aliasing violation. The borrow checker will not allow that and a compile
/// error results. To allow the call to compile, the bindable values must be
/// unbound at the call site.
///
/// Inside the function, the bindable parameter values initially are unbound
/// from the garbage collector lifetime. This means that the borrow checker
/// will not check their usage for use-after-free. To make the borrow checker
/// check them, the values must be bound using the `bind` function.
///
/// ## Implementation
///
/// The implementations for both functions must be equivalent to a `memcpy` or,
/// for collections of bindable values, a new collection of bindable values
/// recursively mapped. The end result should be effectively equal to a
/// transmute from one value to another. If all transmute requirements are
/// upheld, the implementation of the functions is allowed to be a transmute.
pub trait Bindable {
    type Of<'a>;

    /// Unbind this value from the garbage collector lifetime. This is
    /// necessary for eg. when using the value as a parameter in a call that
    /// can perform garbage collection.
    ///
    /// This function's implementation must be equivalent to a (recursive)
    /// `memcpy`. The intention is that the entire function optimises to
    /// nothing in the final binary.
    ///
    /// ## Safety
    ///
    /// This function is conceptually should only be used for one of the following actions:
    ///
    /// 1. Unbind a value to allow passing it as a parameter.
    /// 2. Unbind a value to allow returning as a result, though this should be
    ///    avoided if possible.
    /// 3. Temporarily unbind a value to allow turning a `GcScope` into a
    ///    `NoGcScope`, and immediately rebind it with the `NoGcScope`.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// // Unbind a value to allow passing it as a parameter.
    /// function_call(agent, value.unbind(), gc.reborrow());
    /// ```
    ///
    /// ```rust
    /// // Unbind a value to allow returning as a result.
    /// let result = function_call(agent, gc.reborrow());
    /// if cond {
    ///   // Note: `result` is bound to a local temporary created in
    ///   // `gc.reborrow()`, which is why this will not work without unbind.
    ///   return Ok(result.unbind());
    /// }
    /// ```
    ///
    /// ```rust
    /// // Unbind a value temporarily to immediately rebind it with a
    /// // `NoGcScope`.
    /// let result = function_call(agent, gc.reborrow()).unbind();
    /// let gc = gc.into_nogc();
    /// let result = result.bind(gc);
    /// ```
    ///
    /// *Incrrect* usage of this function: unbind a value into a variable
    /// without immediate rebinding.
    /// ```rust
    /// let result = try_function_call(agent, gc.nogc()).unbind();
    /// function_call(agent, result, gc.reborrow());
    /// // Note: `result` is use-after-free because of above `gc.reborrow()`.
    /// return Ok(result);
    /// ```
    fn unbind(self) -> Self::Of<'static>;

    /// Bind this value to the garbage collector lifetime. This is necessary to
    /// enable the borrow checker to check that bindable values are not
    /// use-after-free.
    ///
    ///
    ///
    /// This function's implementation must be equivalent to a (recursive)
    /// `memcpy`. The intention is that the entire function optimises to
    /// nothing in the final binary.
    ///
    /// ## Safety
    ///
    /// This function is always safe to use. It is required to call it in the
    /// following places:
    ///
    /// 1. Bind every bindable argument when a function with a garbage
    ///    collector safepoint is entered.
    /// 2. Bind a bindable value when it is copied from the engine heap.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// fn function_call<'gc>(
    ///   agent: &mut Agent,
    ///   this_value: Value,
    ///   arguments: Arguments,
    ///   gc: GcScope<'gc, '_>
    /// ) -> Value<'gc> {
    ///   // Bind every bindable argument when a function with a garbage
    ///   // collector safepoint is entered.
    ///   // Note: Because this function takes `GcScope`, it should contain a
    ///   // safepoint.
    ///   let nogc = gc.nogc();
    ///   let this_value = this_value.bind(nogc);
    ///   let arguments = arguments.bind(nogc);
    /// }
    /// ```
    ///
    /// ```rust
    /// // Bind a bindable value when it is copied from the engine heap.
    /// let first = agent[array].as_slice()[0].bind(gc.nogc());
    /// ```
    ///
    /// *Incorrect* usage of this function: skip binding arguments when a
    /// function with a garbage collector safepoint is entered.
    /// ```rust
    /// fn function_call<'gc>(
    ///   agent: &mut Agent,
    ///   this_value: Value,
    ///   arguments: Arguments,
    ///   gc: GcScope<'gc, '_>
    /// ) -> Value<'gc> {
    ///   // Note: This is still technically fine due to no preceding `GcSCope`
    ///   // usage.
    ///   let string = to_string(agent, this_value, gc.reborrow());
    ///   // Note: `arguments` is use-after-free because of above
    ///   // `gc.reborrow()`.
    ///   let value = arguments.get(0);
    /// }
    /// ```
    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a>;
}

impl<T: Bindable> Bindable for Option<T> {
    type Of<'a> = Option<T::Of<'a>>;

    // Note: Option is simple enough to always inline the code.
    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        self.map(T::unbind)
    }

    #[inline(always)]
    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        self.map(|t| t.bind(gc))
    }
}

impl<T: Bindable, E: Bindable> Bindable for Result<T, E> {
    type Of<'a> = Result<T::Of<'a>, E::Of<'a>>;

    fn unbind(self) -> Self::Of<'static> {
        self.map(T::unbind).map_err(E::unbind)
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        self.map(|t| t.bind(gc)).map_err(|e| e.bind(gc))
    }
}

impl<T: Bindable> Bindable for Vec<T> {
    type Of<'a> = Vec<T::Of<'a>>;

    // Note: Vec isn't simple enough to always inline the code. This should
    // optimise down to nothing in release builds but in debug builds it would
    // probably leave behind actual Vec clones everywhere, leading to a lot of
    // code bloat.
    fn unbind(self) -> Self::Of<'static> {
        self.into_iter().map(T::unbind).collect()
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        self.into_iter().map(|t| t.bind(gc)).collect()
    }
}
