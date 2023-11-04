use oxc_span::Atom;

use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::{Agent, JsResult};
// use super::declarative_environment::Binding;
use crate::ecmascript::types::{Object, String, Value};
use crate::heap::element_array::ElementsVector;
use std::collections::HashMap;

use super::{DeclarativeEnvironment, ObjectEnvironment};

/// ### [9.1.1.4 Global Environment Records](https://tc39.es/ecma262/#sec-global-environment-records)
///
/// A Global Environment Record is used to represent the outer most scope that
/// is shared by all of the ECMAScript Script elements that are processed in a
/// common realm. A Global Environment Record provides the bindings for built-in
/// globals (clause 19), properties of the global object, and for all top-level
/// declarations (8.2.9, 8.2.11) that occur within a Script.
#[derive(Debug)]
pub struct GlobalEnvironment {
    /// ### \[\[ObjectRecord\]\]
    ///
    /// Binding object is the global object. It contains global built-in
    /// bindings as well as FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration bindings in global code for the associated realm.
    object_record: ObjectEnvironment,

    /// ### \[\[GlobalThisValue\]\]
    ///
    /// The value returned by this in global scope. Hosts may provide any
    /// ECMAScript Object value.
    pub(crate) global_this_value: Object,

    /// ### \[\[DeclarativeRecord\]\]
    ///
    /// Contains bindings for all declarations in global code for the associated
    /// realm code except for FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration bindings.
    declarative_record: DeclarativeEnvironment,

    /// ### \[\[VarNames\]\]
    ///
    /// The string names bound by FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration declarations in global code for the associated
    /// realm.
    var_names: ElementsVector,
}

impl GlobalEnvironment {
    /// ### [9.1.2.5 NewGlobalEnvironment ( G, thisValue )](https://tc39.es/ecma262/#sec-newglobalenvironment)
    ///
    /// The abstract operation NewGlobalEnvironment takes arguments G (an
    /// Object) and thisValue (an Object) and returns a Global Environment
    /// Record.
    pub(crate) fn new(agent: &mut Agent, global: Object, this_value: Object) -> GlobalEnvironment {
        // 1. Let objRec be NewObjectEnvironment(G, false, null).
        let object_record = ObjectEnvironment::new(global, false, None);

        // 2. Let dclRec be NewDeclarativeEnvironment(null).
        let declarative_record = DeclarativeEnvironment::new(None);

        // 3. Let env be a new Global Environment Record.
        GlobalEnvironment {
            // 4. Set env.[[ObjectRecord]] to objRec.
            object_record,

            // 5. Set env.[[GlobalThisValue]] to thisValue.
            global_this_value: this_value,

            // 6. Set env.[[DeclarativeRecord]] to dclRec.
            declarative_record,

            // 7. Set env.[[VarNames]] to a new empty List.
            var_names: todo!(),
            // 8. Set env.[[OuterEnv]] to null.
            // NOTE: We do not expose an outer environment, so this is implicit.
        }
        // 9. Return env.
    }
}
