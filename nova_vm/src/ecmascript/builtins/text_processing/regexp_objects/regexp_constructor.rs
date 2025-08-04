// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{operations_on_objects::get, testing_and_comparison::is_reg_exp},
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor,
            regexp::{reg_exp_alloc, reg_exp_initialize},
        },
        execution::{Agent, JsResult, Realm},
        types::{
            BUILTIN_STRING_MEMORY, Function, IntoObject, IntoValue, Object, PropertyKey, String,
            Value,
        },
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub struct RegExpConstructor;

impl Builtin for RegExpConstructor {
    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
    const LENGTH: u8 = 1;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.RegExp;
}
impl BuiltinIntrinsicConstructor for RegExpConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::RegExp;
}

struct RegExpGetSpecies;
impl Builtin for RegExpGetSpecies {
    const BEHAVIOUR: Behaviour = Behaviour::Regular(RegExpConstructor::get_species);
    const LENGTH: u8 = 0;
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Species.to_property_key());
}
impl BuiltinGetter for RegExpGetSpecies {}

impl RegExpConstructor {
    /// ### [22.2.4.1 RegExp ( pattern, flags )](https://tc39.es/ecma262/#sec-regexp-pattern-flags)
    ///
    /// > NOTE: If pattern is supplied using a StringLiteral, the usual escape
    /// > sequence substitutions are performed before the String is processed
    /// > by this function. If pattern must contain an escape sequence to be
    /// > recognized by this function, any U+005C (REVERSE SOLIDUS) code points
    /// > must be escaped within the StringLiteral to prevent them being
    /// > removed when the contents of the StringLiteral are formed.
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let pattern = arguments.get(0).bind(gc.nogc());
        let flags = arguments.get(1).bind(gc.nogc());
        let scoped_pattern = pattern.scope(agent, gc.nogc());
        let scoped_flags = flags.scope(agent, gc.nogc());
        let flags_is_undefined = flags.is_undefined();
        // 1. Let patternIsRegExp be ? IsRegExp(pattern).
        let pattern_is_reg_exp = is_reg_exp(agent, pattern.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // 2. If NewTarget is undefined, then
        let new_target = if new_target.is_none() {
            // a. Let newTarget be the active function object.
            let new_target = agent.active_function_object(gc.nogc());
            // b. If patternIsRegExp is true and flags is undefined, then
            if pattern_is_reg_exp && flags_is_undefined {
                let new_target = new_target.scope(agent, gc.nogc());
                // i. Let patternConstructor be ? Get(pattern, "constructor").
                let pattern_constructor = get(
                    agent,
                    Object::try_from(scoped_pattern.get(agent)).unwrap(),
                    BUILTIN_STRING_MEMORY.constructor.into(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared.
                let new_target = unsafe { new_target.take(agent) }.bind(gc.nogc());
                // ii. If SameValue(newTarget, patternConstructor) is true, return pattern.
                if new_target.into_value() == pattern_constructor {
                    return Ok(scoped_pattern.get(agent));
                }
                new_target.into_object()
            } else {
                new_target.into_object()
            }
        } else {
            // 3. Else,
            // a. Let newTarget be NewTarget.
            // SAFETY: checked above.
            unsafe { new_target.unwrap_unchecked() }
        };
        let new_target = new_target.scope(agent, gc.nogc());
        let pattern = scoped_pattern.get(agent).bind(gc.nogc());
        // 4. If pattern is an Object and pattern has a [[RegExpMatcher]] internal slot, then
        let (p, f) = if let Value::RegExp(pattern) = pattern {
            // a. Let P be pattern.[[OriginalSource]].
            let p = pattern.original_source(agent);
            // b. If flags is undefined, let F be pattern.[[OriginalFlags]].
            let f = if flags_is_undefined {
                Ok(pattern.original_flags(agent))
            } else {
                // c. Else, let F be flags.
                Err(scoped_flags)
            };
            (p.into_value(), f)
        } else if pattern_is_reg_exp {
            // 5. Else if patternIsRegExp is true, then
            // a. Let P be ? Get(pattern, "source").
            let mut p = get(
                agent,
                Object::try_from(pattern).unwrap().unbind(),
                BUILTIN_STRING_MEMORY.source.into(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            // b. If flags is undefined, then
            let f = if flags_is_undefined {
                let scoped_p = p.scope(agent, gc.nogc());
                // i. Let F be ? Get(pattern, "flags").
                let f = get(
                    agent,
                    // SAFETY: not shared.
                    Object::try_from(unsafe { scoped_pattern.take(agent) }).unwrap(),
                    BUILTIN_STRING_MEMORY.flags.into(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                // SAFETY: not shared
                p = unsafe { scoped_p.take(agent) }.bind(gc.nogc());
                f.scope(agent, gc.nogc())
            } else {
                // c. Else,
                // i. Let F be flags.
                scoped_flags
            };
            (p, Err(f))
        } else {
            // 6. Else,
            // a. Let P be pattern.
            let p = pattern;
            // b. Let F be flags.
            // SAFETY: not shared
            (p, Err(scoped_flags))
        };
        let p = p.scope(agent, gc.nogc());
        // 7. Let O be ? RegExpAlloc(newTarget).
        let o = reg_exp_alloc(
            agent,
            // SAFETY: not shared.
            Function::try_from(unsafe { new_target.take(agent) })
                .expect("Proxy constructors not yet supported"),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 8. Return ? RegExpInitialize(O, P, F).
        reg_exp_initialize(agent, o.unbind(), p, f, gc).map(|o| o.into_value())
    }

    fn get_species<'gc>(
        _: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Ok(this_value.unbind())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let regexp_prototype = intrinsics.reg_exp_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<RegExpConstructor>(agent, realm)
            .with_property_capacity(2)
            .with_prototype_property(regexp_prototype.into_object())
            .with_builtin_function_getter_property::<RegExpGetSpecies>()
            .build();
    }
}
