use super::{create_builtin_function, ArgumentsList, Behaviour, Builtin, BuiltinFunctionArgs};
use crate::{
    execution::{Agent, JsResult, Realm},
    types::{Object, Value},
};

struct ArrayConstructor;

impl Builtin for ArrayConstructor {
    fn create(realm: &mut Realm) -> Object {
        let object = create_builtin_function(
            &mut realm.agent.clone().borrow_mut(),
            Behaviour::Regular(Self::behaviour),
            BuiltinFunctionArgs::new(1, "Array", realm),
        );

        object
    }
}

impl ArrayConstructor {
    fn behaviour(agent: &mut Agent, value: Value, arguments: ArgumentsList) -> JsResult<Value> {
        todo!();
    }
}
