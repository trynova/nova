// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builtins::{Behaviour, ECMAScriptFunctionObjectHeapData},
        execution::RealmIdentifier,
        types::{OrdinaryObject, String, Value},
    },
    heap::element_array::ElementsVector,
};

use super::Function;

#[derive(Debug, Clone)]
pub struct BoundFunctionHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(crate) length: u8,
    /// ### \[\[BoundTargetFunction\]\]
    ///
    /// The wrapped function object.
    pub(crate) bound_target_function: Function<'gen>,
    /// ### \[\[BoundThis\]\]
    ///
    /// The value that is always passed as the **this** value when calling the
    /// wrapped function.
    pub(crate) bound_this: Value<'gen>,
    /// ### \[\[BoundArguments\]\]
    ///
    /// A list of values whose elements are used as the first arguments to any
    /// call to the wrapped function.
    pub(crate) bound_arguments: ElementsVector<'gen>,
    pub(crate) name: Option<String<'gen>>,
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(crate) length: u8,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: RealmIdentifier<'gen>,
    /// #### \[\[InitialName]]
    /// A String that is the initial name of the function. It is used by
    /// 20.2.3.5 (`Function.prototype.toString()`).
    pub(crate) initial_name: Option<String<'gen>>,
    pub(crate) behaviour: Behaviour,
}

#[derive(Debug)]
pub struct ECMAScriptFunctionHeapData<'gen> {
    pub(crate) object_index: Option<OrdinaryObject<'gen>>,
    pub(crate) length: u8,
    pub(crate) ecmascript_function: ECMAScriptFunctionObjectHeapData<'gen>,
    pub(crate) name: Option<String<'gen>>,
}

unsafe impl Send for ECMAScriptFunctionHeapData<'_> {}
