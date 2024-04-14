use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::IntoFunction;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::WellKnownSymbolIndexes;

pub struct RegExpConstructor;

impl Builtin for RegExpConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
    const LENGTH: u8 = 1;
    const NAME: String = BUILTIN_STRING_MEMORY.RegExp;
}

struct RegExpGetSpecies;
impl Builtin for RegExpGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String = BUILTIN_STRING_MEMORY.get__Symbol_species_;
}
impl RegExpConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!();
    }

    fn get_species(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let regexp_prototype = intrinsics.reg_exp_prototype();
        let this = intrinsics.reg_exp();
        let this_object_index = intrinsics.reg_exp_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<RegExpConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(2)
        .with_prototype_property(regexp_prototype.into_object())
        .with_property(|builder| {
            builder
                .with_key(WellKnownSymbolIndexes::Species.into())
                .with_getter(|agent| {
                    BuiltinFunctionBuilder::new::<RegExpGetSpecies>(agent, realm)
                        .build()
                        .into_function()
                })
                .with_enumerable(false)
                .with_configurable(true)
                .build()
        })
        .build();
    }
}
