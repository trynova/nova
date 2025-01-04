// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::RegExpFlags;

use crate::ecmascript::abstract_operations::operations_on_objects::{set, try_set};
use crate::ecmascript::types::IntoObject;
use crate::engine::context::GcScope;
use crate::{
    ecmascript::{
        builtins::ordinary::ordinary_create_from_constructor,
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{Function, String, BUILTIN_STRING_MEMORY},
    },
    heap::CreateHeapData,
};

use super::{RegExp, RegExpHeapData, RegExpLastIndex};

/// ### [22.2.3.1 RegExpCreate ( P, F )]()
///
/// The abstract operation RegExpCreate takes arguments P (an ECMAScript
/// language value) and F (a String or undefined) and returns either a normal
/// completion containing an Object or a throw completion.
///
/// This is a variant for RegExp literal creation that cannot fail and skips
/// all of the abstract operation busy-work.
pub(crate) fn reg_exp_create_literal(
    agent: &mut Agent,
    p: String,
    f: Option<RegExpFlags>,
) -> RegExp {
    //     1. Let obj be ! RegExpAlloc(%RegExp%).
    //     2. Return ? RegExpInitialize(obj, P, F).
    let f = f.unwrap_or(RegExpFlags::empty());
    agent.heap.create(RegExpHeapData {
        object_index: None,
        original_source: p.unbind(),
        original_flags: f,
        last_index: RegExpLastIndex::ZERO,
    })
}

/// ### [22.2.3.2 RegExpAlloc ( newTarget )]()
///
/// The abstract operation RegExpAlloc takes argument newTarget (a constructor)
/// and returns either a normal completion containing an Object or a throw
/// completion.
pub(crate) fn reg_exp_alloc(
    agent: &mut Agent,
    new_target: Function,
    gc: GcScope<'_, '_>,
) -> JsResult<RegExp> {
    // 1. Let obj be ? OrdinaryCreateFromConstructor(newTarget, "%RegExp.prototype%", « [[OriginalSource]], [[OriginalFlags]], [[RegExpRecord]], [[RegExpMatcher]] »).
    let obj = RegExp::try_from(ordinary_create_from_constructor(
        agent,
        new_target,
        ProtoIntrinsics::RegExp,
        gc,
    )?)
    .unwrap();
    // 2. Perform ! DefinePropertyOrThrow(obj, "lastIndex", PropertyDescriptor { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false }).
    // TODO: lastIndex should be in RegExpHeapData itself, one way or another.
    // 3. Return obj.
    Ok(obj)
}

/// ### [22.2.3.3 RegExpInitialize ( obj, pattern, flags )]()
///
/// The abstract operation RegExpInitialize takes arguments obj (an Object),
/// pattern (an ECMAScript language value), and flags (an ECMAScript language
/// value) and returns either a normal completion containing an Object or a
/// throw completion.
pub(crate) fn reg_exp_initialize_from_string(
    agent: &mut Agent,
    obj: RegExp,
    p: String,
    flags: Option<RegExpFlags>,
    gc: GcScope,
) -> JsResult<RegExp> {
    //     3. If flags is undefined, let F be the empty String.
    let f = flags.unwrap_or(RegExpFlags::empty());
    //     4. Else, let F be ? ToString(flags).
    //     5. If F contains any code unit other than "d", "g", "i", "m", "s", "u", "v", or "y", or if F contains any code unit more than once, throw a SyntaxError exception.
    //     6. If F contains "i", let i be true; else let i be false.
    //     7. If F contains "m", let m be true; else let m be false.
    //     8. If F contains "s", let s be true; else let s be false.
    //     9. If F contains "u", let u be true; else let u be false.
    //     10. If F contains "v", let v be true; else let v be false.
    //     11. If u is true or v is true, then
    //         a. Let patternText be StringToCodePoints(P).
    //     12. Else,
    //         a. Let patternText be the result of interpreting each of P's 16-bit elements as a Unicode BMP code point. UTF-16 decoding is not applied to the elements.
    //     13. Let parseResult be ParsePattern(patternText, u, v).
    //     14. If parseResult is a non-empty List of SyntaxError objects, throw a SyntaxError exception.
    //     15. Assert: parseResult is a Pattern Parse Node.
    //     16. Set obj.[[OriginalSource]] to P.
    agent[obj].original_source = p.unbind();
    //     17. Set obj.[[OriginalFlags]] to F.
    agent[obj].original_flags = f;
    //     18. Let capturingGroupsCount be CountLeftCapturingParensWithin(parseResult).
    //     19. Let rer be the RegExp Record { [[IgnoreCase]]: i, [[Multiline]]: m, [[DotAll]]: s, [[Unicode]]: u, [[UnicodeSets]]: v, [[CapturingGroupsCount]]: capturingGroupsCount }.
    //     20. Set obj.[[RegExpRecord]] to rer.
    //     21. Set obj.[[RegExpMatcher]] to CompilePattern of parseResult with argument rer.
    //     22. Perform ? Set(obj, "lastIndex", +0𝔽, true).
    //     23. Return obj.
    if try_set(
        agent,
        obj.into_object(),
        BUILTIN_STRING_MEMORY.lastIndex.into(),
        0.into(),
        true,
        gc.nogc(),
    )
    .is_continue()
    {
        Ok(obj)
    } else {
        let scoped_obj = obj.into_object().scope(agent, gc.nogc());
        set(
            agent,
            obj.into_object(),
            BUILTIN_STRING_MEMORY.lastIndex.into(),
            0.into(),
            true,
            gc,
        )?;
        Ok(RegExp::try_from(scoped_obj.get(agent)).unwrap())
    }
}

/// ### [22.2.3.4 Static Semantics: ParsePattern ( patternText, u, v )]()
///
/// The abstract operation ParsePattern takes arguments patternText (a sequence of Unicode code points), u (a Boolean), and v (a Boolean) and returns a Parse Node or a non-empty List of SyntaxError objects.
///
/// > #### Note
/// > This section is amended in B.1.2.9.
pub(crate) fn parse_pattern() {
    //     1. If v is true and u is true, then
    //         a. Let parseResult be a List containing one or more SyntaxError objects.
    //     2. Else if v is true, then
    //         a. Let parseResult be ParseText(patternText, Pattern[+UnicodeMode, +UnicodeSetsMode, +NamedCaptureGroups]).
    //     3. Else if u is true, then
    //         a. Let parseResult be ParseText(patternText, Pattern[+UnicodeMode, ~UnicodeSetsMode, +NamedCaptureGroups]).
    //     4. Else,
    //         a. Let parseResult be ParseText(patternText, Pattern[~UnicodeMode, ~UnicodeSetsMode, +NamedCaptureGroups]).
    //     5. Return parseResult.
}
