use clippy_utils::{diagnostics::span_lint_and_help, is_self};
use rustc_hir::{Body, FnDecl, def_id::LocalDefId, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::TyKind;
use rustc_span::Span;

use crate::{is_gc_scope_ty, is_no_gc_scope_ty};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks that the gc scope is only passed by value.
    ///
    /// ### Why is this bad?
    ///
    /// Passing the gc scope by reference is not necessary and should be avoided.
    /// This is because a immutable reference `&GcScope` would be equivalent to
    /// simply using `NoGcScope` while using `&mut GcScope` would technically
    /// work but be less efficient.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn bar(gc: &GcScope<'_, '_>) {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn bar(gc: NoGcScope<'_, '_>) {}
    /// ```
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn bar(gc: &mut GcScope<'_, '_>) {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn bar(gc: GcScope<'_, '_>) {}
    /// ```
    pub GC_SCOPE_IS_ONLY_PASSED_BY_VALUE,
    Deny,
    "the gc scope should only be passed by value"
}

impl<'tcx> LateLintPass<'tcx> for GcScopeIsOnlyPassedByValue {
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

        for param in body.params {
            if is_self(param) {
                continue;
            }

            let ty = cx.typeck_results().pat_ty(param.pat);
            if let TyKind::Ref(_, ty, _) = ty.kind() {
                ty.peel_refs();
                if is_gc_scope_ty(cx, ty) || is_no_gc_scope_ty(cx, ty) {
                    span_lint_and_help(
                        cx,
                        GC_SCOPE_IS_ONLY_PASSED_BY_VALUE,
                        param.ty_span,
                        "gc scope should only be passed by value",
                        None,
                        "remove the reference",
                    );
                }
            }
        }
    }
}
