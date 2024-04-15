use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct AggregateErrorConstructor;
impl Builtin for AggregateErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.AggregateError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(AggregateErrorConstructor::behaviour);
}

impl AggregateErrorConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.aggregate_error();
        let this_base_object = intrinsics.aggregate_error_base_object();
        let aggregate_error_prototype = intrinsics.aggregate_error_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<AggregateErrorConstructor>(
            agent,
            realm,
            this,
            Some(this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(aggregate_error_prototype.into_object())
        .build();
    }
}
