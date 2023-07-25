use crate::{
    execution::{Agent, JsResult},
    types::Object,
};

/// 10.1.1 [[GetPrototypeOf]] ( )
/// https://tc39.es/ecma262/#sec-ordinary-object-internal-methods-and-internal-slots-getprototypeof
fn get_prototype_of(agent: &mut Agent, object: Object) -> Option<Object> {
    // 1. Return OrdinaryGetPrototypeOf(O).
    return ordinary_get_prototype_of(agent, object);
}

/// 10.1.1.1 OrdinaryGetPrototypeOf ( O )
/// https://tc39.es/ecma262/#sec-ordinarygetprototypeof
pub fn ordinary_get_prototype_of(agent: &mut Agent, object: Object) -> Option<Object> {
    // 1. Return O.[[Prototype]].
    return object.prototype(agent);
}
