use clippy_utils::paths::{PathNS, lookup_path_str};
use clippy_utils::ty::implements_trait;
use clippy_utils::usage::local_used_after_expr;
use clippy_utils::diagnostics::span_lint_and_help;
use clippy_utils::{
    get_expr_use_or_unification_node, get_parent_expr, potential_return_of_enclosing_body,
};
use rustc_hir::{Expr, Node};
use rustc_lint::{LateContext, LateLintPass};

use crate::{is_scoped_ty, method_call};

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
        if let Some((method, recv, _, _, _)) = method_call(expr)
            && method == "get"
            && let typeck_results = cx.typeck_results()
            && let recv_ty = typeck_results.expr_ty(recv)
            && is_scoped_ty(cx, &recv_ty)
        {
            // Which is followed by a trait method call to `bind` in which case
            // it is all done properly and we can exit out of the lint
            if let Some(parent) = get_parent_expr(cx, expr)
                && let Some((parent_method, _, _, _, _)) = method_call(parent)
                && parent_method == "bind"
                && let parent_ty = typeck_results.expr_ty(parent)
                && let Some(&trait_def_id) =
                    lookup_path_str(cx.tcx, PathNS::Type, "nova_vm::engine::context::Bindable")
                        .first()
                && implements_trait(cx, parent_ty, trait_def_id, &[])
            {
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
