use clippy_utils::{diagnostics::span_lint_and_help, is_self};
use rustc_hir::{def_id::LocalDefId, intravisit::FnKind, Body, FnDecl};
use rustc_lint::{LateContext, LateLintPass};
use rustc_span::Span;

use crate::{is_agent_ty, is_param_ty};

dylint_linting::declare_late_lint! {
    /// ### What it does
    ///
    /// Checks that the `nova_vm::ecmascript::Agent` is the first parameter of a function.
    ///
    /// ### Why is this bad?
    ///
    /// The `nova_vm::ecmascript::Agent` is expected to be
    /// the first parameter of a function according to the Nova engines conventions.
    ///
    /// ### Example
    ///
    /// ```rust
    /// fn foo(other: &Other, agent: &Agent) {}
    /// ```
    ///
    /// Use instead:
    ///
    /// ```rust
    /// fn bar(agent: &Agent, other: &Other) {}
    /// ```
    pub AGENT_COMES_FIRST,
    Warn,
    "the `nova_vm::ecmascript::Agent` should be the first parameter of any function using it"
}

impl<'tcx> LateLintPass<'tcx> for AgentComesFirst {
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
            // Skip while the first parameter is `self`, a param type or an agent
            .skip_while(|param| {
                if is_self(param) {
                    true
                } else {
                    let ty = cx.typeck_results().pat_ty(param.pat);
                    is_param_ty(&ty) || is_agent_ty(cx, &ty)
                }
            })
            // We hit the first parameter that is not `self`, a param type or
            // an agent, so we can safely skip it without worrying about it
            // being an agent
            .skip(1)
        {
            let ty = cx.typeck_results().pat_ty(param.pat);
            if is_agent_ty(cx, &ty) {
                span_lint_and_help(
                    cx,
                    AGENT_COMES_FIRST,
                    param.span,
                    "the `nova_vm::ecmascript::Agent` should be the first parameter of any function using it",
                    None,
                    "consider moving the `nova_vm::ecmascript::Agent` to the first parameter",
                );
            }
        }
    }
}
