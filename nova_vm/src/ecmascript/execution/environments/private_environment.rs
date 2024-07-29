// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashMap;

use crate::{
    ecmascript::types::{Function, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::PrivateEnvironmentIndex;

#[derive(Debug)]
pub enum PrivateName<'gen> {
    Field(Option<Value<'gen>>),
    Method(Option<Function<'gen>>),
    /// Accessor(get, set)
    Accessor(Option<Function<'gen>>, Option<Function<'gen>>),
}

impl PrivateName<'_> {
    pub fn description(&self) -> &'static str {
        "identifier"
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
pub struct PrivateEnvironment<'gen> {
    /// ### \[\[OuterPrivateEnvironment\]\]
    ///
    /// The PrivateEnvironment Record of the nearest containing class. null if
    /// the class with which this PrivateEnvironment Record is associated is
    /// not contained in any other class.
    pub(crate) outer_private_environment: Option<PrivateEnvironmentIndex<'gen>>,

    /// ### \[\[Names\]\]
    ///
    /// The Private Names declared by this class.
    pub(crate) names: AHashMap<String, PrivateName<'gen>>,
}

impl<'gen> HeapMarkAndSweep<'gen> for PrivateEnvironment<'gen> {
    fn mark_values(&self, _queues: &mut WorkQueues<'gen>) {
        todo!()
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        todo!()
    }
}

/// ### [9.2.1.1 NewPrivateEnvironment ( outerPrivEnv )](https://tc39.es/ecma262/#sec-newprivateenvironment)
///
/// The abstract operation NewPrivateEnvironment takes argument outerPrivEnv (a
/// PrivateEnvironment Record or null) and returns a PrivateEnvironment Record.
pub(crate) fn new_private_environment<'gen>(
    outer_private_environment: Option<PrivateEnvironmentIndex<'gen>>,
) -> PrivateEnvironment<'gen> {
    // 1. Let names be a new empty List.
    // 2. Return the PrivateEnvironment Record {
    PrivateEnvironment {
        // [[OuterPrivateEnvironment]]: outerPrivEnv,
        outer_private_environment,
        // [[Names]]: names
        names: Default::default(),
    }
    // }.
}

impl<'gen> HeapMarkAndSweep<'gen> for PrivateEnvironmentIndex<'gen> {
    fn mark_values(&self, _queues: &mut WorkQueues<'gen>) {
        todo!()
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        todo!()
    }
}
