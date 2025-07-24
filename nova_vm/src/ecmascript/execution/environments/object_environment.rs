// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                define_property_or_throw, get, has_property, set, throw_set_error,
                try_define_property_or_throw, try_get, try_has_property, try_set,
            },
            type_conversion::to_boolean,
        },
        builtins::ordinary::{try_get_ordinary_object_value, try_set_ordinary_object_value},
        execution::{
            Agent, JsResult,
            agent::{ExceptionType, JsError},
            environments::{Environment, ObjectEnvironment, OuterEnv},
        },
        types::{
            BUILTIN_STRING_MEMORY, InternalMethods, IntoValue, Object, PropertyDescriptor,
            PropertyKey, String, Value,
        },
    },
    engine::{
        TryResult,
        context::{Bindable, GcScope, NoGcScope},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WellKnownSymbolIndexes, WorkQueues},
};

/// ### [9.1.1.2 Object Environment Records](https://tc39.es/ecma262/#sec-object-environment-records)
///
/// Each Object Environment Record is associated with an object called its
/// binding object. An Object Environment Record binds the set of string
/// identifier names that directly correspond to the property names of its
/// binding object. Property keys that are not strings in the form of an
/// IdentifierName are not included in the set of bound identifiers. Both own
/// and inherited properties are included in the set regardless of the setting
/// of their \[\[Enumerable\]\] attribute. Because properties can be
/// dynamically added and deleted from objects, the set of identifiers bound by
/// an Object Environment Record may potentially change as a side-effect of any
/// operation that adds or deletes properties. Any bindings that are created as
/// a result of such a side-effect are considered to be a mutable binding even
/// if the Writable attribute of the corresponding property is false. Immutable
/// bindings do not exist for Object Environment Records.
#[derive(Debug, Clone)]
pub struct ObjectEnvironmentRecord {
    /// ### \[\[BindingObject\]\]
    ///
    /// The binding object of this Environment Record.
    binding_object: Object<'static>,

    /// ### \[\[IsWithEnvironment\]\]
    ///
    /// Indicates whether this Environment Record is created for a with
    /// statement.
    is_with_environment: bool,

    /// ### \[\[OuterEnv\]\]
    ///
    /// See [OuterEnv].
    outer_env: OuterEnv<'static>,
}

impl ObjectEnvironmentRecord {
    /// ### [9.1.2.3 NewObjectEnvironment ( O, W, E )](https://tc39.es/ecma262/#sec-newobjectenvironmenthttps://tc39.es/ecma262/#sec-newobjectenvironment)
    ///
    /// The abstract operation NewObjectEnvironment takes arguments O (an
    /// Object), W (a Boolean), and E (an Environment Record or null) and
    /// returns an Object Environment Record.
    pub(crate) fn new(
        binding_object: Object,
        is_with_environment: bool,
        outer_env: OuterEnv,
    ) -> ObjectEnvironmentRecord {
        // 1. Let env be a new Object Environment Record.
        ObjectEnvironmentRecord {
            // 2. Set env.[[BindingObject]] to O.
            binding_object: binding_object.unbind(),
            // 3. Set env.[[IsWithEnvironment]] to W.
            is_with_environment,
            // 4. Set env.[[OuterEnv]] to E.
            outer_env: outer_env.unbind(),
        }
        // 5. Return env.
    }
}

impl HeapMarkAndSweep for ObjectEnvironmentRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            binding_object,
            is_with_environment: _,
            outer_env,
        } = self;
        outer_env.mark_values(queues);
        binding_object.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            binding_object,
            is_with_environment: _,
            outer_env,
        } = self;
        outer_env.sweep_values(compactions);
        binding_object.sweep_values(compactions);
    }
}

impl<'e> ObjectEnvironment<'e> {
    pub(crate) fn get_binding_object(self, agent: &Agent) -> Object<'e> {
        agent[self].binding_object
    }

    pub(crate) fn get_outer_env(self, agent: &Agent) -> Option<Environment<'e>> {
        agent[self].outer_env
    }

    /// ### Try [9.1.1.2.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-object-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of an Object Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if its
    /// associated binding object has a property whose name is N.
    pub(crate) fn try_has_binding(
        self,
        agent: &mut Agent,
        n: String,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc);
        let is_with_environment = env_rec.is_with_environment;
        let name = PropertyKey::from(n).bind(gc);

        if !is_with_environment
            && let Object::Object(binding_object) = binding_object
            && let Ok(value) = try_get_ordinary_object_value(agent, binding_object, name)
        {
            // We either found a value or checked each property in the
            // prototype chain and found nothing.
            return TryResult::Continue(value.is_some());
        }

        // 2. Let foundBinding be ? HasProperty(bindingObject, N).
        let found_binding = try_has_property(agent, binding_object, name, gc)?;
        // 3. If foundBinding is false, return false.
        if !found_binding {
            return TryResult::Continue(false);
        }
        // 4. If envRec.[[IsWithEnvironment]] is false, return true.
        if !is_with_environment {
            return TryResult::Continue(true);
        }
        // 5. Let unscopables be ? Get(bindingObject, @@unscopables).
        let unscopables = try_get(
            agent,
            binding_object,
            PropertyKey::Symbol(WellKnownSymbolIndexes::Unscopables.into()),
            gc,
        )?;
        // 6. If unscopables is an Object, then
        if let Ok(unscopables) = Object::try_from(unscopables) {
            // a. Let blocked be ToBoolean(? Get(unscopables, N)).
            let blocked = try_get(agent, unscopables, name, gc)?;
            let blocked = to_boolean(agent, blocked);
            // b. If blocked is true, return false.
            TryResult::Continue(!blocked)
        } else {
            // 7. Return true.
            TryResult::Continue(true)
        }
    }

    /// ### [9.1.1.2.1 HasBinding ( N )](https://tc39.es/ecma262/#sec-object-environment-records-hasbinding-n)
    ///
    /// The HasBinding concrete method of an Object Environment Record envRec
    /// takes argument N (a String) and returns either a normal completion
    /// containing a Boolean or a throw completion. It determines if its
    /// associated binding object has a property whose name is N.
    pub(crate) fn has_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc.nogc());
        let is_with_environment = env_rec.is_with_environment;
        let name = PropertyKey::from(n).bind(gc.nogc());

        if !is_with_environment
            && let Object::Object(binding_object) = binding_object
            && let Ok(value) = try_get_ordinary_object_value(agent, binding_object, name)
        {
            // We either found a value or checked each property in the
            // prototype chain and found nothing.
            return Ok(value.is_some());
        }

        let scoped_binding_object = binding_object.scope(agent, gc.nogc());
        let scoped_name = name.scope(agent, gc.nogc());
        // 2. Let foundBinding be ? HasProperty(bindingObject, N).
        let found_binding =
            has_property(agent, binding_object.unbind(), name.unbind(), gc.reborrow()).unbind()?;
        // 3. If foundBinding is false, return false.
        if !found_binding {
            return Ok(false);
        }
        // 4. If envRec.[[IsWithEnvironment]] is false, return true.
        if !is_with_environment {
            return Ok(true);
        }
        // 5. Let unscopables be ? Get(bindingObject, @@unscopables).
        let unscopables = get(
            agent,
            scoped_binding_object.get(agent),
            PropertyKey::Symbol(WellKnownSymbolIndexes::Unscopables.into()),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 6. If unscopables is an Object, then
        if let Ok(unscopables) = Object::try_from(unscopables) {
            // a. Let blocked be ToBoolean(? Get(unscopables, N)).
            let blocked = get(
                agent,
                unscopables.unbind(),
                scoped_name.get(agent),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let blocked = to_boolean(agent, blocked);
            // b. If blocked is true, return false.
            Ok(!blocked)
        } else {
            // 7. Return true.
            Ok(true)
        }
    }

    /// ### [9.1.1.2.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-object-environment-records-createmutablebinding-n-d)
    ///
    /// The CreateMutableBinding concrete method of an Object Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and
    /// returns either a normal completion containing UNUSED or a throw
    /// completion. It creates in an Environment Record's associated binding
    /// object a property whose name is N and initializes it to the value
    /// undefined. If D is true, the new property's [[Configurable]] attribute
    /// is set to true; otherwise it is set to false.
    pub(crate) fn try_create_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        d: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, ()>> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc);
        // 2. Perform ? DefinePropertyOrThrow(bindingObject, N, PropertyDescriptor { [[Value]]: undefined, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }).
        // 3. Return UNUSED.
        let n = PropertyKey::from(n).bind(gc);
        try_define_property_or_throw(
            agent,
            binding_object.unbind(),
            n.unbind(),
            PropertyDescriptor {
                value: Some(Value::Undefined),
                writable: Some(true),
                get: None,
                set: None,
                enumerable: Some(true),
                configurable: Some(d),
            },
            gc,
        )
        // NOTE
        // Normally envRec will not have a binding for N but if it does, the
        // semantics of DefinePropertyOrThrow may result in an existing binding
        // being replaced or shadowed or cause an abrupt completion to be
        // returned.
    }

    /// ### [9.1.1.2.2 CreateMutableBinding ( N, D )](https://tc39.es/ecma262/#sec-object-environment-records-createmutablebinding-n-d)
    ///
    /// The CreateMutableBinding concrete method of an Object Environment
    /// Record envRec takes arguments N (a String) and D (a Boolean) and
    /// returns either a normal completion containing UNUSED or a throw
    /// completion. It creates in an Environment Record's associated binding
    /// object a property whose name is N and initializes it to the value
    /// undefined. If D is true, the new property's [[Configurable]] attribute
    /// is set to true; otherwise it is set to false.
    pub(crate) fn create_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        d: bool,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc.nogc());
        let n = PropertyKey::from(n).bind(gc.nogc());

        // 2. Perform ? DefinePropertyOrThrow(bindingObject, N, PropertyDescriptor { [[Value]]: undefined, [[Writable]]: true, [[Enumerable]]: true, [[Configurable]]: D }).
        define_property_or_throw(
            agent,
            binding_object.unbind(),
            n.unbind(),
            PropertyDescriptor {
                value: Some(Value::Undefined),
                writable: Some(true),
                get: None,
                set: None,
                enumerable: Some(true),
                configurable: Some(d),
            },
            gc,
        )?;
        // 3. Return UNUSED.
        Ok(())
        // NOTE
        // Normally envRec will not have a binding for N but if it does, the
        // semantics of DefinePropertyOrThrow may result in an existing binding
        // being replaced or shadowed or cause an abrupt completion to be
        // returned.
    }

    /// ### [9.1.1.2.3 CreateImmutableBinding ( N, S )](https://tc39.es/ecma262/#sec-object-environment-records-createimmutablebinding-n-s)
    pub(crate) fn create_immutable_binding(self, _: &mut Agent, _: String, _: bool) {
        unreachable!(
            "The CreateImmutableBinding concrete method of an Object Environment Record is never used within this specification."
        )
    }

    /// ### Try [9.1.1.2.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-object-environment-records-initializebinding-n-v)
    ///
    /// The InitializeBinding concrete method of an Object Environment Record
    /// envRec takes arguments N (a String) and V (an ECMAScript language
    /// value) and returns either a normal completion containing UNUSED or a
    /// throw completion. It is used to set the bound value of the current
    /// binding of the identifier whose name is N to the value V.
    pub(crate) fn try_initialize_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        v: Value,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, ()>> {
        // 1. Perform ? envRec.SetMutableBinding(N, V, false).
        // 2. Return UNUSED.
        self.try_set_mutable_binding(agent, n, v, false, gc)
        // NOTE
        // In this specification, all uses of CreateMutableBinding for Object
        // Environment Records are immediately followed by a call to
        // InitializeBinding for the same name. Hence, this specification does
        // not explicitly track the initialization state of bindings in Object
        // Environment Records.
    }

    /// ### [9.1.1.2.4 InitializeBinding ( N, V )](https://tc39.es/ecma262/#sec-object-environment-records-initializebinding-n-v)
    ///
    /// The InitializeBinding concrete method of an Object Environment Record
    /// envRec takes arguments N (a String) and V (an ECMAScript language
    /// value) and returns either a normal completion containing UNUSED or a
    /// throw completion. It is used to set the bound value of the current
    /// binding of the identifier whose name is N to the value V.
    pub(crate) fn initialize_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        v: Value,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        // 1. Perform ? envRec.SetMutableBinding(N, V, false).
        self.set_mutable_binding(agent, n, v, false, gc)?;
        // 2. Return UNUSED.
        Ok(())
        // NOTE
        // In this specification, all uses of CreateMutableBinding for Object
        // Environment Records are immediately followed by a call to
        // InitializeBinding for the same name. Hence, this specification does
        // not explicitly track the initialization state of bindings in Object
        // Environment Records.
    }

    /// ### [9.1.1.2.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-object-environment-records-setmutablebinding-n-v-s)
    ///
    /// The SetMutableBinding concrete method of an Object Environment Record
    /// envRec takes arguments N (a String), V (an ECMAScript language value),
    /// and S (a Boolean) and returns either a normal completion containing
    /// UNUSED or a throw completion. It attempts to set the value of the
    /// Environment Record's associated binding object's property whose name is
    /// N to the value V. A property named N normally already exists but if it
    /// does not or is not currently writable, error handling is determined by
    /// S.
    pub(crate) fn try_set_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        v: Value,
        s: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, ()>> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc);
        let n = PropertyKey::from(n).bind(gc);

        if let Object::Object(binding_object) = binding_object
            && let Some(set_value) = try_set_ordinary_object_value(agent, binding_object, n, v)
        {
            // Key was found on binding object directly and contained a
            // data property.
            if !set_value && s {
                // The key was unwritable and we're in strict mode;
                // need to throw an error.
                return TryResult::Continue(throw_set_error(agent, n, gc));
            }
            return TryResult::Continue(Ok(()));
        }

        // 2. Let stillExists be ? HasProperty(bindingObject, N).
        let still_exists = try_has_property(agent, binding_object, n, gc)?;
        // 3. If stillExists is false and S is true, throw a ReferenceError exception.
        if !still_exists && s {
            TryResult::Continue(Err(Self::throw_property_doesnt_exist_error(
                agent,
                BUILTIN_STRING_MEMORY.object,
                n,
                gc,
            )))
        } else {
            // 4. Perform ? Set(bindingObject, N, V, S).
            // 5. Return UNUSED.
            try_set(agent, binding_object, n, v, s, gc)
        }
    }

    /// ### [9.1.1.2.5 SetMutableBinding ( N, V, S )](https://tc39.es/ecma262/#sec-object-environment-records-setmutablebinding-n-v-s)
    ///
    /// The SetMutableBinding concrete method of an Object Environment Record
    /// envRec takes arguments N (a String), V (an ECMAScript language value),
    /// and S (a Boolean) and returns either a normal completion containing
    /// UNUSED or a throw completion. It attempts to set the value of the
    /// Environment Record's associated binding object's property whose name is
    /// N to the value V. A property named N normally already exists but if it
    /// does not or is not currently writable, error handling is determined by
    /// S.
    pub(crate) fn set_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        n: String,
        v: Value,
        s: bool,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc.nogc());
        let n = PropertyKey::from(n).bind(gc.nogc());

        if let Object::Object(binding_object) = binding_object
            && let Some(set_value) = try_set_ordinary_object_value(agent, binding_object, n, v)
        {
            // Key was found on binding object directly and contained a
            // data property.
            if !set_value && s {
                // The key was unwritable and we're in strict mode;
                // need to throw an error.
                return throw_set_error(agent, n.unbind(), gc.into_nogc());
            }
            return Ok(());
        }

        let scoped_binding_object = binding_object.scope(agent, gc.nogc());
        let scoped_n = n.scope(agent, gc.nogc());

        // 2. Let stillExists be ? HasProperty(bindingObject, N).
        let still_exists =
            has_property(agent, binding_object.unbind(), n.unbind(), gc.reborrow()).unbind()?;
        // 3. If stillExists is false and S is true, throw a ReferenceError exception.
        if !still_exists && s {
            let binding_object_repr = scoped_binding_object
                .get(agent)
                .into_value()
                .string_repr(agent, gc.reborrow());
            Err(Self::throw_property_doesnt_exist_error(
                agent,
                binding_object_repr.unbind(),
                scoped_n.get(agent),
                gc.into_nogc(),
            ))
        } else {
            // 4. Perform ? Set(bindingObject, N, V, S).
            set(
                agent,
                scoped_binding_object.get(agent),
                scoped_n.get(agent),
                v,
                s,
                gc,
            )?;
            // 5. Return UNUSED.
            Ok(())
        }
    }

    fn throw_property_doesnt_exist_error<'a>(
        agent: &mut Agent,
        binding_object_repr: String,
        n: PropertyKey,
        gc: NoGcScope<'a, '_>,
    ) -> JsError<'a> {
        let error_message = format!(
            "Property '{}' does not exist in {}.",
            n.as_display(agent),
            binding_object_repr.to_string_lossy(agent)
        );
        agent.throw_exception(ExceptionType::ReferenceError, error_message, gc)
    }

    /// ### [9.1.1.2.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-object-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of an Object Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its associated binding
    /// object's property whose name is N. The property should already exist
    /// but if it does not the result depends upon S.
    pub(crate) fn try_get_binding_value(
        self,
        agent: &mut Agent,
        n: String,
        s: bool,
        gc: NoGcScope<'e, '_>,
    ) -> TryResult<JsResult<'e, Value<'e>>> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc);
        let name = PropertyKey::from(n).bind(gc);

        if let Object::Object(binding_object) = binding_object
            && let Ok(value) = try_get_ordinary_object_value(agent, binding_object, name)
        {
            return if let Some(value) = value {
                // Found the property value.
                TryResult::Continue(Ok(value))
            } else {
                // Property did not exist.
                TryResult::Continue(Self::handle_property_not_found(agent, name, s, gc))
            };
        }

        // 2. Let value be ? HasProperty(bindingObject, N).
        let value = try_has_property(agent, binding_object, name, gc)?;
        // 3. If value is false, then
        if !value {
            TryResult::Continue(Self::handle_property_not_found(agent, name, s, gc))
        } else {
            // 4. Return ? Get(bindingObject, N).
            TryResult::Continue(Ok(try_get(agent, binding_object, name, gc)?))
        }
    }

    /// ### [9.1.1.2.6 GetBindingValue ( N, S )](https://tc39.es/ecma262/#sec-object-environment-records-getbindingvalue-n-s)
    ///
    /// The GetBindingValue concrete method of an Object Environment Record
    /// envRec takes arguments N (a String) and S (a Boolean) and returns
    /// either a normal completion containing an ECMAScript language value or a
    /// throw completion. It returns the value of its associated binding
    /// object's property whose name is N. The property should already exist
    /// but if it does not the result depends upon S.
    pub(crate) fn get_binding_value<'a>(
        self,
        agent: &mut Agent,
        name: String,
        s: bool,
        mut gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object.bind(gc.nogc());
        let name = PropertyKey::from(name).bind(gc.nogc());

        if let Object::Object(binding_object) = binding_object
            && let Ok(value) = try_get_ordinary_object_value(agent, binding_object, name)
        {
            return if let Some(value) = value {
                // Found the property value.
                Ok(value.unbind())
            } else {
                // Property did not exist.
                Self::handle_property_not_found(agent, name.unbind(), s, gc.into_nogc())
            };
        }

        let scoped_binding_object = binding_object.scope(agent, gc.nogc());
        let scoped_name = name.scope(agent, gc.nogc());

        // 2. Let value be ? HasProperty(bindingObject, N).
        let value =
            has_property(agent, binding_object.unbind(), name.unbind(), gc.reborrow()).unbind()?;
        // 3. If value is false, then
        if !value {
            Self::handle_property_not_found(agent, scoped_name.get(agent), s, gc.into_nogc())
        } else {
            // 4. Return ? Get(bindingObject, N).
            get(
                agent,
                scoped_binding_object.get(agent),
                scoped_name.get(agent),
                gc,
            )
        }
    }

    fn handle_property_not_found<'a>(
        agent: &mut Agent,
        name: PropertyKey,
        s: bool,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        // a. If S is false, return undefined; otherwise throw a ReferenceError exception.
        if !s {
            Ok(Value::Undefined)
        } else {
            let error_message = format!(
                "Property '{}' does not exist in object.",
                name.as_display(agent)
            );
            Err(agent.throw_exception(ExceptionType::ReferenceError, error_message, gc))
        }
    }

    /// ### Try [9.1.1.2.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-object-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of an Object Environment Record
    /// envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It can only
    /// delete bindings that correspond to properties of the environment
    /// object whose [[Configurable]] attribute have the value true.
    pub(crate) fn try_delete_binding(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_object = env_rec.binding_object;
        let name = PropertyKey::from(name);
        // 2. Return ? bindingObject.[[Delete]](N).
        binding_object.try_delete(agent, name, gc)
    }

    /// ### [9.1.1.2.7 DeleteBinding ( N )](https://tc39.es/ecma262/#sec-object-environment-records-deletebinding-n)
    ///
    /// The DeleteBinding concrete method of an Object Environment Record
    /// envRec takes argument N (a String) and returns either a normal
    /// completion containing a Boolean or a throw completion. It can only
    /// delete bindings that correspond to properties of the environment
    /// object whose [[Configurable]] attribute have the value true.
    pub(crate) fn delete_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        let env_rec = &agent[self];
        // 1. Let bindingObject be envRec.[[BindingObject]].
        let binding_boject = env_rec.binding_object;
        let name = PropertyKey::from(name);
        // 2. Return ? bindingObject.[[Delete]](N).
        binding_boject.internal_delete(agent, name, gc)
    }

    /// ### [9.1.1.2.8 HasThisBinding ( )](https://tc39.es/ecma262/#sec-object-environment-records-hasthisbinding)
    ///
    /// The HasThisBinding concrete method of an Object Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_this_binding(&self) -> bool {
        // 1. Return false.
        false
        // NOTE
        // Object Environment Records do not provide a this binding.
    }

    /// ### [9.1.1.2.9 HasSuperBinding ( )](https://tc39.es/ecma262/#sec-object-environment-records-hassuperbinding)
    ///
    /// The HasSuperBinding concrete method of an Object Environment Record
    /// envRec takes no arguments and returns false.
    pub(crate) fn has_super_binding(&self) -> bool {
        // 1. Return false.
        false
        // NOTE
        // Object Environment Records do not provide a super binding.
    }

    /// ### [9.1.1.2.10 WithBaseObject ( )](https://tc39.es/ecma262/#sec-object-environment-records-withbaseobject)
    ///
    /// The WithBaseObject concrete method of an Object Environment Record
    /// envRec takes no arguments and returns an Object or undefined.
    pub(crate) fn with_base_object(self, agent: &Agent) -> Option<Object<'e>> {
        let env_rec = &agent[self];
        // 1. If envRec.[[IsWithEnvironment]] is true, return envRec.[[BindingObject]].
        if env_rec.is_with_environment {
            Some(env_rec.binding_object)
        } else {
            // 2. Otherwise, return undefined.
            None
        }
    }
}

impl HeapMarkAndSweep for ObjectEnvironment<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.object_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .object_environments
            .shift_non_zero_u32_index(&mut self.0);
    }
}
