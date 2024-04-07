use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_string,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            error::Error, ordinary::ordinary_create_from_constructor, ArgumentsList, Behaviour,
            Builtin,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{Function, IntoObject, IntoValue, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::GetHeapData,
};

use super::error_constructor::get_error_cause;

struct EvalErrorConstructor;
impl Builtin for EvalErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.EvalError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::eval_behaviour);
}
struct RangeErrorConstructor;
impl Builtin for RangeErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.RangeError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::range_behaviour);
}
struct ReferenceErrorConstructor;
impl Builtin for ReferenceErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.ReferenceError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour =
        Behaviour::Constructor(NativeErrorConstructors::reference_behaviour);
}
struct SyntaxErrorConstructor;
impl Builtin for SyntaxErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.SyntaxError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::syntax_behaviour);
}
struct TypeErrorConstructor;
impl Builtin for TypeErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.TypeError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::type_behaviour);
}
struct URIErrorConstructor;
impl Builtin for URIErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.URIError;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(NativeErrorConstructors::uri_behaviour);
}

pub(crate) struct NativeErrorConstructors;
impl NativeErrorConstructors {
    #[inline(always)]
    fn behaviour(
        agent: &mut Agent,
        intrinsics: ProtoIntrinsics,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let message = arguments.get(0);
        let options = arguments.get(1);

        let new_target = new_target.unwrap_or_else(|| {
            agent
                .running_execution_context()
                .function
                .unwrap()
                .into_object()
        });
        let o = ordinary_create_from_constructor(
            agent,
            Function::try_from(new_target).unwrap(),
            intrinsics,
            (),
        )?;
        let msg = if !message.is_undefined() {
            Some(to_string(agent, message)?)
        } else {
            None
        };
        let cause = get_error_cause(agent, options)?;
        let o = Error::try_from(o).unwrap();
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let heap_data = agent.heap.get_mut(o.0);
        heap_data.kind = ExceptionType::Error;
        heap_data.message = msg;
        heap_data.cause = cause;
        Ok(o.into_value())
    }

    fn eval_behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, ProtoIntrinsics::EvalError, arguments, new_target)
    }

    fn range_behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, ProtoIntrinsics::RangeError, arguments, new_target)
    }

    fn reference_behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(
            agent,
            ProtoIntrinsics::ReferenceError,
            arguments,
            new_target,
        )
    }

    fn syntax_behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, ProtoIntrinsics::SyntaxError, arguments, new_target)
    }

    fn type_behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, ProtoIntrinsics::TypeError, arguments, new_target)
    }

    fn uri_behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        Self::behaviour(agent, ProtoIntrinsics::UriError, arguments, new_target)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let eval_this = intrinsics.eval_error();
        let eval_this_base_object = intrinsics.eval_error_base_object();
        let eval_error_prototype = intrinsics.eval_error_prototype();
        let range_this = intrinsics.range_error();
        let range_this_base_object = intrinsics.range_error_base_object();
        let range_error_prototype = intrinsics.range_error_prototype();
        let reference_this = intrinsics.reference_error();
        let reference_this_base_object = intrinsics.reference_error_base_object();
        let reference_error_prototype = intrinsics.reference_error_prototype();
        let syntax_this = intrinsics.syntax_error();
        let syntax_this_base_object = intrinsics.syntax_error_base_object();
        let syntax_error_prototype = intrinsics.syntax_error_prototype();
        let type_this = intrinsics.type_error();
        let type_this_base_object = intrinsics.type_error_base_object();
        let type_error_prototype = intrinsics.type_error_prototype();
        let uri_this = intrinsics.uri_error();
        let uri_this_base_object = intrinsics.uri_error_base_object();
        let uri_error_prototype = intrinsics.uri_error_prototype();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<EvalErrorConstructor>(
            agent,
            realm,
            eval_this,
            Some(eval_this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(eval_error_prototype.into_object())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<RangeErrorConstructor>(
            agent,
            realm,
            range_this,
            Some(range_this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(range_error_prototype.into_object())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<ReferenceErrorConstructor>(
            agent,
            realm,
            reference_this,
            Some(reference_this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(reference_error_prototype.into_object())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<SyntaxErrorConstructor>(
            agent,
            realm,
            syntax_this,
            Some(syntax_this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(syntax_error_prototype.into_object())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<TypeErrorConstructor>(
            agent,
            realm,
            type_this,
            Some(type_this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(type_error_prototype.into_object())
        .build();
        BuiltinFunctionBuilder::new_intrinsic_constructor::<URIErrorConstructor>(
            agent,
            realm,
            uri_this,
            Some(uri_this_base_object),
        )
        .with_property_capacity(1)
        .with_prototype_property(uri_error_prototype.into_object())
        .build();
    }
}
