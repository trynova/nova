#![allow(dead_code, unused_variables, clippy::disallowed_names)]

use nova_vm::{
    ecmascript::{execution::Agent, types::Value},
    engine::{
        Scoped,
        context::{Bindable, NoGcScope},
    },
};

fn test_scoped_get_is_immediately_bound(agent: &Agent, scoped: Scoped<Value>, gc: NoGcScope) {
    let _a = scoped.get(agent).bind(gc);
}

fn test_scoped_get_can_get_bound_right_after(agent: &Agent, scoped: Scoped<Value>, gc: NoGcScope) {
    let a = scoped.get(agent);
    a.bind(gc);
}

fn test_scoped_get_can_get_bound_in_tuple_right_after(
    agent: &Agent,
    scoped: Scoped<Value>,
    gc: NoGcScope,
) {
    let a = scoped.get(agent);
    (a.bind(gc), ());
}

fn test_scoped_get_can_be_immediately_passed_on(
    agent: &Agent,
    scoped: Scoped<Value>,
    gc: NoGcScope,
) {
    let a = scoped.get(agent);
    test_consumes_unbound_value(a);
}

fn test_scoped_get_can_be_used_as_self(agent: &Agent, scoped: Scoped<Value>) {
    scoped.get(agent).is_undefined();
}

fn test_scoped_get_can_be_used_as_self_immediately_after(
    agent: &Agent,
    scoped: Scoped<Value>,
    gc: NoGcScope,
) {
    let a = scoped.get(agent);
    a.is_undefined();
}

fn test_consumes_unbound_value(value: Value) {
    unimplemented!()
}

fn test_scoped_get_is_not_immediately_bound(agent: &Agent, scoped: Scoped<Value>) {
    let _a = scoped.get(agent);
}

fn test_scoped_get_doesnt_need_to_be_bound_if_not_assigned(agent: &Agent, scoped: Scoped<Value>) {
    scoped.get(agent);
}

fn test_improbable_but_technically_bad_situation(
    agent: &Agent,
    scoped: Scoped<Value>,
    gc: NoGcScope,
) {
    let _a = Scoped::new(agent, Value::Undefined, gc).get(agent);
}

// fn take_and_return_value_with_gc<'gc>(
//     agent: &mut Agent,
//     value: Value,
//     mut gc: GcScope<'gc, '_>,
// ) -> Value<'gc> {
//     Value::Undefined
// }

// fn test_scoped_used_twice_right_after<'gc>(
//     agent: &mut Agent,
//     value: Scoped<Value>,
//     mut gc: GcScope<'gc, '_>,
// ) {
//     let value = value.get(agent);
//     let something = if value.is_undefined() {
//         true
//     } else {
//         take_and_return_value_with_gc(agent, value, gc.reborrow())
//             .unbind()
//             .bind(gc.nogc())
//             .is_undefined()
//     };
// }

fn main() {
    unimplemented!()
}
