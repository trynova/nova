// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod bytecode;
pub mod context;
pub mod rootable;
pub mod small_bigint;
pub mod small_f64;
pub mod small_integer;

use core::ops::ControlFlow;

pub(crate) use bytecode::*;
use context::bindable_handle;
pub use rootable::{Global, ScopableCollection, Scoped, ScopedCollection};

use crate::ecmascript::execution::{JsResult, agent::JsError};

/// Failure conditions for internal method's Try variants.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TryError<'a> {
    /// The method threw an error.
    Err(JsError<'a>),
    /// The method cannot run to completion without calling into JavaScript.
    ///
    /// > Note 1: methods can and are encouraged to delegate any JavaScript
    /// tail calls to the caller (such as getter, setter, or Proxy trap call at
    /// the end of a \[\[Get]] or \[\[Set]] method). This variant should be
    /// used when the method would need to perform additional work after the
    /// JavaScript call is done.
    ///
    /// > Note 2: Returning this error indicates that the entire operation will
    /// be rerun from start to finish in a GC-capable scope. The Try method
    /// variant must therefore be undetectable; it cannot perform mutations
    /// that would affect how the normal variant runs.
    GcError,
}
bindable_handle!(TryError);

pub fn option_into_try<'a, T: 'a>(value: Option<T>) -> TryResult<'a, T> {
    match value {
        Some(value) => TryResult::Continue(value),
        None => TryError::GcError.into(),
    }
}

/// Convert a JsResult into a TryResult.
///
/// This is useful when an abstract operation can throw errors but cannot call
/// into JavaScript, and is called from a Try method. The AO returns a JsResult
/// but the caller wants to convert it into a TryResult before returning.
pub fn js_result_into_try<'a, T: 'a>(value: JsResult<'a, T>) -> TryResult<'a, T> {
    match value {
        Ok(value) => TryResult::Continue(value),
        Err(err) => TryResult::Break(TryError::Err(err)),
    }
}

/// Convert a TryResult<T> into a JsResult of an Option<T>.
///
/// This is useful when a method that may trigger GC calls into a Try method
/// and wants to rethrow any errors and use the result if available.
pub fn try_result_into_js<'a, T: 'a>(value: TryResult<'a, T>) -> JsResult<'a, Option<T>> {
    match value {
        TryResult::Continue(value) => JsResult::Ok(Some(value)),
        TryResult::Break(TryError::GcError) => JsResult::Ok(None),
        TryResult::Break(TryError::Err(err)) => JsResult::Err(err),
    }
}

/// Convert a TryResult<T> into an Option<JsResult<T>>.
///
/// This is useful when a method that may trigger GC calls into a Try method
/// and wants to use the result if available, error or not.
pub fn try_result_into_option_js<'a, T: 'a>(value: TryResult<'a, T>) -> Option<JsResult<'a, T>> {
    match value {
        TryResult::Continue(value) => Some(JsResult::Ok(value)),
        TryResult::Break(TryError::GcError) => None,
        TryResult::Break(TryError::Err(err)) => Some(JsResult::Err(err)),
    }
}

impl<'a, T: 'a> From<JsError<'a>> for TryResult<'a, T> {
    fn from(value: JsError<'a>) -> Self {
        TryResult::Break(TryError::Err(value))
    }
}

impl<'a, T: 'a> From<TryError<'a>> for TryResult<'a, T> {
    fn from(value: TryError<'a>) -> Self {
        TryResult::Break(value)
    }
}

macro_rules! try_result_ok {
    ($self:ident) => {
        impl<'a> core::convert::From<$self<'a>> for TryResult<'a, $self<'a>> {
            fn from(value: $self<'a>) -> Self {
                TryResult::Continue(value)
            }
        }
    };
}
pub(crate) use try_result_ok;

/// Result of methods that are not allowed to call JavaScript or perform
/// garbage collection.
pub type TryResult<'a, T> = ControlFlow<TryError<'a>, T>;

#[inline]
pub fn unwrap_try<'a, T: 'a>(try_result: TryResult<'a, T>) -> T {
    match try_result {
        TryResult::Continue(t) => t,
        TryResult::Break(_) => unreachable!(),
    }
}
