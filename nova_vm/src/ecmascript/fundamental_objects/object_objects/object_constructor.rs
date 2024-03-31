use crate::ecmascript::{
    abstract_operations::{
        operations_on_objects::has_own_property,
        testing_and_comparison::same_value,
        type_conversion::{to_object, to_property_key},
    },
    builders::builtin_function_builder::BuiltinFunctionBuilder,
    builtins::{
        ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
        ArgumentsList, Behaviour, Builtin,
    },
    execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
    types::{InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject, Value},
};

pub(crate) struct ObjectConstructor;

impl Builtin for ObjectConstructor {
    const NAME: &'static str = "Object";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}

struct ObjectAssign;

impl Builtin for ObjectAssign {
    const NAME: &'static str = "Object";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::assign);
}

struct ObjectCreate;

impl Builtin for ObjectCreate {
    const NAME: &'static str = "create";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::create);
}
struct ObjectDefineProperties;

impl Builtin for ObjectDefineProperties {
    const NAME: &'static str = "defineProperties";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_properties);
}
struct ObjectDefineProperty;

impl Builtin for ObjectDefineProperty {
    const NAME: &'static str = "defineProperty";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_property);
}
struct ObjectEntries;

impl Builtin for ObjectEntries {
    const NAME: &'static str = "entries";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::entries);
}
struct ObjectFreeze;

impl Builtin for ObjectFreeze {
    const NAME: &'static str = "freeze";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::freeze);
}
struct ObjectFromEntries;

impl Builtin for ObjectFromEntries {
    const NAME: &'static str = "fromEntries";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::from_entries);
}
struct ObjectGetOwnPropertyDescriptor;

impl Builtin for ObjectGetOwnPropertyDescriptor {
    const NAME: &'static str = "getOwnPropertyDescriptor";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_descriptor);
}
struct ObjectGetOwnPropertyDescriptors;

impl Builtin for ObjectGetOwnPropertyDescriptors {
    const NAME: &'static str = "getOwnPropertyDescriptors";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(ObjectConstructor::get_own_property_descriptors);
}
struct ObjectGetOwnPropertyNames;

impl Builtin for ObjectGetOwnPropertyNames {
    const NAME: &'static str = "getOwnPropertyNames";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_names);
}
struct ObjectGetOwnPropertySymbols;

impl Builtin for ObjectGetOwnPropertySymbols {
    const NAME: &'static str = "getOwnPropertySymbols";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_symbols);
}
struct ObjectGetPrototypeOf;

impl Builtin for ObjectGetPrototypeOf {
    const NAME: &'static str = "getPrototypeOf";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_prototype_of);
}
struct ObjectGroupBy;

impl Builtin for ObjectGroupBy {
    const NAME: &'static str = "groupBy";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::group_by);
}
struct ObjectHasOwn;

impl Builtin for ObjectHasOwn {
    const NAME: &'static str = "hasOwn";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::has_own);
}
struct ObjectIs;

impl Builtin for ObjectIs {
    const NAME: &'static str = "is";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is);
}
struct ObjectIsExtensible;

impl Builtin for ObjectIsExtensible {
    const NAME: &'static str = "isExtensible";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_extensible);
}
struct ObjectIsFrozen;

impl Builtin for ObjectIsFrozen {
    const NAME: &'static str = "isFrozen";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_frozen);
}
struct ObjectIsSealed;

impl Builtin for ObjectIsSealed {
    const NAME: &'static str = "isSealed";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_sealed);
}
struct ObjectKeys;

impl Builtin for ObjectKeys {
    const NAME: &'static str = "keys";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::keys);
}
struct ObjectPreventExtensions;

impl Builtin for ObjectPreventExtensions {
    const NAME: &'static str = "preventExtensions";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::prevent_extensions);
}

struct ObjectSeal;

impl Builtin for ObjectSeal {
    const NAME: &'static str = "seal";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::seal);
}
struct ObjectSetPrototypeOf;

impl Builtin for ObjectSetPrototypeOf {
    const NAME: &'static str = "setPrototypeOf";

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::set_prototype_of);
}
struct ObjectValues;

impl Builtin for ObjectValues {
    const NAME: &'static str = "values";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::values);
}

impl ObjectConstructor {
    /// ### [20.1.1.1 Object ( \[ value \] )](https://tc39.es/ecma262/#sec-object-value)
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let value = arguments.get(0);
        // 1. If NewTarget is neither undefined nor the active function object, then
        if new_target.is_some()
            && new_target
                != agent
                    .running_execution_context()
                    .function
                    .map(|obj| obj.into_object())
        {
            // a. Return ? OrdinaryCreateFromConstructor(NewTarget, "%Object.prototype%").
            ordinary_create_from_constructor(
                agent,
                // SAFETY: 'new_target' is checked to be is_some() above
                unsafe { new_target.unwrap_unchecked() }.try_into().unwrap(),
                ProtoIntrinsics::Object,
                (),
            )
            .map(|value| value.into_value())
        } else if value == Value::Undefined || value == Value::Null {
            // 2. If value is either undefined or null, return OrdinaryObjectCreate(%Object.prototype%).
            Ok(
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object))
                    .into_value(),
            )
        } else {
            // 3. Return ! ToObject(value).
            Ok(to_object(agent, value).unwrap().into_value())
        }
    }

    fn assign(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn create(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let obj: OrdinaryObject = if o == Value::Null {
            agent.heap.create_null_object(vec![]).into()
        } else if let Ok(o) = Object::try_from(o) {
            agent.heap.create_object_with_prototype(o, vec![]).into()
        } else {
            return Err(agent.throw_exception(ExceptionType::TypeError, "fail"));
        };
        let properties = arguments.get(1);
        if properties != Value::Undefined {
            todo!("ObjectDefineProperties");
        }
        Ok(obj.into_value())
    }

    fn define_properties(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn define_property(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn entries(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn freeze(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn from_entries(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn get_own_property_descriptor(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn get_own_property_descriptors(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn get_own_property_names(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn get_own_property_symbols(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn get_prototype_of(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let obj = to_object(agent, arguments.get(0))?;
        obj.get_prototype_of(agent)
            .map(|proto| proto.map_or(Value::Null, |proto| proto.into_value()))
    }

    fn group_by(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn has_own(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let obj = to_object(agent, arguments.get(0))?;
        let key = to_property_key(agent, arguments.get(1))?;
        has_own_property(agent, obj, key).map(|result| result.into())
    }

    fn is(agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(same_value(agent, arguments.get(0), arguments.get(1)).into())
    }

    fn is_extensible(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn is_frozen(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn is_sealed(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn keys(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn prevent_extensions(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn seal(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn set_prototype_of(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    fn values(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.object();
        let this_object_index = intrinsics.object_base_object();
        let object_prototype = intrinsics.object_prototype();
        let _to_string_index = intrinsics.object_prototype_to_string();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ObjectConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(25)
        .with_property(|builder| {
            builder
                .with_key_from_str("assign")
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectAssign>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectCreate::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectCreate>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectDefineProperties::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectDefineProperties>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectDefineProperty::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectDefineProperty>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectEntries::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectEntries>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectFreeze::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectFreeze>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectFromEntries::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectFromEntries>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectGetOwnPropertyDescriptor::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectGetOwnPropertyDescriptor>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectGetOwnPropertyDescriptors::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectGetOwnPropertyDescriptors>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectGetOwnPropertyNames::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectGetOwnPropertyNames>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectGetOwnPropertySymbols::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectGetOwnPropertySymbols>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectGetPrototypeOf::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectGetPrototypeOf>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectGroupBy::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectGroupBy>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectHasOwn::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectHasOwn>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectIs::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectIs>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectIsExtensible::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectIsExtensible>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectIsFrozen::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectIsFrozen>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectIsSealed::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectIsSealed>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectKeys::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectKeys>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectPreventExtensions::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectPreventExtensions>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str("prototype")
                .with_value_readonly(object_prototype.into_value())
                .with_enumerable(false)
                .with_configurable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectSeal::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectSeal>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectSetPrototypeOf::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectSetPrototypeOf>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        })
        .with_property(|builder| {
            builder
                .with_key_from_str(ObjectValues::NAME)
                .with_value_creator(|agent| {
                    BuiltinFunctionBuilder::new::<ObjectValues>(agent, realm)
                        .build()
                        .into_value()
                })
                .with_enumerable(false)
                .build()
        });
    }
}
