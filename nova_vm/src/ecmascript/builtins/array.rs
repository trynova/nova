//! ### 10.4.2 Array Exotic Objects
//!
//! https://tc39.es/ecma262/#sec-array-exotic-objects

pub(crate) mod data;

use super::{create_builtin_function, ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs};
use crate::ecmascript::{
    execution::{Agent, JsResult},
    types::{Object, Value},
};

pub struct ArrayConstructor;

impl Builtin for ArrayConstructor {
    fn create<'a>(agent: &'a mut Agent<'a, 'a>) -> JsResult<Object> {
        let realm = agent.current_realm_id();
        let object = create_builtin_function(
            agent,
            Behaviour::Regular(Self::behaviour),
            BuiltinFunctionArgs::new(1, "Array", realm),
        );

        Ok(object.into_object())
    }
}

impl ArrayConstructor {
    fn behaviour(
        _agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }
}
