// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;

use crate::{
    ecmascript::{
        builtins::ECMAScriptFunction,
        execution::Agent,
        types::{PrivateName, String},
    },
    engine::context::{Bindable, NoGcScope},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::PrivateEnvironment;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PrivateFieldType<'a> {
    Field,
    Getter(ECMAScriptFunction<'a>),
    Setter(ECMAScriptFunction<'a>),
    Accessor {
        get: ECMAScriptFunction<'a>,
        set: ECMAScriptFunction<'a>,
    },
    Method(ECMAScriptFunction<'a>),
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
    /// First Private Name in the record. The rest are in ascending numerical
    /// order, up to `first_private_name + names.size() - 1` in numeric value.
    first_private_name: PrivateName,
    // Private Name field types.
    // field_type: Box<[PrivateFieldType<'static>]>,
}

impl HeapMarkAndSweep for PrivateEnvironmentRecord {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            outer_private_environment,
            names,
            first_private_name: _,
        } = self;
        outer_private_environment.mark_values(queues);
        for key in names.keys() {
            key.mark_values(queues);
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            outer_private_environment,
            names,
            first_private_name: _,
        } = self;
        outer_private_environment.sweep_values(compactions);
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

/// ### [9.2.1.1 NewPrivateEnvironment ( outerPrivEnv )](https://tc39.es/ecma262/#sec-newprivateenvironment)
///
/// The abstract operation NewPrivateEnvironment takes argument outerPrivEnv (a
/// PrivateEnvironment Record or null) and returns a PrivateEnvironment Record.
pub(crate) fn new_private_environment(
    outer_private_environment: Option<PrivateEnvironment>,
) -> PrivateEnvironmentRecord {
    // 1. Let names be a new empty List.
    // 2. Return the PrivateEnvironment Record {
    PrivateEnvironmentRecord {
        // [[OuterPrivateEnvironment]]: outerPrivEnv,
        outer_private_environment: outer_private_environment.unbind(),
        // [[Names]]: names
        names: Default::default(),
        first_private_name: PrivateName::from_u32(0),
    }
    // }.
}

impl PrivateEnvironment<'_> {
    fn get_data(self, agent: &Agent) -> &PrivateEnvironmentRecord {
        &agent.heap.environments.private[self.into_index()]
            .as_ref()
            .unwrap()
    }

    pub(crate) fn get_outer_env<'a>(
        self,
        agent: &Agent,
        gc: NoGcScope<'a, '_>,
    ) -> Option<PrivateEnvironment<'a>> {
        self.get_data(agent).outer_private_environment.bind(gc)
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
                return Some(*description);
            }
        }
        unreachable!()
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
