use clippy_utils::peel_hir_ty_options;
use rustc_hir::{FnSig, HirId, ItemKind, Node, def_id::DefId, intravisit::FnKind};
use rustc_hir_analysis::lower_ty;
use rustc_lint::LateContext;
use rustc_middle::ty::{Ty, TyKind};
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

pub fn match_def_paths(cx: &LateContext<'_>, did: DefId, syms: &[&[&str]]) -> bool {
    let path = cx.get_def_path(did);
    syms.iter().any(|syms| {
        syms.iter()
            .map(|x| Symbol::intern(x))
            .eq(path.iter().copied())
    })
}

pub fn is_trait_item(cx: &LateContext<'_>, hir_id: HirId) -> bool {
    if let Node::Item(item) = cx.tcx.parent_hir_node(hir_id) {
        matches!(item.kind, ItemKind::Trait(..))
    } else {
        false
    }
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
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => {
            match_def_path(cx, def.did(), &["nova_vm", "engine", "context", "GcScope"])
        }
        _ => false,
    }
}

pub fn is_no_gc_method(cx: &LateContext<'_>, did: DefId) -> bool {
    match_def_paths(
        cx,
        did,
        &[
            &["nova_vm", "engine", "context", "GcScope", "nogc"],
            &["nova_vm", "engine", "context", "GcScope", "into_nogc"],
        ],
    )
}

pub fn is_no_gc_scope_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &["nova_vm", "engine", "context", "NoGcScope"],
        ),
        _ => false,
    }
}

pub fn is_value_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &[
                "nova_vm",
                "ecmascript",
                "types",
                "language",
                "value",
                "Value",
            ],
        ),
        _ => false,
    }
}

pub fn is_object_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &[
                "nova_vm",
                "ecmascript",
                "types",
                "language",
                "object",
                "Object",
            ],
        ),
        _ => false,
    }
}

pub fn is_arguments_list_ty(cx: &LateContext<'_>, ty: &Ty) -> bool {
    match ty.peel_refs().kind() {
        TyKind::Adt(def, _) => match_def_path(
            cx,
            def.did(),
            &[
                "nova_vm",
                "ecmascript",
                "builtins",
                "builtin_function",
                "ArgumentsList",
            ],
        ),
        _ => false,
    }
}

pub fn could_be_builtin_method_sig<'tcx>(cx: &LateContext<'tcx>, sig: &FnSig<'tcx>) -> bool {
    sig.decl.inputs.len() == 4
        && is_agent_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[0]))
        && is_value_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[1]))
        && is_arguments_list_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[2]))
        && is_gc_scope_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[3]))
}

pub fn could_be_builtin_constructor_sig<'tcx>(cx: &LateContext<'tcx>, sig: &FnSig<'tcx>) -> bool {
    sig.decl.inputs.len() == 5
        && is_agent_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[0]))
        && is_value_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[1]))
        && is_arguments_list_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[2]))
        && is_object_ty(
            cx,
            &lower_ty(cx.tcx, peel_hir_ty_options(cx, &sig.decl.inputs[3])),
        )
        && is_gc_scope_ty(cx, &lower_ty(cx.tcx, &sig.decl.inputs[4]))
}

pub fn could_be_builtin_method_def<'tcx>(cx: &LateContext<'tcx>, kind: FnKind<'tcx>) -> bool {
    match kind {
        FnKind::Method(_, sig) => {
            could_be_builtin_method_sig(cx, sig) || could_be_builtin_constructor_sig(cx, sig)
        }
        _ => false,
    }
}
