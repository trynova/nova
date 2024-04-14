use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct WeakSetConstructor;
impl Builtin for WeakSetConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(WeakSetConstructor::behaviour);
}

impl WeakSetConstructor {
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
        let weak_set_prototype = intrinsics.weak_set_prototype();
        let this = intrinsics.weak_set();
        let this_object_index = intrinsics.weak_set_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakSetConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(1)
        .with_prototype_property(weak_set_prototype.into_object())
        .build();
    }
}
