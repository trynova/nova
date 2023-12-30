use oxc_span::Atom;

use crate::ecmascript::execution::Agent;
use crate::ecmascript::types::Object;
use std::collections::HashSet;

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
    pub(crate) declarative_record: DeclarativeEnvironment,

    /// ### \[\[VarNames\]\]
    ///
    /// The string names bound by FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration declarations in global code for the associated
    /// realm.
    // TODO: Use the Heap to set this.
    var_names: HashSet<Atom>,
}

impl GlobalEnvironment {
    /// ### [9.1.2.5 NewGlobalEnvironment ( G, thisValue )](https://tc39.es/ecma262/#sec-newglobalenvironment)
    ///
    /// The abstract operation NewGlobalEnvironment takes arguments G (an
    /// Object) and thisValue (an Object) and returns a Global Environment
    /// Record.
    pub(crate) fn new(_agent: &mut Agent, global: Object, this_value: Object) -> GlobalEnvironment {
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
            var_names: HashSet::new(),
            // 8. Set env.[[OuterEnv]] to null.
            // NOTE: We do not expose an outer environment, so this is implicit.
        }
        // 9. Return env.
    }

    /// ### [9.1.1.4.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if the
    /// argument identifier is one of the identifiers bound by the record.
    pub(crate) fn has_binding(&self, name: &str) -> bool {
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        // 2. If ! DclRec.HasBinding(N) is true, return true.
        if self.declarative_record.has_binding(name) {
            return true;
        }

        // TODO: Implement steps 3 and 4 and remove this anti-spec code.
        self.var_names.contains(name)

        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        // 4. Return ? ObjRec.HasBinding(N).
    }

    /// ### [9.1.1.4.12 HasVarDeclaration ( N )](https://tc39.es/ecma262/#sec-hasvardeclaration)
    ///
    /// The HasVarDeclaration concrete method of a Global Environment Record
    /// envRec takes argument N (a String) and returns a Boolean. It determines
    /// if the argument identifier has a binding in this record that was created
    /// using a VariableStatement or a FunctionDeclaration.
    pub(crate) fn has_var_declaration(&self, name: &str) -> bool {
        // 1. Let varDeclaredNames be envRec.[[VarNames]].
        // 2. If varDeclaredNames contains N, return true.
        // 3. Return false.
        self.var_names.contains(name)
    }

    /// ### [9.1.1.4.13 HasLexicalDeclaration ( N )](https://tc39.es/ecma262/#sec-haslexicaldeclaration)
    ///
    /// The HasLexicalDeclaration concrete method of a Global Environment Record
    /// envRec takes argument N (a String) and returns a Boolean. It determines
    /// if the argument identifier has a binding in this record that was created
    /// using a lexical declaration such as a LexicalDeclaration or a
    /// ClassDeclaration.
    pub(crate) fn has_lexical_declaration(&self, name: &str) -> bool {
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        // 2. Return ! DclRec.HasBinding(N).
        self.declarative_record.has_binding(name)
    }

    /// ### [9.1.1.4.14 HasRestrictedGlobalProperty ( N )](https://tc39.es/ecma262/#sec-hasrestrictedglobalproperty)
    ///
    /// The HasRestrictedGlobalProperty concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// the argument identifier is the name of a property of the global object
    /// that must not be shadowed by a global lexical binding.
    pub(crate) fn has_restricted_global_property(&self, _name: &str) -> bool {
        // TODO: Implement this. We just return true for testing.
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        // 4. If existingProp is undefined, return false.
        // 5. If existingProp.[[Configurable]] is true, return false.
        // 6. Return true.
        false
    }

    /// ### [9.1.1.4.17 CreateGlobalVarBinding ( N, D )](https://tc39.es/ecma262/#sec-createglobalvarbinding)
    ///
    /// The CreateGlobalVarBinding concrete method of a Global Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and returns
    /// either a normal completion containing UNUSED or a throw completion. It
    /// creates and initializes a mutable binding in the associated Object
    /// Environment Record and records the bound name in the associated
    /// \[\[VarNames]] List. If a binding already exists, it is reused and
    /// assumed to be initialized.
    pub(crate) fn create_global_var_binding(&mut self, name: Atom, is_deletable: bool) {
        // TODO: Follow steps 1-5.
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        // 4. Let extensible be ? IsExtensible(globalObject).
        // 5. If hasProperty is false and extensible is true, then
        // a. Perform ? ObjRec.CreateMutableBinding(N, D).
        // b. Perform ? ObjRec.InitializeBinding(N, undefined).

        // TODO: Remove this once steps 1-5 are implemented.
        self.declarative_record
            .create_mutable_binding(name.clone(), is_deletable);

        // 6. If envRec.[[VarNames]] does not contain N, then
        //    a. Append N to envRec.[[VarNames]].
        // NOTE: This does both of the steps because it is a set.
        self.var_names.insert(name);

        // 7. Return UNUSED.
    }
}
