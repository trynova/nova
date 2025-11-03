// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_span::Span;

use crate::{
    ecmascript::{
        builtins::{Behaviour, ECMAScriptFunctionObjectHeapData},
        execution::{Environment, PrivateEnvironment, Realm},
        scripts_and_modules::source_code::SourceCode,
        types::{OrdinaryObject, String, Value},
    },
    engine::Executable,
    heap::element_array::ElementsVector,
};

use super::Function;

#[derive(Debug, Clone)]
pub struct BoundFunctionHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) length: u8,
    /// ### \[\[BoundTargetFunction\]\]
    ///
    /// The wrapped function object.
    pub(crate) bound_target_function: Function<'a>,
    /// ### \[\[BoundThis\]\]
    ///
    /// The value that is always passed as the **this** value when calling the
    /// wrapped function.
    pub(crate) bound_this: Value<'a>,
    /// ### \[\[BoundArguments\]\]
    ///
    /// A list of values whose elements are used as the first arguments to any
    /// call to the wrapped function.
    pub(crate) bound_arguments: ElementsVector<'a>,
    pub(crate) name: Option<String<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuiltinFunctionHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) length: u8,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: Realm<'a>,
    /// #### \[\[InitialName]]
    /// A String that is the initial name of the function. It is used by
    /// 20.2.3.5 (`Function.prototype.toString()`).
    pub(crate) initial_name: Option<String<'a>>,
    pub(crate) behaviour: Behaviour,
}

impl BuiltinFunctionHeapData<'_> {
    pub(crate) const BLANK: Self = Self {
        object_index: None,
        length: 0,
        realm: Realm::from_u32(u32::MAX - 1),
        initial_name: None,
        behaviour: Behaviour::Regular(|_, _, _, _| Ok(Value::Undefined)),
    };
}

#[derive(Debug, Clone)]
pub struct BuiltinConstructorRecord<'a> {
    pub(crate) backing_object: Option<OrdinaryObject<'a>>,
    /// #### \[\[Realm]]
    /// A Realm Record that represents the realm in which the function was
    /// created.
    pub(crate) realm: Realm<'a>,
    /// ### \[\[ConstructorKind]]
    ///
    /// If the boolean is `true` then ConstructorKind is Derived, else it is
    /// Base.
    pub(crate) is_derived: bool,
    /// Stores the compiled bytecode of class field initializers.
    pub(crate) compiled_initializer_bytecode: Option<Executable<'a>>,
    /// ### \[\[Environment]]
    ///
    /// This is required for class field initializers.
    pub(crate) environment: Environment<'a>,
    /// ### \[\[PrivateEnvironment]]
    ///
    /// This is required for class field initializers.
    pub(crate) private_environment: Option<PrivateEnvironment<'a>>,
    ///  \[\[SourceText]]
    pub(crate) source_text: Span,

    /// \[\[SourceCode]]
    ///
    /// Nova specific addition: This SourceCode is where \[\[SourceText]]
    /// refers to.
    pub(crate) source_code: SourceCode<'a>,

    /// Name of the class that this constructor belongs to.
    pub(crate) class_name: String<'a>,
}

#[derive(Debug)]
pub struct ECMAScriptFunctionHeapData<'a> {
    pub(crate) object_index: Option<OrdinaryObject<'a>>,
    pub(crate) length: u8,
    pub(crate) ecmascript_function: ECMAScriptFunctionObjectHeapData<'a>,
    /// Stores the compiled bytecode of an ECMAScript function.
    pub(crate) compiled_bytecode: Option<Executable<'a>>,
    pub(crate) name: Option<String<'a>>,
}

unsafe impl Send for ECMAScriptFunctionHeapData<'_> {}
