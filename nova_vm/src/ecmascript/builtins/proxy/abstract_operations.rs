// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        execution::{agent::ExceptionType, Agent, JsResult},
        types::Object,
    },
    engine::context::NoGcScope,
};

use super::Proxy;

#[derive(Debug, Clone, Copy)]
pub(crate) struct NonRevokedProxy<'a> {
    pub(crate) target: Object<'a>,
    pub(crate) handler: Object<'a>,
}

/// ### [10.5.14 ValidateNonRevokedProxy ( proxy )](https://tc39.es/ecma262/#sec-validatenonrevokedproxy)
///
/// The abstract operation ValidateNonRevokedProxy takes argument
/// proxy (a Proxy exotic object) and returns either a normal completion containing unused or a throw completion.
/// It throws a TypeError exception if proxy has been revoked.
pub(crate) fn validate_non_revoked_proxy<'a>(
    agent: &mut Agent,
    proxy: Proxy,
    gc: NoGcScope<'a, '_>,
) -> JsResult<NonRevokedProxy<'a>> {
    let proxy_data = &agent[proxy];

    // 1. If proxy.[[ProxyTarget]] is null, throw a TypeError exception.
    let Some(target) = proxy_data.proxy_target else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy target is missing",
            gc,
        ));
    };

    // 2. Assert: proxy.[[ProxyHandler]] is not null.
    let Some(handler) = proxy_data.proxy_handler else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy handler is missing",
            gc,
        ));
    };

    // 3. Return unused.
    Ok(NonRevokedProxy { target, handler })
}
