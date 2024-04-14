use crate::ecmascript::{
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{Object, String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct ProxyConstructor;
impl Builtin for ProxyConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Map;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(ProxyConstructor::behaviour);
}

struct ProxyRevocable;
impl Builtin for ProxyRevocable {
    const NAME: String = BUILTIN_STRING_MEMORY.revocable;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ProxyConstructor::revocable);
}

impl ProxyConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        _new_target: Option<Object>,
    ) -> JsResult<Value> {
        todo!()
    }

    fn revocable(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.proxy();
        let this_object_index = intrinsics.proxy_base_object();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ProxyConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(1)
        .with_builtin_function_property::<ProxyRevocable>()
        .build();
    }
}
