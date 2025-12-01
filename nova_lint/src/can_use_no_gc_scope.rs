use clippy_utils::{diagnostics::span_lint_and_sugg, fn_def_id};
use rustc_errors::Applicability;
use rustc_hir::{Body, ExprKind, FnDecl, Param, def_id::LocalDefId, intravisit::FnKind};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

use crate::is_gc_scope_ty;

dylint_linting::impl_late_lint! {
    /// ### What it does
    ///
    /// Checks that a function which only needs `NoGcScope` uses it instead of
    /// `GcScope`.
    ///
    /// ### Why is this bad?
    ///
    /// TODO
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn foo(gc: GcScope<'_, '_>) {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn foo(gc: NoGcScope<'_, '_>) {}
    /// ```
    pub CAN_USE_NO_GC_SCOPE,
    Warn,
    "you should use `NoGcScope` instead of `GcScope` if you don't need the latter",
    CanUseNoGcScope::new()
}

struct CanUseNoGcScope {
    in_target_fn: bool,
    requires_gc: bool,
    param_span: Option<Span>,
}

impl CanUseNoGcScope {
    fn new() -> Self {
        Self {
            in_target_fn: false,
            requires_gc: false,
            param_span: None,
        }
    }

    fn enter(&mut self, param: &Param<'_>) {
        self.in_target_fn = true;
        self.requires_gc = false;
        self.param_span = Some(param.ty_span);
    }

    fn exit(&mut self, cx: &LateContext<'_>) {
        if let Some(span) = self.param_span
            && self.in_target_fn
            && !self.requires_gc
        {
            span_lint_and_sugg(
                cx,
                CAN_USE_NO_GC_SCOPE,
                span,
                "you can use `NoGcScope` instead of `GcScope` here",
                "use `NoGcScope`",
                "NoGcScope".to_owned(),
                Applicability::MachineApplicable,
            );
        }

        self.in_target_fn = false;
        self.requires_gc = false;
        self.param_span = None;
    }
}

impl<'tcx> LateLintPass<'tcx> for CanUseNoGcScope {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        _: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: LocalDefId,
    ) {
        self.exit(cx);

        if span.from_expansion() {
            return;
        }

        let Some(gc_scope) = body.params.iter().find(|param| {
            let ty = cx.typeck_results().pat_ty(param.pat);
            is_gc_scope_ty(cx, &ty)
        }) else {
            // Either the function already takes `NoGcScope` or it doesn't take any
            // at all in which case we don't need to lint.
            return;
        };

        self.enter(gc_scope);
    }

    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx rustc_hir::Expr<'tcx>) {
        if !self.in_target_fn || !matches!(expr.kind, ExprKind::MethodCall(..) | ExprKind::Call(..))
        {
            return;
        }

        let Some(def_id) = fn_def_id(cx, expr) else {
            return;
        };
        let sig = cx.tcx.fn_sig(def_id).instantiate_identity();

        self.requires_gc = sig.inputs().iter().any(|input| {
            let ty = input.skip_binder();
            is_gc_scope_ty(cx, &ty)
        });
    }
}
