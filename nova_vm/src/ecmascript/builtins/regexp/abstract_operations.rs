// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::ControlFlow;

use oxc_allocator::Allocator;
use oxc_ast::ast::RegExpFlags;
use oxc_regular_expression::{LiteralParser, Options};

use crate::{
    ecmascript::{
        Agent, ArgumentsList, Array, BUILTIN_STRING_MEMORY, ExceptionType, Function, JsResult,
        Number, Object, PropertyKey, PropertyLookupCache, ProtoIntrinsics, RegExp, RegExpHeapData,
        RegExpLastIndex, String, TryError, TryGetResult, Value, array_create, call_function,
        handle_try_get_result, is_callable, ordinary_create_from_constructor,
        ordinary_object_create_null, throw_set_error, to_length, to_string,
        try_create_data_property_or_throw, try_get, try_result_into_js, try_to_length, unwrap_try,
        unwrap_try_get_value,
    },
    engine::{
        Scoped,
        Bindable, GcScope, NoGcScope, bindable_handle,
        Scopable,
    },
    heap::{ArenaAccess, ArenaAccessMut, CreateHeapData, DirectArenaAccessMut},
};

/// ### [22.2.3.1 RegExpCreate ( P, F )](https://tc39.es/ecma262/#sec-regexpcreate)
///
/// The abstract operation RegExpCreate takes arguments P (an ECMAScript
/// language value) and F (a String or undefined) and returns either a normal
/// completion containing an Object or a throw completion.
pub(crate) fn reg_exp_create<'a>(
    agent: &mut Agent,
    p: Scoped<Value>,
    f: Option<String>,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, RegExp<'a>> {
    let f = f.map_or(Ok(RegExpFlags::empty()), |f| {
        Err(Value::from(f).scope(agent, gc.nogc()))
    });
    // 1. Let obj be ! RegExpAlloc(%RegExp%).
    let obj = agent.heap.create(RegExpHeapData::default()).bind(gc.nogc());
    // 2. Return ? RegExpInitialize(obj, P, F).
    reg_exp_initialize(agent, obj.unbind(), p, f, gc)
}

/// ### [22.2.3.1 RegExpCreate ( P, F )](https://tc39.es/ecma262/#sec-regexpcreate)
///
/// The abstract operation RegExpCreate takes arguments P (an ECMAScript
/// language value) and F (a String or undefined) and returns either a normal
/// completion containing an Object or a throw completion.
///
/// This is a variant for RegExp literal creation that cannot fail and skips
/// all of the abstract operation busy-work.
pub(crate) fn reg_exp_create_literal<'a>(
    agent: &mut Agent,
    p: String,
    f: Option<RegExpFlags>,
    gc: NoGcScope<'a, '_>,
) -> RegExp<'a> {
    // 1. Let obj be ! RegExpAlloc(%RegExp%).
    // 2. Return ? RegExpInitialize(obj, P, F).
    let f = f.unwrap_or(RegExpFlags::empty());
    agent.heap.create(RegExpHeapData::new(agent, p, f)).bind(gc)
}

/// ### [22.2.3.2 RegExpAlloc ( newTarget )](https://tc39.es/ecma262/#sec-regexpalloc)
///
/// The abstract operation RegExpAlloc takes argument newTarget (a constructor)
/// and returns either a normal completion containing an Object or a throw
/// completion.
pub(crate) fn reg_exp_alloc<'a>(
    agent: &mut Agent,
    new_target: Function,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, RegExp<'a>> {
    // 1. Let obj be ? OrdinaryCreateFromConstructor(newTarget, "%RegExp.prototype%", « [[OriginalSource]], [[OriginalFlags]], [[RegExpRecord]], [[RegExpMatcher]] »).
    let obj = RegExp::try_from(ordinary_create_from_constructor(
        agent,
        new_target,
        ProtoIntrinsics::RegExp,
        gc,
    )?)
    .unwrap();
    // 2. Perform ! DefinePropertyOrThrow(obj, "lastIndex", PropertyDescriptor { [[Writable]]: true, [[Enumerable]]: false, [[Configurable]]: false }).
    // 3. Return obj.
    Ok(obj)
}

/// ### [22.2.3.3 RegExpInitialize ( obj, pattern, flags )](https://tc39.es/ecma262/#sec-regexpinitialize)
///
/// The abstract operation RegExpInitialize takes arguments obj (an Object),
/// pattern (an ECMAScript language value), and flags (an ECMAScript language
/// value) and returns either a normal completion containing an Object or a
/// throw completion.
pub(crate) fn reg_exp_initialize<'a>(
    agent: &mut Agent,
    obj: RegExp,
    scoped_pattern: Scoped<Value>,
    scoped_flags: Result<RegExpFlags, Scoped<Value>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, RegExp<'a>> {
    let obj = obj.bind(gc.nogc());
    // SAFETY: not shared.
    let pattern = unsafe { scoped_pattern.take(agent).bind(gc.nogc()) };
    let flags = scoped_flags
        .as_ref()
        .map(|f| *f)
        .map_err(|v| v.get(agent).bind(gc.nogc()));
    let quick_pattern = if pattern.is_undefined() {
        Some(None)
    } else if let Ok(pattern) = String::try_from(pattern) {
        Some(Some(pattern))
    } else {
        None
    };
    let quick_flags = match flags {
        Ok(f) => Some(Ok(f)),
        Err(flags) => {
            if flags.is_undefined() {
                Some(Ok(RegExpFlags::empty()))
            } else if let Ok(f) = String::try_from(flags) {
                Some(Err(f))
            } else {
                None
            }
        }
    };
    let (obj, p, f) = if let (Some(p), Some(f)) = (quick_pattern, quick_flags) {
        let p = p.unwrap_or(String::EMPTY_STRING);
        (obj, p, f)
    } else {
        let obj = obj.scope(agent, gc.nogc());
        let flags = scoped_flags;
        // 1. If pattern is undefined, let P be the empty String.
        let mut p = if pattern.is_undefined() {
            String::EMPTY_STRING
        } else {
            // 2. Else, let P be ? ToString(pattern).
            to_string(agent, pattern.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc())
        };
        let f = match flags {
            Ok(f) => Ok(f),
            Err(flags) => {
                let flags = unsafe { flags.take(agent) }.bind(gc.nogc());
                // 3. If flags is undefined,
                if flags.is_undefined() {
                    // let F be the empty String.
                    Ok(RegExpFlags::empty())
                } else {
                    // 4. Else, let F be ? ToString(flags).
                    let scoped_p = p.scope(agent, gc.nogc());
                    let f = to_string(agent, flags.unbind(), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                    p = unsafe { scoped_p.take(agent) }.bind(gc.nogc());
                    Err(f)
                }
            }
        };
        let obj = unsafe { obj.take(agent) }.bind(gc.nogc());
        (obj, p, f)
    };
    // 5. If F contains any code unit other than "d", "g", "i", "m", "s", "u",
    //    "v", or "y", or if F contains any code unit more than once, throw a
    //    SyntaxError exception.
    // 6. If F contains "i", let i be true; else let i be false.
    // 7. If F contains "m", let m be true; else let m be false.
    // 8. If F contains "s", let s be true; else let s be false.
    // 9. If F contains "u", let u be true; else let u be false.
    // 10. If F contains "v", let v be true; else let v be false.
    // 11. If u is true or v is true, then
    //     a. Let patternText be StringToCodePoints(P).
    // 12. Else,
    //     a. Let patternText be the result of interpreting each of P's 16-bit
    //     elements as a Unicode BMP code point. UTF-16 decoding is not applied
    //     to the elements.
    let f_str = f.map(|f| f.to_inline_string());
    let f_str = match &f_str {
        Ok(f) => f.as_str().into(),
        Err(f) => f.to_string_lossy_(agent),
    };
    let flags: Option<&str> = if f_str.is_empty() {
        None
    } else {
        Some(f_str.as_ref())
    };

    let allocator = Allocator::new();
    // 13. Let parseResult be ParsePattern(patternText, u, v).
    match LiteralParser::new(
        &allocator,
        &p.to_string_lossy_(agent),
        flags,
        Options::default(),
    )
    .parse()
    {
        Ok(_) => {
            // 15. Assert: parseResult is a Pattern Parse Node.
        }
        // 14. If parseResult is a non-empty List of SyntaxError objects,
        Err(err) => {
            // throw a SyntaxError exception.
            return Err(agent.throw_exception(
                ExceptionType::SyntaxError,
                err.message.to_string(),
                gc.into_nogc(),
            ));
        }
    };
    let f = f.unwrap_or_else(|f| parse_flags(&f.to_string_lossy_(agent)).unwrap());
    // 18. Let capturingGroupsCount be CountLeftCapturingParensWithin(parseResult).
    // 19. Let rer be the RegExp Record { [[IgnoreCase]]: i, [[Multiline]]: m, [[DotAll]]: s, [[Unicode]]: u, [[UnicodeSets]]: v, [[CapturingGroupsCount]]: capturingGroupsCount }.
    // 21. Set obj.[[RegExpMatcher]] to CompilePattern of parseResult with argument rer.
    let reg_exp_matcher = RegExpHeapData::compile_pattern(&p.to_string_lossy_(agent), f);
    {
        let data = obj.get_mut(agent);
        // 16. Set obj.[[OriginalSource]] to P.
        data.original_source = p.unbind();
        // 17. Set obj.[[OriginalFlags]] to F.
        data.original_flags = f;
        // 20. Set obj.[[RegExpRecord]] to rer.
        // 21. Set obj.[[RegExpMatcher]] to CompilePattern of parseResult with argument rer.
        data.reg_exp_matcher = reg_exp_matcher;
    }

    // 22. Perform ? Set(obj, "lastIndex", +0𝔽, true).
    if !obj.set_last_index(agent, RegExpLastIndex::ZERO, gc.nogc()) {
        return throw_set_error(
            agent,
            BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
            gc.into_nogc(),
        )
        .into();
    }
    // 23. Return obj.
    Ok(obj.unbind())
}

fn parse_flags(f: &str) -> Option<RegExpFlags> {
    let mut flags: u8 = 0;
    for cu in f.as_bytes() {
        match cu {
            b'd' => flags |= RegExpFlags::D.bits(),
            b'g' => flags |= RegExpFlags::G.bits(),
            // 6. If F contains "i", let i be true; else let i be false.
            b'i' => flags |= RegExpFlags::I.bits(),
            // 7. If F contains "m", let m be true; else let m be false.
            b'm' => flags |= RegExpFlags::M.bits(),
            // 8. If F contains "s", let s be true; else let s be false.
            b's' => flags |= RegExpFlags::S.bits(),
            // 9. If F contains "u", let u be true; else let u be false.
            b'u' => flags |= RegExpFlags::U.bits(),
            // 10. If F contains "v", let v be true; else let v be false.
            b'v' => flags |= RegExpFlags::V.bits(),
            b'y' => flags |= RegExpFlags::Y.bits(),
            // 5. If F contains any code unit other than "d", "g", "i", "m",
            //    "s", "u", "v", or "y", or if F contains any code unit more
            //    than once, throw a SyntaxError exception.
            _ => return None,
        }
    }
    Some(RegExpFlags::from_bits_retain(flags))
}

#[inline]
pub(crate) fn require_internal_slot_reg_exp<'a>(
    agent: &mut Agent,
    o: Value,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, RegExp<'a>> {
    // 1. If O is not an Object, throw a TypeError exception.
    let Ok(o) = Object::try_from(o) else {
        let error_message = format!(
            "{} is not an object",
            o.unbind()
                .try_string_repr(agent, gc)
                .to_string_lossy_(agent)
        );
        return Err(agent.throw_exception(ExceptionType::TypeError, error_message, gc));
    };
    require_internal_slot_reg_exp_object(agent, o, gc)
}

#[inline]
pub(crate) fn require_internal_slot_reg_exp_object<'a>(
    agent: &mut Agent,
    o: Object,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, RegExp<'a>> {
    match o {
        // 1. Perform ? RequireInternalSlot(O, [[RegExpMatcher]]).
        Object::RegExp(reg_exp) => Ok(reg_exp.unbind().bind(gc)),
        _ => Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Expected this to be RegExp",
            gc,
        )),
    }
}

/// ### [22.2.7.1 RegExpExec ( R, S )](https://tc39.es/ecma262/#sec-regexpexec)
///
/// The abstract operation RegExpExec takes arguments R (an Object) and S (a
/// String) and returns either a normal completion containing either an Object
/// or null, or a throw completion.
///
/// > NOTE: If a callable "exec" property is not found this algorithm falls
/// > back to attempting to use the built-in RegExp matching algorithm. This
/// > provides compatible behaviour for code written for prior editions where
/// > most built-in algorithms that use regular expressions did not perform a
/// > dynamic property lookup of "exec".
pub(crate) fn reg_exp_exec<'a>(
    agent: &mut Agent,
    r: Object,
    s: String,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Object<'a>>> {
    let (r, s) = match reg_exp_exec_prepare(agent, r, s, gc.reborrow()) {
        ControlFlow::Continue(r) => r,
        ControlFlow::Break(result) => return result.unbind().bind(gc.into_nogc()),
    };

    // 4. Return ? RegExpBuiltinExec(R, S).
    reg_exp_builtin_exec(agent, r.unbind(), s.unbind(), gc).map(|o| o.map(|o| o.into()))
}

/// Performs steps 1-3 of RegExpExec
fn reg_exp_exec_prepare<'a>(
    agent: &mut Agent,
    r: Object,
    s: String,
    mut gc: GcScope<'a, '_>,
) -> ControlFlow<JsResult<'a, Option<Object<'a>>>, (RegExp<'a>, String<'a>)> {
    let mut s = s.bind(gc.nogc());
    let mut r = r.bind(gc.nogc());
    // 1. Let exec be ? Get(R, "exec").
    let key = BUILTIN_STRING_MEMORY.exec.to_property_key();
    let exec = try_get(
        agent,
        r,
        key,
        PropertyLookupCache::get(agent, key),
        gc.nogc(),
    );
    let exec = match exec {
        ControlFlow::Continue(TryGetResult::Unset) => Value::Undefined,
        ControlFlow::Continue(TryGetResult::Value(v)) => v,
        ControlFlow::Break(TryError::Err(e)) => {
            return ControlFlow::Break(Err(e.unbind().bind(gc.into_nogc())));
        }
        _ => {
            let scoped_r = r.scope(agent, gc.nogc());
            let scoped_s = s.scope(agent, gc.nogc());
            let exec = handle_try_get_result(
                agent,
                r.unbind(),
                BUILTIN_STRING_MEMORY.exec.to_property_key(),
                exec.unbind(),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());
            let exec = match exec {
                Ok(e) => e,
                Err(err) => return ControlFlow::Break(Err(err.unbind())),
            };
            let gc = gc.nogc();
            // SAFETY: Not shared.
            unsafe {
                s = scoped_s.take(agent).bind(gc);
                r = scoped_r.take(agent).bind(gc);
            }
            exec
        }
    };

    // Fast path: native RegExp object and intrinsic exec function.
    if let Object::RegExp(r) = r
        && exec
            == agent
                .current_realm_record()
                .intrinsics()
                .reg_exp_prototype_exec()
                .into()
    {
        return ControlFlow::Continue((r.unbind(), s.unbind()));
    }

    // 2. If IsCallable(exec) is true, then
    if let Some(exec) = is_callable(exec, gc.nogc()) {
        // a. Let result be ? Call(exec, R, « S »).
        let result = call_function(
            agent,
            exec.unbind(),
            r.unbind().into(),
            Some(ArgumentsList::from_mut_value(&mut s.unbind().into())),
            gc.reborrow(),
        )
        .unbind();
        let gc = gc.into_nogc();
        let result = result.bind(gc);
        let result = match result {
            Ok(r) => r,
            Err(err) => return ControlFlow::Break(Err(err)),
        };
        // b. If result is not an Object and result is not null,
        let result = if let Ok(result) = Object::try_from(result) {
            Some(result)
        } else if result.is_null() {
            None
        } else {
            // throw a TypeError exception.
            return ControlFlow::Break(Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'exec' function result was not object or null",
                gc,
            )));
        };
        // c. Return result.
        return ControlFlow::Break(Ok(result));
    }
    // 3. Perform ? RequireInternalSlot(R, [[RegExpMatcher]]).
    let r = require_internal_slot_reg_exp_object(agent, r, gc.nogc()).unbind();
    let s = s.unbind();
    let gc = gc.into_nogc();
    let r = r.bind(gc);
    let s = s.bind(gc);
    let r = match r {
        Ok(r) => r,
        Err(err) => return ControlFlow::Break(Err(err)),
    };
    ControlFlow::Continue((r, s))
}

pub(crate) fn reg_exp_test<'a>(
    agent: &mut Agent,
    r: Object,
    s: String,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let (r, s) = match reg_exp_exec_prepare(agent, r, s, gc.reborrow()) {
        ControlFlow::Continue(r) => r,
        ControlFlow::Break(result) => {
            return result.unbind().bind(gc.into_nogc()).map(|r| r.is_some());
        }
    };
    // 4. Return ? RegExpBuiltinExec(R, S).
    reg_exp_builtin_test(agent, r.unbind(), s.unbind(), gc)
}

pub(crate) struct RegExpExecBase<'gc> {
    pub(crate) r: RegExp<'gc>,
    pub(crate) s: String<'gc>,
    pub(crate) last_index: usize,
    pub(crate) global: bool,
    pub(crate) sticky: bool,
    pub(crate) has_indices: bool,
    #[expect(dead_code)]
    pub(crate) full_unicode: bool,
}

bindable_handle!(RegExpExecBase);

pub(crate) fn reg_exp_builtin_exec_prepare<'a>(
    agent: &mut Agent,
    r: RegExp,
    s: String,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, RegExpExecBase<'a>> {
    let mut r = r.bind(gc.nogc());
    let mut s = s.bind(gc.nogc());

    // 1. Let length be the length of S.
    // 2. Let lastIndex be ℝ(? ToLength(? Get(R, "lastIndex"))).
    let mut last_index = if let Some(last_index) = r.try_get_last_index(agent) {
        last_index as usize
    } else {
        // Note: calling Get(R, "lastIndex") cannot trigger JavaScript
        // execution, as the "lastIndex" property is always an unconfigurable
        // data property of every RegExp object.
        let last_index = unwrap_try_get_value(try_get(
            agent,
            r,
            BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
            None,
            gc.nogc(),
        ));
        if let Some(last_index) =
            try_result_into_js(try_to_length(agent, last_index, gc.nogc())).unbind()?
        {
            last_index as usize
        } else {
            let scoped_r = r.scope(agent, gc.nogc());
            let scoped_s = s.scope(agent, gc.nogc());
            let last_index =
                to_length(agent, last_index.unbind(), gc.reborrow()).unbind()? as usize;
            // SAFETY: Not shared.
            unsafe {
                s = scoped_s.take(agent).bind(gc.nogc());
                r = scoped_r.take(agent).bind(gc.nogc());
            }
            last_index
        }
    };
    let r = r.unbind();
    let s = s.unbind();
    let gc = gc.into_nogc();
    let r = r.bind(gc);
    let s = s.bind(gc);

    // 3. Let flags be R.[[OriginalFlags]].
    let flags = r.original_flags(agent);
    // 4. If flags contains "g", let global be true; else let global be false.
    let global = (flags & RegExpFlags::G).bits() > 0;
    // 5. If flags contains "y", let sticky be true; else let sticky be false.
    let sticky = (flags & RegExpFlags::Y).bits() > 0;
    // 6. If flags contains "d", let hasIndices be true; else let hasIndices be false.
    let has_indices = (flags & RegExpFlags::D).bits() > 0;
    // 7. If global is false and sticky is false, set lastIndex to 0.
    if !global && !sticky {
        last_index = 0;
    }
    let last_index = if last_index > s.len_(agent) {
        last_index
    } else {
        s.utf8_index_(agent, last_index).unwrap_or(last_index)
    };
    // 8. Let matcher be R.[[RegExpMatcher]].
    if let Err(err) = &r.get(agent).reg_exp_matcher {
        return Err(agent.throw_exception(ExceptionType::SyntaxError, err.to_string(), gc));
    };
    // 9. If flags contains "u" or flags contains "v", let fullUnicode be true;
    //    else let fullUnicode be false.
    let full_unicode = (flags & (RegExpFlags::U | RegExpFlags::V)).bits() > 0;
    Ok(RegExpExecBase {
        r,
        s,
        last_index,
        global,
        sticky,
        has_indices,
        full_unicode,
    })
}

/// ### [22.2.7.2 RegExpBuiltinExec ( R, S )](https://tc39.es/ecma262/#sec-regexpbuiltinexec)
///
/// The abstract operation RegExpBuiltinExec takes arguments R (an initialized
/// RegExp instance) and S (a String) and returns either a normal completion
/// containing either an Array exotic object or null, or a throw completion.
pub(crate) fn reg_exp_builtin_exec<'a>(
    agent: &mut Agent,
    r: RegExp,
    s: String,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, Option<Array<'a>>> {
    let r = r.bind(gc.nogc());
    let s = s.bind(gc.nogc());
    let result =
        reg_exp_builtin_exec_prepare(agent, r.unbind(), s.unbind(), gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let RegExpExecBase {
        r,
        s,
        last_index,
        global,
        sticky,
        has_indices,
        full_unicode: _,
    } = result.bind(gc);
    // 1. Let length be the length of S.
    let length = s.len_(agent);
    let r_data = r.get_direct_mut(&mut agent.heap.regexps);
    let s_bytes = s.as_bytes_(&agent.heap.strings);
    // 8. Let matcher be R.[[RegExpMatcher]].
    // SAFETY: reg_exp_builtin_exec_base checks that the matcher is set.
    let matcher = unsafe { r_data.reg_exp_matcher.as_mut().unwrap_unchecked() };
    // 10. Let matchSucceeded be false.
    // 11. If fullUnicode is true, let input be StringToCodePoints(S).
    //     Otherwise, let input be a List whose elements are the code units
    //     that are the elements of S.
    // 12. NOTE: Each element of input is considered to be a character.
    // 13. Repeat, while matchSucceeded is false,
    // a. If lastIndex > length, then
    if last_index > length {
        // i. If global is true or sticky is true, then
        if global || sticky {
            // 1. Perform ? Set(R, "lastIndex", +0𝔽, true).
            r_data.last_index = RegExpLastIndex::ZERO;
        }
        // ii. Return null.
        return Ok(None);
    }
    // b. Let inputIndex be the index into input of the character that was
    //    obtained from element lastIndex of S.
    let input_index = last_index;
    // c. Let r be matcher(input, inputIndex).
    let result = matcher.captures_at(s_bytes, input_index);
    // d. If r is failure, then
    let Some(result) = result else {
        // i. If global is true or sticky is true, then
        if global || sticky {
            // 1. Perform ? Set(R, "lastIndex", +0𝔽, true).
            r_data.last_index = RegExpLastIndex::ZERO;
        }
        // ii. Return null.
        return Ok(None);
    };
    // SAFETY: first capture group is always the full match.
    let full_match = unsafe { result.get(0).unwrap_unchecked() };
    // i. If sticky is true, then
    if sticky && full_match.start() != last_index {
        // sticky did match but not at the start position.
        // 1. Perform ? Set(R, "lastIndex", +0𝔽, true).
        r_data.last_index = RegExpLastIndex::ZERO;
        // 2. Return null.
        return Ok(None);
        // ii. Set lastIndex to AdvanceStringIndex(S, lastIndex, fullUnicode).
    }
    let last_index = full_match.start();
    // e. Else,
    // i. Assert: r is a MatchState.
    // ii. Set matchSucceeded to true.
    // 14. Let e be r.[[EndIndex]].
    let e = full_match.end();
    // 15. If fullUnicode is true, set e to GetStringIndex(S, e).
    let e = s.utf16_index_(&agent.heap.strings, e);
    // 16. If global is true or sticky is true, then
    if global || sticky {
        // a. Perform ? Set(R, "lastIndex", 𝔽(e), true).
        r_data.last_index = e.into();
    }
    // 17. Let n be the number of elements in r.[[Captures]].
    let n = result.len();
    // 18. Assert: n = R.[[RegExpRecord]].[[CapturingGroupsCount]].
    debug_assert_eq!(n, matcher.captures_len());
    // 19. Assert: n < 2**32 - 1.
    debug_assert!(n < 2usize.pow(32) - 1);
    let has_group_name = matcher.capture_names().any(|n| n.is_some());
    // 20. Let A be ! ArrayCreate(n + 1).
    // Note: we use n because it already contains the full-match group in it.
    let a = array_create(agent, n, n, None, gc).unwrap();
    // 21. Assert: The mathematical value of A's "length" property is n + 1.
    debug_assert_eq!(a.len(agent) as usize, n);
    // 22. Perform ! CreateDataPropertyOrThrow(A, "index", 𝔽(lastIndex)).
    unwrap_try(try_create_data_property_or_throw(
        agent,
        a,
        BUILTIN_STRING_MEMORY.index.to_property_key(),
        Number::try_from(last_index).unwrap().into(),
        None,
        gc,
    ));
    let input = String::from_static_str(agent, "input", gc).to_property_key();
    // 23. Perform ! CreateDataPropertyOrThrow(A, "input", S).
    unwrap_try(try_create_data_property_or_throw(
        agent,
        a,
        input,
        s.into(),
        None,
        gc,
    ));
    // 24. Let match be the Match Record { [[StartIndex]]: lastIndex, [[EndIndex]]: e }.
    // 25. Let indices be a new empty List.
    // let mut indices = vec![];
    // 26. Let groupNames be a new empty List.
    // 27. Append match to indices.
    // 28. Let matchedSubstr be GetMatchString(S, match).
    // 29. Perform ! CreateDataPropertyOrThrow(A, "0", matchedSubstr).
    // 30. If R contains any GroupName, then
    let groups = if has_group_name {
        // a. Let groups be OrdinaryObjectCreate(null).
        // b. Let hasGroups be true.
        Some(ordinary_object_create_null(agent, gc))
    } else {
        // 31. Else,
        // a. Let groups be undefined.
        // b. Let hasGroups be false.
        None
    };
    let key = String::from_static_str(agent, "groups", gc).to_property_key();
    // 32. Perform ! CreateDataPropertyOrThrow(A, "groups", groups).
    unwrap_try(try_create_data_property_or_throw(
        agent,
        a,
        key,
        groups.map_or(Value::Undefined, |g| g.into()),
        None,
        gc,
    ));
    // 33. Let matchedGroupNames be a new empty List.
    // let mut matched_group_names = vec![];
    // 34. For each integer i such that 1 ≤ i ≤ n, in ascending order, do
    for (i, capture_i) in result.iter().enumerate() {
        // a. Let captureI be ith element of r.[[Captures]].
        if has_indices {
            // b. If captureI is undefined, then
            //         i. Let capturedValue be undefined.
            //         ii. Append undefined to indices.
            // c. Else,
            //         i. Let captureStart be captureI.[[StartIndex]].
            //         ii. Let captureEnd be captureI.[[EndIndex]].
            //         iii. If fullUnicode is true, then
            //                 1. Set captureStart to GetStringIndex(S, captureStart).
            //                 2. Set captureEnd to GetStringIndex(S, captureEnd).
            //         iv. Let capture be the Match Record { [[StartIndex]]: captureStart, [[EndIndex]]: captureEnd }.
            //         v. Let capturedValue be GetMatchString(S, capture).
            //         vi. Append capture to indices.
        }
        let captured_value = if let Some(capture_i) = capture_i {
            match std::string::String::from_utf8_lossy(capture_i.as_bytes()) {
                std::borrow::Cow::Borrowed(str) => String::from_str(agent, str, gc).into(),
                std::borrow::Cow::Owned(string) => String::from_string(agent, string, gc).into(),
            }
        } else {
            Value::Undefined
        };
        // d. Perform ! CreateDataPropertyOrThrow(A, ! ToString(𝔽(i)), capturedValue).
        unwrap_try(try_create_data_property_or_throw(
            agent,
            a,
            PropertyKey::try_from(i).unwrap(),
            captured_value,
            None,
            gc,
        ));
        // e. If the ith capture of R was defined with a GroupName, then
        //         i. Let s be the CapturingGroupName of that GroupName.
        //         ii. If matchedGroupNames contains s, then
        //                 1. Assert: capturedValue is undefined.
        //                 2. Append undefined to groupNames.
        //         iii. Else,
        //                 1. If capturedValue is not undefined, append s to matchedGroupNames.
        //                 2. NOTE: If there are multiple groups named s, groups may already have an s property at this point. However, because groups is an ordinary object whose properties are all writable data properties, the call to CreateDataPropertyOrThrow is nevertheless guaranteed to succeed.
        //                 3. Perform ! CreateDataPropertyOrThrow(groups, s, capturedValue).
        //                 4. Append s to groupNames.
        // f. Else,
        //         i. Append undefined to groupNames.
    }
    // 35. If hasIndices is true, then
    //         a. Let indicesArray be MakeMatchIndicesIndexPairArray(S, indices, groupNames, hasGroups).
    //         b. Perform ! CreateDataPropertyOrThrow(A, "indices", indicesArray).
    // 36. Return A.
    Ok(Some(a))
}

pub(crate) fn reg_exp_builtin_test<'a>(
    agent: &mut Agent,
    r: RegExp,
    s: String,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, bool> {
    let r = r.bind(gc.nogc());
    let s = s.bind(gc.nogc());
    // if (r.get(agent).original_flags & (RegExpFlags::G | RegExpFlags::S)).bits() > 0 {
    //     // We have to perform the actual matching because global and sticky
    //     // RegExp's can observe its match results from lastIndex.
    //     return reg_exp_builtin_exec(agent, r.unbind(), s.unbind(), gc).map(|a| a.is_some());
    // }
    let result =
        reg_exp_builtin_exec_prepare(agent, r.unbind(), s.unbind(), gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let RegExpExecBase {
        r,
        s,
        last_index,
        sticky,
        global,
        ..
    } = result.bind(gc);

    // 1. Let length be the length of S.
    let length = s.len_(agent);
    let r_data = r.get_direct_mut(&mut agent.heap.regexps);
    if last_index > length {
        // i. If global is true or sticky is true, then
        if global || sticky {
            // 1. Perform ? Set(R, "lastIndex", +0𝔽, true).
            r_data.last_index = RegExpLastIndex::ZERO;
        }
        // ii. Return null.
        return Ok(false);
    }
    let s_bytes = s.as_bytes_(&agent.heap.strings);
    // 8. Let matcher be R.[[RegExpMatcher]].
    // SAFETY: reg_exp_builtin_exec_base checks that the matcher is set.
    let matcher = unsafe { r_data.reg_exp_matcher.as_mut().unwrap_unchecked() };
    // 10. Let matchSucceeded be false.
    // 11. If fullUnicode is true, let input be StringToCodePoints(S).
    //     Otherwise, let input be a List whose elements are the code units
    //     that are the elements of S.
    // 12. NOTE: Each element of input is considered to be a character.
    // 13. Repeat, while matchSucceeded is false,
    // c. Let r be matcher(input, inputIndex).
    if global || sticky {
        // Global and sticky flags can observe where we found a test result, so
        // we need to actually find properly.
        let result = matcher.find_at(s_bytes, last_index);
        if let Some(result) = result {
            if sticky && result.start() != last_index {
                // sticky did match but not at the start position.
                // 1. Perform ? Set(R, "lastIndex", +0𝔽, true).
                r_data.last_index = RegExpLastIndex::ZERO;
                // 2. Return null.
                Ok(false)
            } else {
                // ii. Set lastIndex to AdvanceStringIndex(S, lastIndex, fullUnicode).
                let e = result.end();
                // 15. If fullUnicode is true, set e to GetStringIndex(S, e).
                let e = s.utf16_index_(&agent.heap.strings, e);
                // 16. If global is true or sticky is true, then
                // a. Perform ? Set(R, "lastIndex", 𝔽(e), true).
                r_data.last_index = e.into();
                Ok(true)
            }
        } else {
            // No match.
            r_data.last_index = RegExpLastIndex::ZERO;
            Ok(false)
        }
    } else {
        // Otherwise we can simply try-match.
        let result = matcher.is_match_at(s_bytes, last_index);
        r_data.last_index = RegExpLastIndex::ZERO;
        Ok(result)
    }
}

/// ### [22.2.7.3 AdvanceStringIndex ( S, index, unicode )](https://tc39.es/ecma262/#sec-advancestringindex)
///
/// The abstract operation AdvanceStringIndex takes arguments S (a String),
/// index (a non-negative integer), and unicode (a Boolean) and returns an
/// integer.
pub(crate) fn advance_string_index(agent: &Agent, s: String, index: usize, unicode: bool) -> usize {
    // 1. Assert: index ≤ 2**53 - 1.
    assert!(index < 2usize.pow(53));
    // 2. If unicode is false, return index + 1.
    if !unicode {
        return index + 1;
    }
    // 3. Let length be the length of S.
    let length = s.utf16_len_(agent);
    // 4. If index + 1 ≥ length, return index + 1.
    if index + 1 >= length {
        return index + 1;
    }
    // 5. Let cp be CodePointAt(S, index).
    let cp = s.code_point_at_(agent, index);
    // 6. Return index + cp.[[CodeUnitCount]].
    let code = cp.to_u32();
    if (code & !0xFFFF) > 0 {
        // Two-code-unit character.
        index + 2
    } else {
        index + 1
    }
}
