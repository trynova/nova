use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakRefConstructor;
impl Builtin for WeakRefConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(WeakRefConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for WeakRefConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakRef;
}

impl WeakRefConstructor {
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
        let weak_ref_prototype = intrinsics.weak_ref_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakRefConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_ref_prototype.into_object())
            .build();
    }
}
