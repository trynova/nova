// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[cfg(feature = "proposal-atomics-microwait")]
use crate::ecmascript::execution::agent::ExceptionType;
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_index, try_to_index, validate_index},
        builders::ordinary_object_builder::OrdinaryObjectBuilder,
        builtins::{
            array_buffer::{get_modify_set_value_in_buffer, AnyArrayBuffer}, indexed_collections::typed_array_objects::abstract_operations::{
                validate_typed_array, TypedArrayAbstractOperations, TypedArrayWithBufferWitnessRecords
            }, typed_array::{for_any_typed_array, AnyTypedArray}, ArgumentsList, Behaviour, Builtin
        },
        execution::{
            agent::{try_result_into_js, ExceptionType}, Agent, JsResult, Realm
        },
        types::{Numeric, String, Value, BUILTIN_STRING_MEMORY},
    },
    engine::{
        context::{Bindable, GcScope},
        rootable::Scopable,
    },
    heap::WellKnownSymbolIndexes,
};

pub(crate) struct AtomicsObject;

struct AtomicsObjectAdd;
impl Builtin for AtomicsObjectAdd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.add;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::add);
}

struct AtomicsObjectAnd;
impl Builtin for AtomicsObjectAnd {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.and;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::and);
}
struct AtomicsObjectCompareExchange;
impl Builtin for AtomicsObjectCompareExchange {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.compareExchange;

    const LENGTH: u8 = 4;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::compare_exchange);
}
struct AtomicsObjectExchange;
impl Builtin for AtomicsObjectExchange {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.exchange;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::exchange);
}
struct AtomicsObjectIsLockFree;
impl Builtin for AtomicsObjectIsLockFree {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isLockFree;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::is_lock_free);
}
struct AtomicsObjectLoad;
impl Builtin for AtomicsObjectLoad {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.load;

    const LENGTH: u8 = 2;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::load);
}
struct AtomicsObjectOr;
impl Builtin for AtomicsObjectOr {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.or;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::or);
}
struct AtomicsObjectStore;
impl Builtin for AtomicsObjectStore {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.store;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::store);
}
struct AtomicsObjectSub;
impl Builtin for AtomicsObjectSub {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.sub;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::sub);
}
struct AtomicsObjectWait;
impl Builtin for AtomicsObjectWait {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.wait;

    const LENGTH: u8 = 4;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::wait);
}
struct AtomicsObjectWaitAsync;
impl Builtin for AtomicsObjectWaitAsync {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.waitAsync;

    const LENGTH: u8 = 4;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::wait_async);
}
struct AtomicsObjectNotify;
impl Builtin for AtomicsObjectNotify {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.notify;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::notify);
}
struct AtomicsObjectXor;
impl Builtin for AtomicsObjectXor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.xor;

    const LENGTH: u8 = 3;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::xor);
}

#[cfg(feature = "proposal-atomics-microwait")]
struct AtomicsObjectPause;
#[cfg(feature = "proposal-atomics-microwait")]
impl Builtin for AtomicsObjectPause {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.pause;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(AtomicsObject::pause);
}

impl AtomicsObject {
    /// ### [25.4.4 Atomics.add ( typedArray, index, value )](https://tc39.es/ecma262/#sec-atomics.add)
    fn add<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.add", gc.into_nogc()))
    }

    fn and<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.and", gc.into_nogc()))
    }

    fn compare_exchange<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.compareExchange", gc.into_nogc()))
    }

    fn exchange<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.exchange", gc.into_nogc()))
    }

    fn is_lock_free<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.isLockFree", gc.into_nogc()))
    }

    fn load<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.load", gc.into_nogc()))
    }

    fn or<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.or", gc.into_nogc()))
    }

    fn store<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.store", gc.into_nogc()))
    }

    fn sub<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.sub", gc.into_nogc()))
    }

    fn wait<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.wait", gc.into_nogc()))
    }

    fn wait_async<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.waitAsync", gc.into_nogc()))
    }

    fn notify<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.notify", gc.into_nogc()))
    }

    fn xor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        _arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        Err(agent.todo("Atomics.xor", gc.into_nogc()))
    }

    /// ### [1 Atomics.pause ( [ N ] )](https://tc39.es/proposal-atomics-microwait/#Atomics.pause)
    ///
    /// > NOTE: This method is designed for programs implementing spin-wait
    /// > loops, such as spinlock fast paths inside of mutexes, to provide a
    /// > hint to the CPU that it is spinning while waiting on a value. It has
    /// > no observable behaviour other than timing.
    /// >
    /// > Implementations are expected to implement a pause or yield instruction
    /// > if the best practices of the underlying architecture recommends such
    /// > instructions in spin loops. For example, the [Intel Optimization Manual](https://www.intel.com/content/www/us/en/content-details/671488/intel-64-and-ia-32-architectures-optimization-reference-manual-volume-1.html)
    /// > recommends the **pause** instruction.
    ///
    /// > NOTE: The N parameter controls how long an implementation pauses.
    /// > Larger values result in longer waits. Implementations are encouraged
    /// > to have an internal upper bound on the maximum amount of time paused
    /// > on the order of tens to hundreds of nanoseconds.
    ///
    /// > NOTE: Due to the overhead of function calls, it is reasonable that an
    /// > inlined call to this method in an optimizing compiler waits a
    /// > different amount of time than a non-inlined call.
    #[cfg(feature = "proposal-atomics-microwait")]
    fn pause<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.into_nogc();
        let n = arguments.get(0);

        // 1. If N is neither undefined nor an integral Number, throw a TypeError exception.
        if !n.is_undefined() && !n.is_integer() {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "Atomics.pause called with non-integral Number",
                nogc,
            ));
        }

        // Consider this the "internal upper bound" on the maximum amount of
        // time paused.
        let n = if let Value::Integer(n) = n {
            let n = n.into_i64();
            u16::try_from(n).unwrap_or(if n > 0 { u16::MAX } else { 1 })
        } else {
            1
        };

        // TODO: This should be implemented in a similar manner to `eval`
        // where we compile calls to `Atomics.pause` as a `pause` instruction
        // directly in the bytecode.

        // 2. If the execution environment of the ECMAScript implementation supports
        // signaling to the operating system or CPU that the current executing
        // code is in a spin-wait loop, such as executing a pause CPU instruction,
        // send that signal. When N is not undefined, it determines the number
        // of times that signal is sent. The number of times the signal is sent
        // for an integral Number N is less than or equal to the number times it
        // is sent for N + 1 if both N and N + 1 have the same sign.
        for _ in 0..n {
            std::hint::spin_loop();
        }

        // 3. Return undefined.
        Ok(Value::Undefined)
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let object_prototype = intrinsics.object_prototype();
        let this = intrinsics.atomics();

        let mut property_capacity = 14;
        if cfg!(feature = "proposal-atomics-microwait") {
            property_capacity += 1;
        }

        let builder = OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, this)
            .with_property_capacity(property_capacity)
            .with_prototype(object_prototype)
            .with_builtin_function_property::<AtomicsObjectAdd>()
            .with_builtin_function_property::<AtomicsObjectAnd>()
            .with_builtin_function_property::<AtomicsObjectCompareExchange>()
            .with_builtin_function_property::<AtomicsObjectExchange>()
            .with_builtin_function_property::<AtomicsObjectIsLockFree>()
            .with_builtin_function_property::<AtomicsObjectLoad>()
            .with_builtin_function_property::<AtomicsObjectOr>()
            .with_builtin_function_property::<AtomicsObjectStore>()
            .with_builtin_function_property::<AtomicsObjectSub>()
            .with_builtin_function_property::<AtomicsObjectWait>()
            .with_builtin_function_property::<AtomicsObjectWaitAsync>()
            .with_builtin_function_property::<AtomicsObjectNotify>()
            .with_builtin_function_property::<AtomicsObjectXor>()
            .with_property(|builder| {
                builder
                    .with_key(WellKnownSymbolIndexes::ToStringTag.into())
                    .with_value_readonly(BUILTIN_STRING_MEMORY.Atomics.into())
                    .with_enumerable(false)
                    .with_configurable(true)
                    .build()
            });

        #[cfg(feature = "proposal-atomics-microwait")]
        let builder = builder.with_builtin_function_property::<AtomicsObjectPause>();

        builder.build();
    }
}

/// ### [25.4.3.17 AtomicReadModifyWrite ( typedArray, index, value, op )](https://tc39.es/ecma262/#sec-atomicreadmodifywrite)
///
/// The abstract operation AtomicReadModifyWrite takes arguments typedArray (an
/// ECMAScript language value), index (an ECMAScript language value), value (an
/// ECMAScript language value), and op (a read-modify-write modification
/// function) and returns either a normal completion containing either a Number
/// or a BigInt, or a throw completion. op takes two List of byte values
/// arguments and returns a List of byte values. This operation atomically
/// loads a value, combines it with another value, and stores the combination.
/// It returns the loaded value.
fn atomic_read_modify_write<'gc, const Op: u8>(
    agent: &mut Agent,
    typed_array: Value,
    index: Value,
    value: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Numeric<'gc>> {
    let typed_array = typed_array.bind(gc.nogc());
    let index = index.bind(gc.nogc());
    let value = value.bind(gc.nogc());

    // 1. Let byteIndexInBuffer be ? ValidateAtomicAccessOnIntegerTypedArray(typedArray, index).
    let ta_record = validate_typed_array(
        agent,
        typed_array,
        ecmascript_atomics::Ordering::Unordered,
        gc.nogc(),
    )
    .unbind()?
    .bind(gc.nogc());
    // a. Let type be TypedArrayElementType(typedArray).
    // b. If IsUnclampedIntegerElementType(type) is false and
    //    IsBigIntElementType(type) is false, throw a TypeError exception.
    if !ta_record.object.is_integer() {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "cannot use TypedArray in Atomics",
            gc.into_nogc(),
        ));
    }
    // 1. Let length be TypedArrayLength(taRecord).
    let length = ta_record.typed_array_length(agent);
    let index = if let (Value::Integer(index), Ok(value)) = (index, Numeric::try_from(value)) {
        let gc = gc.into_nogc();
        // 2. Let accessIndex be ? ToIndex(requestIndex).
        let access_index = validate_index(agent, index.into_i64(), gc)?;
        // 3. If accessIndex ≥ length, throw a RangeError exception.
        if access_index >= length as u64 {
            todo!();
        }
        // 5. Let typedArray be taRecord.[[Object]].
        let typed_array = ta_record.object;
        // 7. Let offset be typedArray.[[ByteOffset]].
        let offset = typed_array.byte_offset(agent);
        // 2. If typedArray.[[ContentType]] is bigint, let v be ? ToBigInt(value).
        if typed_array.is_bigint() != value.is_bigint() {
            todo!();
        }
        let byte_index_in_buffer = offset + access_index as usize * typed_array.typed_array_element_size();
        // 5. Let buffer be typedArray.[[ViewedArrayBuffer]].
        let buffer = typed_array.viewed_array_buffer(agent);
        // 6. Let elementType be TypedArrayElementType(typedArray).
        // 7. Return GetModifySetValueInBuffer(buffer, byteIndexInBuffer, elementType, v, op).
        for_any_typed_array!(typed_array, _t, {
            get_modify_set_value_in_buffer::<ElementType, Op>(agent, buffer, byte_index_in_buffer, value, gc)
        }, ElementType)
    } else {
        atomic_read_modify_write_slow(
            agent,
            ta_record.unbind(),
            index.unbind(),
            value.unbind(),
            gc,
        )
    }
    // 2. If typedArray.[[ContentType]] is bigint, let v be ? ToBigInt(value).
    // 3. Otherwise, let v be 𝔽(? ToIntegerOrInfinity(value)).
    // 4. Perform ? RevalidateAtomicAccess(typedArray, byteIndexInBuffer).
    // 5. Let buffer be typedArray.[[ViewedArrayBuffer]].
    // 6. Let elementType be TypedArrayElementType(typedArray).
    // 7. Return GetModifySetValueInBuffer(buffer, byteIndexInBuffer, elementType, v, op).
}

#[inline(never)]
#[cold]
fn atomic_read_modify_write_slow<'gc>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    index: Value,
    value: Value,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Numeric<'gc>> {
    if let Some(index) = try_result_into_js(try_to_index(agent, index, gc.nogc())).unbind()? {
        todo!()
    } else {
        let ta = ta_record.object.scope(agent, gc.nogc());
        let cached_buffer_byte_length = ta_record.cached_buffer_byte_length;
        let value = value.scope(agent, gc.nogc());
        let index = to_index(agent, index, gc).unbind()?;
        // SAFETY: not shared.
        let (ta_record, value) = unsafe {
            (
                TypedArrayWithBufferWitnessRecords {
                    object: ta.take(agent),
                    cached_buffer_byte_length,
                },
                value.take(agent),
            )
        };
        todo!()
    }
}
