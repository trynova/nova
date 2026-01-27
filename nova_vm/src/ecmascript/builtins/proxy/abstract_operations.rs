// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{Agent, ExceptionType, JsResult, Object},
    engine::context::{Bindable, NoGcScope, bindable_handle},
    heap::ArenaAccess,
};

use super::{Proxy, data::ProxyHeapData};

#[derive(Debug, Clone, Copy)]
pub(crate) struct NonRevokedProxy<'a> {
    pub(crate) target: Object<'a>,
    pub(crate) handler: Object<'a>,
}

bindable_handle!(NonRevokedProxy);

/// ### [10.5.14 ValidateNonRevokedProxy ( proxy )](https://tc39.es/ecma262/#sec-validatenonrevokedproxy)
///
/// The abstract operation ValidateNonRevokedProxy takes argument
/// proxy (a Proxy exotic object) and returns either a normal completion
/// containing unused or a throw completion. It throws a TypeError exception if
/// proxy has been revoked.
pub(crate) fn validate_non_revoked_proxy<'a>(
    agent: &mut Agent,
    proxy: Proxy,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, NonRevokedProxy<'a>> {
    let ProxyHeapData::NonRevoked {
        proxy_handler: handler,
        proxy_target: target,
    } = proxy.get(agent)
    else {
        // 1. If proxy.[[ProxyTarget]] is null, throw a TypeError exception.
        // 2. Assert: proxy.[[ProxyHandler]] is not null.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Proxy target is missing",
            gc,
        ));
    };

    // 3. Return unused.
    Ok(NonRevokedProxy {
        target: target.bind(gc),
        handler: handler.bind(gc),
    })
}

/// ### [10.5.14 ValidateNonRevokedProxy ( proxy )](https://tc39.es/ecma262/#sec-validatenonrevokedproxy)
///
/// The abstract operation ValidateNonRevokedProxy takes argument
/// proxy (a Proxy exotic object) and returns either a normal completion
/// containing unused or a throw completion.
///
/// NOTE: This method returns None if the proxy has been revoked.
pub(crate) fn try_validate_non_revoked_proxy<'a>(
    agent: &Agent,
    proxy: Proxy,
    gc: NoGcScope<'a, '_>,
) -> Option<NonRevokedProxy<'a>> {
    let ProxyHeapData::NonRevoked {
        proxy_handler: handler,
        proxy_target: target,
    } = proxy.get(agent)
    else {
        // 1. If proxy.[[ProxyTarget]] is null, throw a TypeError exception.
        // 2. Assert: proxy.[[ProxyHandler]] is not null.
        return None;
    };

    // 3. Return unused.
    Some(NonRevokedProxy {
        target: target.bind(gc),
        handler: handler.bind(gc),
    })
}
