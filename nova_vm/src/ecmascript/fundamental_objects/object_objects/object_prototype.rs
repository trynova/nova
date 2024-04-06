use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{get, has_own_property, invoke},
            testing_and_comparison::same_value,
            type_conversion::{to_object, to_property_key},
        },
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{InternalMethods, IntoValue, Object, PropertyKey, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct ObjectPrototype;

struct ObjectPrototypeHasOwnProperty;
impl Builtin for ObjectPrototypeHasOwnProperty {
    const NAME: &'static str = "hasOwnProperty";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::has_own_property);
}

struct ObjectPrototypeIsPrototypeOf;
impl Builtin for ObjectPrototypeIsPrototypeOf {
    const NAME: &'static str = "isPrototypeOf";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::is_prototype_of);
}

struct ObjectPrototypePropertyIsEnumerable;
impl Builtin for ObjectPrototypePropertyIsEnumerable {
    const NAME: &'static str = "propertyIsEnumerable";

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::property_is_enumerable);
}

struct ObjectPrototypeToLocaleString;
impl Builtin for ObjectPrototypeToLocaleString {
    const NAME: &'static str = "toLocaleString";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::to_locale_string);
}

struct ObjectPrototypeToString;
impl Builtin for ObjectPrototypeToString {
    const NAME: &'static str = "toString";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::to_string);
}

struct ObjectPrototypeValueOf;
impl Builtin for ObjectPrototypeValueOf {
    const NAME: &'static str = "valueOf";

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ObjectPrototype::value_of);
}

impl ObjectPrototype {
    fn has_own_property(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let p = to_property_key(agent, arguments.get(0))?;
        let o = to_object(agent, this_value)?;
        has_own_property(agent, o, p).map(|result| result.into())
    }

    fn is_prototype_of(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let v = arguments.get(0);
        let Ok(mut v) = Object::try_from(v) else {
            return Ok(false.into());
        };
        let o = to_object(agent, this_value)?;
        loop {
            let proto = v.get_prototype_of(agent)?;
            if let Some(proto) = proto {
                v = proto;
                if same_value(agent, o, v) {
                    return Ok(true.into());
                }
            } else {
                return Ok(false.into());
            }
        }
    }

    fn property_is_enumerable(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let p = to_property_key(agent, arguments.get(0))?;
        let o = to_object(agent, this_value)?;
        let desc = o.get_own_property(agent, p)?;
        if let Some(desc) = desc {
            Ok(desc.enumerable.unwrap_or(false).into())
        } else {
            Ok(false.into())
        }
    }

    fn to_locale_string(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        let o = this_value;
        let p = PropertyKey::from_str(&mut agent.heap, "toString");
        invoke(agent, o, p, None)
    }

    fn to_string(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        match this_value {
            // 1. If the this value is undefined, return "[object Undefined]".
            Value::Undefined => Ok(Value::from_str(&mut agent.heap, "[object Undefined]")),
            // 2. If the this value is null, return "[object Null]".
            Value::Null => Ok(Value::from_str(&mut agent.heap, "[object Null]")),
            // 9. Else if O has a [[BooleanData]] internal slot, let builtinTag be "Boolean".
            // 17. Return the string-concatenation of "[object ", tag, and "]".
            Value::Boolean(_) => Ok(Value::from_str(&mut agent.heap, "[object Boolean]")),
            // 6. Else if O has a [[ParameterMap]] internal slot, let builtinTag be "Arguments".
            // 11. Else if O has a [[StringData]] internal slot, let builtinTag be "String".
            Value::String(_) | Value::SmallString(_) => {
                Ok(Value::from_str(&mut agent.heap, "[object String]"))
            }
            // 10. Else if O has a [[NumberData]] internal slot, let builtinTag be "Number".
            Value::Number(_) | Value::Integer(_) | Value::Float(_) => {
                Ok(Value::from_str(&mut agent.heap, "[object Error]"))
            }
            Value::Object(_) => todo!(),
            // 4. Let isArray be ? IsArray(O).
            // 5. If isArray is true, let builtinTag be "Array".
            Value::Array(_) => Ok(Value::from_str(&mut agent.heap, "[object Array]")),
            // 12. Else if O has a [[DateValue]] internal slot, let builtinTag be "Date".
            Value::Date(_) => Ok(Value::from_str(&mut agent.heap, "[object Date]")),
            // 8. Else if O has an [[ErrorData]] internal slot, let builtinTag be "Error".
            Value::Error(_) => Ok(Value::from_str(&mut agent.heap, "[object Error]")),
            // 7. Else if O has a [[Call]] internal method, let builtinTag be "Function".
            Value::BoundFunction(_) | Value::BuiltinFunction(_) | Value::ECMAScriptFunction(_) => {
                Ok(Value::from_str(&mut agent.heap, "[object Function]"))
            }
            // 13. Else if O has a [[RegExpMatcher]] internal slot, let builtinTag be "RegExp".
            Value::RegExp(_) => Ok(Value::from_str(&mut agent.heap, "[object RegExp]")),
            Value::Symbol(_) | Value::BigInt(_) | Value::SmallBigInt(_) | Value::ArrayBuffer(_) => {
                // 14. Else, let builtinTag be "Object".
                // 3. Let O be ! ToObject(this value).
                // 15. Let tag be ? Get(O, @@toStringTag).
                // 16. If tag is not a String, set tag to builtinTag.
                let builtin_tag = String::from_str(agent, "Object");
                let o = to_object(agent, this_value).unwrap();
                let tag = get(agent, o, WellKnownSymbolIndexes::ToStringTag.into())?;
                if let Ok(tag) = String::try_from(tag) {
                    let str = format!("[object {}]", tag.as_str(agent).unwrap());
                    return Ok(Value::from_str(&mut agent.heap, str.as_str()));
                } else {
                    let str = format!("[object {}]", builtin_tag.as_str(agent).unwrap());
                    return Ok(Value::from_str(&mut agent.heap, str.as_str()));
                }
            }
        }
    }

    fn value_of(
        agent: &mut Agent,
        this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        to_object(agent, this_value).map(|result| result.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        // The Object prototype object:
        let intrinsics = agent.get_realm(realm).intrinsics();
        // is %Object.prototype%.
        let this = intrinsics.object_prototype();
        let _to_string_index = intrinsics.object_prototype_to_string();
        let object_constructor = intrinsics.object();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            // has an [[Extensible]] internal slot whose value is true.
            .with_extensible(true)
            // has a [[Prototype]] internal slot whose value is null.
            // .with_prototype(None)
            .with_property_capacity(7)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key_from_str("constructor")
                    .with_value(object_constructor.into_value())
                    .build()
            })
            .with_builtin_function_property::<ObjectPrototypeHasOwnProperty>()
            .with_builtin_function_property::<ObjectPrototypeIsPrototypeOf>()
            .with_builtin_function_property::<ObjectPrototypePropertyIsEnumerable>()
            .with_builtin_function_property::<ObjectPrototypeToLocaleString>()
            .with_builtin_function_property::<ObjectPrototypeToString>()
            .with_builtin_function_property::<ObjectPrototypeValueOf>()
            .build();
    }
}
