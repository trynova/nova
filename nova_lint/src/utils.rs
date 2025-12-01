use rustc_hir::{Expr, ExprKind, def_id::DefId};
use rustc_lint::LateContext;
use rustc_middle::ty::{Ty, TyKind};
use rustc_span::{Span, symbol::Symbol};

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
