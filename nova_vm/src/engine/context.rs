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
pub struct Gc<'a> {
    inner: GcToken,
    _marker: PhantomData<&'a mut GcToken>,
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
    /// SAFETY: Only one GcToken should exist at any point in time.
    ///
    /// The caller must make sure to only create a new token when a new
    /// JavaScript call stack is initialized.
    pub(crate) unsafe fn new() -> Self {
        Self
    }
}

impl ScopeToken {
    pub(crate) unsafe fn new() -> Self {
        Self
    }
}

impl Gc<'_> {
    #[inline]
    pub(crate) fn new(_: &mut GcToken) -> Self {
        Self {
            inner: GcToken,
            _marker: PhantomData,
        }
    }

    #[inline]
    pub(crate) fn reborrow(&mut self) -> Self {
        Self {
            inner: GcToken,
            _marker: PhantomData,
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
    pub(crate) fn reborrow(&self) -> Self {
        Self {
            inner: ScopeToken,
            _marker: PhantomData,
        }
    }

    pub(crate) fn print(&self) {
        println!("GC!");
    }
}
