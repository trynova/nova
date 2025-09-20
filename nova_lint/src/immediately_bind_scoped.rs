use crate::{is_scoped_ty, method_call};
use clippy_utils::{
    diagnostics::span_lint_and_help,
    get_expr_use_or_unification_node, get_parent_expr,
    paths::{PathNS, lookup_path_str},
    potential_return_of_enclosing_body,
    ty::implements_trait,
    usage::local_used_after_expr,
};
use rustc_hir::{Expr, Node};
use rustc_lint::{LateContext, LateLintPass};
use rustc_middle::ty::Ty;

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Makes sure that the user immediately binds `Scoped<Value>::get` results.
    ///
    /// ### Why is this bad?
    ///
    /// TODO: Write an explanation of why this is bad.
    ///
    /// ### Example
    ///
    /// ```
    /// let a = scoped_a.get(agent);
    /// ```
    ///
    /// Use instead:
    ///
    /// ```
    /// let a = scoped_a.get(agent).bind(gc.nogc());
    /// ```
    ///
    /// Which ensures that no odd bugs occur.
    ///
    /// ### Exception: If the result is immediately used without assigning to a
    /// variable, binding can be skipped.
    ///
    /// ```
    /// scoped_a.get(agent).internal_delete(agent, scoped_b.get(agent), gc.reborrow());
    /// ```
    ///
    /// Here it is perfectly okay to skip the binding for both `scoped_a` and
    /// `scoped_b` as the borrow checker would force you to again unbind both
    /// `Value`s immediately.
    pub IMMEDIATELY_BIND_SCOPED,
    Deny,
    "the result of `Scoped<Value>::get` should be immediately bound"
}

impl<'tcx> LateLintPass<'tcx> for ImmediatelyBindScoped {
    fn check_expr(&mut self, cx: &LateContext<'tcx>, expr: &'tcx Expr<'tcx>) {
        // First we check if we have found a `Scoped<Value>::get` call
        if is_scoped_get_method_call(cx, expr) {
            // Which is followed by a trait method call to `bind` in which case
            // it is all done properly and we can exit out of the lint
            if let Some(parent) = get_parent_expr(cx, expr)
                && is_bindable_bind_method_call(cx, parent)
            {
                return;
            }

            // If the `Scoped<Value>::get` call is never used or unified we can
            // safely exit out of the rule, otherwise we need to look into how
            // it's used.
            let Some((usage, hir_id)) = get_expr_use_or_unification_node(cx.tcx, expr) else {
                return;
            };

            if !local_used_after_expr(cx, hir_id, expr) {
                return;
            }

            // Now we are onto something! If the expression is returned, used
            // after the expression or assigned to a variable we might have
            // found an issue.
            if let Some((usage, hir_id)) = get_expr_use_or_unification_node(cx.tcx, expr)
                && (potential_return_of_enclosing_body(cx, expr)
                    || local_used_after_expr(cx, hir_id, expr)
                    || matches!(usage, Node::LetStmt(_)))
            {
                span_lint_and_help(
                    cx,
                    IMMEDIATELY_BIND_SCOPED,
                    expr.span,
                    "the result of `Scoped<Value>::get` should be immediately bound",
                    None,
                    "immediately bind the value",
                );
            }
        }
    }
}

fn is_scoped_get_method_call(cx: &LateContext<'_>, expr: &Expr) -> bool {
    if let Some((method, recv, _, _, _)) = method_call(expr)
        && method == "get"
        && let typeck_results = cx.typeck_results()
        && let recv_ty = typeck_results.expr_ty(recv)
        && is_scoped_ty(cx, &recv_ty)
    {
        true
    } else {
        false
    }
}

fn is_bindable_bind_method_call(cx: &LateContext<'_>, expr: &Expr) -> bool {
    if let Some((method, _, _, _, _)) = method_call(expr)
        && method == "bind"
        && let expr_ty = cx.typeck_results().expr_ty(expr)
        && implements_bindable_trait(cx, &expr_ty)
    {
        true
    } else {
        false
    }
}

fn is_bindable_unbind_method_call(cx: &LateContext<'_>, expr: &Expr) -> bool {
    if let Some((method, _, _, _, _)) = method_call(expr)
        && method == "unbind"
        && let expr_ty = cx.typeck_results().expr_ty(expr)
        && implements_bindable_trait(cx, &expr_ty)
    {
        true
    } else {
        false
    }
}

fn implements_bindable_trait<'tcx>(cx: &LateContext<'tcx>, ty: &Ty<'tcx>) -> bool {
    lookup_path_str(cx.tcx, PathNS::Type, "nova_vm::engine::context::Bindable")
        .first()
        .is_some_and(|&trait_def_id| implements_trait(cx, *ty, trait_def_id, &[]))
}
