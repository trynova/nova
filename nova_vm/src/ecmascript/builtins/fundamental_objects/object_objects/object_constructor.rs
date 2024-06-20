use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                define_property_or_throw, get, has_own_property,
                integrity::{Frozen, Sealed},
                set_integrity_level,
            },
            testing_and_comparison::{require_object_coercible, same_value},
            type_conversion::{to_object, to_property_key},
        },
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ordinary::{ordinary_create_from_constructor, ordinary_object_create_with_intrinsics},
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{
            InternalMethods, IntoObject, IntoValue, Object, OrdinaryObject, PropertyDescriptor,
            String, Value, BUILTIN_STRING_MEMORY,
        },
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct ObjectConstructor;

impl Builtin for ObjectConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Object;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}
impl BuiltinIntrinsicConstructor for ObjectConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::Object;
}

struct ObjectAssign;

impl Builtin for ObjectAssign {
    const NAME: String = BUILTIN_STRING_MEMORY.assign;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::assign);
}

struct ObjectCreate;

impl Builtin for ObjectCreate {
    const NAME: String = BUILTIN_STRING_MEMORY.create;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::create);
}
struct ObjectDefineProperties;

impl Builtin for ObjectDefineProperties {
    const NAME: String = BUILTIN_STRING_MEMORY.defineProperties;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_properties);
}
struct ObjectDefineProperty;

impl Builtin for ObjectDefineProperty {
    const NAME: String = BUILTIN_STRING_MEMORY.defineProperty;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::define_property);
}
struct ObjectEntries;

impl Builtin for ObjectEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.entries;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::entries);
}
struct ObjectFreeze;

impl Builtin for ObjectFreeze {
    const NAME: String = BUILTIN_STRING_MEMORY.freeze;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::freeze);
}
struct ObjectFromEntries;

impl Builtin for ObjectFromEntries {
    const NAME: String = BUILTIN_STRING_MEMORY.fromEntries;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::from_entries);
}
struct ObjectGetOwnPropertyDescriptor;

impl Builtin for ObjectGetOwnPropertyDescriptor {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptor;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_descriptor);
}
struct ObjectGetOwnPropertyDescriptors;

impl Builtin for ObjectGetOwnPropertyDescriptors {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyDescriptors;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Regular(ObjectConstructor::get_own_property_descriptors);
}
struct ObjectGetOwnPropertyNames;

impl Builtin for ObjectGetOwnPropertyNames {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertyNames;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_names);
}
struct ObjectGetOwnPropertySymbols;

impl Builtin for ObjectGetOwnPropertySymbols {
    const NAME: String = BUILTIN_STRING_MEMORY.getOwnPropertySymbols;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_own_property_symbols);
}
struct ObjectGetPrototypeOf;

impl Builtin for ObjectGetPrototypeOf {
    const NAME: String = BUILTIN_STRING_MEMORY.getPrototypeOf;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::get_prototype_of);
}
struct ObjectGroupBy;

impl Builtin for ObjectGroupBy {
    const NAME: String = BUILTIN_STRING_MEMORY.groupBy;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::group_by);
}
struct ObjectHasOwn;

impl Builtin for ObjectHasOwn {
    const NAME: String = BUILTIN_STRING_MEMORY.hasOwn;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::has_own);
}
struct ObjectIs;

impl Builtin for ObjectIs {
    const NAME: String = BUILTIN_STRING_MEMORY.is;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is);
}
struct ObjectIsExtensible;

impl Builtin for ObjectIsExtensible {
    const NAME: String = BUILTIN_STRING_MEMORY.isExtensible;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_extensible);
}
struct ObjectIsFrozen;

impl Builtin for ObjectIsFrozen {
    const NAME: String = BUILTIN_STRING_MEMORY.isFrozen;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_frozen);
}
struct ObjectIsSealed;

impl Builtin for ObjectIsSealed {
    const NAME: String = BUILTIN_STRING_MEMORY.isSealed;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::is_sealed);
}
struct ObjectKeys;

impl Builtin for ObjectKeys {
    const NAME: String = BUILTIN_STRING_MEMORY.keys;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::keys);
}
struct ObjectPreventExtensions;

impl Builtin for ObjectPreventExtensions {
    const NAME: String = BUILTIN_STRING_MEMORY.preventExtensions;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::prevent_extensions);
}

struct ObjectSeal;

impl Builtin for ObjectSeal {
    const NAME: String = BUILTIN_STRING_MEMORY.seal;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::seal);
}
struct ObjectSetPrototypeOf;

impl Builtin for ObjectSetPrototypeOf {
    const NAME: String = BUILTIN_STRING_MEMORY.setPrototypeOf;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectConstructor::set_prototype_of);
}
struct ObjectValues;

impl Builtin for ObjectValues {
    const NAME: String = BUILTIN_STRING_MEMORY.values;

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
            )
            .map(|value| value.into_value())
        } else if value == Value::Undefined || value == Value::Null {
            // 2. If value is either undefined or null, return OrdinaryObjectCreate(%Object.prototype%).
            Ok(
                ordinary_object_create_with_intrinsics(agent, Some(ProtoIntrinsics::Object), None)
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
            agent.heap.create_null_object(&[])
        } else if let Ok(o) = Object::try_from(o) {
            agent.heap.create_object_with_prototype(o, &[])
        } else {
            return Err(agent.throw_exception(ExceptionType::TypeError, "fail"));
        };
        let properties = arguments.get(1);
        if properties != Value::Undefined {
            object_define_properties(agent, obj, properties)?;
        }
        Ok(obj.into_value())
    }

    /// ### [20.1.2.3 Object.defineProperties ( O, Properties )](https://tc39.es/ecma262/#sec-object.defineproperties)
    ///
    /// This function adds own properties and/or updates the attributes of
    /// existing own properties of an object.
    fn define_properties(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let properties = arguments.get(1);
        // 1. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Argument is not an object")
            );
        };
        // 2. Return ? ObjectDefineProperties(O, Properties).
        let result = object_define_properties(agent, o, properties)?;
        Ok(result.into_value())
    }

    /// ### [20.1.2.4 Object.defineProperty ( O, P, Attributes )](https://tc39.es/ecma262/#sec-object.defineproperty)
    ///
    /// This function adds an own property and/or updates the attributes of an
    /// existing own property of an object.
    fn define_property(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let p = arguments.get(1);
        let attributes = arguments.get(2);
        // 1. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            return Err(
                agent.throw_exception(ExceptionType::TypeError, "Argument is not an object")
            );
        };
        // 2. Let key be ? ToPropertyKey(P).
        let key = to_property_key(agent, p)?;
        // 3. Let desc be ? ToPropertyDescriptor(Attributes).
        let desc = PropertyDescriptor::to_property_descriptor(agent, attributes)?;
        // 4. Perform ? DefinePropertyOrThrow(O, key, desc).
        define_property_or_throw(agent, o, key, desc)?;
        // 5. Return O.
        Ok(o.into_value())
    }

    fn entries(
        _agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    /// ### [20.1.2.6 Object.freeze ( O )](https://tc39.es/ecma262/#sec-object.freeze)
    fn freeze(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 2. Let status be ? SetIntegrityLevel(O, FROZEN).
        let status = set_integrity_level::<Frozen>(agent, o)?;
        if !status {
            // 3. If status is false, throw a TypeError exception.
            Err(agent.throw_exception(ExceptionType::TypeError, "Could not freeze object"))
        } else {
            // 4. Return O.
            Ok(o.into_value())
        }
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
        obj.internal_get_prototype_of(agent)
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

    fn is_extensible(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        let result = o.internal_is_extensible(agent)?;
        Ok(result.into())
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

    /// ### [20.1.2.20 Object.preventExtensions ( O )](https://tc39.es/ecma262/#sec-object.preventextensions)
    fn prevent_extensions(
        agent: &mut Agent,
        _: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 2. Let status be ? O.[[PreventExtensions]]().
        let status = o.internal_prevent_extensions(agent)?;
        // 3. If status is false, throw a TypeError exception.
        if !status {
            Err(agent.throw_exception(ExceptionType::TypeError, "Could not prevent extensions"))
        } else {
            // 4. Return O.
            Ok(o.into_value())
        }
    }

    /// ### [20.1.2.22 Object.seal ( O )](https://tc39.es/ecma262/#sec-object.seal)
    fn seal(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        // 1. If O is not an Object, return O.
        let o = arguments.get(0);
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 2. Let status be ? SetIntegrityLevel(O, SEALED).
        let status = set_integrity_level::<Sealed>(agent, o)?;
        if !status {
            // 3. If status is false, throw a TypeError exception.
            Err(agent.throw_exception(ExceptionType::TypeError, "Could not seal object"))
        } else {
            // 4. Return O.
            Ok(o.into_value())
        }
    }

    /// ### [20.1.2.23 Object.setPrototypeOf ( O, proto )](https://tc39.es/ecma262/#sec-object.setprototypeof)
    fn set_prototype_of(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let o = arguments.get(0);
        let proto = arguments.get(0);
        // 1. Set O to ? RequireObjectCoercible(O).
        let o = require_object_coercible(agent, o)?;
        // 2. If proto is not an Object and proto is not null, throw a TypeError exception.
        let proto = if let Ok(proto) = Object::try_from(proto) {
            Some(proto)
        } else if proto.is_null() {
            None
        } else {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Invalid prototype"));
        };
        // 3. If O is not an Object, return O.
        let Ok(o) = Object::try_from(o) else {
            return Ok(o);
        };
        // 4. Let status be ? O.[[SetPrototypeOf]](proto).
        let status = o.internal_set_prototype_of(agent, proto)?;
        // 5. If status is false, throw a TypeError exception.
        if !status {
            return Err(agent.throw_exception(ExceptionType::TypeError, "Could not set prototype"));
        }
        // 6. Return O.
        Ok(o.into_value())
    }

    fn values(_agent: &mut Agent, _this_value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        Ok(arguments.get(0))
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ObjectConstructor>(agent, realm)
            .with_property_capacity(24)
            .with_builtin_function_property::<ObjectAssign>()
            .with_builtin_function_property::<ObjectCreate>()
            .with_builtin_function_property::<ObjectDefineProperties>()
            .with_builtin_function_property::<ObjectDefineProperty>()
            .with_builtin_function_property::<ObjectEntries>()
            .with_builtin_function_property::<ObjectFreeze>()
            .with_builtin_function_property::<ObjectFromEntries>()
            .with_builtin_function_property::<ObjectGetOwnPropertyDescriptor>()
            .with_builtin_function_property::<ObjectGetOwnPropertyDescriptors>()
            .with_builtin_function_property::<ObjectGetOwnPropertyNames>()
            .with_builtin_function_property::<ObjectGetOwnPropertySymbols>()
            .with_builtin_function_property::<ObjectGetPrototypeOf>()
            .with_builtin_function_property::<ObjectGroupBy>()
            .with_builtin_function_property::<ObjectHasOwn>()
            .with_builtin_function_property::<ObjectIs>()
            .with_builtin_function_property::<ObjectIsExtensible>()
            .with_builtin_function_property::<ObjectIsFrozen>()
            .with_builtin_function_property::<ObjectIsSealed>()
            .with_builtin_function_property::<ObjectKeys>()
            .with_builtin_function_property::<ObjectPreventExtensions>()
            .with_prototype_property(object_prototype.into_object())
            .with_builtin_function_property::<ObjectSeal>()
            .with_builtin_function_property::<ObjectSetPrototypeOf>()
            .with_builtin_function_property::<ObjectValues>()
            .build();
    }
}

/// ### [20.1.2.3.1 ObjectDefineProperties ( O, Properties )](https://tc39.es/ecma262/#sec-objectdefineproperties)
///
/// The abstract operation ObjectDefineProperties takes arguments O (an Object)
/// and Properties (an ECMAScript language value) and returns either a normal
/// completion containing an Object or a throw completion.
fn object_define_properties<T: IntoObject>(
    agent: &mut Agent,
    o: T,
    properties: Value,
) -> JsResult<T> {
    // 1. Let props be ? ToObject(Properties).
    let props = to_object(agent, properties)?;
    // 2. Let keys be ? props.[[OwnPropertyKeys]]().
    let keys = props.internal_own_property_keys(agent)?;
    // 3. Let descriptors be a new empty List.
    let mut descriptors = Vec::with_capacity(keys.len());
    // 4. For each element nextKey of keys, do
    for next_key in keys {
        // a. Let propDesc be ? props.[[GetOwnProperty]](nextKey).
        let prop_desc = props.internal_get_own_property(agent, next_key)?;
        // b. If propDesc is not undefined and propDesc.[[Enumerable]] is true, then
        let Some(prop_desc) = prop_desc else {
            continue;
        };
        if prop_desc.enumerable != Some(true) {
            continue;
        }
        // i. Let descObj be ? Get(props, nextKey).
        let desc_obj = get(agent, props, next_key)?;
        // ii. Let desc be ? ToPropertyDescriptor(descObj).
        let desc = PropertyDescriptor::to_property_descriptor(agent, desc_obj)?;
        // iii. Append the Record { [[Key]]: nextKey, [[Descriptor]]: desc } to descriptors.
        descriptors.push((next_key, desc));
    }
    // 5. For each element property of descriptors, do
    let o_obj = o.into_object();
    for (property_key, property_descriptor) in descriptors {
        // a. Perform ? DefinePropertyOrThrow(O, property.[[Key]], property.[[Descriptor]]).
        define_property_or_throw(agent, o_obj, property_key, property_descriptor)?;
    }
    // 6. Return O.
    Ok(o)
}
