#![allow(dead_code, unused_variables)]

use nova_vm::{
    ecmascript::{
        builtins::ArgumentsList,
        Agent, JsResult,
        Object, Value,
    },
    engine::{GcScope, NoGcScope},
};

fn test_doesnt_need_gc_scope(gc: GcScope<'_, '_>) {
    needs_nogc(gc.nogc());
}

fn test_doesnt_need_gc_scope_at_all(gc: GcScope<'_, '_>) {
    unimplemented!()
}

fn test_doesnt_need_gc_scope_with_qualified_path(gc: nova_vm::engine::GcScope<'_, '_>) {
    unimplemented!()
}

fn test_uses_gc_in_closure<'a, 'b>(work: impl FnOnce(GcScope<'a, 'b>) -> (), gc: GcScope<'a, 'b>) {
    work(gc)
}

fn test_needs_gc_scope(mut gc: GcScope<'_, '_>) {
    needs_gc(gc.reborrow());
}

fn test_needs_both(mut gc: GcScope<'_, '_>) {
    needs_gc(gc.reborrow());
    needs_nogc(gc.into_nogc());
}

fn test_uses_gc_method(mut gc: GcScope<'_, '_>) {
    gc.reborrow();
}

struct BuiltinObject;

impl BuiltinObject {
    fn test_doesnt_need_gc_scope(&self, gc: GcScope<'_, '_>) {
        needs_nogc(gc.nogc());
    }

    fn test_skips_builtin_method<'gc>(
        _: &mut Agent,
        _: Value,
        _: ArgumentsList,
        _: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }

    fn test_skips_builtin_constructor<'gc>(
        _: &mut Agent,
        _: Value,
        _: ArgumentsList,
        _: Option<Object>,
        _: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        unimplemented!()
    }
}

trait TakesGC {
    fn test_doesnt_actually_need_gc(&mut self, gc: GcScope<'_, '_>);

    fn test_doesnt_actually_need_gc_with_default_impl(&mut self, gc: GcScope<'_, '_>) {
        unimplemented!()
    }
}

impl TakesGC for BuiltinObject {
    fn test_doesnt_actually_need_gc(&mut self, gc: GcScope<'_, '_>) {
        unimplemented!()
    }
}

fn needs_nogc(gc: NoGcScope<'_, '_>) {
    unimplemented!()
}

fn needs_gc(mut gc: GcScope<'_, '_>) {
    gc.reborrow();
    unimplemented!()
}

fn main() {
    unimplemented!()
}
