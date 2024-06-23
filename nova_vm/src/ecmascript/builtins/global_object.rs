use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_number,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        execution::{Agent, JsResult, RealmIdentifier},
        types::{String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::IntrinsicFunctionIndexes,
};

use super::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsic};

pub(crate) struct GlobalObject;

struct GlobalObjectEval;
impl Builtin for GlobalObjectEval {
    const NAME: String = BUILTIN_STRING_MEMORY.eval;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::eval);
}
impl BuiltinIntrinsic for GlobalObjectEval {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::Eval;
}
struct GlobalObjectIsFinite;
impl Builtin for GlobalObjectIsFinite {
    const NAME: String = BUILTIN_STRING_MEMORY.isFinite;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::is_finite);
}
impl BuiltinIntrinsic for GlobalObjectIsFinite {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::IsFinite;
}
struct GlobalObjectIsNaN;
impl Builtin for GlobalObjectIsNaN {
    const NAME: String = BUILTIN_STRING_MEMORY.isNaN;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::is_nan);
}
impl BuiltinIntrinsic for GlobalObjectIsNaN {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::IsNaN;
}
struct GlobalObjectParseFloat;
impl Builtin for GlobalObjectParseFloat {
    const NAME: String = BUILTIN_STRING_MEMORY.parseFloat;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::parse_float);
}
impl BuiltinIntrinsic for GlobalObjectParseFloat {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ParseFloat;
}
struct GlobalObjectParseInt;
impl Builtin for GlobalObjectParseInt {
    const NAME: String = BUILTIN_STRING_MEMORY.parseInt;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::parse_int);
}
impl BuiltinIntrinsic for GlobalObjectParseInt {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::ParseInt;
}
struct GlobalObjectDecodeURI;
impl Builtin for GlobalObjectDecodeURI {
    const NAME: String = BUILTIN_STRING_MEMORY.decodeURI;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::decode_uri);
}
impl BuiltinIntrinsic for GlobalObjectDecodeURI {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::DecodeURI;
}
struct GlobalObjectDecodeURIComponent;
impl Builtin for GlobalObjectDecodeURIComponent {
    const NAME: String = BUILTIN_STRING_MEMORY.decodeURIComponent;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::decode_uri_component);
}
impl BuiltinIntrinsic for GlobalObjectDecodeURIComponent {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::DecodeURIComponent;
}
struct GlobalObjectEncodeURI;
impl Builtin for GlobalObjectEncodeURI {
    const NAME: String = BUILTIN_STRING_MEMORY.encodeURI;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::encode_uri);
}
impl BuiltinIntrinsic for GlobalObjectEncodeURI {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::EncodeURI;
}
struct GlobalObjectEncodeURIComponent;
impl Builtin for GlobalObjectEncodeURIComponent {
    const NAME: String = BUILTIN_STRING_MEMORY.encodeURIComponent;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::encode_uri_component);
}
impl BuiltinIntrinsic for GlobalObjectEncodeURIComponent {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::EncodeURIComponent;
}
struct GlobalObjectEscape;
impl Builtin for GlobalObjectEscape {
    const NAME: String = BUILTIN_STRING_MEMORY.escape;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::escape);
}
impl BuiltinIntrinsic for GlobalObjectEscape {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::Escape;
}
struct GlobalObjectUnescape;
impl Builtin for GlobalObjectUnescape {
    const NAME: String = BUILTIN_STRING_MEMORY.unescape;
    const LENGTH: u8 = 1;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(GlobalObject::unescape);
}
impl BuiltinIntrinsic for GlobalObjectUnescape {
    const INDEX: IntrinsicFunctionIndexes = IntrinsicFunctionIndexes::Unescape;
}

impl GlobalObject {
    fn eval(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    /// ### [19.2.2 isFinite ( number )](https://tc39.es/ecma262/#sec-isfinite-number)
    ///
    /// This function is the %isFinite% intrinsic object.

    fn is_finite(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let number = arguments.get(0);
        // 1. Let num be ? ToNumber(number).
        let num = to_number(agent, number)?;
        // 2. If num is not finite, return false.
        // 3. Otherwise, return true.
        Ok(num.is_finite(agent).into())
    }

    /// ### [19.2.3 isNaN ( number )](https://tc39.es/ecma262/#sec-isnan-number)
    ///
    /// This function is the %isNaN% intrinsic object.
    ///
    /// > NOTE: A reliable way for ECMAScript code to test if a value X is NaN
    /// > is an expression of the form X !== X. The result will be true if and
    /// > only if X is NaN.
    fn is_nan(agent: &mut Agent, _: Value, arguments: ArgumentsList) -> JsResult<Value> {
        let number = arguments.get(0);
        // 1. Let num be ? ToNumber(number).
        let num = to_number(agent, number)?;
        // 2. If num is NaN, return true.
        // 3. Otherwise, return false.
        Ok(num.is_nan(agent).into())
    }
    fn parse_float(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn parse_int(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn decode_uri(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn decode_uri_component(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn encode_uri(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn encode_uri_component(
        _agent: &mut Agent,
        _this_value: Value,
        _: ArgumentsList,
    ) -> JsResult<Value> {
        todo!()
    }
    fn escape(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }
    fn unescape(_agent: &mut Agent, _this_value: Value, _: ArgumentsList) -> JsResult<Value> {
        todo!()
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEval>(agent, realm).build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectIsFinite>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectIsNaN>(agent, realm).build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectParseFloat>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectParseInt>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectDecodeURI>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectDecodeURIComponent>(
            agent, realm,
        )
        .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEncodeURI>(agent, realm)
            .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEncodeURIComponent>(
            agent, realm,
        )
        .build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectEscape>(agent, realm).build();
        BuiltinFunctionBuilder::new_intrinsic_function::<GlobalObjectUnescape>(agent, realm)
            .build();
    }
}
