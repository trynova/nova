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
    #[inline]
    pub(crate) fn from_gc(_: &GcScope<'a, 'b>) -> Self {
        Self {
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }
}
