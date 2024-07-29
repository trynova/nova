// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct AtomicsObject;

struct AtomicsObjectAdd;
impl Builtin for AtomicsObjectAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::add);
}

struct AtomicsObjectAnd;
impl Builtin for AtomicsObjectAnd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.and;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::and);
}
struct AtomicsObjectCompareExchange;
impl Builtin for AtomicsObjectCompareExchange {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.compareExchange;

    const LENGTH: u8 = 4;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::compare_exchange);
}
struct AtomicsObjectExchange;
impl Builtin for AtomicsObjectExchange {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.exchange;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::exchange);
}
struct AtomicsObjectIsLockFree;
impl Builtin for AtomicsObjectIsLockFree {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isLockFree;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::is_lock_free);
}
struct AtomicsObjectLoad;
impl Builtin for AtomicsObjectLoad {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.load;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::load);
}
struct AtomicsObjectOr;
impl Builtin for AtomicsObjectOr {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.or;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::or);
}
struct AtomicsObjectStore;
impl Builtin for AtomicsObjectStore {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.store;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::store);
}
struct AtomicsObjectSub;
impl Builtin for AtomicsObjectSub {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sub;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::sub);
}
struct AtomicsObjectWait;
impl Builtin for AtomicsObjectWait {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.wait;

    const LENGTH: u8 = 4;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::wait);
}
struct AtomicsObjectWaitAsync;
impl Builtin for AtomicsObjectWaitAsync {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.waitAsync;

    const LENGTH: u8 = 4;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::wait_async);
}
struct AtomicsObjectNotify;
impl Builtin for AtomicsObjectNotify {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.notify;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::notify);
}
struct AtomicsObjectXor;
impl Builtin for AtomicsObjectXor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.xor;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(AtomicsObject::xor);
}

impl AtomicsObject {
    fn add<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn and<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn compare_exchange<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn exchange<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn is_lock_free<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn load<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn or<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn store<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn sub<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn wait<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn wait_async<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn notify<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    fn xor<'gen>(_agent: &mut Agent<'gen>, _this_value: Value<'gen>, _arguments: ArgumentsList<'_, 'gen>) -> JsResult<'gen, Value<'gen>> {
        todo!();
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.atomics();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(14)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<AtomicsObjectAdd>()
            .with_builtin_function_property::<AtomicsObjectAnd>()
            .with_builtin_function_property::<AtomicsObjectCompareExchange>()
            .with_builtin_function_property::<AtomicsObjectExchange>()
            .with_builtin_function_property::<AtomicsObjectIsLockFree>()
            .with_builtin_function_property::<AtomicsObjectLoad>()
            .with_builtin_function_property::<AtomicsObjectOr>()
            .with_builtin_function_property::<AtomicsObjectStore>()
            .with_builtin_function_property::<AtomicsObjectSub>()
            .with_builtin_function_property::<AtomicsObjectWait>()
            .with_builtin_function_property::<AtomicsObjectWaitAsync>()
            .with_builtin_function_property::<AtomicsObjectNotify>()
            .with_builtin_function_property::<AtomicsObjectXor>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Atomics.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
