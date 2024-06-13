use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct ReflectObject;

struct ReflectObjectApply;
impl Builtin for ReflectObjectApply {
    const NAME: String = BUILTIN_STRING_MEMORY.apply;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::apply);
}

struct ReflectObjectConstruct;
impl Builtin for ReflectObjectConstruct {
    const NAME: String = BUILTIN_STRING_MEMORY.construct;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::construct);
}
struct ReflectObjectDefineProperty;
impl Builtin for ReflectObjectDefineProperty {
    const NAME: String = BUILTIN_STRING_MEMORY.defineProperty;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::define_property);
}
struct ReflectObjectDeleteProperty;
impl Builtin for ReflectObjectDeleteProperty {
    const NAME: String = BUILTIN_STRING_MEMORY.deleteProperty;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::delete_property);
}
struct ReflectObjectGet;
impl Builtin for ReflectObjectGet {
    const NAME: String = BUILTIN_STRING_MEMORY.get;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::get);
}
struct ReflectObjectGetOwnPropertyDescriptor;
impl Builtin for ReflectObjectGetOwnPropertyDescriptor {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::get_own_property_descriptor);
}
struct ReflectObjectGetPrototypeOf;
impl Builtin for ReflectObjectGetPrototypeOf {
    const NAME: String = BUILTIN_STRING_MEMORY.getPrototypeOf;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::get_prototype_of);
}

struct ReflectObjectHas;
impl Builtin for ReflectObjectHas {
    const NAME: String = BUILTIN_STRING_MEMORY.has;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::has);
}
struct ReflectObjectIsExtensible;
impl Builtin for ReflectObjectIsExtensible {
    const NAME: String = BUILTIN_STRING_MEMORY.isExtensible;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::is_extensible);
}
struct ReflectObjectOwnKeys;
impl Builtin for ReflectObjectOwnKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.ownKeys;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::own_keys);
}
struct ReflectObjectPreventExtensions;
impl Builtin for ReflectObjectPreventExtensions {
    const NAME: String = BUILTIN_STRING_MEMORY.preventExtensions;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::prevent_extensions);
}
struct ReflectObjectSet;
impl Builtin for ReflectObjectSet {
    const NAME: String = BUILTIN_STRING_MEMORY.set;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::set);
}
struct ReflectObjectSetPrototypeOf;
impl Builtin for ReflectObjectSetPrototypeOf {
    const NAME: String = BUILTIN_STRING_MEMORY.setPrototypeOf;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: crate::ecmascript::builtins::Behaviour =
        crate::ecmascript::builtins::Behaviour::Regular(ReflectObject::set_prototype_of);
}

impl ReflectObject {
    fn apply(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn construct(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn define_property(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn delete_property(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn get(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn get_own_property_descriptor(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn get_prototype_of(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn has(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn is_extensible(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn own_keys(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn prevent_extensions(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    fn set(_agent: &mut Agent, _this_value: Value, _arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }

    fn set_prototype_of(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.reflect();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(14)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<ReflectObjectApply>()
            .with_builtin_function_property::<ReflectObjectConstruct>()
            .with_builtin_function_property::<ReflectObjectDefineProperty>()
            .with_builtin_function_property::<ReflectObjectDeleteProperty>()
            .with_builtin_function_property::<ReflectObjectGet>()
            .with_builtin_function_property::<ReflectObjectGetOwnPropertyDescriptor>()
            .with_builtin_function_property::<ReflectObjectGetPrototypeOf>()
            .with_builtin_function_property::<ReflectObjectHas>()
            .with_builtin_function_property::<ReflectObjectIsExtensible>()
            .with_builtin_function_property::<ReflectObjectOwnKeys>()
            .with_builtin_function_property::<ReflectObjectPreventExtensions>()
            .with_builtin_function_property::<ReflectObjectSet>()
            .with_builtin_function_property::<ReflectObjectSetPrototypeOf>()
            .with_property(|builder| {
                builder
                    .with_key(BUILTIN_STRING_MEMORY.Reflect.into())
                    .with_value_readonly(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
