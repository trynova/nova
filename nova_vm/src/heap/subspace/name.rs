use core::{ffi, fmt, ptr::NonNull};

/// This is a &'static CStr converted into a pointer to avoid storying a
/// word for the string's length. this comes at the cost of O(n) casts into
/// &'static str, which is fine because if we're doing so its either for
/// debugging or because Nova is about to panic.
///
/// Do not expose this outside of the `subspace` module.
#[derive(Clone, Copy)]
pub(super) struct Name(
    // invariant: never dereference a `*mut`, or create a `&mut`, from this pointer.
    // NonNull is used over *const c_char for non-null optimization.
    NonNull<ffi::c_char>,
);
// pub(super) struct Name(ptr::NonNull<ffi::c_char>);
// SAFETY: pointer is &'static and never mutable.
unsafe impl Send for Name {}
unsafe impl Sync for Name {}

impl Name {
    pub const fn new(s: &'static ffi::CStr) -> Self {
        assert!(s.to_str().is_ok());
        let p = NonNull::new(s.as_ptr() as *mut _).unwrap();
        Self(p)
    }
    pub const fn as_str(self) -> &'static str {
        // SAFETY: inner string is always created from a &'static CStr that is
        // known to be valid utf-8
        match unsafe { ffi::CStr::from_ptr(self.0.as_ref() as *const _).to_str() } {
            Ok(s) => s,
            Err(_) => unreachable!(),
        }
    }
}

impl fmt::Debug for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        (*self).as_str().fmt(f)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

impl From<Name> for &'static str {
    fn from(name: Name) -> Self {
        name.as_str()
    }
}
