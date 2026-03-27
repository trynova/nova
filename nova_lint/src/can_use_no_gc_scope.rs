use std::ops::ControlFlow;

use clippy_utils::{
    diagnostics::span_lint_and_sugg, fn_def_id, is_trait_impl_item, source::HasSession,
    visitors::for_each_expr,
};
use rustc_errors::Applicability;
use rustc_hir::{Body, Expr, ExprKind, FnDecl, def::Res, def_id::LocalDefId, intravisit::FnKind};
use rustc_hir_analysis::lower_ty;
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::{BytePos, Span};

use crate::{could_be_builtin_method_def, is_gc_scope_ty, is_no_gc_method, is_trait_item};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks that a function which only needs `NoGcScope` uses it instead of
    /// `GcScope`.
    ///
    /// ### Why is this bad?
    ///
    /// You usually should use `NoGcScope` instead of `GcScope` if you don't
    /// need the latter. The reason this is bad is that it forces the caller
    /// to scope any heap references held past the call site unnecessarily.
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
    "you should use `NoGcScope` instead of `GcScope` if you don't need the latter"
}

impl<'tcx> LateLintPass<'tcx> for CanUseNoGcScope {
    fn check_fn(
        &mut self,
        cx: &LateContext<'tcx>,
        kind: FnKind<'tcx>,
        _: &'tcx FnDecl<'tcx>,
        body: &'tcx Body<'tcx>,
        span: Span,
        _: LocalDefId,
    ) {
        if span.from_expansion() {
            return;
        }

        // Skip closures
        if matches!(kind, FnKind::Closure) {
            return;
        }

        // Skip trait definitions and methods
        let res = {
            let body_id = cx.tcx.hir_body_owner(body.id());
            is_trait_impl_item(cx, body_id) || is_trait_item(cx, body_id)
        };
        if res {
            return;
        }

        // Skip builtin methods
        if could_be_builtin_method_def(cx, kind) {
            return;
        }

        // Skip `GcScope` methods
        if let FnKind::Method(_, sig) = kind
            && let Some(maybe_self) = sig.decl.inputs.first()
            && is_gc_scope_ty(cx, &lower_ty(cx.tcx, maybe_self))
        {
            return;
        }

        let typeck = cx.typeck_results();

        let Some(gc_scope) = body.params.iter().find(|param| {
            let ty = typeck.pat_ty(param.pat);
            is_gc_scope_ty(cx, &ty)
        }) else {
            // Either the function already takes `NoGcScope` or it doesn't take any
            // at all in which case we don't need to lint.
            return;
        };

        if for_each_expr(cx, body.value, |expr| {
            // Checks if the expression is function or method call
            if let Some(did) = fn_def_id(cx, expr)
                // If we encountered either a `nogc` och `into_nogc` method call
                // we skip them because they don't count as needing a `GcScope`.
                && !is_no_gc_method(cx, did)
            {
                // Check if the function actually uses `GcScope` in its signature
                let sig = cx.tcx.fn_sig(did).instantiate_identity().skip_binder();
                if sig.inputs().iter().any(|input| is_gc_scope_ty(cx, input)) {
                    return ControlFlow::Break(());
                }
            }

            // Calls to closures and other functions may also use `GcScope`,
            // we need to check those as well.
            if let ExprKind::Call(
                Expr {
                    kind: ExprKind::Path(qpath),
                    hir_id: path_hir_id,
                    ..
                },
                args,
            ) = expr.kind
                && let Res::Local(_) = typeck.qpath_res(qpath, *path_hir_id)
                && args.iter().any(|arg| {
                    let ty = typeck.expr_ty(arg);
                    is_gc_scope_ty(cx, &ty)
                })
            {
                return ControlFlow::Break(());
            }

            ControlFlow::Continue(())
        })
        .is_none()
        {
            // We didn't find any calls in the body that would require a `GcScope`
            // so we can suggest using `NoGcScope` instead.
            let ty_span = cx
                .sess()
                .source_map()
                .span_until_char(gc_scope.ty_span, '<');
            // Trim the start of the span to just before the `GcScope` type name
            let ty_span = ty_span.with_lo(ty_span.hi() - BytePos(7));
            span_lint_and_sugg(
                cx,
                CAN_USE_NO_GC_SCOPE,
                ty_span,
                "you can use `NoGcScope` instead of `GcScope` here",
                "replace with",
                "NoGcScope".to_owned(),
                Applicability::MaybeIncorrect,
            );
        }
    }
}
