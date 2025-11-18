use std::ops::ControlFlow;

use crate::{is_scoped_ty, method_call};
use clippy_utils::{
    diagnostics::span_lint_and_help,
    get_enclosing_block, get_parent_expr, path_to_local_id,
    paths::{PathNS, lookup_path_str},
    ty::implements_trait,
    visitors::for_each_expr,
};

use rustc_hir::{Expr, ExprKind, HirId, Node, PatKind, StmtKind};
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
            // it is all done properly and we can exit out of the lint.
            if let Some(parent) = get_parent_expr(cx, expr)
                && is_bindable_bind_method_call(cx, parent)
            {
                return;
            }

            // Check if the unbound value is used in an argument position of a
            // method or function call where binding can be safely skipped.
            if is_in_argument_position(cx, expr) {
                return;
            }

            // If the expression is assigned to a local variable, we need to
            // check that it's next use is binding or as a function argument.
            if let Some(local_hir_id) = get_assigned_local(cx, expr)
                && let Some(enclosing_block) = get_enclosing_block(cx, expr.hir_id)
            {
                let mut found_valid_next_use = false;

                // Look for the next use of this local after the current expression.
                // We need to traverse the statements in the block to find proper usage
                for stmt in enclosing_block
                    .stmts
                    .iter()
                    .skip_while(|s| s.span.lo() < expr.span.hi())
                {
                    // Extract relevant expressions from the statement and check
                    // it for a use valid of the local variable.
                    let Some(stmt_expr) = (match &stmt.kind {
                        StmtKind::Expr(expr) | StmtKind::Semi(expr) => Some(*expr),
                        StmtKind::Let(local) => local.init,
                        _ => None,
                    }) else {
                        continue;
                    };

                    // Check each expression in the current statement for use
                    // of the value, breaking when found and optionally marking
                    // it as valid.
                    if for_each_expr(cx, stmt_expr, |expr_in_stmt| {
                        if path_to_local_id(expr_in_stmt, local_hir_id) {
                            if is_valid_use_of_unbound_value(cx, expr_in_stmt, local_hir_id) {
                                found_valid_next_use = true;
                            }

                            return ControlFlow::Break(true);
                        }
                        ControlFlow::Continue(())
                    })
                    .unwrap_or(false)
                    {
                        break;
                    }
                }

                if !found_valid_next_use {
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
}

/// Check if an expression is assigned to a local variable and return the local's HirId
fn get_assigned_local(cx: &LateContext<'_>, expr: &Expr) -> Option<HirId> {
    let parent_node = cx.tcx.parent_hir_id(expr.hir_id);

    if let Node::LetStmt(local) = cx.tcx.hir_node(parent_node)
        && let Some(init) = local.init
        && init.hir_id == expr.hir_id
        && let PatKind::Binding(_, hir_id, _, _) = local.pat.kind
    {
        Some(hir_id)
    } else {
        None
    }
}

/// Check if a use of an unbound value is valid (binding or function argument)
fn is_valid_use_of_unbound_value(cx: &LateContext<'_>, expr: &Expr, hir_id: HirId) -> bool {
    // Check if we're in a method call and if so, check if it's a bind call
    if let Some(parent) = get_parent_expr(cx, expr)
        && is_bindable_bind_method_call(cx, parent)
    {
        return true;
    }

    // If this is a method call to bind() on our local, it's valid
    if is_bindable_bind_method_call(cx, expr) {
        return true;
    }

    // If this is the local being used as a function argument, it's valid
    if path_to_local_id(expr, hir_id) && is_in_argument_position(cx, expr) {
        return true;
    }

    // If this is the self value of a method call, it's valid
    if path_to_local_id(expr, hir_id) && is_in_self_position(cx, expr) {
        return true;
    }

    false
}

fn is_in_self_position(cx: &LateContext<'_>, expr: &Expr) -> bool {
    let mut current_expr = expr;

    // Walk up the parent chain to see if we're in a method call
    while let Some(parent) = get_parent_expr(cx, current_expr) {
        match parent.kind {
            // If we find a method call where our expression is in the receiver position
            ExprKind::MethodCall(_, receiver, args, _) => {
                if receiver.hir_id == current_expr.hir_id {
                    return true;
                }
            }
            // Continue walking up for other expression types
            _ => {}
        }
        current_expr = parent;
    }

    false
}

/// Check if an expression is in an argument position where binding can be skipped
fn is_in_argument_position(cx: &LateContext<'_>, expr: &Expr) -> bool {
    let mut current_expr = expr;

    // Walk up the parent chain to see if we're in a function call argument
    while let Some(parent) = get_parent_expr(cx, current_expr) {
        match parent.kind {
            // If we find a method call where our expression is an argument (not receiver)
            ExprKind::MethodCall(_, receiver, args, _) => {
                if receiver.hir_id != current_expr.hir_id
                    && args.iter().any(|arg| arg.hir_id == current_expr.hir_id)
                {
                    return true;
                }
            }
            // If we find a function call where our expression is an argument
            ExprKind::Call(_, args) => {
                if args.iter().any(|arg| arg.hir_id == current_expr.hir_id) {
                    return true;
                }
            }
            // Continue walking up for other expression types
            _ => {}
        }
        current_expr = parent;
    }

    false
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

fn implements_bindable_trait<'tcx>(cx: &LateContext<'tcx>, ty: &Ty<'tcx>) -> bool {
    lookup_path_str(cx.tcx, PathNS::Type, "nova_vm::engine::context::Bindable")
        .first()
        .is_some_and(|&trait_def_id| implements_trait(cx, *ty, trait_def_id, &[]))
}
