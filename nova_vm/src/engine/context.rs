// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::marker::PhantomData;

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

/// # Access to garbage collector
///
/// Holding this token is required for garbage collection.
#[derive(Debug)]
pub struct GcScope<'a, 'b> {
    gc: GcToken,
    scope: ScopeToken,
    _gc_marker: PhantomData<&'a mut GcToken>,
    _scope_marker: PhantomData<&'b ScopeToken>,
}

/// # Access to the JavaScript call stack
///
/// Holding this token is required for JavaScript calls.
#[derive(Debug)]
pub struct Scope<'a> {
    inner: ScopeToken,
    _marker: PhantomData<&'a ScopeToken>,
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

    #[inline]
    pub fn reborrow(&mut self) -> Self {
        Self {
            gc: GcToken,
            scope: ScopeToken,
            _gc_marker: PhantomData,
            _scope_marker: PhantomData,
        }
    }

    pub(crate) fn print(&mut self) {
        println!("GC!");
    }
}

impl Scope<'_> {
    #[inline]
    pub(crate) fn new(_: &mut ScopeToken) -> Self {
        Self {
            inner: ScopeToken,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub fn reborrow(&self) -> Self {
        Self {
            inner: ScopeToken,
            _marker: PhantomData,
        }
    }

    pub(crate) fn print(&self) {
        println!("GC!");
    }
}
