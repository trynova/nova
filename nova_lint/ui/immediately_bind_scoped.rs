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

// TODO: These are valid patterns, which are found in certain parts of the
// codebase so should ideally be implemented.
// fn test_scoped_get_can_get_bound_right_after(agent: &Agent, scoped: Scoped<Value>, gc: NoGcScope) {
//     let a = scoped.get(agent);
//     a.bind(gc);
// }
//
// fn test_scoped_get_can_get_bound_right_after_and_never_used_again(
//     agent: &Agent,
//     scoped: Scoped<Value>,
//     gc: NoGcScope,
// ) {
//     let a = scoped.get(agent);
//     let b = a.bind(gc);
//     a;
// }
//
// fn test_scoped_get_can_be_immediately_passed_on(
//     agent: &Agent,
//     scoped: Scoped<Value>,
//     gc: NoGcScope,
// ) {
//     let a = scoped.get(agent);
//     test_consumes_unbound_value(value);
// }
//
// fn test_consumes_unbound_value(value: Value) {
//     unimplemented!()
// }

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

fn main() {
    unimplemented!()
}
