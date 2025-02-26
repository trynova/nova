use clippy_utils::match_def_path;
use rustc_lint::LateContext;
use rustc_middle::ty::{Ty, TyKind};

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
