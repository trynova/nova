use clippy_utils::diagnostics::span_lint_and_help;
use rustc_hir::{def_id::LocalDefId, intravisit::FnKind, Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

use crate::{is_gc_scope_ty, is_no_gc_scope_ty};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks that the gc scope is the last parameter of a function.
    ///
    /// ### Why is this bad?
    ///
    /// The gc scope parameter should be passed as the last parameter of a
    /// function because it invalidates all values which refer to it, take
    /// for example the following code:
    ///
    /// ```rust
    /// let data = data.bind(gc.nogc());
    /// call(agent, gc.reborrow(), data.unbind());
    /// ```
    ///
    /// This wouldn't work beause `gc.reborrow()` invalidates `data` immediately,
    /// meaning that when `data.unbind()` is being called the `data` is already
    /// invalidated and illegal to use, leading to a borrow checker error.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn bar(gc: GcScope<'_, '_>, other: &Other) {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn foo(other: &Other, gc: GcScope<'_, '_>) {}
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
                is_gc_scope_ty(cx, &ty) || is_no_gc_scope_ty(cx, &ty)
            })
            // We hit the first parameter that is not a gc scope, so we can
            // safely skip it without worrying about it being a gc scope
            .skip(1)
        {
            let ty = cx.typeck_results().pat_ty(param.pat);
            if is_gc_scope_ty(cx, &ty) || is_no_gc_scope_ty(cx, &ty) {
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
