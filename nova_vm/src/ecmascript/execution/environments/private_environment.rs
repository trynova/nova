// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::hash_map::Entry;

use ahash::AHashMap;

use crate::{
    ecmascript::{
        builtins::ECMAScriptFunction,
        execution::Agent,
        types::{IntoFunction, IntoValue, PrivateName, String, Value},
    },
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues, element_array::ElementDescriptor},
};

use super::PrivateEnvironment;

#[derive(Debug, Clone, Copy)]
pub(crate) enum PrivateMethod<'a> {
    Getter(ECMAScriptFunction<'a>),
    Setter(ECMAScriptFunction<'a>),
    Method(ECMAScriptFunction<'a>),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum PrivateField<'a> {
    Field {
        key: PrivateName,
    },
    Getter {
        key: PrivateName,
        get: ECMAScriptFunction<'a>,
    },
    Setter {
        key: PrivateName,
        set: ECMAScriptFunction<'a>,
    },
    Accessor {
        key: PrivateName,
        get: ECMAScriptFunction<'a>,
        set: ECMAScriptFunction<'a>,
    },
    Method {
        key: PrivateName,
        method: ECMAScriptFunction<'a>,
    },
}

impl<'a> PrivateField<'a> {
    /// Returns true if the PrivateField is a method or accessor.
    pub(crate) fn is_method(self) -> bool {
        matches!(
            self,
            PrivateField::Getter { .. }
                | PrivateField::Setter { .. }
                | PrivateField::Accessor { .. }
                | PrivateField::Method { .. }
        )
    }

    /// Returns the PrivateField as an ElementDescriptor
    ///
    /// ## Panics
    ///
    /// Panics if the PrivateField is not a method.
    pub(crate) fn into_element_descriptor(self) -> ElementDescriptor<'a> {
        match self {
            PrivateField::Getter { get, .. } => {
                ElementDescriptor::ReadOnlyUnenumerableUnconfigurableAccessor {
                    get: get.into_function(),
                }
            }
            PrivateField::Setter { set, .. } => {
                ElementDescriptor::WriteOnlyUnenumerableUnconfigurableAccessor {
                    set: set.into_function(),
                }
            }
            PrivateField::Accessor { get, set, .. } => {
                ElementDescriptor::ReadWriteUnenumerableUnconfigurableAccessor {
                    get: get.into_function(),
                    set: set.into_function(),
                }
            }
            PrivateField::Method { .. } => {
                ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData
            }
            _ => unreachable!(),
        }
    }

    /// Get the PrivateName of a PrivateField.
    pub(crate) fn get_key(self) -> PrivateName {
        match self {
            PrivateField::Field { key }
            | PrivateField::Getter { key, .. }
            | PrivateField::Setter { key, .. }
            | PrivateField::Accessor { key, .. }
            | PrivateField::Method { key, .. } => key,
        }
    }

    /// Get the statically knowable Value for a PrivateField.
    ///
    /// Only non-accessor methods' Value is statically known.
    pub(crate) fn get_value(self) -> Option<Value<'a>> {
        match self {
            PrivateField::Field { .. }
            | PrivateField::Getter { .. }
            | PrivateField::Setter { .. }
            | PrivateField::Accessor { .. } => None,
            PrivateField::Method { method, .. } => Some(method.into_value()),
        }
    }
}

// SAFETY: Trivially safe.
unsafe impl Bindable for PrivateField<'_> {
    type Of<'a> = PrivateField<'a>;

    fn unbind(self) -> Self::Of<'static> {
        match self {
            Self::Field { key } => PrivateField::Field { key },
            Self::Getter { key, get } => PrivateField::Getter {
                key,
                get: get.unbind(),
            },
            Self::Setter { key, set } => PrivateField::Setter {
                key,
                set: set.unbind(),
            },
            Self::Accessor { key, get, set } => PrivateField::Accessor {
                key,
                get: get.unbind(),
                set: set.unbind(),
            },
            Self::Method { key, method } => PrivateField::Method {
                key,
                method: method.unbind(),
            },
        }
    }

    fn bind<'a>(self, gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        match self {
            Self::Field { key } => PrivateField::Field { key },
            Self::Getter { key, get } => PrivateField::Getter {
                key,
                get: get.bind(gc),
            },
            Self::Setter { key, set } => PrivateField::Setter {
                key,
                set: set.bind(gc),
            },
            Self::Accessor { key, get, set } => PrivateField::Accessor {
                key,
                get: get.bind(gc),
                set: set.bind(gc),
            },
            Self::Method { key, method } => PrivateField::Method {
                key,
                method: method.bind(gc),
            },
        }
    }
}

/// ### [9.2 PrivateEnvironment Records](https://tc39.es/ecma262/#sec-privateenvironment-records)
///
/// A PrivateEnvironment Record is a specification mechanism used to track
/// Private Names based upon the lexical nesting structure of ClassDeclarations
/// and ClassExpressions in ECMAScript code. They are similar to, but distinct
/// from, Environment Records. Each PrivateEnvironment Record is associated
/// with a ClassDeclaration or ClassExpression. Each time such a class is
/// evaluated, a new PrivateEnvironment Record is created to record the Private
/// Names declared by that class.
#[derive(Debug)]
pub struct PrivateEnvironmentRecord {
    /// ### \[\[OuterPrivateEnvironment\]\]
    ///
    /// The PrivateEnvironment Record of the nearest containing class. null if
    /// the class with which this PrivateEnvironment Record is associated is
    /// not contained in any other class.
    outer_private_environment: Option<PrivateEnvironment<'static>>,

    /// ### \[\[Names\]\]
    ///
    /// The Private Names declared by this class.
    names: AHashMap<String<'static>, PrivateName>,
    /// ### \[\[PrivateMethods]]
    ///
    /// This stores the instanec private methods of a class. Per specification,
    /// these should be stored on the constructor function but we do not want
    /// to keep that sort of memory around for each
    private_fields: Vec<PrivateField<'static>>,
    /// First PrivateName field in the record. The rest are in ascending
    /// numerical order, up to `first_private_field + names.len() - 1` in
    /// numeric value.
    first_private_name: PrivateName,
    /// Number of PrivateName fields.
    ///
    /// PrivateName fields come first in the range of PrivateNames.
    instance_private_field_count: u32,
    /// Number of PrivateName methods.
    ///
    /// PrivateName methods come second in the range of PrivateNames after
    /// fields.
    instance_private_method_count: u32,
    /// Number of static PrivateName fields.
    ///
    /// Static PrivateName fields come third in the range of PrivateNames after
    /// methods.
    ///
    /// The remaining PrivateNames after static fields are static methods.
    static_private_field_count: u32,
}

impl PrivateEnvironmentRecord {
    /// Add a new PrivateName with the given \[\[Description]] field into the
    /// Private Environment. Returns the added PrivateName and true if the
    /// PrivateName was added to the environment, false if it was already in
    /// it.
    fn add_private_name(&mut self, description: String) -> (PrivateName, bool) {
        let current_number_of_names = self.names.len();
        match self.names.entry(description.unbind()) {
            Entry::Occupied(occupied_entry) => {
                // Occupied means we're adding a Get or Set to the other.
                (*occupied_entry.get(), false)
            }
            Entry::Vacant(vacant_entry) => {
                let next_private_name = PrivateName::from_u32(
                    self.first_private_name.into_u32() + current_number_of_names as u32,
                );
                vacant_entry.insert(next_private_name);
                (next_private_name, true)
            }
        }
    }

    /// Get list of all instance PrivateFields in this PrivateEnvironment.
    pub(crate) fn get_instance_private_fields<'gc>(
        &self,
        _: NoGcScope<'gc, '_>,
    ) -> &[PrivateField<'gc>] {
        &self.private_fields
    }
}

/// ### [9.2.1.1 NewPrivateEnvironment ( outerPrivEnv )](https://tc39.es/ecma262/#sec-newprivateenvironment)
///
/// The abstract operation NewPrivateEnvironment takes argument outerPrivEnv (a
/// PrivateEnvironment Record or null) and returns a PrivateEnvironment Record.
pub(crate) fn new_private_environment<'gc>(
    agent: &mut Agent,
    outer_private_environment: Option<PrivateEnvironment>,
    private_names_count: usize,
    gc: NoGcScope<'gc, '_>,
) -> PrivateEnvironment<'gc> {
    let first_private_name = agent.create_private_names(private_names_count);
    // 1. Let names be a new empty List.
    // 2. Return the PrivateEnvironment Record {
    agent.heap.alloc_counter += core::mem::size_of::<Option<PrivateEnvironmentRecord>>();
    let record = PrivateEnvironmentRecord {
        // [[OuterPrivateEnvironment]]: outerPrivEnv,
        outer_private_environment: outer_private_environment.unbind(),
        // [[Names]]: names
        names: AHashMap::with_capacity(private_names_count),
        private_fields: vec![],
        first_private_name,
        // Note: we assume that all PrivateNames are instance fields.
        instance_private_field_count: private_names_count as u32,
        instance_private_method_count: 0,
        static_private_field_count: 0,
    };
    // }.
    agent.heap.environments.push_private_environment(record, gc)
}

/// ### [9.2.1.2 ResolvePrivateIdentifier ( privateEnv, identifier )](https://tc39.es/ecma262/#sec-resolve-private-identifier)
///
/// The abstract operation ResolvePrivateIdentifier takes arguments privateEnv
/// (a PrivateEnvironment Record) and identifier (a String) and returns a
/// Private Name.
pub(crate) fn resolve_private_identifier(
    agent: &Agent,
    private_env: PrivateEnvironment,
    identifier: String,
) -> PrivateName {
    let data = private_env.get_data(agent);
    // 1. Let names be privateEnv.[[Names]].
    let names = &data.names;
    // 2. For each Private Name pn of names, do
    // a. If pn.[[Description]] is identifier, then
    // i. Return pn.
    if let Some(pn) = names.get(&identifier.unbind()) {
        return *pn;
    }
    // 3. Let outerPrivateEnv be privateEnv.[[OuterPrivateEnvironment]].
    // 4. Assert: outerPrivateEnv is not null.
    let outer_private_env = data
        .outer_private_environment
        .expect("outerPrivateEnv is null");
    // 5. Return ResolvePrivateIdentifier(outerPrivateEnv, identifier).
    resolve_private_identifier(agent, outer_private_env, identifier)
}

impl PrivateEnvironment<'_> {
    fn get_data(self, agent: &Agent) -> &PrivateEnvironmentRecord {
        agent.heap.environments.get_private_environment(self)
    }

    fn get_data_mut(self, agent: &mut Agent) -> &mut PrivateEnvironmentRecord {
        agent.heap.environments.get_private_environment_mut(self)
    }

    pub(crate) fn get_outer_env<'a>(
        self,
        agent: &Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<PrivateEnvironment<'a>> {
        self.get_data(agent).outer_private_environment.bind(gc)
    }

    /// Gets a PrivateName by offset in the Private Environment.
    ///
    /// ## Safety
    ///
    /// This method leaks out PrivateNames without checking if the caller has
    /// the right to access them. This should only be used when the caller
    /// statically knows they have that right.
    ///
    /// ## Panics
    ///
    /// The method panics if the `offset` is outside the range of PrivateNames
    /// in the Private Environment.
    pub(crate) unsafe fn get_private_name(self, agent: &Agent, offset: usize) -> PrivateName {
        let data = self.get_data(agent);
        if offset >= data.names.len() {
            panic!("Attempted to get PrivateName outside the Private Environment's range");
        }
        PrivateName::from_u32(data.first_private_name.into_u32() + offset as u32)
    }

    /// Get the offset to Private Environment's first PrivateName.
    pub(crate) fn get_base_offset(self, agent: &Agent) -> usize {
        let data = self.get_data(agent);
        if let Some(parent) = data.outer_private_environment {
            parent.get_private_name_count(agent) + parent.get_base_offset(agent)
        } else {
            0
        }
    }

    /// Get the number of PrivateNames in this Private Environment.
    pub(crate) fn get_private_name_count(self, agent: &Agent) -> usize {
        self.get_data(agent).names.len()
    }

    /// Resolves a PrivateName into its \[\[Description]] String if found in
    /// this environment or in an outer one.
    pub(crate) fn resolve_description<'a>(
        self,
        agent: &Agent,
        name: PrivateName,
        gc: NoGcScope<'a, '_>,
    ) -> Option<String<'a>> {
        let data = self.get_data(agent);
        let inclusive_lower_bound = data.first_private_name.into_u32();
        let exclusive_upper_bound = inclusive_lower_bound + data.names.len() as u32;
        let name_value = name.into_u32();
        if name_value >= exclusive_upper_bound {
            // Name newer than our environment holds; it cannot be found here.
            return None;
        } else if name_value < inclusive_lower_bound {
            // Name is not here can still be found in outer private envs.
            return data
                .outer_private_environment?
                .resolve_description(agent, name, gc);
        }
        // Name falls within our bounds, so it is in this environment. Now we
        // just need to find it to get its name.
        for (description, private_name) in data.names.iter() {
            if name == *private_name {
                return Some(description.bind(gc));
            }
        }
        unreachable!()
    }

    /// Adds an instance PrivateName field to the current Private Environment.
    ///
    /// This method does not return the added PrivateName as the caller should
    /// be merely initialising the environment instead of initialising a class
    /// instance.
    pub(crate) fn add_instance_private_field(self, agent: &mut Agent, description: String) {
        let record = self.get_data_mut(agent);
        let (private_name, added) = record.add_private_name(description);
        // Note: private fields should never overlap with one another.
        assert!(added);
        record
            .private_fields
            .push(PrivateField::Field { key: private_name });
    }

    /// Adds an instance PrivateName method to the current Private Environment.
    ///
    /// This method does not return the added PrivateName as the caller should
    /// be merely initialising the environment instead of initialising a class
    /// instance.
    pub(crate) fn add_instance_private_method(
        self,
        agent: &mut Agent,
        description: String,
        closure: PrivateMethod,
    ) {
        let record = self.get_data_mut(agent);
        let (private_name, added) = record.add_private_name(description);
        if added {
            let private_field = match closure {
                PrivateMethod::Getter(f) => PrivateField::Getter {
                    key: private_name,
                    get: f,
                },
                PrivateMethod::Setter(f) => PrivateField::Setter {
                    key: private_name,
                    set: f,
                },
                PrivateMethod::Method(f) => PrivateField::Method {
                    key: private_name,
                    method: f,
                },
            };
            record.private_fields.push(private_field.unbind());
            record.instance_private_field_count -= 1;
            record.instance_private_method_count += 1;
        } else {
            todo!("Support getter/setter PrivateName pairs");
        }
    }

    /// Adds a static PrivateName field to the current PrivateEnvironment.
    ///
    /// This method returns the added PrivateName as the caller should be
    /// in the process of initialising the constructor during this call.
    pub(crate) fn add_static_private_field(
        self,
        agent: &mut Agent,
        description: String,
    ) -> PrivateName {
        let record = self.get_data_mut(agent);
        let (private_name, added) = record.add_private_name(description);
        // Note: private fields should never overlap with one another.
        assert!(added);
        record.instance_private_field_count -= 1;
        record.static_private_field_count += 1;
        private_name
    }

    /// Adds a static PrivateName method to the current PrivateEnvironment.
    ///
    /// This method returns the added PrivateName as the caller should be
    /// in the process of initialising the constructor during this call.
    pub(crate) fn add_static_private_method(
        self,
        agent: &mut Agent,
        description: String,
    ) -> PrivateName {
        let record = self.get_data_mut(agent);
        let (private_name, added) = record.add_private_name(description);
        if added {
            record.instance_private_field_count -= 1;
        }
        private_name
    }
}

impl HeapMarkAndSweep for PrivateEnvironment<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.private_environments.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions
            .private_environments
            .shift_non_zero_u32_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for PrivateField<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Self::Field { .. } => {}
            PrivateField::Accessor { get, set, .. } => {
                get.mark_values(queues);
                set.mark_values(queues);
            }
            PrivateField::Getter { get: f, .. }
            | PrivateField::Setter { set: f, .. }
            | PrivateField::Method { method: f, .. } => f.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Self::Field { .. } => {}
            PrivateField::Accessor { get, set, .. } => {
                get.sweep_values(compactions);
                set.sweep_values(compactions);
            }
            PrivateField::Getter { get: f, .. }
            | PrivateField::Setter { set: f, .. }
            | PrivateField::Method { method: f, .. } => f.sweep_values(compactions),
        }
    }
}

impl HeapMarkAndSweep for PrivateEnvironmentRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            outer_private_environment,
            names,
            private_fields: private_methods,
            first_private_name: _,
            instance_private_field_count: _,
            instance_private_method_count: _,
            static_private_field_count: _,
        } = self;
        outer_private_environment.mark_values(queues);
        for key in names.keys() {
            key.mark_values(queues);
        }
        for func in private_methods {
            func.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            outer_private_environment,
            names,
            private_fields: private_methods,
            first_private_name: _,
            instance_private_field_count: _,
            instance_private_method_count: _,
            static_private_field_count: _,
        } = self;
        outer_private_environment.sweep_values(compactions);
        for func in private_methods {
            func.sweep_values(compactions);
        }
        let mut replacements = Vec::new();
        // Sweep all binding values, while also sweeping keys and making note
        // of all changes in them: Those need to be updated in a separate loop.
        for (key, _) in names.iter_mut() {
            if let String::String(old_key) = key {
                let old_key = *old_key;
                let mut new_key = old_key;
                new_key.sweep_values(compactions);
                if old_key != new_key {
                    replacements.push((old_key, new_key));
                }
            }
        }
        // Note: Replacement keys are in indeterminate order, we need to sort
        // them so that "cascading" replacements are applied in the correct
        // order.
        replacements.sort();
        for (old_key, new_key) in replacements.into_iter() {
            let binding = names.remove(&old_key.into()).unwrap();
            let did_insert = names.insert(new_key.into(), binding).is_none();
            assert!(did_insert, "Failed to insert PrivateName {new_key:#?}");
        }
    }
}
