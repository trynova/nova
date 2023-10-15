use super::{JsResult, Realm};
use crate::types::Function;

/// 19.2.1.2 HostEnsureCanCompileStrings ( calleeRealm )
/// https://tc39.es/ecma262/#sec-hostensurecancompilestrings
pub fn host_ensure_can_compile_strings(_: &mut Realm) -> JsResult<()> {
    Ok(())
}

/// 20.2.5 HostHasSourceTextAvailable ( func )
/// https://tc39.es/ecma262/#sec-hosthassourcetextavailable
pub fn host_has_source_text_available(_: Function) -> bool {
    // The default implementation of HostHasSourceTextAvailable is to return true.
    return true;
}
