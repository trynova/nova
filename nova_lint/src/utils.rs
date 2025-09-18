use rustc_hir::{Expr, ExprKind, def_id::DefId};
use rustc_lint::LateContext;
use rustc_middle::ty::{Ty, TyKind};
use rustc_span::Span;
use rustc_span::symbol::Symbol;

// Copyright (c) 2014-2025 The Rust Project Developers
//
// Originally copied from `dylint` which in turn copied it from `clippy_utils`:
// - https://github.com/trailofbits/dylint/blob/a2dd5c60d53d66fc791fa8184bed27a4cb142e74/internal/src/match_def_path.rs
// - https://github.com/rust-lang/rust-clippy/blob/f62f26965817f2573c2649288faa489a03ed1665/clippy_utils/src/lib.rs
// It was removed from `clippy_utils` by the following PR:
// - https://github.com/rust-lang/rust-clippy/pull/14705
/// Checks if the given `DefId` matches the path.
pub fn match_def_path(cx: &LateContext<'_>, did: DefId, syms: &[&str]) -> bool {
    // We should probably move to Symbols in Clippy as well rather than interning every time.
    let path = cx.get_def_path(did);
    syms.iter()
        .map(|x| Symbol::intern(x))
        .eq(path.iter().copied())
}

// Copyright (c) 2014-2025 The Rust Project Developers
//
// Originally copied from `dylint` which in turn copied it from `clippy_lints`:
// - https://github.com/trailofbits/dylint/blob/d1be1c42f363ca11f8ebce0ff0797ecbbcc3680b/examples/restriction/collapsible_unwrap/src/lib.rs#L180
// - https://github.com/rust-lang/rust-clippy/blob/3f015a363020d3811e1f028c9ce4b0705c728289/clippy_lints/src/methods/mod.rs#L3293-L3304
/// Extracts a method call name, args, and `Span` of the method name.
pub fn method_call<'tcx>(
    recv: &'tcx Expr<'tcx>,
) -> Option<(&'tcx str, &'tcx Expr<'tcx>, &'tcx [Expr<'tcx>], Span, Span)> {
    if let ExprKind::MethodCall(path, receiver, args, call_span) = recv.kind
        && !args.iter().any(|e| e.span.from_expansion())
        && !receiver.span.from_expansion()
    {
        let name = path.ident.name.as_str();
        return Some((name, receiver, args, path.ident.span, call_span));
    }
    None
}

pub fn is_param_ty(ty: &Ty) -> bool {
    matches!(ty.kind(), TyKind::Param(_))
}

pub fn is_agent_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &["nova_vm", "ecmascript", "execution", "agent", "Agent"],
        ),
        _ => false,
    }
}

pub fn is_gc_scope_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.kind() {
        TyKind::Adt(def, _) => {
            match_def_path(cx, def.did(), &["nova_vm", "engine", "context", "GcScope"])
        }
        _ => false,
    }
}

pub fn is_no_gc_scope_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &["nova_vm", "engine", "context", "NoGcScope"],
        ),
        _ => false,
    }
}

pub fn is_scoped_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &["nova_vm", "engine", "rootable", "scoped", "Scoped"],
        ),
        _ => false,
    }
}

// Checks if a given expression is assigned to a variable.
//
// Copyright (c) 2014-2025 The Rust Project Developers
//
// Copied and modified from `clippy_utils`:
// - https://github.com/rust-lang/rust-clippy/blob/8a5dc7c1713a7eb9af28bf9f53dc6b61da7aad90/clippy_utils/src/lib.rs#L1369-L1388
// pub fn is_inside_let(tcx: TyCtxt<'_>, expr: &Expr<'_>) -> bool {
//     let mut child_id = expr.hir_id;
//     for (parent_id, node) in tcx.hir_parent_iter(child_id) {
//         if let Node::LetStmt(LetStmt {
//             init: Some(init),
//             els: Some(els),
//             ..
//         }) = node
//             && (init.hir_id == child_id || els.hir_id == child_id)
//         {
//             return true;
//         }

//         child_id = parent_id;
//     }

//     false
// }
