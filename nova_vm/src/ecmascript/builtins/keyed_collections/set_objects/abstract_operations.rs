// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, BUILTIN_STRING_MEMORY,
        ExceptionType::{self},
        Function, IntegerOrInfinity, JsResult, Object, Value, get, is_callable,
        to_integer_or_infinity_number, to_number,
    },
    engine::{Bindable, GcScope, Scopable, bindable_handle},
};

/// ### [24.2.1.1 Set Records](https://tc39.es/ecma262/#sec-set-records)
///
/// A Set Record is a Record used to encapsulate the interface of a Set or
/// similar object.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SetRecord<'a> {
    pub(crate) set_object: Object<'a>,
    pub(crate) size: IntegerOrInfinity,
    pub(crate) has: Function<'a>,
    #[expect(dead_code, reason = "used by upcoming Set methods")]
    pub(crate) keys: Function<'a>,
}

bindable_handle!(SetRecord);

/// ### [24.2.1.2 GetSetRecord ( obj )](https://tc39.es/ecma262/#sec-getsetrecord)
///
/// The abstract operation GetSetRecord takes argument obj (an ECMAScript
/// language value) and returns either a normal completion containing a Set
/// Record or a throw completion.
pub(crate) fn get_set_record<'a>(
    agent: &mut Agent,
    obj: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, SetRecord<'a>> {
    let obj = obj.bind(gc.nogc());

    // 1. If obj is not an Object, throw a TypeError exception.
    let Ok(obj) = Object::try_from(obj) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Value is not an object",
            gc.into_nogc(),
        ));
    };
    // 2. Let rawSize be ? Get(obj, "size").
    let scoped_obj = obj.scope(agent, gc.nogc());
    let raw_size = get(
        agent,
        obj.unbind(),
        BUILTIN_STRING_MEMORY.size.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 3. Let numberSize be ? ToNumber(rawSize).
    let number_size = to_number(agent, raw_size.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // 4. NOTE: If rawSize is undefined, then numberSize will be NaN.
    // 5. If numberSize is NaN, throw a TypeError exception.
    if number_size.is_nan(agent) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Set size is NaN",
            gc.into_nogc(),
        ));
    };
    // 6. Let intSize be ! ToIntegerOrInfinity(numberSize).
    let int_size = to_integer_or_infinity_number(agent, number_size);
    // 7. If intSize < 0, throw a RangeError exception.
    if int_size < 0 {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Size is less than 0",
            gc.into_nogc(),
        ));
    }
    // 8. Let has be ? Get(obj, "has").
    let has_fn = get(
        agent,
        scoped_obj.get(agent),
        BUILTIN_STRING_MEMORY.has.into(),
        gc.reborrow(),
    )
    .unbind()?
    .bind(gc.nogc());
    // 9. If IsCallable(has) is false, throw a TypeError exception.
    let Some(has_fn) = is_callable(has_fn, gc.nogc()) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Has is not callable",
            gc.into_nogc(),
        ));
    };
    let scoped_has_fn = has_fn.scope(agent, gc.nogc());
    // 10. Let keys be ? Get(obj, "keys").
    let keys_fn = get(
        agent,
        scoped_obj.get(agent),
        BUILTIN_STRING_MEMORY.keys.into(),
        gc.reborrow(),
    )
    .unbind()?;
    let gc = gc.into_nogc();
    // 11. If IsCallable(keys) is false, throw a TypeError exception.
    let Some(keys_fn) = is_callable(keys_fn, gc) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "Keys is not callable",
            gc,
        ));
    };
    // 12. Return a new Set Record { [[SetObject]]: obj, [[Size]]: intSize, [[Has]]: has, [[Keys]]: keys }.
    Ok(SetRecord {
        set_object: scoped_obj.get(agent).bind(gc),
        size: int_size,
        has: scoped_has_fn.get(agent).bind(gc),
        keys: keys_fn,
    })
}
