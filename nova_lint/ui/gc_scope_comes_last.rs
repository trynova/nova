#![allow(
    dead_code,
    unused_variables,
    clippy::disallowed_names,
    unknown_lints,
    can_use_no_gc_scope
)]

type GcScope<'a, 'b> = nova_vm::engine::context::GcScope<'a, 'b>;
type NoGcScope<'a, 'b> = nova_vm::engine::context::NoGcScope<'a, 'b>;

fn test_no_params() {
    unimplemented!()
}

fn test_one_param(_foo: ()) {
    unimplemented!()
}

fn test_owned_qualified_gc_scope_only(gc_scope: nova_vm::engine::context::GcScope<'_, '_>) {
    unimplemented!()
}

fn test_owned_gc_scope_only(gc_scope: GcScope<'_, '_>) {
    unimplemented!()
}

fn test_multiple_gc_scopes(gc_scope1: GcScope<'_, '_>, gc_scope2: GcScope<'_, '_>) {
    unimplemented!()
}

fn test_something_else_after_gc_scope(gc_scope: GcScope<'_, '_>, foo: ()) {
    unimplemented!()
}

fn test_multiple_gc_scopes_with_something_in_between(
    gc_scope1: GcScope<'_, '_>,
    foo: (),
    gc_scope2: GcScope<'_, '_>,
) {
    unimplemented!()
}

fn test_multiple_no_gc_scopes_with_something_in_between(
    gc_scope1: NoGcScope<'_, '_>,
    foo: (),
    gc_scope2: GcScope<'_, '_>,
) {
    unimplemented!()
}

struct Test;

impl Test {
    fn test_no_params(&self) {
        unimplemented!()
    }

    fn test_one_param(&self, _foo: ()) {
        unimplemented!()
    }

    fn test_self_and_owned_gc_scope_only(&self, gc_scope: GcScope<'_, '_>) {
        unimplemented!()
    }

    fn test_self_and_something_after_gc_scope(&self, gc_scope: GcScope<'_, '_>, foo: ()) {
        unimplemented!()
    }

    fn test_something_after_gc_scope(gc_scope: GcScope<'_, '_>, foo: ()) {
        unimplemented!()
    }
}

fn main() {
    unimplemented!()
}
