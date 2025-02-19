// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

// Nova-specific test262 harness.

// This file is run for every test262 test, before any imports. The reason its
// contents aren't provided by test262 itself is because they are meant to
// provide test262 access to host-specific capabilities that are not part of
// the JS standard, such as creating realms and workers, triggering GC...
// For more info, see
// https://github.com/tc39/test262/blob/main/INTERPRETING.md#host-defined-functions
//
// However, since Nova's test262 runner uses nova_cli, which does not provide
// access to any of those capabilities, this file currently only provides access
// to `$262.global` (needed for old tests, since `globalThis` was only added in
// ES2020).

globalThis.$262 = {
    global: globalThis,
    detachArrayBuffer: __nova__.detachArrayBuffer,
};
delete globalThis.__nova__;