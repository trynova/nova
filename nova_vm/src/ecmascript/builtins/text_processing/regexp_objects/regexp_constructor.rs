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
use crate::heap::IntrinsicConstructorIndexes;
use crate::heap::WellKnownSymbolIndexes;

pub struct RegExpConstructor;

impl Builtin for RegExpConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.RegExp;
}
impl BuiltinIntrinsicConstructor for RegExpConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::RegExp;
}

struct RegExpGetSpecies;
impl Builtin for RegExpGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> = Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for RegExpGetSpecies {}

impl RegExpConstructor {
    fn behaviour<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
        _new_target: Option<Object<'gen>>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn get_species(_: &mut Agent<'gen>, this_value: Value<'gen>, _: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        Ok(this_value)
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let regexp_prototype = intrinsics.reg_exp_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<RegExpConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype_property(regexp_prototype.into_object())
            .with_builtin_function_getter_property::<RegExpGetSpecies>()
            .build();
    }
}
