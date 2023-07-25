use super::{
    builtin_function::define_builtin_function, create_builtin_function, todo_builtin,
    ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs,
};
use crate::{
    execution::{Agent, JsResult, Realm},
    types::{Object, Value},
};

pub struct NumberConstructor;

impl Builtin for NumberConstructor {
    fn create<'a>(realm: &'a mut Realm<'a, 'a>) -> JsResult<Object> {
        let object = create_builtin_function(
            Behaviour::Regular(NumberConstructor::behaviour),
            BuiltinFunctionArgs::new(1, "Array", realm),
        )
        .into_object();

        define_builtin_function(object, "isFinite", todo_builtin, 1, realm)?;

        Ok(object)
    }
}

impl NumberConstructor {
    fn behaviour(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
    ) -> JsResult<Value> {
        todo!();
    }
}
