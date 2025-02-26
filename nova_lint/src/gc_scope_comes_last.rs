use clippy_utils::{diagnostics::span_lint_and_help, match_def_path};
use rustc_hir::{def_id::LocalDefId, intravisit::FnKind, Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::{Ty, TyKind};
use rustc_span::Span;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks that the gc scope is the last parameter of a function.
    ///
    /// ### Why is this bad?
    ///
    /// The gc scope is expected to be the last parameter of a function
    /// according to the Nova engines conventions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn bar(gc: &GcScope<'_, '_>, other: &Other) {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn foo(other: &Other, gc: &GcScope<'_, '_>) {}
    /// ```
    pub GC_SCOPE_COMES_LAST,
    Warn,
    "the gc scope should be the last parameter of any function using it"
}

impl<'tcx> LateLintPass<'tcx> for GcScopeComesLast {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: LocalDefId,
    ) {
        if span.from_expansion() {
            return;
        }

        for param in body
            .params
            .iter()
            .rev()
            // Skip while the last parameter is the gc scope
            .skip_while(|param| {
                let ty = cx.typeck_results().pat_ty(param.pat);
                is_gc_scope(cx, &ty) || is_no_gc_scope(cx, &ty)
            })
            // We hit the first parameter that is not a gc scope, so we can
            // safely skip it without worrying about it being a gc scope
            .skip(1)
        {
            let ty = cx.typeck_results().pat_ty(param.pat);
            if is_gc_scope(cx, &ty) || is_no_gc_scope(cx, &ty) {
                span_lint_and_help(
                    cx,
                    GC_SCOPE_COMES_LAST,
                    param.span,
                    "the gc scope should be the last parameter of any function using it",
                    None,
                    "consider moving the gc scope to the last parameter",
                );
            }
        }
    }
}

fn is_gc_scope(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => {
            match_def_path(cx, def.did(), &["nova_vm", "engine", "context", "GcScope"])
        }
        _ => false,
    }
}

fn is_no_gc_scope(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &["nova_vm", "engine", "context", "NoGcScope"],
        ),
        _ => false,
    }
}
