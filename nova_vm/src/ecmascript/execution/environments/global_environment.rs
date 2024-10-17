// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashSet;

use crate::ecmascript::abstract_operations::operations_on_objects::{
    define_property_or_throw, has_own_property, set,
};
use crate::ecmascript::abstract_operations::testing_and_comparison::is_extensible;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::types::{Object, PropertyDescriptor, PropertyKey, String, Value};
use crate::ecmascript::{execution::Agent, types::InternalMethods};
use crate::heap::{CompactionLists, HeapMarkAndSweep, WorkQueues};

use super::{
    DeclarativeEnvironment, DeclarativeEnvironmentIndex, GlobalEnvironmentIndex, ObjectEnvironment,
    ObjectEnvironmentIndex,
};

/// ### [9.1.1.4 Global Environment Records](https://tc39.es/ecma262/#sec-global-environment-records)
///
/// A Global Environment Record is used to represent the outer most scope that
/// is shared by all of the ECMAScript Script elements that are processed in a
/// common realm. A Global Environment Record provides the bindings for
/// built-in globals (clause 19), properties of the global object, and for all
/// top-level declarations (8.2.9, 8.2.11) that occur within a Script.
#[derive(Debug, Clone)]
pub struct GlobalEnvironment {
    /// ### \[\[ObjectRecord\]\]
    ///
    /// Binding object is the global object. It contains global built-in
    /// bindings as well as FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration bindings in global code for the associated realm.
    pub(crate) object_record: ObjectEnvironmentIndex,

    /// ### \[\[GlobalThisValue\]\]
    ///
    /// The value returned by this in global scope. Hosts may provide any
    /// ECMAScript Object value.
    pub(crate) global_this_value: Object,

    /// ### \[\[DeclarativeRecord\]\]
    ///
    /// Contains bindings for all declarations in global code for the
    /// associated realm code except for FunctionDeclaration,
    /// GeneratorDeclaration, AsyncFunctionDeclaration,
    /// AsyncGeneratorDeclaration, and VariableDeclaration bindings.
    pub(crate) declarative_record: DeclarativeEnvironmentIndex,

    /// ### \[\[VarNames\]\]
    ///
    /// The string names bound by FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration declarations in global code for the associated
    /// realm.
    // TODO: Use the Heap to set this.
    var_names: AHashSet<String>,
}

impl GlobalEnvironment {
    /// ### [9.1.2.5 NewGlobalEnvironment ( G, thisValue )](https://tc39.es/ecma262/#sec-newglobalenvironment)
    ///
    /// The abstract operation NewGlobalEnvironment takes arguments G (an
    /// Object) and thisValue (an Object) and returns a Global Environment
    /// Record.
    pub(crate) fn new(agent: &mut Agent, global: Object, this_value: Object) -> GlobalEnvironment {
        // 1. Let objRec be NewObjectEnvironment(G, false, null).
        let obj_rec = ObjectEnvironment::new(global, false, None);
        agent.heap.environments.object.push(Some(obj_rec));
        let object_record = ObjectEnvironmentIndex::last(&agent.heap.environments.object);

        // 2. Let dclRec be NewDeclarativeEnvironment(null).
        let dcl_rec = DeclarativeEnvironment::new(None);
        agent.heap.environments.declarative.push(Some(dcl_rec));
        let declarative_record =
            DeclarativeEnvironmentIndex::last(&agent.heap.environments.declarative);

        // 3. Let env be a new Global Environment Record.
        GlobalEnvironment {
            // 4. Set env.[[ObjectRecord]] to objRec.
            object_record,

            // 5. Set env.[[GlobalThisValue]] to thisValue.
            global_this_value: this_value,

            // 6. Set env.[[DeclarativeRecord]] to dclRec.
            declarative_record,

            // 7. Set env.[[VarNames]] to a new empty List.
            var_names: AHashSet::default(),
            // 8. Set env.[[OuterEnv]] to null.
            // NOTE: We do not expose an outer environment, so this is implicit.
        }
        // 9. Return env.
    }
}

impl HeapMarkAndSweep for GlobalEnvironment {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_record,
            global_this_value,
            declarative_record,
            var_names,
        } = self;
        declarative_record.mark_values(queues);
        global_this_value.mark_values(queues);
        object_record.mark_values(queues);
        for ele in var_names {
            ele.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_record,
            global_this_value,
            declarative_record,
            var_names,
        } = self;
        declarative_record.sweep_values(compactions);
        global_this_value.sweep_values(compactions);
        object_record.sweep_values(compactions);
        for key in var_names.clone() {
            let mut new_key = key;
            new_key.sweep_values(compactions);
            if key != new_key {
                self.var_names.remove(&key);
                self.var_names.insert(new_key);
            }
        }
    }
}

impl GlobalEnvironmentIndex {
    /// ### [9.1.1.4.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if the
    /// argument identifier is one of the identifiers bound by the record.
    pub(crate) fn has_binding(self, agent: &mut Agent, name: String) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        // 2. If ! DclRec.HasBinding(N) is true, return true.
        if env_rec.declarative_record.has_binding(agent, name) {
            return Ok(true);
        }

        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 4. Return ? ObjRec.HasBinding(N).
        obj_rec.has_binding(agent, name)
    }

    /// ### [9.1.1.4.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-global-environment-records-createmutablebinding-n-d)
    ///
    /// The CreateMutableBinding concrete method of a Global Environment Record
    /// envRec takes arguments N (a String) and D (a Boolean) and returns
    /// either a normal completion containing UNUSED or a throw completion. It
    /// creates a new mutable binding for the name N that is uninitialized. The
    /// binding is created in the associated DeclarativeRecord. A binding for N
    /// must not already exist in the DeclarativeRecord. If D is true, the new
    /// binding is marked as being subject to deletion.
    pub(crate) fn create_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, throw a TypeError exception.
        if dcl_rec.has_binding(agent, name) {
            let error_message =
                format!("Redeclaration of global binding '{}'.", name.as_str(agent));
            Err(agent.throw_exception(ExceptionType::TypeError, error_message))
        } else {
            // 3. Return ! DclRec.CreateMutableBinding(N, D).
            dcl_rec.create_mutable_binding(agent, name, is_deletable);
            Ok(())
        }
    }

    /// ### [9.1.1.4.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-global-environment-records-createimmutablebinding-n-s)
    ///
    /// The CreateImmutableBinding concrete method of a Global Environment
    /// Record envRec takes arguments N (a String) and S (a Boolean) and
    /// returns either a normal completion containing UNUSED or a throw
    /// completion. It creates a new immutable binding for the name N that is
    /// uninitialized. A binding must not already exist in this Environment
    /// Record for N. If S is true, the new binding is marked as a strict
    /// binding.
    pub(crate) fn create_immutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, throw a TypeError exception.
        if dcl_rec.has_binding(agent, name) {
            let error_message =
                format!("Redeclaration of global binding '{}'.", name.as_str(agent));
            Err(agent.throw_exception(ExceptionType::TypeError, error_message))
        } else {
            // 3. Return ! DclRec.CreateImmutableBinding(N, S).
            dcl_rec.create_immutable_binding(agent, name, is_strict);
            Ok(())
        }
    }

    /// ### [9.1.1.4.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-global-environment-records-initializebinding-n-v)
    ///
    /// The InitializeBinding concrete method of a Global Environment Record
    /// envRec takes arguments N (a String) and V (an ECMAScript language
    /// value) and returns either a normal completion containing UNUSED or a
    /// throw completion. It is used to set the bound value of the current
    /// binding of the identifier whose name is N to the value V. An
    /// uninitialized binding for N must already exist.
    pub(crate) fn initialize_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ! DclRec.InitializeBinding(N, V).
            dcl_rec.initialize_binding(agent, name, value);
            Ok(())
        } else {
            // 3. Assert: If the binding exists, it must be in the Object Environment Record.
            // 4. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 5. Return ? ObjRec.InitializeBinding(N, V).
            obj_rec.initialize_binding(agent, name, value)
        }
    }

    /// ### [9.1.1.4.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-global-environment-records-setmutablebinding-n-v-s)
    ///
    /// The SetMutableBinding concrete method of a Global Environment Record
    /// envRec takes arguments N (a String), V (an ECMAScript language value),
    /// and S (a Boolean) and returns either a normal completion containing
    /// UNUSED or a throw completion. It attempts to change the bound value of
    /// the current binding of the identifier whose name is N to the value V.
    /// If the binding is an immutable binding and S is true, a TypeError is
    /// thrown. A property named N normally already exists but if it does not
    /// or is not currently writable, error handling is determined by S.
    pub(crate) fn set_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        is_strict: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ? DclRec.SetMutableBinding(N, V, S).
            dcl_rec.set_mutable_binding(agent, name, value, is_strict)
        } else {
            // 3. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 4. Return ? ObjRec.SetMutableBinding(N, V, S).
            obj_rec.set_mutable_binding(agent, name, value, is_strict)
        }
    }

    /// ### [9.1.1.4.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-global-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of a Global Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its bound identifier whose
    /// name is N. If the binding is an uninitialized binding throw a
    /// ReferenceError exception. A property named N normally already exists
    /// but if it does not or is not currently writable, error handling is
    /// determined by S.
    pub(crate) fn get_binding_value(
        self,
        agent: &mut Agent,
        n: String,
        s: bool,
    ) -> JsResult<Value> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, n) {
            // a. Return ? DclRec.GetBindingValue(N, S).
            dcl_rec.get_binding_value(agent, n, s)
        } else {
            // 3. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 4. Return ? ObjRec.GetBindingValue(N, S).
            obj_rec.get_binding_value(agent, n, s)
        }
    }

    /// ### [9.1.1.4.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It can only delete bindings
    /// that have been explicitly designated as being subject to deletion.
    pub(crate) fn delete_binding(self, agent: &mut Agent, name: String) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ! DclRec.DeleteBinding(N).
            return Ok(dcl_rec.delete_binding(agent, name));
        }
        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 4. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = agent[obj_rec].binding_object;
        // 5. Let existingProp be ? HasOwnProperty(globalObject, N).
        let n = PropertyKey::from(name);
        let existing_prop = has_own_property(agent, global_object, n)?;
        // 6. If existingProp is true, then
        if existing_prop {
            // a. Let status be ? ObjRec.DeleteBinding(N).
            let status = obj_rec.delete_binding(agent, name)?;
            // b. If status is true and envRec.[[VarNames]] contains N, then
            if status {
                let env_rec = &mut agent[self];
                if env_rec.var_names.contains(&name) {
                    // i. Remove N from envRec.[[VarNames]].
                    env_rec.var_names.remove(&name);
                }
            }
            // c. Return status.
            Ok(status)
        } else {
            // 7. Return true.
            Ok(true)
        }
    }

    /// ### [9.1.1.4.8 HasThisBinding ( )](https://tc39.es/ecma262/#sec-global-environment-records-hasthisbinding)
    ///
    /// The HasThisBinding concrete method of a Global Environment Record
    /// envRec takes no arguments and returns true.
    pub(crate) fn has_this_binding(self) -> bool {
        // 1. Return true.
        true
        // NOTE
        // Global Environment Records always provide a this binding.
    }

    /// ### [9.1.1.4.9 HasSuperBinding ( )](https://tc39.es/ecma262/#sec-global-environment-records-hassuperbinding)
    ///
    /// The HasSuperBinding concrete method of a Global Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_super_binding(self) -> bool {
        // 1. Return false.
        false
        // NOTE
        // Global Environment Records do not provide a super binding.
    }

    /// ### [9.1.1.4.10 WithBaseObject ( )](https://tc39.es/ecma262/#sec-global-environment-records-withbaseobject)
    ///
    /// The WithBaseObject concrete method of a Global Environment Record
    /// envRec takes no arguments and returns undefined.
    pub(crate) fn with_base_object(self) -> Option<Object> {
        // 1. Return undefined.
        None
    }

    /// ### [9.1.1.4.11 GetThisBinding ( )](https://tc39.es/ecma262/#sec-global-environment-records-getthisbinding)
    ///
    /// The GetThisBinding concrete method of a Global Environment Record
    /// envRec takes no arguments and returns a normal completion containing an
    /// Object.
    pub(crate) fn get_this_binding(self, agent: &Agent) -> Object {
        let env_rec = &agent[self];
        // 1. Return envRec.[[GlobalThisValue]].
        env_rec.global_this_value
    }

    /// ### [9.1.1.4.12 HasVarDeclaration ( N )](https://tc39.es/ecma262/#sec-hasvardeclaration)
    ///
    /// The HasVarDeclaration concrete method of a Global Environment Record
    /// envRec takes argument N (a String) and returns a Boolean. It determines
    /// if the argument identifier has a binding in this record that was
    /// created
    /// using a VariableStatement or a FunctionDeclaration.
    pub(crate) fn has_var_declaration(self, agent: &Agent, name: String) -> bool {
        let env_rec = &agent[self];
        // 1. Let varDeclaredNames be envRec.[[VarNames]].
        let var_declared_names = &env_rec.var_names;
        // 2. If varDeclaredNames contains N, return true.
        // 3. Return false.
        var_declared_names.contains(&name)
    }

    /// ### [9.1.1.4.13 HasLexicalDeclaration ( N )](https://tc39.es/ecma262/#sec-haslexicaldeclaration)
    ///
    /// The HasLexicalDeclaration concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns a Boolean. It
    /// determines if the argument identifier has a binding in this record that
    /// was created using a lexical declaration such as a LexicalDeclaration or
    /// a ClassDeclaration.
    pub(crate) fn has_lexical_declaration(self, agent: &Agent, name: String) -> bool {
        let env_rec = &agent[self];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. Return ! DclRec.HasBinding(N).
        dcl_rec.has_binding(agent, name)
    }

    /// ### [9.1.1.4.14 HasRestrictedGlobalProperty ( N )](https://tc39.es/ecma262/#sec-hasrestrictedglobalproperty)
    ///
    /// The HasRestrictedGlobalProperty concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// the argument identifier is the name of a property of the global object
    /// that must not be shadowed by a global lexical binding.
    pub(crate) fn has_restricted_global_property(
        self,
        agent: &mut Agent,
        name: String,
    ) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = agent[obj_rec].binding_object;
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let n = PropertyKey::from(name);
        let existing_prop = global_object.internal_get_own_property(agent, n)?;
        let Some(existing_prop) = existing_prop else {
            // 4. If existingProp is undefined, return false.
            return Ok(false);
        };
        // 5. If existingProp.[[Configurable]] is true, return false.
        // 6. Return true.
        Ok(existing_prop.configurable != Some(true))
    }

    /// ### [9.1.1.4.15 CanDeclareGlobalVar ( N )](https://tc39.es/ecma262/#sec-candeclareglobalvar)
    ///
    /// The CanDeclareGlobalVar concrete method of a Global Environment Record
    /// envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// a corresponding CreateGlobalVarBinding call would succeed if called for
    /// the same argument N. Redundant var declarations and var declarations
    /// for pre-existing global object properties are allowed.
    pub(crate) fn can_declare_global_var(self, agent: &mut Agent, name: String) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = agent[obj_rec].binding_object;
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        let n = PropertyKey::from(name);
        let has_property = has_own_property(agent, global_object, n)?;
        // 4. If hasProperty is true, return true.
        if has_property {
            Ok(true)
        } else {
            // 5. Return ? IsExtensible(globalObject).
            is_extensible(agent, global_object)
        }
    }

    /// ### [9.1.1.4.16 CanDeclareGlobalFunction ( N )](https://tc39.es/ecma262/#sec-candeclareglobalfunction)
    ///
    /// The CanDeclareGlobalFunction concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// a corresponding CreateGlobalFunctionBinding call would succeed if
    /// called for the same argument N.
    pub(crate) fn can_declare_global_function(
        self,
        agent: &mut Agent,
        name: String,
    ) -> JsResult<bool> {
        let env_rec = &agent[self];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = agent[obj_rec].binding_object;
        let n = PropertyKey::from(name);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let existing_prop = global_object.internal_get_own_property(agent, n)?;
        // 4. If existingProp is undefined, return ? IsExtensible(globalObject).
        let Some(existing_prop) = existing_prop else {
            return is_extensible(agent, global_object);
        };
        // 5. If existingProp.[[Configurable]] is true, return true.
        if existing_prop.configurable == Some(true)
            || existing_prop.is_data_descriptor()
                && existing_prop.writable == Some(true)
                && existing_prop.enumerable == Some(true)
        {
            // 6. If IsDataDescriptor(existingProp) is true and existingProp has attribute values { [[Writable]]: true, [[Enumerable]]: true }, true.
            Ok(true)
        } else {
            // 7. Return false.
            Ok(false)
        }
    }

    /// ### [9.1.1.4.17 CreateGlobalVarBinding ( N, D )](https://tc39.es/ecma262/#sec-createglobalvarbinding)
    ///
    /// The CreateGlobalVarBinding concrete method of a Global Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and
    /// returns either a normal completion containing UNUSED or a throw
    /// completion. It creates and initializes a mutable binding in the
    /// associated Object Environment Record and records the bound name in the
    /// associated \[\[VarNames]] List. If a binding already exists, it is
    /// reused and assumed to be initialized.
    pub(crate) fn create_global_var_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = agent[obj_rec].binding_object;
        let n = PropertyKey::from(name);
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        let has_property = has_own_property(agent, global_object, n)?;
        // 4. Let extensible be ? IsExtensible(globalObject).
        let extensible = is_extensible(agent, global_object).unwrap();
        // 5. If hasProperty is false and extensible is true, then
        if !has_property && extensible {
            // a. Perform ? ObjRec.CreateMutableBinding(N, D).
            obj_rec.create_mutable_binding(agent, name, is_deletable)?;
            // b. Perform ? ObjRec.InitializeBinding(N, undefined).
            obj_rec.initialize_binding(agent, name, Value::Undefined)?;
        }

        // 6. If envRec.[[VarNames]] does not contain N, then
        //    a. Append N to envRec.[[VarNames]].
        let env_rec = &mut agent[self];
        env_rec.var_names.insert(name);

        // 7. Return UNUSED.
        Ok(())
    }

    /// ### [9.1.1.4.18 CreateGlobalFunctionBinding ( N, V, D )](https://tc39.es/ecma262/#sec-createglobalfunctionbinding)
    ///
    /// The CreateGlobalFunctionBinding concrete method of a Global Environment
    /// Record envRec takes arguments N (a String), V (an ECMAScript language
    /// value), and D (a Boolean) and returns either a normal completion
    /// containing UNUSED or a throw completion. It creates and initializes a
    /// mutable binding in the associated Object Environment Record and records
    /// the bound name in the associated [[VarNames]] List. If a binding
    /// already exists, it is replaced.
    pub(crate) fn create_global_function_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        d: bool,
    ) -> JsResult<()> {
        let env_rec = &agent[self];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = agent[obj_rec].binding_object;
        let n = PropertyKey::from(name);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let existing_prop = global_object.internal_get_own_property(agent, n)?;
        // 4. If existingProp is undefined or existingProp.[[Configurable]] is true, then
        let desc = if existing_prop.is_none() || existing_prop.unwrap().configurable == Some(true) {
            // a. Let desc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }.
            PropertyDescriptor {
                value: Some(value),
                writable: Some(true),
                get: None,
                set: None,
                enumerable: Some(true),
                configurable: Some(d),
            }
        } else {
            // 5. Else,
            // a. Let desc be the PropertyDescriptor { [[Value]]: V }.
            PropertyDescriptor {
                value: Some(value),
                writable: None,
                get: None,
                set: None,
                enumerable: None,
                configurable: None,
            }
        };
        // 6. Perform ? DefinePropertyOrThrow(globalObject, N, desc).
        define_property_or_throw(agent, global_object, n, desc)?;
        // 7. Perform ? Set(globalObject, N, V, false).
        set(agent, global_object, n, value, false)?;
        // 8. If envRec.[[VarNames]] does not contain N, then
        // a. Append N to envRec.[[VarNames]].
        let env_rec = &mut agent[self];
        env_rec.var_names.insert(name);
        // 9. Return UNUSED.
        Ok(())
        // NOTE
        // Global function declarations are always represented as own
        // properties of the global object. If possible, an existing own
        // property is reconfigured to have a standard set of attribute values.
        // Step 7 is equivalent to what calling the InitializeBinding concrete
        // method would do and if globalObject is a Proxy will produce the same
        // sequence of Proxy trap calls.
    }
}

impl HeapMarkAndSweep for GlobalEnvironmentIndex {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.global_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let self_index = self.into_u32_index();
        *self = Self::from_u32_index(
            self_index
                - compactions
                    .global_environments
                    .get_shift_for_index(self_index),
        );
    }
}
