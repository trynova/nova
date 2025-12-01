type GcScope<'a, 'b> = nova_vm::engine::context::GcScope<'a, 'b>;
type NoGcScope<'a, 'b> = nova_vm::engine::context::NoGcScope<'a, 'b>;

fn test_doesnt_need_gc_scope(gc: GcScope<'_, '_>) {
    needs_nogc(gc.nogc());
}

fn test_needs_gc_scope(mut gc: GcScope<'_, '_>) {
    needs_gc(gc.reborrow());
}

fn test_needs_both(mut gc: GcScope<'_, '_>) {
    needs_gc(gc.reborrow());
    needs_nogc(gc.into_nogc());
}

fn needs_nogc(gc: NoGcScope<'_, '_>) {
    unimplemented!()
}

fn needs_gc(gc: GcScope<'_, '_>) {
    unimplemented!()
}

fn main() {
    unimplemented!()
}
