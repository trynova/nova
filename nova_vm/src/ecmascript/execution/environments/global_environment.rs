// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::ControlFlow;

use ahash::AHashSet;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                define_property_or_throw, has_own_property, set, try_define_property_or_throw,
                try_has_own_property, try_set,
            },
            testing_and_comparison::{is_extensible, try_is_extensible},
        },
        builtins::ordinary::caches::PropertyLookupCache,
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, TryError, TryResult, js_result_into_try},
            environments::{
                DeclarativeEnvironment, DeclarativeEnvironmentRecord, GlobalEnvironment,
                ObjectEnvironment, ObjectEnvironmentRecord,
            },
        },
        types::{
            InternalMethods, Object, PropertyDescriptor, PropertyKey, SetResult, String, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::TryHasBindingContinue;

/// ### [9.1.1.4 Global Environment Records](https://tc39.es/ecma262/#sec-global-environment-records)
///
/// A Global Environment Record is used to represent the outer most scope that
/// is shared by all of the ECMAScript Script elements that are processed in a
/// common realm. A Global Environment Record provides the bindings for
/// built-in globals (clause 19), properties of the global object, and for all
/// top-level declarations (8.2.9, 8.2.11) that occur within a Script.
#[derive(Debug, Clone)]
pub struct GlobalEnvironmentRecord {
    /// ### \[\[ObjectRecord\]\]
    ///
    /// Binding object is the global object. It contains global built-in
    /// bindings as well as FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration bindings in global code for the associated realm.
    object_record: ObjectEnvironment<'static>,

    /// ### \[\[GlobalThisValue\]\]
    ///
    /// The value returned by this in global scope. Hosts may provide any
    /// ECMAScript Object value.
    global_this_value: Object<'static>,

    /// ### \[\[DeclarativeRecord\]\]
    ///
    /// Contains bindings for all declarations in global code for the
    /// associated realm code except for FunctionDeclaration,
    /// GeneratorDeclaration, AsyncFunctionDeclaration,
    /// AsyncGeneratorDeclaration, and VariableDeclaration bindings.
    declarative_record: DeclarativeEnvironment<'static>,

    /// ### \[\[VarNames\]\]
    ///
    /// The string names bound by FunctionDeclaration, GeneratorDeclaration,
    /// AsyncFunctionDeclaration, AsyncGeneratorDeclaration, and
    /// VariableDeclaration declarations in global code for the associated
    /// realm.
    // TODO: Use the Heap to set this.
    var_names: AHashSet<String<'static>>,
}

impl HeapMarkAndSweep for GlobalEnvironmentRecord {
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

/// ### [9.1.2.5 NewGlobalEnvironment ( G, thisValue )](https://tc39.es/ecma262/#sec-newglobalenvironment)
///
/// The abstract operation NewGlobalEnvironment takes arguments G (an
/// Object) and thisValue (an Object) and returns a Global Environment
/// Record.
pub(crate) fn new_global_environment<'a>(
    agent: &mut Agent,
    global: Object,
    this_value: Object,
    gc: NoGcScope<'a, '_>,
) -> GlobalEnvironment<'a> {
    // 1. Let objRec be NewObjectEnvironment(G, false, null).
    let obj_rec = ObjectEnvironmentRecord::new(global, false, None);
    // 2. Let dclRec be NewDeclarativeEnvironment(null).
    let dcl_rec = DeclarativeEnvironmentRecord::new(None);
    agent.heap.alloc_counter += core::mem::size_of::<Option<ObjectEnvironmentRecord>>()
        + core::mem::size_of::<Option<DeclarativeEnvironmentRecord>>();
    let (object_record, declarative_record) = agent
        .heap
        .environments
        .push_object_environment(obj_rec, dcl_rec, gc);

    // 3. Let env be a new Global Environment Record.
    agent.heap.alloc_counter += core::mem::size_of::<Option<GlobalEnvironmentRecord>>();
    agent.heap.environments.push_global_environment(
        GlobalEnvironmentRecord {
            // 4. Set env.[[ObjectRecord]] to objRec.
            object_record: object_record.unbind(),

            // 5. Set env.[[GlobalThisValue]] to thisValue.
            global_this_value: this_value.unbind(),

            // 6. Set env.[[DeclarativeRecord]] to dclRec.
            declarative_record: declarative_record.unbind(),

            // 7. Set env.[[VarNames]] to a new empty List.
            var_names: AHashSet::default(),
            // 8. Set env.[[OuterEnv]] to null.
            // NOTE: We do not expose an outer environment, so this is implicit.
        },
        gc,
    )
    // 9. Return env.
}

impl<'e> GlobalEnvironment<'e> {
    pub(crate) fn get_binding_object(self, agent: &Agent) -> Object<'e> {
        agent[self].object_record.get_binding_object(agent)
    }

    /// ### Try [9.1.1.4.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if the
    /// argument identifier is one of the identifiers bound by the record.
    pub(crate) fn try_has_binding<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        cache: Option<PropertyLookupCache>,
        gc: NoGcScope<'gc, '_>,
    ) -> ControlFlow<TryError<'gc>, TryHasBindingContinue<'gc>> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        // 2. If ! DclRec.HasBinding(N) is true, return true.
        if env_rec.declarative_record.has_binding(agent, name) {
            return TryHasBindingContinue::Result(true).into();
        }

        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 4. Return ? ObjRec.HasBinding(N).
        obj_rec.try_has_binding(agent, name, cache, gc)
    }

    /// ### [9.1.1.4.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if the
    /// argument identifier is one of the identifiers bound by the record.
    pub(crate) fn has_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let env = self.bind(gc.nogc());
        let name = name.bind(gc.nogc());
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        // 2. If ! DclRec.HasBinding(N) is true, return true.
        if env_rec.declarative_record.has_binding(agent, name) {
            return Ok(true);
        }

        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record;
        // 4. Return ? ObjRec.HasBinding(N).
        obj_rec.has_binding(agent, name.unbind(), gc)
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
    pub(crate) fn create_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, throw a TypeError exception.
        if dcl_rec.has_binding(agent, name) {
            let error_message = format!(
                "Redeclaration of global binding '{}'.",
                name.to_string_lossy(agent)
            );
            Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc))
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
    pub(crate) fn create_immutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, throw a TypeError exception.
        if dcl_rec.has_binding(agent, name) {
            let error_message = format!(
                "Redeclaration of global binding '{}'.",
                name.to_string_lossy(agent)
            );
            Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc))
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
    pub(crate) fn try_initialize_binding<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        cache: Option<PropertyLookupCache>,
        value: Value,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ! DclRec.InitializeBinding(N, V).
            dcl_rec.initialize_binding(agent, name, value);
            SetResult::Done.into()
        } else {
            // 3. Assert: If the binding exists, it must be in the Object Environment Record.
            // 4. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 5. Return ? ObjRec.InitializeBinding(N, V).
            obj_rec.try_initialize_binding(agent, name, cache, value, gc)
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
    pub(crate) fn initialize_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        cache: Option<PropertyLookupCache>,
        value: Value,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let nogc = gc.nogc();
        let env = self.bind(nogc);
        let name = name.bind(nogc);
        let cache = cache.bind(nogc);
        let value = value.bind(nogc);
        let env_rec = &agent[env];
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
            obj_rec.initialize_binding(agent, name.unbind(), cache.unbind(), value.unbind(), gc)
        }
    }

    /// ### Try [9.1.1.4.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-global-environment-records-setmutablebinding-n-v-s)
    ///
    /// The SetMutableBinding concrete method of a Global Environment Record
    /// envRec takes arguments N (a String), V (an ECMAScript language value),
    /// and S (a Boolean) and returns either a normal completion containing
    /// UNUSED or a throw completion. It attempts to change the bound value of
    /// the current binding of the identifier whose name is N to the value V.
    /// If the binding is an immutable binding and S is true, a TypeError is
    /// thrown. A property named N normally already exists but if it does not
    /// or is not currently writable, error handling is determined by S.
    pub(crate) fn try_set_mutable_binding<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        cache: Option<PropertyLookupCache>,
        value: Value,
        is_strict: bool,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, SetResult<'gc>> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ? DclRec.SetMutableBinding(N, V, S).
            js_result_into_try(
                dcl_rec
                    .set_mutable_binding(agent, name, value, is_strict, gc)
                    .map(|_| SetResult::Done),
            )
        } else {
            // 3. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 4. Return ? ObjRec.SetMutableBinding(N, V, S).
            obj_rec.try_set_mutable_binding(agent, name, cache, value, is_strict, gc)
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
    pub(crate) fn set_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        cache: Option<PropertyLookupCache>,
        value: Value,
        is_strict: bool,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let nogc = gc.nogc();
        let env = self.bind(nogc);
        let name = name.bind(nogc);
        let cache = cache.bind(nogc);
        let value = value.bind(nogc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ? DclRec.SetMutableBinding(N, V, S).
            dcl_rec.set_mutable_binding(
                agent,
                name.unbind(),
                value.unbind(),
                is_strict,
                gc.into_nogc(),
            )
        } else {
            // 3. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 4. Return ? ObjRec.SetMutableBinding(N, V, S).
            obj_rec.set_mutable_binding(
                agent,
                name.unbind(),
                cache.unbind(),
                value.unbind(),
                is_strict,
                gc,
            )
        }
    }

    /// ### Try [9.1.1.4.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-global-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of a Global Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its bound identifier whose
    /// name is N. If the binding is an uninitialized binding throw a
    /// ReferenceError exception. A property named N normally already exists
    /// but if it does not or is not currently writable, error handling is
    /// determined by S.
    pub(crate) fn try_get_binding_value(
        self,
        agent: &mut Agent,
        n: String,
        cache: Option<PropertyLookupCache>,
        s: bool,
        gc: NoGcScope<'e, '_>,
    ) -> TryResult<'e, Value<'e>> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, n) {
            // a. Return ? DclRec.GetBindingValue(N, S).
            js_result_into_try(dcl_rec.get_binding_value(agent, n, s, gc))
        } else {
            // 3. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 4. Return ? ObjRec.GetBindingValue(N, S).
            obj_rec.try_get_binding_value(agent, n, cache, s, gc)
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
    pub(crate) fn get_binding_value<'a>(
        self,
        agent: &mut Agent,
        n: String,
        s: bool,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        let env = self.bind(gc.nogc());
        let n = n.bind(gc.nogc());
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, n) {
            // a. Return ? DclRec.GetBindingValue(N, S).
            dcl_rec.get_binding_value(agent, n.unbind(), s, gc.into_nogc())
        } else {
            // 3. Let ObjRec be envRec.[[ObjectRecord]].
            let obj_rec = env_rec.object_record;
            // 4. Return ? ObjRec.GetBindingValue(N, S).
            obj_rec.get_binding_value(agent, n.unbind(), s, gc)
        }
    }

    /// ### Try [9.1.1.4.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It can only delete bindings
    /// that have been explicitly designated as being subject to deletion.
    pub(crate) fn try_delete_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<'a, bool> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ! DclRec.DeleteBinding(N).
            return TryResult::Continue(dcl_rec.delete_binding(agent, name));
        }
        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc);
        // 4. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        // 5. Let existingProp be ? HasOwnProperty(globalObject, N).
        let n = PropertyKey::from(name);
        let existing_prop = try_has_own_property(agent, global_object, n, gc)?;
        // 6. If existingProp is true, then
        if existing_prop {
            // a. Let status be ? ObjRec.DeleteBinding(N).
            let status = obj_rec.try_delete_binding(agent, name, gc)?;
            // b. If status is true and envRec.[[VarNames]] contains N, then
            if status {
                let env_rec = &mut agent[env];
                if env_rec.var_names.contains(&name) {
                    // i. Remove N from envRec.[[VarNames]].
                    env_rec.var_names.remove(&name.unbind());
                }
            }
            // c. Return status.
            TryResult::Continue(status)
        } else {
            // 7. Return true.
            TryResult::Continue(true)
        }
    }

    /// ### [9.1.1.4.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-global-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of a Global Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It can only delete bindings
    /// that have been explicitly designated as being subject to deletion.
    pub(crate) fn delete_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let env = self.bind(gc.nogc());
        let name = name.bind(gc.nogc());
        let env_rec = &agent[env];
        // 1. Let DclRec be envRec.[[DeclarativeRecord]].
        let dcl_rec = env_rec.declarative_record;
        // 2. If ! DclRec.HasBinding(N) is true, then
        if dcl_rec.has_binding(agent, name) {
            // a. Return ! DclRec.DeleteBinding(N).
            return Ok(dcl_rec.delete_binding(agent, name));
        }
        // 3. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc.nogc());
        // 4. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        // 5. Let existingProp be ? HasOwnProperty(globalObject, N).
        let n = PropertyKey::from(name);
        let scoped_name = name.scope(agent, gc.nogc());
        let env = env.scope(agent, gc.nogc());
        let obj_rec = obj_rec.scope(agent, gc.nogc());
        let existing_prop =
            has_own_property(agent, global_object.unbind(), n.unbind(), gc.reborrow()).unbind()?;
        // SAFETY: obj_rec not shared.
        let obj_rec = unsafe { obj_rec.take(agent).bind(gc.nogc()) };
        // 6. If existingProp is true, then
        if existing_prop {
            // a. Let status be ? ObjRec.DeleteBinding(N).
            let status = obj_rec
                .unbind()
                .delete_binding(agent, scoped_name.get(agent), gc.reborrow())
                .unbind()?;
            let env = unsafe { env.take(agent) }.bind(gc.nogc());
            // b. If status is true and envRec.[[VarNames]] contains N, then
            if status {
                let name = scoped_name.get(agent);
                let env_rec = &mut agent[env];
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
    pub(crate) fn with_base_object(self) -> Option<Object<'static>> {
        // 1. Return undefined.
        None
    }

    /// ### [9.1.1.4.11 GetThisBinding ( )](https://tc39.es/ecma262/#sec-global-environment-records-getthisbinding)
    ///
    /// The GetThisBinding concrete method of a Global Environment Record
    /// envRec takes no arguments and returns a normal completion containing an
    /// Object.
    pub(crate) fn get_this_binding(self, agent: &Agent) -> Object<'e> {
        // 1. Return envRec.[[GlobalThisValue]].
        agent[self].global_this_value
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

    /// ### Try [9.1.1.4.14 HasRestrictedGlobalProperty ( N )](https://tc39.es/ecma262/#sec-hasrestrictedglobalproperty)
    ///
    /// The HasRestrictedGlobalProperty concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// the argument identifier is the name of a property of the global object
    /// that must not be shadowed by a global lexical binding.
    pub(crate) fn try_has_restricted_global_property<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc);
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let n = PropertyKey::from(name);
        let existing_prop = global_object.try_get_own_property(agent, n, gc)?;
        let Some(existing_prop) = existing_prop else {
            // 4. If existingProp is undefined, return false.
            return TryResult::Continue(false);
        };
        // 5. If existingProp.[[Configurable]] is true, return false.
        // 6. Return true.
        TryResult::Continue(existing_prop.configurable != Some(true))
    }

    /// ### [9.1.1.4.14 HasRestrictedGlobalProperty ( N )](https://tc39.es/ecma262/#sec-hasrestrictedglobalproperty)
    ///
    /// The HasRestrictedGlobalProperty concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// the argument identifier is the name of a property of the global object
    /// that must not be shadowed by a global lexical binding.
    pub(crate) fn has_restricted_global_property<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let env = self.bind(gc.nogc());
        let name = name.bind(gc.nogc());
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc.nogc());
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let n = PropertyKey::from(name);
        let existing_prop =
            global_object
                .unbind()
                .internal_get_own_property(agent, n.unbind(), gc)?;
        let Some(existing_prop) = existing_prop else {
            // 4. If existingProp is undefined, return false.
            return Ok(false);
        };
        // 5. If existingProp.[[Configurable]] is true, return false.
        // 6. Return true.
        Ok(existing_prop.configurable != Some(true))
    }

    /// ### Try [9.1.1.4.15 CanDeclareGlobalVar ( N )](https://tc39.es/ecma262/#sec-candeclareglobalvar)
    ///
    /// The CanDeclareGlobalVar concrete method of a Global Environment Record
    /// envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// a corresponding CreateGlobalVarBinding call would succeed if called for
    /// the same argument N. Redundant var declarations and var declarations
    /// for pre-existing global object properties are allowed.
    pub(crate) fn try_can_declare_global_var<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc);
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        let n = PropertyKey::from(name);
        let has_property = try_has_own_property(agent, global_object, n, gc)?;
        // 4. If hasProperty is true, return true.
        if has_property {
            TryResult::Continue(true)
        } else {
            // 5. Return ? IsExtensible(globalObject).
            try_is_extensible(agent, global_object, gc)
        }
    }

    /// ### [9.1.1.4.15 CanDeclareGlobalVar ( N )](https://tc39.es/ecma262/#sec-candeclareglobalvar)
    ///
    /// The CanDeclareGlobalVar concrete method of a Global Environment Record
    /// envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// a corresponding CreateGlobalVarBinding call would succeed if called for
    /// the same argument N. Redundant var declarations and var declarations
    /// for pre-existing global object properties are allowed.
    pub(crate) fn can_declare_global_var<'a>(
        self,
        agent: &mut Agent,
        name: String,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let env = self.bind(gc.nogc());
        let name = name.bind(gc.nogc());
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc.nogc());
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let scoped_global_object = global_object.scope(agent, gc.nogc());
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        let n = PropertyKey::from(name);
        let has_property =
            has_own_property(agent, global_object.unbind(), n.unbind(), gc.reborrow()).unbind()?;
        // 4. If hasProperty is true, return true.
        if has_property {
            Ok(true)
        } else {
            // 5. Return ? IsExtensible(globalObject).
            is_extensible(agent, scoped_global_object.get(agent), gc)
        }
    }

    /// ### Try [9.1.1.4.16 CanDeclareGlobalFunction ( N )](https://tc39.es/ecma262/#sec-candeclareglobalfunction)
    ///
    /// The CanDeclareGlobalFunction concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// a corresponding CreateGlobalFunctionBinding call would succeed if
    /// called for the same argument N.
    pub(crate) fn try_can_declare_global_function<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<'gc, bool> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc);
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let n = PropertyKey::from(name);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let existing_prop = global_object.try_get_own_property(agent, n, gc)?;
        // 4. If existingProp is undefined, return ? IsExtensible(globalObject).
        let Some(existing_prop) = existing_prop else {
            return try_is_extensible(agent, global_object, gc);
        };
        // 5. If existingProp.[[Configurable]] is true, return true.
        if existing_prop.configurable == Some(true)
            || existing_prop.is_data_descriptor()
                && existing_prop.writable == Some(true)
                && existing_prop.enumerable == Some(true)
        {
            // 6. If IsDataDescriptor(existingProp) is true and existingProp has attribute values { [[Writable]]: true, [[Enumerable]]: true }, true.
            TryResult::Continue(true)
        } else {
            // 7. Return false.
            TryResult::Continue(false)
        }
    }

    /// ### [9.1.1.4.16 CanDeclareGlobalFunction ( N )](https://tc39.es/ecma262/#sec-candeclareglobalfunction)
    ///
    /// The CanDeclareGlobalFunction concrete method of a Global Environment
    /// Record envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It determines if
    /// a corresponding CreateGlobalFunctionBinding call would succeed if
    /// called for the same argument N.
    pub(crate) fn can_declare_global_function<'a>(
        self,
        agent: &mut Agent,
        name: String,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let name = name.bind(gc.nogc());
        let env = self.bind(gc.nogc());
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc.nogc());
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let scoped_global_object = global_object.scope(agent, gc.nogc());
        let n = PropertyKey::from(name);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let existing_prop = global_object
            .unbind()
            .internal_get_own_property(agent, n.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 4. If existingProp is undefined, return ? IsExtensible(globalObject).
        let Some(existing_prop) = existing_prop else {
            return is_extensible(agent, scoped_global_object.get(agent), gc);
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

    /// ### Try [9.1.1.4.17 CreateGlobalVarBinding ( N, D )](https://tc39.es/ecma262/#sec-createglobalvarbinding)
    ///
    /// The CreateGlobalVarBinding concrete method of a Global Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and
    /// returns either a normal completion containing UNUSED or a throw
    /// completion. It creates and initializes a mutable binding in the
    /// associated Object Environment Record and records the bound name in the
    /// associated \[\[VarNames]] List. If a binding already exists, it is
    /// reused and assumed to be initialized.
    pub(crate) fn try_create_global_var_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        cache: PropertyLookupCache,
        is_deletable: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<'a, ()> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc);
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let n = PropertyKey::from(name);
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        let has_property = try_has_own_property(agent, global_object, n, gc)?;
        // 4. Let extensible be ? IsExtensible(globalObject).
        let extensible = try_is_extensible(agent, global_object, gc)?;
        // 5. If hasProperty is false and extensible is true, then
        if !has_property && extensible {
            // a. Perform ? ObjRec.CreateMutableBinding(N, D).
            obj_rec.try_create_mutable_binding(agent, name, is_deletable, gc)?;
            // b. Perform ? ObjRec.InitializeBinding(N, undefined).
            obj_rec.try_initialize_binding(agent, name, Some(cache), Value::Undefined, gc)?;
        }

        // 6. If envRec.[[VarNames]] does not contain N, then
        //    a. Append N to envRec.[[VarNames]].
        let env_rec = &mut agent[env];
        env_rec.var_names.insert(name.unbind());

        // 7. Return UNUSED.
        TryResult::Continue(())
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
    pub(crate) fn create_global_var_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        cache: PropertyLookupCache,
        is_deletable: bool,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let nogc = gc.nogc();
        let env = self.bind(nogc);
        let name = name.bind(nogc);
        let cache = cache.bind(nogc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc.nogc());
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let scoped_global_object = global_object.scope(agent, nogc);
        let n = PropertyKey::from(name);
        let name = name.scope(agent, nogc);
        let env = env.scope(agent, nogc);
        let cache = cache.scope(agent, nogc);
        let obj_rec = obj_rec.scope(agent, nogc);
        // 3. Let hasProperty be ? HasOwnProperty(globalObject, N).
        let has_property =
            has_own_property(agent, global_object.unbind(), n.unbind(), gc.reborrow()).unbind()?;
        // 4. Let extensible be ? IsExtensible(globalObject).
        let extensible =
            is_extensible(agent, scoped_global_object.get(agent), gc.reborrow()).unwrap();
        // 5. If hasProperty is false and extensible is true, then
        if !has_property && extensible {
            // a. Perform ? ObjRec.CreateMutableBinding(N, D).
            obj_rec
                .get(agent)
                .create_mutable_binding(agent, name.get(agent), is_deletable, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. Perform ? ObjRec.InitializeBinding(N, undefined).
            // SAFETY: obj_rec is not shared.
            unsafe { obj_rec.take(agent) }.initialize_binding(
                agent,
                name.get(agent),
                Some(cache.get(agent)),
                Value::Undefined,
                gc,
            )?;
        } else {
            // SAFETY: obj_rec is not shared
            let _ = unsafe { obj_rec.take(agent) };
        }

        // SAFETY: cache is not shared.
        let _ = unsafe { cache.take(agent) };
        // SAFETY: env is not shared.
        let env = unsafe { env.take(agent) };
        // SAFETY: name is not shared.
        let name = unsafe { name.take(agent) };
        // 6. If envRec.[[VarNames]] does not contain N, then
        //    a. Append N to envRec.[[VarNames]].
        agent[env].var_names.insert(name);

        // 7. Return UNUSED.
        Ok(())
    }

    /// ### Try [9.1.1.4.18 CreateGlobalFunctionBinding ( N, V, D )](https://tc39.es/ecma262/#sec-createglobalfunctionbinding)
    ///
    /// The CreateGlobalFunctionBinding concrete method of a Global Environment
    /// Record envRec takes arguments N (a String), V (an ECMAScript language
    /// value), and D (a Boolean) and returns either a normal completion
    /// containing UNUSED or a throw completion. It creates and initializes a
    /// mutable binding in the associated Object Environment Record and records
    /// the bound name in the associated [[VarNames]] List. If a binding
    /// already exists, it is replaced.
    pub(crate) fn try_create_global_function_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        d: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<'a, ()> {
        let env = self.bind(gc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc);
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let n = PropertyKey::from(name);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let existing_prop = global_object.try_get_own_property(agent, n, gc)?;
        // 4. If existingProp is undefined or existingProp.[[Configurable]] is true, then
        let desc = if existing_prop.is_none() || existing_prop.unwrap().configurable == Some(true) {
            // a. Let desc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }.
            PropertyDescriptor {
                value: Some(value.unbind()),
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
                value: Some(value.unbind()),
                writable: None,
                get: None,
                set: None,
                enumerable: None,
                configurable: None,
            }
        };
        // 6. Perform ? DefinePropertyOrThrow(globalObject, N, desc).
        try_define_property_or_throw(agent, global_object, n, desc, gc)?;
        // 7. Perform ? Set(globalObject, N, V, false).
        try_set(agent, global_object, n, value, false, None, gc)?;

        // 8. If envRec.[[VarNames]] does not contain N, then
        // a. Append N to envRec.[[VarNames]].
        let env_rec = &mut agent[env];
        env_rec.var_names.insert(name.unbind());
        // 9. Return UNUSED.
        TryResult::Continue(())
        // NOTE
        // Global function declarations are always represented as own
        // properties of the global object. If possible, an existing own
        // property is reconfigured to have a standard set of attribute values.
        // Step 7 is equivalent to what calling the InitializeBinding concrete
        // method would do and if globalObject is a Proxy will produce the same
        // sequence of Proxy trap calls.
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
    pub(crate) fn create_global_function_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        d: bool,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let nogc = gc.nogc();
        let env = self.bind(nogc);
        let name = name.bind(nogc);
        let value = value.scope(agent, nogc);
        let env_rec = &agent[env];
        // 1. Let ObjRec be envRec.[[ObjectRecord]].
        let obj_rec = env_rec.object_record.bind(gc.nogc());
        // 2. Let globalObject be ObjRec.[[BindingObject]].
        let global_object = obj_rec.get_binding_object(agent);
        let scoped_global_object = global_object.scope(agent, nogc);
        let n = PropertyKey::from(name);
        let scoped_n = n.scope(agent, nogc);
        let env = env.scope(agent, nogc);
        // 3. Let existingProp be ? globalObject.[[GetOwnProperty]](N).
        let existing_prop = global_object
            .unbind()
            .internal_get_own_property(agent, n.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 4. If existingProp is undefined or existingProp.[[Configurable]] is true, then
        let desc = if existing_prop.is_none() || existing_prop.unwrap().configurable == Some(true) {
            // a. Let desc be the PropertyDescriptor { [[Value]]: V, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }.
            PropertyDescriptor {
                value: Some(value.get(agent)),
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
                value: Some(value.get(agent)),
                writable: None,
                get: None,
                set: None,
                enumerable: None,
                configurable: None,
            }
        };
        // 6. Perform ? DefinePropertyOrThrow(globalObject, N, desc).
        define_property_or_throw(
            agent,
            scoped_global_object.get(agent),
            scoped_n.get(agent),
            desc,
            gc.reborrow(),
        )
        .unbind()?;
        // 7. Perform ? Set(globalObject, N, V, false).
        set(
            agent,
            scoped_global_object.get(agent),
            scoped_n.get(agent),
            value.get(agent),
            false,
            gc,
        )?;
        // SAFETY: env is not shared.
        let env = unsafe { env.take(agent) };
        // SAFETY: scoped_n is not shared.
        let name = unsafe { scoped_n.take(agent) };
        // 8. If envRec.[[VarNames]] does not contain N, then
        // a. Append N to envRec.[[VarNames]].
        // SAFETY: Name of a global function cannot be a numeric string.
        let n = unsafe { String::try_from(name.into_value_unchecked()).unwrap() };
        agent[env].var_names.insert(n);
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

impl HeapMarkAndSweep for GlobalEnvironment<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.global_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .global_environments
            .shift_non_zero_u32_index(&mut self.0);
    }
}
