use crate::ecmascript::{
    builders::ordinary_object_builder::OrdinaryObjectBuilder,
    builtins::{ArgumentsList, Behaviour, Builtin},
    execution::{Agent, JsResult, RealmIdentifier},
    types::{String, Value, BUILTIN_STRING_MEMORY},
};

pub(crate) struct AsyncFromSyncIteratorPrototype;

struct AsyncFromSyncIteratorPrototypeNext;
impl Builtin for AsyncFromSyncIteratorPrototypeNext {
    const NAME: String = BUILTIN_STRING_MEMORY.next;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncFromSyncIteratorPrototype::next);
}
struct AsyncFromSyncIteratorPrototypeReturn;
impl Builtin for AsyncFromSyncIteratorPrototypeReturn {
    const NAME: String = BUILTIN_STRING_MEMORY.r#return;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncFromSyncIteratorPrototype::r#return);
}
struct AsyncFromSyncIteratorPrototypeThrow;
impl Builtin for AsyncFromSyncIteratorPrototypeThrow {
    const NAME: String = BUILTIN_STRING_MEMORY.throw;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(AsyncFromSyncIteratorPrototype::throw);
}

impl AsyncFromSyncIteratorPrototype {
    fn next(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn r#return(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    fn throw(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let async_iterator_prototype = intrinsics.async_iterator_prototype();
        let this = intrinsics.async_from_sync_iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(3)
            .with_prototype(async_iterator_prototype)
            .with_builtin_function_property::<AsyncFromSyncIteratorPrototypeNext>()
            .with_builtin_function_property::<AsyncFromSyncIteratorPrototypeReturn>()
            .with_builtin_function_property::<AsyncFromSyncIteratorPrototypeThrow>()
            .build();
    }
}
