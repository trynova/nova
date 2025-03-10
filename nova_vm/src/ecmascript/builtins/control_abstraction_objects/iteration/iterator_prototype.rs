// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::{
    get_iterator_direct, if_abrupt_close_iterator, iterator_close, iterator_step_value,
};
use crate::ecmascript::abstract_operations::operations_on_objects::call;
use crate::ecmascript::abstract_operations::testing_and_comparison::is_callable;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::types::Object;
use crate::engine::context::{Bindable, GcScope};
use crate::engine::rootable::Scopable;
use crate::{
    ecmascript::{
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{BUILTIN_STRING_MEMORY, PropertyKey, String, Value},
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct IteratorPrototype;

struct IteratorPrototypeIterator;
impl Builtin for IteratorPrototypeIterator {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY._Symbol_iterator_;
    const KEY: Option<PropertyKey<'static>> =
        Some(WellKnownSymbolIndexes::Iterator.to_property_key());
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::iterator);
}

struct IteratorPrototypeForEach;
impl Builtin for IteratorPrototypeForEach {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.forEach;
    const KEY: Option<PropertyKey<'static>> = None;
    const LENGTH: u8 = 0;
    const BEHAVIOUR: Behaviour = Behaviour::Regular(IteratorPrototype::for_each);
}

impl IteratorPrototype {
    fn iterator<'gc>(
        _agent: &mut Agent,
        this_value: Value,
        _: ArgumentsList,
        _gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        Ok(this_value.unbind())
    }

    /// ### [27.1.4.7 Iterator.prototype.forEach ( procedure )](https://tc39.es/ecma262/#sec-iterator.prototype.foreach)
    fn for_each<'gc>(
        agent: &mut Agent,
        this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        let nogc = gc.nogc();
        let this_value = this_value.bind(nogc);
        let procedure = arguments.get(0).bind(nogc);

        // 1. Let O be the this value.
        // 2. If O is not an Object, throw a TypeError exception.
        let Ok(o) = Object::try_from(this_value) else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'this' is not an object",
                nogc,
            ));
        };

        // 3. Let iterated be the Iterator Record { [[Iterator]]: O, [[NextMethod]]: undefined, [[Done]]: false }.
        // 4. If IsCallable(procedure) is false, then
        let Some(procedure) = is_callable(procedure, nogc).unbind().bind(gc.nogc()) else {
            // a. Let error be ThrowCompletion(a newly created TypeError object).
            let error = agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "'procedure' is not callable",
                nogc,
            );
            // b. Return ? IteratorClose(iterated, error).
            return iterator_close(agent, o.unbind(), Err(error), gc);
        };
        let scoped_procedure = Value::from(procedure).scope(agent, gc.nogc());

        // 5. Set iterated to ? GetIteratorDirect(O).
        // if i rebind `iterated` here borrow checker complains
        let iterated = get_iterator_direct(agent, o.unbind(), gc.reborrow()).unbind()?;
        let Some(iterated) = iterated else {
            // what should happen here?
            todo!();
        };

        // 6. Let counter be 0.
        let mut counter = 0;

        // 7. Repeat,
        loop {
            // a. Let value be ? IteratorStepValue(iterated).
            let value = iterator_step_value(agent, iterated, gc.reborrow())
                .unbind()
                .bind(gc.nogc())?;
            // b. If value is done, return undefined.
            let Some(value) = value else {
                return Ok(Value::Undefined);
            };

            // c. Let result be Completion(Call(procedure, undefined, « value, 𝔽(counter) »)).
            let result = call(
                agent,
                scoped_procedure.get(agent),
                Value::Undefined,
                Some(ArgumentsList(&[value.unbind(), counter.into()])),
                gc.reborrow(),
            )
            .unbind()
            .bind(gc.nogc());
            // d. IfAbruptCloseIterator(result, iterated).
			if_abrupt_close_iterator!(agent, result, iterated, gc);
            // e. Set counter to counter + 1.
			counter += 1;
        }
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.iterator_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(2)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<IteratorPrototypeIterator>()
			.with_builtin_function_property::<IteratorPrototypeForEach>()
            .build();
    }
}
