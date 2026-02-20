// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::marker::PhantomData;
use std::ops::ControlFlow;

use wtf8::CodePoint;

use super::{Rootable, Scoped};

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
    #[expect(dead_code)]
    gc: GcToken,
    /// A GcScope also "owns" the scope access: This is the access to the
    /// Scoped roots stack. This is not yet well-defined but probably only
    /// GC scopes are allowed to shrink the Scoped roots stack.
    #[expect(dead_code)]
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

impl<'a, 'b> NoGcScope<'a, 'b> {
    /// If you're intentionally calling this function, you've probably made a
    /// mistake. You already hold a NoGcScope and are trying to it into a
    /// NoGcScope again: there is no need. Just remove this call.
    #[inline(always)]
    #[must_use]
    pub(crate) fn into_nogc(self) -> Self {
        self
    }
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
        (unsafe { GcToken::new() }, unsafe { ScopeToken::new() })
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

    /// Bind a handle to the garbage collector lifetime, ensuring that the
    /// handle is invalidated if a garbage collection safepoint is reached.
    #[inline(always)]
    pub fn join<'gc, T: Bindable<'gc>>(&'gc self, _: &T) {}

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

    /// Create a GcScope marker that inherits the current GcScope's garbage
    /// collector lifetime but creates a new scope lifetime. This should be
    /// used when deeper JavaScript call stacks are entered.
    #[inline]
    pub(crate) fn subscope(&mut self) -> GcScope<'a, '_> {
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
    /// that all engine values that were bound to the NoGcScope are dropped or
    /// are registered with the heap using Scoped or Global roots.
    #[inline]
    #[must_use]
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
    #[must_use]
    pub fn into_nogc(self) -> NoGcScope<'a, 'b> {
        NoGcScope {
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }
}

impl<'a, 'b> NoGcScope<'a, 'b> {
    /// Bind a handle to the garbage collector lifetime, ensuring that the
    /// handle is invalidated if a garbage collection safepoint is reached.
    #[inline(always)]
    pub fn join<T: Bindable<'a>>(self, _: &T) {}

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
/// ## Safety
///
/// The implementations for both functions must be equivalent to a `memcpy` or,
/// for collections of bindable values, a new collection of bindable values
/// recursively mapped. The end result should be entirely equal to a lifetime
/// transmute on performed on Self. The implementation of the functions
/// themselves are also allowed to be a plain transmute.
///
/// ```rust,compile_fail
/// let result = unsafe { core::mem::transmute::<Value<'a>, Value<'b>(value) };
/// ```
pub unsafe trait Bindable<'a>: Sized {
    /// Bound representation of self. This must always be effectively equal to
    /// `Self<'a>`. Note that we cannot write `Self<'a>` directly because
    /// `Self` cannot have lifetime parameters attached to it.
    ///
    /// ## Safety
    ///
    /// This type is the most important part of this trait. It _must be_
    /// correctly set to be effectively equal to `Self<'a>`.
    ///
    /// ## Examples
    ///
    /// This is the only correct way to define the type:
    ///
    /// ```rust
    /// use nova_vm::engine::{Bindable, NoGcScope};
    /// struct MyType<'a>(std::marker::PhantomData<&'a ()>);
    /// unsafe impl Bindable for MyType<'_> {
    ///   type Of<'a> = MyType<'a>;
    ///
    ///   #[inline(always)]
    ///   fn local<'a>(self) -> Self::Of<'a> {
    ///     unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    ///   }
    /// }
    /// ```
    type Of<'l>
    where
        Self: 'l;

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
    /// ```rust,ignore
    /// // Unbind a value to allow passing it as a parameter.
    /// function_call(agent, value,gc.reborrow());
    /// ```
    ///
    /// ```rust,ignore
    /// // Unbind a value to allow returning as a result.
    /// let result = function_call(agent, gc.reborrow());
    /// if cond {
    ///   // Note: `result` is bound to a local temporary created in
    ///   // `gc.reborrow()`, which is why this will not work without unbind.
    ///   return Ok(result);
    /// }
    /// ```
    ///
    /// ```rust,ignore
    /// // Unbind a value temporarily to immediately rebind it with a
    /// // `NoGcScope`.
    /// let result = function_call(agent, gc.reborrow());
    /// let gc = gc.into_nogc();
    /// crate::engine::bind!(let result = result, gc);
    /// ```
    ///
    /// *Incrrect* usage of this function: unbind a value into a variable
    /// without immediate rebinding.
    /// ```rust,ignore
    /// let result = try_function_call(agent, gc.nogc());
    /// function_call(agent, result, gc.reborrow());
    /// // Note: `result` is use-after-free because of above `gc.reborrow()`.
    /// return Ok(result);
    /// ```
    fn local<'l>(&'l self) -> Self::Of<'l>;
}

macro_rules! bind {
    ($handle:ident, $gc:ident) => {
        $gc.join(&$handle);
    };
    (let $handle:ident = $handle_creation: expr, $gc:ident) => {
        let $handle = $handle_creation.local();
        $gc.join(&$handle);
    };
    (let mut $handle:ident = $handle_creation: expr, $gc:ident) => {
        let mut $handle = $handle_creation.local();
        $gc.join(&$handle);
    };
}
pub(crate) use bind;

macro_rules! bindable_handle {
    ($self:ident) => {
        // SAFETY: Bindable handle.
        unsafe impl<'a> crate::engine::Bindable<'a> for $self<'a> {
            type Of<'l> = $self<'l>;

            #[inline(always)]
            fn local<'l>(self) -> Self::Of<'l> {
                unsafe { core::mem::transmute::<Self, Self::Of<'l>>(self) }
            }
        }
    };
}
pub(crate) use bindable_handle;

macro_rules! trivially_bindable {
    ($self:ty) => {
        // SAFETY: Trivially safe.
        unsafe impl<'a> crate::engine::Bindable<'a> for $self {
            type Of<'l> = Self;

            #[inline(always)]
            fn local<'l>(self) -> Self::Of<'l> {
                self
            }
        }
    };
}
pub(crate) use trivially_bindable;

trivially_bindable!(());
trivially_bindable!(bool);
trivially_bindable!(i8);
trivially_bindable!(u8);
trivially_bindable!(i16);
trivially_bindable!(u16);
trivially_bindable!(i32);
trivially_bindable!(u32);
trivially_bindable!(i64);
trivially_bindable!(u64);
trivially_bindable!(i128);
trivially_bindable!(u128);
trivially_bindable!(isize);
trivially_bindable!(usize);
#[cfg(feature = "proposal-float16array")]
trivially_bindable!(f16);
trivially_bindable!(f32);
trivially_bindable!(f64);
trivially_bindable!(CodePoint);

// SAFETY: Trivially safe.
unsafe impl<'a, 'b, T: 'static + Rootable> Bindable<'a> for Scoped<'b, T> {
    type Of<'l> = Scoped<'b, T>;

    #[inline(always)]
    fn local<'l>(self) -> Self::Of<'l> {
        self
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>> Bindable<'a> for Option<T> {
    type Of<'l> = Option<T::Of<'l>>;

    // Note: Option is simple enough to always inline the code.
    #[inline(always)]
    fn local<'l>(self) -> Self::Of<'l> {
        const {
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        self.map(T::local)
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>> Bindable<'a> for Box<T> {
    type Of<'l> = Box<T::Of<'l>>;

    // Note: Box is simple enough to always inline the code.
    #[inline(always)]
    fn local<'l>(self) -> Self::Of<'l> {
        const {
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        unsafe { std::mem::transmute::<_, _>(self) }
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>, E: Bindable<'a>> Bindable<'a> for Result<T, E> {
    type Of<'l> = Result<T::Of<'l>, E::Of<'l>>;

    #[inline(always)]
    fn local<'l>(self) -> Self::Of<'l> {
        const {
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        self.map(T::local).map_err(E::local)
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>, E: Bindable<'a>> Bindable<'a> for ControlFlow<T, E> {
    type Of<'l> = ControlFlow<T::Of<'l>, E::Of<'l>>;

    #[inline(always)]
    fn local<'l>(self) -> Self::Of<'l> {
        const {
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        match self {
            ControlFlow::Continue(c) => ControlFlow::Continue(c),
            ControlFlow::Break(b) => ControlFlow::Break(b),
        }
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>> Bindable<'a> for Vec<T> {
    type Of<'l> = Vec<T::Of<'l>>;

    #[inline(always)]
    fn local<'l>(self) -> Self::Of<'l> {
        const {
            // Note: These checks do not guarantee that the Vec transmute is
            // truly safe: Vec is free to rearrange its fields if its type
            // parameter changes. These checks will only catch flagrant misuse.
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        // SAFETY: We assume that T properly implements Bindable. In that case
        // we can safely transmute the lifetime out of the T's in the Vec.
        unsafe { core::mem::transmute::<Vec<T>, Vec<T::Of<'l>>>(self) }
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>> Bindable<'a> for Box<[T]> {
    type Of<'l> = Box<[T::Of<'l>]>;

    fn local<'l>(self) -> Self::Of<'l> {
        const {
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        // SAFETY: We assume that T properly implements Bindable. In that case
        // we can safely transmute the lifetime out of the T's in the slice.
        unsafe { core::mem::transmute::<Box<[T]>, Box<[T::Of<'l>]>>(self) }
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, 'slice, T: Bindable<'a>> Bindable<'a> for &'slice [T]
where
    for<'l> <T as Bindable<'a>>::Of<'l>: 'slice,
{
    type Of<'l> = &'slice [T::Of<'l>];

    fn local<'l>(self) -> Self::Of<'l> {
        const {
            assert!(core::mem::size_of::<T>() == core::mem::size_of::<T::Of<'_>>());
            assert!(core::mem::align_of::<T>() == core::mem::align_of::<T::Of<'_>>());
        }
        // SAFETY: We assume that T properly implements Bindable. In that case
        // we can safely transmute the lifetime out of the T's in the slice.
        unsafe { core::mem::transmute::<&'slice [T], &'slice [T::Of<'l>]>(self) }
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>, U: Bindable<'a>> Bindable<'a> for (T, U) {
    type Of<'l> = (T::Of<'l>, U::Of<'l>);

    fn local<'l>(self) -> Self::Of<'l> {
        (self.0.local(), self.1.local())
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>, U: Bindable<'a>, V: Bindable<'a>> Bindable<'a> for (T, U, V) {
    type Of<'l> = (T::Of<'l>, U::Of<'l>, V::Of<'l>);

    fn local<'l>(self) -> Self::Of<'l> {
        (self.0.local(), self.1.local(), self.2.local())
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<'a, T: Bindable<'a>, U: Bindable<'a>, V: Bindable<'a>, W: Bindable<'a>> Bindable<'a>
    for (T, U, V, W)
{
    type Of<'l> = (T::Of<'l>, U::Of<'l>, V::Of<'l>, W::Of<'l>);

    fn local<'l>(self) -> Self::Of<'l> {
        (
            self.0.local(),
            self.1.local(),
            self.2.local(),
            self.3.local(),
        )
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<T: Bindable, U: Bindable, V: Bindable, W: Bindable, X: Bindable> Bindable
    for (T, U, V, W, X)
{
    type Of<'gc> = (T::Of<'gc>, U::Of<'gc>, V::Of<'gc>, W::Of<'gc>, X::Of<'gc>);

    fn local<'a>(self) -> Self::Of<'a> {
        (self.0, self.1, self.2, self.3, self.4)
    }
}

// SAFETY: The blanket impls are safe if the implementors are.
unsafe impl<T: Bindable, U: Bindable, V: Bindable, W: Bindable, X: Bindable, Y: Bindable> Bindable
    for (T, U, V, W, X, Y)
{
    type Of<'gc> = (
        T::Of<'gc>,
        U::Of<'gc>,
        V::Of<'gc>,
        W::Of<'gc>,
        X::Of<'gc>,
        Y::Of<'gc>,
    );

    fn local<'a>(self) -> Self::Of<'a> {
        (self.0, self.1, self.2, self.3, self.4, self.5)
    }
}
