use crate::ecmascript::abstract_operations::operations_on_objects::get;
use crate::ecmascript::abstract_operations::operations_on_objects::has_property;
use crate::ecmascript::abstract_operations::type_conversion::to_string;
use crate::ecmascript::builders::builtin_function_builder::BuiltinFunctionBuilder;
use crate::ecmascript::builtins::error::Error;
use crate::ecmascript::builtins::ordinary::ordinary_create_from_constructor;
use crate::ecmascript::builtins::ArgumentsList;
use crate::ecmascript::builtins::Behaviour;
use crate::ecmascript::builtins::Builtin;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::Agent;
use crate::ecmascript::execution::JsResult;
use crate::ecmascript::execution::ProtoIntrinsics;
use crate::ecmascript::execution::RealmIdentifier;
use crate::ecmascript::types::Function;
use crate::ecmascript::types::IntoObject;
use crate::ecmascript::types::IntoValue;
use crate::ecmascript::types::Object;
use crate::ecmascript::types::PropertyKey;
use crate::ecmascript::types::String;
use crate::ecmascript::types::Value;
use crate::ecmascript::types::BUILTIN_STRING_MEMORY;
use crate::heap::GetHeapData;

pub(crate) struct ErrorConstructor;

impl Builtin for ErrorConstructor {
    const NAME: String = BUILTIN_STRING_MEMORY.Error;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::behaviour);
}

impl ErrorConstructor {
    /// ### [20.5.1.1 Error ( message \[ , options \] )](https://tc39.es/ecma262/#sec-error-message)
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        let message = arguments.get(0);
        let options = arguments.get(1);

        // 3. If message is not undefined, then
        let message = if !message.is_undefined() {
            // a. Let msg be ? ToString(message).
            Some(to_string(agent, message)?)
        } else {
            None
        };
        // 4. Perform ? InstallErrorCause(O, options).
        let cause = get_error_cause(agent, options)?;

        // 1. If NewTarget is undefined, let newTarget be the active function object; else let newTarget be NewTarget.
        let new_target = new_target.map_or_else(
            || agent.running_execution_context().function.unwrap(),
            |new_target| Function::try_from(new_target).unwrap(),
        );
        // 2. Let O be ? OrdinaryCreateFromConstructor(newTarget, "%Error.prototype%", « [[ErrorData]] »).
        let o = ordinary_create_from_constructor(agent, new_target, ProtoIntrinsics::Error, ())?;
        let o = Error::try_from(o).unwrap();
        // b. Perform CreateNonEnumerableDataPropertyOrThrow(O, "message", msg).
        let heap_data = agent.heap.get_mut(o.0);
        heap_data.kind = ExceptionType::Error;
        heap_data.message = message;
        heap_data.cause = cause;
        // 5. Return O.
        Ok(o.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let this = intrinsics.error();
        let this_object_index = intrinsics.error_base_object();
        let error_prototype = intrinsics.error_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ErrorConstructor>(
            agent,
            realm,
            this,
            Some(this_object_index),
        )
        .with_property_capacity(1)
        .with_prototype_property(error_prototype.into_object())
        .build();
    }
}

pub(super) fn get_error_cause(agent: &mut Agent, options: Value) -> JsResult<Option<Value>> {
    let Ok(options) = Object::try_from(options) else {
        return Ok(None);
    };
    let key = PropertyKey::from_str(&mut agent.heap, "cause");
    if has_property(agent, options, key)? {
        Ok(Some(get(agent, options, key)?))
    } else {
        Ok(None)
    }
}
