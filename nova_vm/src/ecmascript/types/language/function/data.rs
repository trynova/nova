// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_span::Span;

use crate::{
    ecmascript::{
        builtins::{Behaviour, ECMAScriptFunctionObjectHeapData},
        execution::{EnvironmentIndex, PrivateEnvironmentIndex, RealmIdentifier},
        scripts_and_modules::source_code::SourceCode,
        types::{OrdinaryObject, String, Value},
    },
    engine::Executable,
    heap::element_array::ElementsVector,
};

use super::Function;

#[derive(Debug, Clone)]
pub struct BoundFunctionHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) length: u8,
    /// ### \[\[BoundTargetFunction\]\]
    ///
    /// The wrapped function object.
    pub(crate) bound_target_function: Function,
    /// ### \[\[BoundThis\]\]
    ///
    /// The value that is always passed as the **this** value when calling the
    /// wrapped function.
    pub(crate) bound_this: Value,
    /// ### \[\[BoundArguments\]\]
    ///
    /// A list of values whose elements are used as the first arguments to any
    /// call to the wrapped function.
    pub(crate) bound_arguments: ElementsVector,
    pub(crate) name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuiltinFunctionHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) length: u8,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: RealmIdentifier,
    /// #### \[\[InitialName]]
    /// A String that is the initial name of the function. It is used by
    /// 20.2.3.5 (`Function.prototype.toString()`).
    pub(crate) initial_name: Option<String>,
    pub(crate) behaviour: Behaviour,
}

#[derive(Debug, Clone)]
pub struct BuiltinConstructorHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: RealmIdentifier,
    /// ### \[\[ConstructorKind]]
    ///
    /// If the boolean is `true` then ConstructorKind is Derived, else it is
    /// Base.
    pub(crate) is_derived: bool,
    /// Stores the compiled bytecode of class field initializers.
    pub(crate) compiled_initializer_bytecode: Option<Executable>,
    /// ### \[\[Environment]]
    ///
    /// This is required for class field initializers.
    pub(crate) environment: EnvironmentIndex,
    /// ### \[\[PrivateEnvironment]]
    ///
    /// This is required for class field initializers.
    pub(crate) private_environment: Option<PrivateEnvironmentIndex>,
    ///  \[\[SourceText]]
    pub(crate) source_text: Span,

    /// \[\[SourceCode]]
    ///
    /// Nova specific addition: This SourceCode is where \[\[SourceText]]
    /// refers to.
    pub(crate) source_code: SourceCode,
}

#[derive(Debug)]
pub struct ECMAScriptFunctionHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    pub(crate) length: u8,
    pub(crate) ecmascript_function: ECMAScriptFunctionObjectHeapData,
    /// Stores the compiled bytecode of an ECMAScript function.
    pub(crate) compiled_bytecode: Option<Executable>,
    pub(crate) name: Option<String>,
}

unsafe impl Send for ECMAScriptFunctionHeapData {}
