#![allow(dead_code, unused_variables, clippy::disallowed_names)]

type GcScope<'a, 'b> = nova_vm::engine::context::GcScope<'a, 'b>;
type NoGcScope<'a, 'b> = nova_vm::engine::context::NoGcScope<'a, 'b>;

fn test_owned_qualified_gc_scope_only(gc_scope: nova_vm::engine::context::GcScope<'_, '_>) {
    unimplemented!()
}

fn test_owned_gc_scope_only(gc_scope: GcScope<'_, '_>) {
    unimplemented!()
}

fn test_owned_qualified_no_gc_scope_only(gc_scope: nova_vm::engine::context::NoGcScope<'_, '_>) {
    unimplemented!()
}

fn test_owned_no_gc_scope_only(gc_scope: NoGcScope<'_, '_>) {
    unimplemented!()
}

fn test_borrowed_qualified_gc_scope_only(gc_scope: &nova_vm::engine::context::GcScope<'_, '_>) {
    unimplemented!()
}

fn test_borrowed_gc_scope_only(gc_scope: &GcScope<'_, '_>) {
    unimplemented!()
}

fn test_borrowed_qualified_no_gc_scope_only(
    gc_scope: &nova_vm::engine::context::NoGcScope<'_, '_>,
) {
    unimplemented!()
}

fn test_borrowed_no_gc_scope_only(gc_scope: &NoGcScope<'_, '_>) {
    unimplemented!()
}

fn test_mut_borrowed_qualified_gc_scope_only(
    gc_scope: &mut nova_vm::engine::context::GcScope<'_, '_>,
) {
    unimplemented!()
}

fn test_mut_borrowed_gc_scope_only(gc_scope: &mut GcScope<'_, '_>) {
    unimplemented!()
}

fn test_mut_borrowed_qualified_no_gc_scope_only(
    gc_scope: &mut nova_vm::engine::context::NoGcScope<'_, '_>,
) {
    unimplemented!()
}

fn test_mut_borrowed_no_gc_scope_only(gc_scope: &mut NoGcScope<'_, '_>) {
    unimplemented!()
}

trait TestTrait {
    fn test_trait_fn(self);
}

impl TestTrait for GcScope<'_, '_> {
    fn test_trait_fn(self) {
        unimplemented!()
    }
}

impl TestTrait for NoGcScope<'_, '_> {
    fn test_trait_fn(self) {
        unimplemented!()
    }
}

fn main() {
    unimplemented!()
}
