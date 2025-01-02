use crate::{
    ecmascript::execution::{agent::ExceptionType, Agent, JsResult},
    engine::context::NoGcScope,
};

use super::Proxy;

/// ### [10.5.14 ValidateNonRevokedProxy ( proxy )](https://tc39.es/ecma262/#sec-validatenonrevokedproxy)
///
/// The abstract operation ValidateNonRevokedProxy takes argument
/// proxy (a Proxy exotic object) and returns either a normal completion containing unused or a throw completion.
/// It throws a TypeError exception if proxy has been revoked.
pub(crate) fn validate_non_revoked_proxy(
    agent: &mut Agent,
    proxy: Proxy,
    gc: NoGcScope<'_, '_>,
) -> JsResult<()> {
    let proxy_data = &agent[proxy];

    // 1. If proxy.[[ProxyTarget]] is null, throw a TypeError exception.
    if proxy_data.target.is_none() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy target is missing",
            gc,
        ));
    }

    // 2. Assert: proxy.[[ProxyHandler]] is not null.
    if proxy_data.handler.is_none() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy handler is missing",
            gc,
        ));
    }

    // 3. Return unused.
    Ok(())
}
