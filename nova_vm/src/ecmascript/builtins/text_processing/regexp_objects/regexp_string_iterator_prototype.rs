// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin, ExceptionType, JsResult,
        Number, Object, Realm, String, Value, advance_string_index,
        builders::OrdinaryObjectBuilder, create_iter_result_object, get, reg_exp_exec, set,
        to_length, to_string, try_get, try_get_result_into_value, try_result_into_js,
        try_result_into_option_js, try_to_string,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::WellKnownSymbolIndexes,
};

/// ### [22.2.9.2 The %RegExpStringIteratorPrototype% Object](https://tc39.es/ecma262/#sec-%regexpstringiteratorprototype%-object)
///
/// The %RegExpStringIteratorPrototype% object:
/// * has properties that are inherited by all RegExp String Iterator objects.
/// * is an ordinary object.
/// * has a \[\[Prototype]] internal slot whose value is %Iterator.prototype%.
/// * has the following properties:
pub(crate) struct RegExpStringIteratorPrototype;

struct RegExpStringIteratorPrototypeNext;
impl Builtin for RegExpStringIteratorPrototypeNext {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.next;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpStringIteratorPrototype::next);
}

impl RegExpStringIteratorPrototype {
    /// ### [22.2.9.2.1 %RegExpStringIteratorPrototype%.next ( )](https://tc39.es/ecma262/#sec-%regexpstringiteratorprototype%.next)
    fn next<'gc>(
        agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Let O be the this value.
        let o = this_value.bind(gc.nogc());
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(o) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "O is not an object",
                gc.into_nogc(),
            ));
        };
        // 3. If O does not have all of the internal slots of a RegExp String
        //    Iterator Object Instance (see 22.2.9.3), throw a TypeError
        //    exception.
        let Object::RegExpStringIterator(o) = o else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "O is not an RegExp String Iterator object",
                gc.into_nogc(),
            ));
        };
        // 4. If O.[[Done]] is true, then
        if o.done(agent) {
            // a. Return CreateIteratorResultObject(undefined, true).
            return create_iter_result_object(agent, Value::Undefined, true, gc.into_nogc())
                .map(|o| o.into());
        }
        // 5. Let R be O.[[IteratingRegExp]].
        let r = o.iterating_regexp(agent);
        // 6. Let S be O.[[IteratedString]].
        let s = o.iterated_string(agent);
        // 7. Let global be O.[[Global]].
        let global = o.global(agent);
        // 8. Let fullUnicode be O.[[Unicode]].
        let full_unicode = o.unicode(agent);
        let scoped_o = o.scope(agent, gc.nogc());
        // 9. Let match be ? RegExpExec(R, S).
        let r#match = reg_exp_exec(agent, r.unbind(), s.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 10. If match is null, then
        let Some(mut r#match) = r#match else {
            // a. Set O.[[Done]] to true.
            scoped_o.get(agent).set_done(agent);
            // b. Return CreateIteratorResultObject(undefined, true).
            return create_iter_result_object(agent, Value::Undefined, true, gc.into_nogc())
                .map(|o| o.into());
        };
        // 11. If global is false, then
        if !global {
            // a. Set O.[[Done]] to true.
            scoped_o.get(agent).set_done(agent);
            // b. Return CreateIteratorResultObject(match, false).
            return create_iter_result_object(
                agent,
                r#match.unbind().into(),
                false,
                gc.into_nogc(),
            )
            .map(|o| o.into());
        }
        // 12. Let matchStr be ? ToString(? Get(match, "0")).
        let match_str = if let Some(s) = try_result_into_js(try_get_result_into_value(try_get(
            agent,
            r#match,
            0.into(),
            None,
            gc.nogc(),
        )))
        .unbind()?
        .bind(gc.nogc())
        .and_then(|s| try_result_into_option_js(try_to_string(agent, s, gc.nogc())))
        {
            s.unbind()?.bind(gc.nogc())
        } else {
            let scoped_match = r#match.scope(agent, gc.nogc());
            let s = get(agent, r#match.unbind(), 0.into(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let s = to_string(agent, s.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: not shared.
            r#match = unsafe { scoped_match.take(agent) }.bind(gc.nogc());
            s
        };
        // 13. If matchStr is the empty String, then
        if match_str.is_empty_string() {
            let scoped_match = r#match.scope(agent, gc.nogc());
            // a. Let thisIndex be ℝ(? ToLength(? Get(R, "lastIndex"))).
            let this_index = get(
                agent,
                scoped_o.get(agent).iterating_regexp(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            let this_index = to_length(agent, this_index.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let this_index = usize::try_from(this_index).expect("Length value not valid usize");
            // b. Let nextIndex be AdvanceStringIndex(S, thisIndex, fullUnicode).
            let next_index = advance_string_index(
                agent,
                scoped_o.get(agent).iterated_string(agent),
                this_index,
                full_unicode,
            );
            // c. Perform ? Set(R, "lastIndex", 𝔽(nextIndex), true).
            set(
                agent,
                scoped_o.get(agent).iterating_regexp(agent),
                BUILTIN_STRING_MEMORY.lastIndex.to_property_key(),
                Number::try_from(next_index).unwrap().into(),
                true,
                gc.reborrow(),
            )
            .unbind()?;
            r#match = unsafe { scoped_match.take(agent) }.bind(gc.nogc());
        }
        // 14. Return CreateIteratorResultObject(match, false).
        create_iter_result_object(agent, r#match.unbind().into(), false, gc.into_nogc())
            .map(|o| o.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let this = intrinsics.reg_exp_string_iterator_prototype();
        let iterator_prototype = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(iterator_prototype)
            .with_builtin_function_property::<RegExpStringIteratorPrototypeNext>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.RegExp_String_Iterator.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            })
            .build();
    }
}
