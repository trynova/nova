// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::operations_on_iterator_objects::{
    IteratorRecord, get_iterator, if_abrupt_close_iterator, iterator_step_value,
};
use crate::ecmascript::abstract_operations::operations_on_objects::{
    call_function, get, throw_not_callable,
};
use crate::ecmascript::abstract_operations::testing_and_comparison::is_callable;
use crate::ecmascript::builtins::Array;
use crate::ecmascript::builtins::array::ArrayHeap;
use crate::ecmascript::builtins::ordinary::ordinary_create_from_constructor;
use crate::ecmascript::builtins::weak_set::WeakSet;
use crate::ecmascript::execution::agent::ExceptionType;
use crate::ecmascript::execution::{ProtoIntrinsics, can_be_held_weakly, throw_not_weak_key_error};
use crate::ecmascript::types::{Function, IntoValue};
use crate::engine::Scoped;
use crate::engine::context::{Bindable, GcScope, NoGcScope};
use crate::engine::rootable::Scopable;
use crate::heap::Heap;
use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, Realm},
        types::{BUILTIN_STRING_MEMORY, IntoObject, Object, String, Value},
    },
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct WeakSetConstructor;
impl Builtin for WeakSetConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.WeakSet;

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for WeakSetConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::WeakSet;
}

impl WeakSetConstructor {
    /// ### [24.4.1.1 WeakSet ( \[ iterable \] )](https://tc39.es/ecma262/#sec-weakset-iterable)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let scoped_iterable = arguments.get(0).scope(agent, gc.nogc());
        let new_target = new_target.bind(gc.nogc());
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin WeakSet constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();
        // 2. Let set be ? OrdinaryCreateFromConstructor(NewTarget, "%WeakSet.prototype%", « [[WeakSetData]] »).
        // 3. Set set.[[WeakSetData]] to a new empty List.
        let Object::WeakSet(set) = ordinary_create_from_constructor(
            agent,
            new_target.unbind(),
            ProtoIntrinsics::WeakSet,
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc()) else {
            unreachable!()
        };
        let iterable = scoped_iterable.get(agent).bind(gc.nogc());
        // 4. If iterable is either undefined or null, return set.
        if iterable.is_undefined() || iterable.is_null() {
            return Ok(set.unbind().into_value());
        }
        let scoped_set = set.scope(agent, gc.nogc());
        // 5. Let adder be ? Get(set, "add").
        let adder = get(
            agent,
            set.unbind(),
            BUILTIN_STRING_MEMORY.add.into(),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // 6. If IsCallable(adder) is false, throw a TypeError exception.
        let Some(adder) = is_callable(adder, gc.nogc()) else {
            return Err(throw_not_callable(agent, gc.into_nogc()));
        };
        let iterable = scoped_iterable.get(agent).bind(gc.nogc());
        if WeakSet::is_weak_set_prototype_add(agent, adder) {
            // Adder function is the normal WeakSet.prototype.add; if the Array
            // is trivially iterable then we can skip all the complicated song
            // and dance.
            match iterable {
                Value::Array(iterable) if iterable.is_trivially_iterable(agent, gc.nogc()) => {
                    let iterable = iterable.unbind();
                    let gc = gc.into_nogc();
                    let set = scoped_set.get(agent).bind(gc);
                    let iterable = iterable.bind(gc);
                    weak_set_add_trivially_iterable_array_elements(agent, set, iterable, gc)?;
                    return Ok(set.into_value());
                }
                _ => {}
            }
        }
        weak_set_constructor_slow_path(agent, scoped_set, adder.unbind(), scoped_iterable, gc)
            .map(|set| set.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let weak_set_prototype = intrinsics.weak_set_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<WeakSetConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(weak_set_prototype.into_object())
            .build();
    }
}

/// This function implements steps 7 and onwards of the WeakSet constructor
/// function. These steps are here outside of the main constructor function
/// because it is fairly uncommon that we end up here: the common cases are
/// no-iterable and normal-Array-iterable.
fn weak_set_constructor_slow_path<'a>(
    agent: &mut Agent,
    scoped_set: Scoped<WeakSet>,
    adder: Function,
    scoped_iterable: Scoped<Value>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, WeakSet<'a>> {
    let adder = adder.scope(agent, gc.nogc());
    // 7. Let iteratorRecord be ? GetIterator(iterable, sync).
    let Some(IteratorRecord {
        iterator,
        next_method,
        ..
    }) = get_iterator(agent, scoped_iterable.get(agent), false, gc.reborrow())
        .unbind()?
        .bind(gc.nogc())
    else {
        return Err(throw_not_callable(agent, gc.into_nogc()));
    };
    let iterator = iterator.scope(agent, gc.nogc());
    let next_method = next_method.scope(agent, gc.nogc());
    // 8. Repeat,
    loop {
        // a. Let next be ? IteratorStepValue(iteratorRecord).
        let next = iterator_step_value(
            agent,
            IteratorRecord {
                iterator: iterator.get(agent),
                next_method: next_method.get(agent),
            },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        // b. If next is done, return set.
        let Some(next) = next else {
            return Ok(scoped_set.get(agent));
        };
        let set = scoped_set.get(agent).bind(gc.nogc());
        // c. Let status be Completion(Call(adder, set, « next »)).
        let status = call_function(
            agent,
            adder.get(agent),
            set.unbind().into_value(),
            Some(ArgumentsList::from_mut_value(&mut next.unbind())),
            gc.reborrow(),
        );
        let iterator_record = IteratorRecord {
            iterator: iterator.get(agent),
            next_method: next_method.get(agent),
        };
        // d. IfAbruptCloseIterator(status, iteratorRecord).
        if_abrupt_close_iterator!(agent, status, iterator_record, gc);
    }
}

/// Fast path for adding elements from a trivially iterable Array (contains no
/// getters or holes; setters without corresponding getter are possible and
/// correspond to `undefined`) into a WeakSet using the normal
/// `WeakSet.prototype.add` function.
fn weak_set_add_trivially_iterable_array_elements<'a>(
    agent: &mut Agent,
    set: WeakSet,
    iterable: Array,
    gc: NoGcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let Heap {
        arrays,
        elements,
        weak_sets,
        ..
    } = &mut agent.heap;
    let array_heap = ArrayHeap::new(elements, arrays);
    let slice = iterable.as_slice(&array_heap);
    let weak_set_data = &mut weak_sets[set];
    for value in slice {
        let value = value.unwrap_or(Value::Undefined);
        // 3. If CanBeHeldWeakly(value) is false, throw a TypeError exception.
        let Some(value) = can_be_held_weakly(value) else {
            return Err(throw_not_weak_key_error(agent, value, gc));
        };
        weak_set_data.add(value);
    }
    Ok(())
}
