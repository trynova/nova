// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::builtins::BuiltinGetter;
use crate::ecmascript::builtins::BuiltinIntrinsicConstructor;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;

use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::engine::context::GcScope;
use crate::heap::IntrinsicConstructorIndexes;
use crate::heap::WellKnownSymbolIndexes;

pub struct RegExpConstructor;

impl Builtin for RegExpConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.RegExp;
}
impl BuiltinIntrinsicConstructor for RegExpConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::RegExp;
}

struct RegExpGetSpecies;
impl Builtin for RegExpGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey> = Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for RegExpGetSpecies {}

impl RegExpConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _gc: GcScope<'_, '_>,

        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!();
    }

    fn get_species(
        _: &mut Agent,
        _gc: GcScope<'_, '_>,

        this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let regexp_prototype = intrinsics.reg_exp_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<RegExpConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype_property(regexp_prototype.into_object())
            .with_builtin_function_getter_property::<RegExpGetSpecies>()
            .build();
    }
}
