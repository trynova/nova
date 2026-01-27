// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{
    hint::assert_unchecked,
    ops::ControlFlow,
    sync::{Arc, atomic::AtomicBool},
    thread::{self, JoinHandle},
    time::Duration,
};

use ecmascript_atomics::Ordering;
use ecmascript_futex::{ECMAScriptAtomicWait, FutexError};

use crate::{
    ecmascript::{
        Agent, AnyArrayBuffer, AnyTypedArray, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour,
        BigInt, Builtin, ExceptionType, InnerJob, Job, JsResult, Number, Numeric, OrdinaryObject,
        builders::OrdinaryObjectBuilder, Promise, PromiseCapability, Realm, SharedArrayBuffer,
        SharedDataBlock, SharedTypedArray, String, TryError, TryResult,
        TypedArrayAbstractOperations, TypedArrayWithBufferWitnessRecords, Value,
        compare_exchange_in_buffer, for_any_typed_array, get_modify_set_value_in_buffer,
        get_value_from_buffer, make_typed_array_with_buffer_witness_record,
        number_convert_to_integer_or_infinity, set_value_in_buffer, to_big_int, to_big_int64,
        to_big_int64_big_int, to_index, to_int32, to_int32_number, to_integer_number_or_infinity,
        to_integer_or_infinity, to_number, try_result_into_js, try_to_index, unwrap_try,
        validate_index, validate_typed_array,
    },
    engine::{Bindable, GcScope, Global, NoGcScope, Scopable},
    heap::{ObjectEntry, WellKnownSymbolIndexes},
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
        atomic_read_modify_write::<0>(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            gc,
        )
        .map(|v| v.into())
    }

    fn and<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        atomic_read_modify_write::<1>(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            gc,
        )
        .map(|v| v.into())
    }

    /// ### [25.4.6 Atomics.compareExchange ( typedArray, index, expectedValue, replacementValue )](https://tc39.es/ecma262/#sec-atomics.compareexchange)
    fn compare_exchange<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let typed_array = arguments.get(0).bind(gc.nogc());
        let index = arguments.get(1).bind(gc.nogc());
        let expected_value = arguments.get(2).bind(gc.nogc());
        let replacement_value = arguments.get(3).bind(gc.nogc());

        // 1. Let byteIndexInBuffer be ? ValidateAtomicAccessOnIntegerTypedArray(typedArray, index).
        let (ta_record, byte_index_in_buffer) =
            try_validate_atomic_access_on_integer_typed_array(agent, typed_array, index, gc.nogc())
                .unbind()?
                .bind(gc.nogc());
        let typed_array = ta_record.object;
        let (byte_index_in_buffer, typed_array, expected, replacement) =
            if let (Some(byte_index_in_buffer), (Ok(expected), Ok(replacement))) = (
                byte_index_in_buffer,
                // 4. If typedArray.[[ContentType]] is bigint, then
                if typed_array.is_bigint() {
                    // a. Let expected be ? ToBigInt(expectedValue).
                    // b. Let replacement be ? ToBigInt(replacementValue).
                    (
                        BigInt::try_from(expected_value).map(|value| value.into()),
                        BigInt::try_from(replacement_value).map(|value| value.into()),
                    )
                } else {
                    // a. Let expected be 𝔽(? ToIntegerOrInfinity(expectedValue)).
                    // b. Let replacement be 𝔽(? ToIntegerOrInfinity(replacementValue)).
                    (
                        Number::try_from(expected_value).map(|value| {
                            number_convert_to_integer_or_infinity(agent, value, gc.nogc()).into()
                        }),
                        Number::try_from(replacement_value).map(|value| {
                            number_convert_to_integer_or_infinity(agent, value, gc.nogc()).into()
                        }),
                    )
                },
            ) {
                (byte_index_in_buffer, typed_array, expected, replacement)
            } else {
                handle_typed_array_index_two_values_slow(
                    agent,
                    ta_record.unbind(),
                    index.unbind(),
                    expected_value.unbind(),
                    replacement_value.unbind(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc())
            };
        let typed_array = typed_array.unbind();
        let expected = expected.unbind();
        let replacement = replacement.unbind();
        let gc = gc.into_nogc();
        let typed_array = typed_array.bind(gc);
        let expected = expected.bind(gc);
        let replacement = replacement.bind(gc);

        // 2. Let buffer be typedArray.[[ViewedArrayBuffer]].
        let buffer = typed_array.viewed_array_buffer(agent);
        // 3. Let block be buffer.[[ArrayBufferData]].
        // 7. Let elementType be TypedArrayElementType(typedArray).
        // 8. Let elementSize be TypedArrayElementSize(typedArray).
        // 9. Let AR be the Agent Record of the surrounding agent.
        // 10. Let isLittleEndian be AR.[[LittleEndian]].
        // 11. Let expectedBytes be NumericToRawBytes(elementType, expected, isLittleEndian).
        // 12. Let replacementBytes be NumericToRawBytes(elementType, replacement, isLittleEndian).
        // 13. If IsSharedArrayBuffer(buffer) is true, then
        //         a. Let rawBytesRead be AtomicCompareExchangeInSharedBlock(block, byteIndexInBuffer, elementSize, expectedBytes, replacementBytes).
        // 14. Else,
        //         a. Let rawBytesRead be a List of length elementSize whose elements are the sequence of elementSize bytes starting with block[byteIndexInBuffer].
        //         b. If ByteListEqual(rawBytesRead, expectedBytes) is true, then
        //                 i. Store the individual bytes of replacementBytes into block, starting at block[byteIndexInBuffer].
        // 15. Return RawBytesToNumeric(elementType, rawBytesRead, isLittleEndian).
        Ok(for_any_typed_array!(
            typed_array,
            _t,
            {
                compare_exchange_in_buffer::<ElementType>(
                    agent,
                    buffer,
                    byte_index_in_buffer,
                    expected,
                    replacement,
                    gc,
                )
                .into()
            },
            ElementType
        ))
    }

    fn exchange<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        atomic_read_modify_write::<2>(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            gc,
        )
        .map(|v| v.into())
    }

    /// ### [25.4.8 Atomics.isLockFree ( size )](https://tc39.es/ecma262/#sec-atomics.islockfree)
    ///
    /// > NOTE: This function is an optimization primitive. The intuition is
    /// > that if the atomic step of an atomic primitive (**compareExchange**,
    /// > **load**, **store**, **add**, **sub**, **and**, **or**, **xor**, or
    /// > **exchange**) on a datum of size `n` bytes will be performed without
    /// > the surrounding agent acquiring a lock outside the n bytes comprising
    /// > the datum, then **Atomics.isLockFree**(`n`) will return **true**.
    /// > High-performance algorithms will use this function to determine
    /// > whether to use locks or atomic operations in critical sections. If an
    /// > atomic primitive is not lock-free then it is often more efficient for
    /// > an algorithm to provide its own locking.
    /// >
    /// > **Atomics.isLockFree**(4) always returns **true** as that can be
    /// > supported on all known relevant hardware. Being able to assume this
    /// > will generally simplify programs.
    /// >
    /// > Regardless of the value returned by this function, all atomic
    /// > operations are guaranteed to be atomic. For example, they will never
    /// > have a visible operation take place in the middle of the operation
    /// > (e.g., "tearing").
    fn is_lock_free<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let size = arguments.get(0).bind(gc.nogc());
        // 1. Let n be ? ToIntegerOrInfinity(size).
        let n = to_integer_or_infinity(agent, size.unbind(), gc)?.into_i64();
        // 2. Let AR be the Agent Record of the surrounding agent.
        // 3. If n = 1, return AR.[[IsLockFree1]].
        #[cfg(target_has_atomic = "8")]
        if n == 1 {
            return Ok(true.into());
        }
        // 4. If n = 2, return AR.[[IsLockFree2]].
        #[cfg(target_has_atomic = "16")]
        if n == 2 {
            return Ok(true.into());
        }
        // 5. If n = 4, return true.
        #[cfg(target_has_atomic = "32")]
        if n == 4 {
            return Ok(true.into());
        }
        #[cfg(not(target_has_atomic = "32"))]
        const {
            panic!("Atomics requires 32-bit lock-free atomics")
        };
        // 6. If n = 8, return AR.[[IsLockFree8]].
        #[cfg(target_has_atomic = "64")]
        if n == 8 {
            return Ok(true.into());
        }
        // 7. Return false.
        Ok(false.into())
    }

    /// ### [25.4.9 Atomics.load ( typedArray, index )](https://tc39.es/ecma262/#sec-atomics.load)
    fn load<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let arguments = arguments.bind(gc.nogc());
        let typed_array = arguments.get(0);
        let index = arguments.get(1);
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
        let (byte_index_in_buffer, typed_array) = if let Value::Integer(index) = index {
            // 7. Let offset be typedArray.[[ByteOffset]].
            let typed_array = ta_record.object.bind(gc.nogc());
            // 2. Let accessIndex be ? ToIndex(requestIndex).
            let access_index = validate_index(agent, index.into_i64(), gc.nogc()).unbind()?;
            // 3. If accessIndex ≥ length, throw a RangeError exception.
            if access_index >= length as u64 {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "accessIndex out of bounds",
                    gc.into_nogc(),
                ));
            }
            let access_index = access_index as usize;
            // 5. Let typedArray be taRecord.[[Object]].
            let offset = typed_array.byte_offset(agent);
            let byte_index_in_buffer =
                offset + access_index * typed_array.typed_array_element_size();
            (byte_index_in_buffer, typed_array)
        } else {
            // 2. Perform ? RevalidateAtomicAccess(typedArray, byteIndexInBuffer).
            atomic_load_slow(
                agent,
                ta_record.unbind(),
                index.unbind(),
                length,
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };
        let typed_array = typed_array.unbind();
        let gc = gc.into_nogc();
        let typed_array = typed_array.bind(gc);
        // 3. Let buffer be typedArray.[[ViewedArrayBuffer]].
        let buffer = typed_array.viewed_array_buffer(agent);
        // 4. Let elementType be TypedArrayElementType(typedArray).
        // 5. Return GetValueFromBuffer(buffer, byteIndexInBuffer, elementType, true, seq-cst).
        Ok(for_any_typed_array!(
            typed_array,
            _t,
            {
                get_value_from_buffer::<ElementType>(
                    agent,
                    buffer,
                    byte_index_in_buffer,
                    true,
                    Ordering::SeqCst,
                    None,
                    gc,
                )
            },
            ElementType
        )
        .into())
    }

    fn or<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        atomic_read_modify_write::<3>(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            gc,
        )
        .map(|v| v.into())
    }

    /// ### [25.4.11 Atomics.store ( typedArray, index, value )](https://tc39.es/ecma262/#sec-atomics.store)
    fn store<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let typed_array = arguments.get(0).bind(gc.nogc());
        let index = arguments.get(1).bind(gc.nogc());
        let value = arguments.get(2).bind(gc.nogc());

        let (typed_array, byte_index_in_buffer, v) = handle_typed_array_index_value(
            agent,
            typed_array.unbind(),
            index.unbind(),
            value.unbind(),
            gc,
        )?;

        // 5. Let buffer be typedArray.[[ViewedArrayBuffer]].
        // 6. Let elementType be TypedArrayElementType(typedArray).
        let buffer = typed_array.viewed_array_buffer(agent);

        // 7. Perform SetValueInBuffer(buffer, byteIndexInBuffer, elementType, v, true, seq-cst).
        for_any_typed_array!(
            typed_array,
            _t,
            {
                set_value_in_buffer::<ElementType>(
                    agent,
                    buffer,
                    byte_index_in_buffer,
                    v,
                    true,
                    Ordering::SeqCst,
                    None,
                );
            },
            ElementType
        );
        // 8. Return v.
        Ok(v.into())
    }

    fn sub<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        atomic_read_modify_write::<4>(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            gc,
        )
        .map(|v| v.into())
    }

    /// ### [25.4.13 Atomics.wait ( typedArray, index, value, timeout )](https://tc39.es/ecma262/#sec-atomics.wait)
    ///
    /// This function puts the surrounding agent in a wait queue and suspends
    /// it until notified or until the wait times out, returning a String
    /// differentiating those cases.
    fn wait<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? DoWait(sync, typedArray, index, value, timeout).
        let (buffer, byte_index_in_buffer, value, is_i64, t) = do_wait_preparation(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            arguments.get(3),
            gc.reborrow(),
        )
        .unbind()?;
        // 10. If mode is sync and AgentCanSuspend() is false,
        if !agent.can_suspend() {
            // throw a TypeError exception.
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "agent is not allowed to suspend",
                gc.into_nogc(),
            ));
        }
        if is_i64 {
            Ok(do_wait_critical::<false, true>(
                agent,
                buffer,
                byte_index_in_buffer,
                value,
                t,
                gc.into_nogc(),
            ))
        } else {
            Ok(do_wait_critical::<false, false>(
                agent,
                buffer,
                byte_index_in_buffer,
                value,
                t,
                gc.into_nogc(),
            ))
        }
    }

    /// ### [25.4.14 Atomics.waitAsync ( typedArray, index, value, timeout )](https://tc39.es/ecma262/#sec-atomics.waitasync)
    ///
    /// This function returns a Promise that is resolved when the calling agent
    /// is notified or the timeout is reached.
    fn wait_async<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? DoWait(async, typedArray, index, value, timeout).
        let (buffer, byte_index_in_buffer, value, is_i64, t) = do_wait_preparation(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            arguments.get(3),
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        if is_i64 {
            Ok(do_wait_critical::<true, true>(
                agent,
                buffer.unbind(),
                byte_index_in_buffer,
                value,
                t,
                gc.into_nogc(),
            ))
        } else {
            Ok(do_wait_critical::<true, false>(
                agent,
                buffer.unbind(),
                byte_index_in_buffer,
                value,
                t,
                gc.into_nogc(),
            ))
        }
    }

    /// ### [25.4.15 Atomics.notify ( typedArray, index, count )](https://tc39.es/ecma262/#sec-atomics.notify)
    ///
    /// This function notifies some agents that are sleeping in the wait queue.
    fn notify<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        let nogc = gc.nogc();
        let typed_array = arguments.get(0).bind(nogc);
        let index = arguments.get(1).bind(nogc);
        let count = arguments.get(2).scope(agent, nogc);
        // 1. Let taRecord be ? ValidateIntegerTypedArray(typedArray, true).
        let ta_record = validate_integer_typed_array::<true>(agent, typed_array, nogc)
            .unbind()?
            .bind(nogc);
        let typed_array = ta_record.object.scope(agent, nogc);
        // 2. Let byteIndexInBuffer be ? ValidateAtomicAccess(taRecord, index).
        let byte_index_in_buffer =
            validate_atomic_access(agent, ta_record.unbind(), index.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        // SAFETY: not shared.
        let count = unsafe { count.take(agent) }.bind(gc.nogc());
        // 3. If count is undefined, then
        let c = if count.is_undefined() {
            // a. Let c be +∞.
            usize::MAX
        } else {
            // 4. Else,
            // a. Let intCount be ? ToIntegerOrInfinity(count).
            let int_count = to_integer_or_infinity(agent, count.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // b. Let c be max(intCount, 0).
            usize::try_from(int_count.into_i64().max(0).cast_unsigned()).unwrap_or(usize::MAX)
        };
        let gc = gc.into_nogc();
        // SAFETY: not shared.
        let typed_array = unsafe { typed_array.take(agent) }.bind(gc);

        if c == 0 {
            return Ok(0.into());
        }
        // 5. Let buffer be typedArray.[[ViewedArrayBuffer]].
        let buffer = typed_array.viewed_array_buffer(agent);
        // 7. If IsSharedArrayBuffer(buffer) is false,
        let AnyArrayBuffer::SharedArrayBuffer(buffer) = buffer else {
            // return +0𝔽.
            return Ok(0.into());
        };
        // 6. Let block be buffer.[[ArrayBufferData]].
        // 8. Let WL be GetWaiterList(block, byteIndexInBuffer).
        let is_big_int_64_array = matches!(typed_array, AnyTypedArray::SharedBigInt64Array(_));
        let slot = buffer.as_slice(agent).slice_from(byte_index_in_buffer);
        let n = if is_big_int_64_array {
            // SAFETY: offset was checked.
            let slot = unsafe { slot.as_aligned::<u64>().unwrap_unchecked() };
            if c == usize::MAX {
                // Force the notify count down into a reasonable range: the
                // ecmascript_futex may return usize::MAX if the OS doesn't
                // give us a count number.
                slot.notify_all().min(i32::MAX as usize)
            } else {
                slot.notify_many(c)
            }
        } else {
            // SAFETY: offset was checked.
            let slot = unsafe { slot.as_aligned::<u32>().unwrap_unchecked() };
            if c == usize::MAX {
                // Force the notify count down into a reasonable range: the
                // ecmascript_futex may return usize::MAX if the OS doesn't
                // give us a count number.
                slot.notify_all().min(i32::MAX as usize)
            } else {
                slot.notify_many(c)
            }
        };
        // 9. Perform EnterCriticalSection(WL).
        // 10. Let S be RemoveWaiters(WL, c).
        // 11. For each element W of S, do
        //         a. Perform NotifyWaiter(WL, W).
        // 12. Perform LeaveCriticalSection(WL).
        // 13. Let n be the number of elements in S.
        // 14. Return 𝔽(n).
        Ok(Number::from_usize(agent, n, gc).into())
    }

    fn xor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        atomic_read_modify_write::<5>(
            agent,
            arguments.get(0),
            arguments.get(1),
            arguments.get(2),
            gc,
        )
        .map(|v| v.into())
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

/// ### [25.4.3.1 ValidateIntegerTypedArray ( typedArray, waitable )](https://tc39.es/ecma262/#sec-validateintegertypedarray)
///
/// The abstract operation ValidateIntegerTypedArray takes arguments typedArray
/// (an ECMAScript language value) and waitable (a Boolean) and returns either
/// a normal completion containing a TypedArray With Buffer Witness Record, or
/// a throw completion.
fn validate_integer_typed_array<'gc, const WAITABLE: bool>(
    agent: &mut Agent,
    typed_array: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, TypedArrayWithBufferWitnessRecords<'gc>> {
    // 1. Let taRecord be ? ValidateTypedArray(typedArray, unordered).
    let ta_record = validate_typed_array(
        agent,
        typed_array,
        ecmascript_atomics::Ordering::Unordered,
        gc,
    )?;
    // 2. NOTE: Bounds checking is not a synchronizing operation when
    //    typedArray's backing buffer is a growable SharedArrayBuffer.
    // 3. If waitable is true, then
    let is_valid_type = if WAITABLE {
        // a. If typedArray.[[TypedArrayName]] is neither "Int32Array" nor
        //    "BigInt64Array", throw a TypeError exception.
        ta_record.object.is_waitable()
    } else {
        // 4. Else,
        // a. Let type be TypedArrayElementType(typedArray).
        // b. If IsUnclampedIntegerElementType(type) is false and
        //    IsBigIntElementType(type) is false, throw a TypeError exception.
        ta_record.object.is_integer()
    };
    if is_valid_type {
        // 5. Return taRecord.
        Ok(ta_record)
    } else {
        // throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "cannot use TypedArray in Atomics",
            gc,
        ))
    }
}

/// 25.4.3.2 ValidateAtomicAccess ( taRecord, requestIndex )
///
/// The abstract operation ValidateAtomicAccess takes arguments taRecord (a
/// TypedArray With Buffer Witness Record) and requestIndex (an ECMAScript
/// language value) and returns either a normal completion containing an
/// integer or a throw completion.
fn validate_atomic_access<'gc>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    request_index: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, usize> {
    // 1. Let length be TypedArrayLength(taRecord).
    let length = ta_record.typed_array_length(agent);
    // 2. Let accessIndex be ? ToIndex(requestIndex).
    let access_index = to_index(agent, request_index, gc.reborrow()).unbind()?;
    // 3. Assert: accessIndex ≥ 0.
    // 4. If accessIndex ≥ length, throw a RangeError exception.
    if usize::try_from(access_index)
        .ok()
        .is_none_or(|access_index| access_index >= length)
    {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "accessIndex out of bounds",
            gc.into_nogc(),
        ));
    }
    let access_index = access_index as usize;
    // 5. Let typedArray be taRecord.[[Object]].
    let typed_array = ta_record.object;
    // 6. Let elementSize be TypedArrayElementSize(typedArray).
    let element_size = typed_array.typed_array_element_size();
    // 7. Let offset be typedArray.[[ByteOffset]].
    let offset = typed_array.byte_offset(agent);
    // 8. Return (accessIndex × elementSize) + offset.
    // SAFETY: access_index has been checked to be within length of the
    // typed_array buffer, which means that its byte_offset must also be.
    Ok(unsafe {
        access_index
            .unchecked_mul(element_size)
            .unchecked_add(offset)
    })
}

fn try_validate_atomic_access<'gc>(
    agent: &mut Agent,
    ta_record: &TypedArrayWithBufferWitnessRecords,
    request_index: Value,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, usize> {
    // 1. Let length be TypedArrayLength(taRecord).
    let length = ta_record.typed_array_length(agent);
    // 2. Let accessIndex be ? ToIndex(requestIndex).
    let access_index = try_to_index(agent, request_index, gc)?;
    // 3. Assert: accessIndex ≥ 0.
    // 4. If accessIndex ≥ length, throw a RangeError exception.
    if usize::try_from(access_index)
        .ok()
        .is_none_or(|access_index| access_index >= length)
    {
        return agent
            .throw_exception_with_static_message(
                ExceptionType::RangeError,
                "accessIndex out of bounds",
                gc,
            )
            .into();
    }
    let access_index = access_index as usize;
    // 5. Let typedArray be taRecord.[[Object]].
    let typed_array = ta_record.object;
    // 6. Let elementSize be TypedArrayElementSize(typedArray).
    let element_size = typed_array.typed_array_element_size();
    // 7. Let offset be typedArray.[[ByteOffset]].
    let offset = typed_array.byte_offset(agent);
    // 8. Return (accessIndex × elementSize) + offset.
    // SAFETY: access_index has been checked to be within length of the
    // typed_array buffer, which means that its byte_offset must also be.
    TryResult::Continue(unsafe {
        access_index
            .unchecked_mul(element_size)
            .unchecked_add(offset)
    })
}

/// 25.4.3.3 ValidateAtomicAccessOnIntegerTypedArray ( typedArray, requestIndex )
///
/// The abstract operation ValidateAtomicAccessOnIntegerTypedArray takes
/// arguments typedArray (an ECMAScript language value) and requestIndex (an
/// ECMAScript language value) and returns either a normal completion containing
/// an integer or a throw completion.
fn try_validate_atomic_access_on_integer_typed_array<'gc>(
    agent: &mut Agent,
    typed_array: Value,
    request_index: Value,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, (TypedArrayWithBufferWitnessRecords<'gc>, Option<usize>)> {
    // 1. Let taRecord be ? ValidateIntegerTypedArray(typedArray, false).
    let ta_record = validate_integer_typed_array::<false>(agent, typed_array, gc)?;
    // 2. Return ? ValidateAtomicAccess(taRecord, requestIndex).
    match try_validate_atomic_access(agent, &ta_record, request_index, gc) {
        ControlFlow::Continue(i) => Ok((ta_record, Some(i))),
        ControlFlow::Break(b) => match b {
            TryError::Err(err) => Err(err),
            // If atomic access couldn't be validated it means that the
            // requestIndex value couldn't be converted into an index.
            TryError::GcError => Ok((ta_record, None)),
        },
    }
}

/// ### [25.4.3.4 RevalidateAtomicAccess ( typedArray, byteIndexInBuffer )](https://tc39.es/ecma262/#sec-revalidateatomicaccess)
///
/// The abstract operation RevalidateAtomicAccess takes arguments typedArray (a
/// TypedArray) and byteIndexInBuffer (an integer) and returns either a normal
/// completion containing unused or a throw completion. This operation
/// revalidates the index within the backing buffer for atomic operations after
/// all argument coercions are performed in Atomics methods, as argument
/// coercions can have arbitrary side effects, which could cause the buffer to
/// become out of bounds. This operation does not throw when typedArray's
/// backing buffer is a SharedArrayBuffer.
fn revalidate_atomic_access<'gc>(
    agent: &mut Agent,
    typed_array: AnyTypedArray,
    byte_index_in_buffer: usize,
    gc: NoGcScope<'gc, '_>,
) -> JsResult<'gc, ()> {
    // 1. Let taRecord be MakeTypedArrayWithBufferWitnessRecord(typedArray, unordered).
    let ta_record = make_typed_array_with_buffer_witness_record(
        agent,
        typed_array,
        ecmascript_atomics::Ordering::Unordered,
    );
    // 2. NOTE: Bounds checking is not a synchronizing operation when
    //    typedArray's backing buffer is a growable SharedArrayBuffer.
    // 3. If IsTypedArrayOutOfBounds(taRecord) is true,
    if ta_record.is_typed_array_out_of_bounds(agent) {
        // throw a TypeError exception.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "TypedArray out of bounds",
            gc.into_nogc(),
        ));
    }
    // 4. Assert: byteIndexInBuffer ≥ typedArray.[[ByteOffset]].
    debug_assert!(byte_index_in_buffer >= typed_array.byte_offset(agent));
    // 5. If byteIndexInBuffer ≥ taRecord.[[CachedBufferByteLength]],
    if byte_index_in_buffer >= ta_record.cached_buffer_byte_length.0 {
        // throw a RangeError exception.
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "accessIndex out of bounds",
            gc.into_nogc(),
        ));
    }
    // 6. Return unused.
    Ok(())
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
fn atomic_read_modify_write<'gc, const OP: u8>(
    agent: &mut Agent,
    typed_array: Value,
    index: Value,
    value: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Numeric<'gc>> {
    let typed_array = typed_array.bind(gc.nogc());
    let index = index.bind(gc.nogc());
    let value = value.bind(gc.nogc());

    let (typed_array, byte_index_in_buffer, v) = handle_typed_array_index_value(
        agent,
        typed_array.unbind(),
        index.unbind(),
        value.unbind(),
        gc.reborrow(),
    )
    .unbind()?;
    let gc = gc.into_nogc();
    let typed_array = typed_array.bind(gc);
    let v = v.bind(gc);

    // 5. Let buffer be typedArray.[[ViewedArrayBuffer]].
    let buffer = typed_array.viewed_array_buffer(agent);
    // 6. Let elementType be TypedArrayElementType(typedArray).
    // 7. Return GetModifySetValueInBuffer(buffer, byteIndexInBuffer, elementType, v, op).
    Ok(for_any_typed_array!(
        typed_array,
        _t,
        {
            get_modify_set_value_in_buffer::<ElementType, OP>(
                agent,
                buffer,
                byte_index_in_buffer,
                v,
                gc,
            )
        },
        ElementType
    ))
}

fn handle_typed_array_index_value<'gc>(
    agent: &mut Agent,
    typed_array: Value,
    index: Value,
    value: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (AnyTypedArray<'gc>, usize, Numeric<'gc>)> {
    let typed_array = typed_array.bind(gc.nogc());
    let index = index.bind(gc.nogc());
    let value = value.bind(gc.nogc());
    // 1. Let byteIndexInBuffer be ? ValidateAtomicAccessOnIntegerTypedArray(typedArray, index).
    let (ta_record, byte_index_in_buffer) =
        try_validate_atomic_access_on_integer_typed_array(agent, typed_array, index, gc.nogc())
            .unbind()?
            .bind(gc.nogc());
    let (byte_index_in_buffer, typed_array, value) =
        if let (Some(byte_index_in_buffer), Ok(value)) = (
            byte_index_in_buffer,
            if ta_record.object.is_bigint() {
                BigInt::try_from(value).map(|value| value.into())
            } else {
                Number::try_from(value).map(|value| {
                    number_convert_to_integer_or_infinity(agent, value, gc.nogc()).into()
                })
            },
        ) {
            let typed_array = ta_record.object;
            (byte_index_in_buffer, typed_array, value)
        } else {
            handle_typed_array_index_value_slow(
                agent,
                ta_record.unbind(),
                index.unbind(),
                value.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };
    let typed_array = typed_array.unbind();
    let value = value.unbind();
    let gc = gc.into_nogc();
    let typed_array = typed_array.bind(gc);
    let value = value.bind(gc);
    Ok((typed_array, byte_index_in_buffer, value))
}

#[inline(never)]
#[cold]
fn handle_typed_array_index_value_slow<'gc>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    index: Value,
    value: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (usize, AnyTypedArray<'gc>, Numeric<'gc>)> {
    let ta_record = ta_record.bind(gc.nogc());
    let is_bigint = ta_record.object.is_bigint();
    let typed_array = ta_record.object.scope(agent, gc.nogc());
    let index = index.bind(gc.nogc());
    let value = value.scope(agent, gc.nogc());

    let byte_index_in_buffer =
        validate_atomic_access(agent, ta_record.unbind(), index.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

    let value = unsafe { value.take(agent) }.bind(gc.nogc());

    // 2. If typedArray.[[ContentType]] is bigint,
    let v: Numeric = if is_bigint {
        // let v be ? ToBigInt(value).
        to_big_int(agent, value.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into()
    } else {
        // 3. Otherwise, let v be 𝔽(? ToIntegerOrInfinity(value)).
        to_integer_number_or_infinity(agent, value.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
            .into()
    };
    let v = v.unbind();
    let gc = gc.into_nogc();
    let v = v.bind(gc);
    let typed_array = unsafe { typed_array.take(agent) }.bind(gc);
    revalidate_atomic_access(agent, typed_array, byte_index_in_buffer, gc)?;
    Ok((byte_index_in_buffer, typed_array, v))
}

#[inline(never)]
#[cold]
fn handle_typed_array_index_two_values_slow<'gc>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    index: Value,
    expected_value: Value,
    replacement_value: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (usize, AnyTypedArray<'gc>, Numeric<'gc>, Numeric<'gc>)> {
    let ta_record = ta_record.bind(gc.nogc());
    let is_bigint = ta_record.object.is_bigint();
    let typed_array = ta_record.object.scope(agent, gc.nogc());
    let index = index.bind(gc.nogc());
    let expected_value = expected_value.scope(agent, gc.nogc());
    let replacement_value = replacement_value.scope(agent, gc.nogc());

    let byte_index_in_buffer =
        validate_atomic_access(agent, ta_record.unbind(), index.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());

    // 4. If typedArray.[[ContentType]] is bigint, then
    let (expected, replacement): (Numeric, Numeric) = if is_bigint {
        // a. Let expected be ? ToBigInt(expectedValue).
        let expected = to_big_int(agent, expected_value.get(agent), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        // SAFETY: not shared.
        let expected = unsafe { expected_value.replace_self(agent, expected.unbind()) };
        // b. Let replacement be ? ToBigInt(replacementValue).
        // SAFETY: not shared.
        let replacement = to_big_int(
            agent,
            unsafe { replacement_value.take(agent) },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        (
            unsafe { expected.take(agent) }.bind(gc.nogc()).into(),
            replacement.into(),
        )
    } else {
        // 5. Else,
        // a. Let expected be 𝔽(? ToIntegerOrInfinity(expectedValue)).
        let expected =
            to_integer_number_or_infinity(agent, expected_value.get(agent), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
        // SAFETY: not shared.
        let expected = unsafe { expected_value.replace_self(agent, expected.unbind()) };
        // b. Let replacement be 𝔽(? ToIntegerOrInfinity(replacementValue)).
        // SAFETY: not shared.
        let replacement = to_integer_number_or_infinity(
            agent,
            unsafe { replacement_value.take(agent) },
            gc.reborrow(),
        )
        .unbind()?
        .bind(gc.nogc());
        (
            unsafe { expected.take(agent) }.bind(gc.nogc()).into(),
            replacement.into(),
        )
    };
    let expected = expected.unbind();
    let replacement = replacement.unbind();
    let gc = gc.into_nogc();
    let expected = expected.bind(gc);
    let replacement = replacement.bind(gc);
    let typed_array = unsafe { typed_array.take(agent) }.bind(gc);
    // 6. Perform ? RevalidateAtomicAccess(typedArray, byteIndexInBuffer).
    revalidate_atomic_access(agent, typed_array, byte_index_in_buffer, gc)?;
    Ok((byte_index_in_buffer, typed_array, expected, replacement))
}

#[inline(never)]
#[cold]
fn atomic_load_slow<'gc>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    index: Value,
    length: usize,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (usize, AnyTypedArray<'gc>)> {
    let mut ta_record = ta_record.bind(gc.nogc());
    let index = index.bind(gc.nogc());
    let mut revalidate = false;

    // 2. Let accessIndex be ? ToIndex(requestIndex).
    let access_index =
        if let Some(index) = try_result_into_js(try_to_index(agent, index, gc.nogc())).unbind()? {
            index
        } else {
            let ta = ta_record.object.scope(agent, gc.nogc());
            let cached_buffer_byte_length = ta_record.cached_buffer_byte_length;
            let access_index = to_index(agent, index.unbind(), gc.reborrow()).unbind()?;
            revalidate = true;
            // SAFETY: not shared.
            ta_record = unsafe {
                TypedArrayWithBufferWitnessRecords {
                    object: ta.take(agent),
                    cached_buffer_byte_length,
                }
            };
            access_index
        };
    // 3. Assert: accessIndex ≥ 0.
    // 4. If accessIndex ≥ length, throw a RangeError exception.
    if access_index >= length as u64 {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "accessIndex out of bounds",
            gc.into_nogc(),
        ));
    }
    let access_index = access_index as usize;
    // 5. Let typedArray be taRecord.[[Object]].
    // 6. Let elementSize be TypedArrayElementSize(typedArray).
    // 7. Let offset be typedArray.[[ByteOffset]].
    let offset = ta_record.object.byte_offset(agent);
    // 8. Return (accessIndex × elementSize) + offset.
    let byte_index_in_buffer = offset + access_index * ta_record.object.typed_array_element_size();
    let typed_array = ta_record.object.unbind();
    let gc = gc.into_nogc();
    let typed_array = typed_array.bind(gc);
    if revalidate {
        // 2. Perform ? RevalidateAtomicAccess(typedArray, byteIndexInBuffer).
        revalidate_atomic_access(agent, typed_array, byte_index_in_buffer, gc)?;
    }
    Ok((byte_index_in_buffer, typed_array))
}

/// ### [25.4.3.14 DoWait ( mode, typedArray, index, value, timeout )](https://tc39.es/ecma262/#sec-dowait)
///
/// The abstract operation DoWait takes arguments mode (sync or async),
/// typedArray (an ECMAScript language value), index (an ECMAScript language
/// value), value (an ECMAScript language value), and timeout (an ECMAScript
/// language value) and returns either a normal completion containing either an
/// Object, "not-equal", "timed-out", or "ok", or a throw completion. It
/// performs the following steps when called:
///
/// > NOTE: `additionalTimeout` allows implementations to pad timeouts as
/// > necessary, such as for reducing power consumption or coarsening timer
/// > resolution to mitigate timing attacks. This value may differ from call to
/// > call of DoWait.
fn do_wait_preparation<'gc>(
    agent: &mut Agent,
    typed_array: Value,
    index: Value,
    value: Value,
    timeout: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (SharedArrayBuffer<'gc>, usize, i64, bool, u64)> {
    let nogc = gc.nogc();
    let typed_array = typed_array.bind(nogc);
    let index = index.bind(nogc);
    let value = value.bind(nogc);
    let timeout = timeout.bind(nogc);

    // 1. Let taRecord be ? ValidateIntegerTypedArray(typedArray, true).
    let ta_record = validate_integer_typed_array::<true>(agent, typed_array, nogc)
        .unbind()?
        .bind(nogc);
    // 2. Let buffer be taRecord.[[Object]].[[ViewedArrayBuffer]].
    // 3. If IsSharedArrayBuffer(buffer) is false, throw a TypeError exception.
    let Ok(typed_array) = SharedTypedArray::try_from(ta_record.object) else {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "cannot wait on ArrayBuffer",
            gc.into_nogc(),
        ));
    };
    // These are the only waitable integer TypedArrays.
    unsafe {
        assert_unchecked(matches!(
            typed_array,
            SharedTypedArray::SharedInt32Array(_) | SharedTypedArray::SharedBigInt64Array(_)
        ));
    };
    // 4. Let i be ? ValidateAtomicAccess(taRecord, index).
    let (ta_record, byte_index_in_buffer) =
        match try_validate_atomic_access(agent, &ta_record, index, nogc) {
            ControlFlow::Continue(byte_index_in_buffer) => (ta_record, Some(byte_index_in_buffer)),
            ControlFlow::Break(b) => match b {
                TryError::Err(err) => return Err(err.unbind()),
                // If atomic access couldn't be validated it means that the
                // requestIndex value couldn't be converted into an index.
                TryError::GcError => (ta_record, None),
            },
        };
    // 5. Let arrayTypeName be typedArray.[[TypedArrayName]].
    // 6. If arrayTypeName is "BigInt64Array",
    let is_big_int_64_array = matches!(typed_array, SharedTypedArray::SharedBigInt64Array(_));
    let (typed_array, byte_index_in_buffer, v, t) =
        if let (Some(byte_index_in_buffer), Ok(v), Some(q)) = (
            byte_index_in_buffer,
            if is_big_int_64_array {
                // 6. If arrayTypeName is "BigInt64Array", let v be ? ToBigInt64(value).
                BigInt::try_from(value).map(|v| to_big_int64_big_int(agent, v))
            } else {
                // 7. Else, let v be ? ToInt32(value).
                Number::try_from(value).map(|v| to_int32_number(agent, v) as i64)
            },
            // 8. Let q be ? ToNumber(timeout).
            if timeout.is_undefined() {
                // 9. If q is either NaN or +∞𝔽,
                // let t be +∞;
                Some(u64::MAX)
            } else if let Value::Integer(q) = timeout {
                // else let t be max(ℝ(q), 0).
                Some(q.into_i64().max(0).unsigned_abs())
            } else {
                None
            },
        ) {
            (typed_array, byte_index_in_buffer, v, q)
        } else {
            do_wait_slow(
                agent,
                ta_record.unbind(),
                is_big_int_64_array,
                index.unbind(),
                value.unbind(),
                timeout.unbind(),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc())
        };
    // 11. Let block be buffer.[[ArrayBufferData]].
    let typed_array = typed_array.unbind().bind(gc.into_nogc());
    let buffer = typed_array.viewed_array_buffer(agent);
    Ok((buffer, byte_index_in_buffer, v, is_big_int_64_array, t))
}

fn do_wait_critical<'gc, const IS_ASYNC: bool, const IS_I64: bool>(
    agent: &mut Agent,
    buffer: SharedArrayBuffer,
    byte_index_in_buffer: usize,
    v: i64,
    t: u64,
    gc: NoGcScope<'gc, '_>,
) -> Value<'gc> {
    let slot = buffer.as_slice(agent).slice_from(byte_index_in_buffer);
    // 14. Let WL be GetWaiterList(block, byteIndexInBuffer).
    // 15. If mode is sync, then
    // a. Let promiseCapability be blocking.
    // b. Let resultObject be undefined.
    // 16. Else,
    // a. Let promiseCapability be ! NewPromiseCapability(%Promise%).
    // b. Let resultObject be OrdinaryObjectCreate(%Object.prototype%).
    // 17. Perform EnterCriticalSection(WL).
    // 18. Let elementType be TypedArrayElementType(typedArray).
    // 19. Let w be GetValueFromBuffer(buffer, byteIndexInBuffer, elementType, true, seq-cst).
    let v_not_equal_to_w = if IS_I64 {
        let v = v as u64;
        // SAFETY: buffer is still live and index was checked.
        let slot = unsafe { slot.as_aligned::<u64>().unwrap_unchecked() };
        let w = slot.load(Ordering::SeqCst);
        v != w
    } else {
        let v = v as i32 as u32;
        // SAFETY: buffer is still live and index was checked.
        let slot = unsafe { slot.as_aligned::<u32>().unwrap_unchecked() };
        let w = slot.load(Ordering::SeqCst);
        v != w
    };
    // 20. If v ≠ w, then
    if v_not_equal_to_w {
        // a. Perform LeaveCriticalSection(WL).
        // b. If mode is sync, return "not-equal".
        if !IS_ASYNC {
            return BUILTIN_STRING_MEMORY.not_equal.into();
        }
        // c. Perform ! CreateDataPropertyOrThrow(resultObject, "async", false).
        // d. Perform ! CreateDataPropertyOrThrow(resultObject, "value", "not-equal").
        let result_object =
            create_wait_result_object(agent, false, BUILTIN_STRING_MEMORY.not_equal.into());
        // e. Return resultObject.
        return result_object.into();
    }
    // 21. If t = 0 and mode is async, then
    if t == 0 && IS_ASYNC {
        // a. NOTE: There is no special handling of synchronous immediate
        //    timeouts. Asynchronous immediate timeouts have special handling
        //    in order to fail fast and avoid unnecessary Promise jobs.
        // b. Perform LeaveCriticalSection(WL).
        // c. Perform ! CreateDataPropertyOrThrow(resultObject, "async", false).
        // d. Perform ! CreateDataPropertyOrThrow(resultObject, "value", "timed-out").
        let result_object =
            create_wait_result_object(agent, false, BUILTIN_STRING_MEMORY.timed_out.into());
        // e. Return resultObject.
        return result_object.into();
    }
    // 22. Let thisAgent be AgentSignifier().
    // 23. Let now be the time value (UTC) identifying the current time.
    // 24. Let additionalTimeout be an implementation-defined non-negative
    //     mathematical value.
    // 25. Let timeoutTime be ℝ(now) + t + additionalTimeout.
    // 26. NOTE: When t is +∞, timeoutTime is also +∞.
    // 27. Let waiterRecord be a new Waiter Record {
    //         [[AgentSignifier]]: thisAgent,
    //         [[PromiseCapability]]: promiseCapability,
    //         [[TimeoutTime]]: timeoutTime,
    //         [[Result]]: "ok"
    // }.
    // 28. Perform AddWaiter(WL, waiterRecord).
    // 29. If mode is sync, then
    if !IS_ASYNC {
        // a. Perform SuspendThisAgent(WL, waiterRecord).
        let result = if IS_I64 {
            let v = v as u64;
            // SAFETY: buffer is still live and index was checked.
            let slot = unsafe { slot.as_aligned::<u64>().unwrap_unchecked() };
            if t == u64::MAX {
                slot.wait(v)
            } else {
                slot.wait_timeout(v, Duration::from_millis(t))
            }
        } else {
            let v = v as u32;
            // SAFETY: buffer is still live and index was checked.
            let slot = unsafe { slot.as_aligned::<u32>().unwrap_unchecked() };
            if t == u64::MAX {
                slot.wait(v)
            } else {
                slot.wait_timeout(v, Duration::from_millis(t))
            }
        };
        // 31. Perform LeaveCriticalSection(WL).
        // 32. If mode is sync, return waiterRecord.[[Result]].

        match result {
            Ok(_) => BUILTIN_STRING_MEMORY.ok.into(),
            Err(err) => match err {
                FutexError::Timeout => BUILTIN_STRING_MEMORY.timed_out.into(),
                FutexError::NotEqual => BUILTIN_STRING_MEMORY.not_equal.into(),
                FutexError::Unknown => panic!(),
            },
        }
    } else {
        let promise_capability = PromiseCapability::new(agent, gc);
        let promise = Global::new(agent, promise_capability.promise.unbind());
        // 30. Else if timeoutTime is finite, then
        // a. Perform EnqueueAtomicsWaitAsyncTimeoutJob(WL, waiterRecord).
        let buffer = buffer.get_data_block(agent).clone();
        enqueue_atomics_wait_async_job::<IS_I64>(
            agent,
            buffer,
            byte_index_in_buffer,
            v,
            t,
            promise,
            gc,
        );
        // 31. Perform LeaveCriticalSection(WL).
        // 33. Perform ! CreateDataPropertyOrThrow(resultObject, "async", true).
        // 34. Perform ! CreateDataPropertyOrThrow(resultObject, "value", promiseCapability.[[Promise]]).
        let result_object =
            create_wait_result_object(agent, true, promise_capability.promise().into());
        // 35. Return resultObject.
        result_object.into()
    }
}

#[cold]
#[inline(never)]
fn do_wait_slow<'gc>(
    agent: &mut Agent,
    ta_record: TypedArrayWithBufferWitnessRecords,
    is_big_int_64_array: bool,
    index: Value,
    value: Value,
    timeout: Value,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, (SharedTypedArray<'gc>, usize, i64, u64)> {
    let nogc = gc.nogc();
    let ta_record = ta_record.bind(nogc);
    // SAFETY: TypedArray is guaranteed to be a shared TypedArray at this point.
    let typed_array = unsafe { SharedTypedArray::try_from(ta_record.object).unwrap_unchecked() };
    let scoped_typed_array = typed_array.scope(agent, nogc);
    let index = index.bind(nogc);
    let scoped_timeout = timeout.scope(agent, nogc);
    let scoped_value = value.scope(agent, nogc);
    let i = validate_atomic_access(agent, ta_record.unbind(), index.unbind(), gc.reborrow())
        .unbind()?
        .bind(gc.nogc());
    // SAFETY: not shared.
    let value = unsafe { scoped_value.take(agent) }.bind(gc.nogc());
    let v = if is_big_int_64_array {
        // 6. If arrayTypeName is "BigInt64Array", let v be ? ToBigInt64(value).
        to_big_int64(agent, value.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc())
    } else {
        // 7. Else, let v be ? ToInt32(value).
        to_int32(agent, value.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc()) as i64
    };
    // 8. Let q be ? ToNumber(timeout).
    // SAFETY: not shared.
    let q = to_number(agent, unsafe { scoped_timeout.take(agent) }, gc.reborrow()).unbind()?;
    let gc = gc.into_nogc();
    let q = q.bind(gc);
    // 9. If q is either NaN or +∞𝔽,
    let t = if q.is_nan_(agent) || q.is_pos_infinity_(agent) {
        // let t be +∞;
        u64::MAX
    } else if q.is_neg_infinity_(agent) {
        // else if q is -∞𝔽, let t be 0;
        0
    } else {
        // else let t be max(ℝ(q), 0).
        q.into_i64_(agent).max(0) as u64
    };
    Ok((unsafe { scoped_typed_array.take(agent) }.bind(gc), i, v, t))
}

fn create_wait_result_object<'gc>(
    agent: &mut Agent,
    is_async: bool,
    value: Value<'gc>,
) -> OrdinaryObject<'gc> {
    OrdinaryObject::create_object(
        agent,
        Some(
            agent
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .into(),
        ),
        &[
            // 1. Perform ! CreateDataPropertyOrThrow(resultObject, "async", isAsync).
            ObjectEntry::new_data_entry(BUILTIN_STRING_MEMORY.r#async.into(), is_async.into()),
            // 34. Perform ! CreateDataPropertyOrThrow(resultObject, "value", value).
            ObjectEntry::new_data_entry(BUILTIN_STRING_MEMORY.value.into(), value),
        ],
    )
    .expect("Should perform GC here")
}

#[derive(Debug)]
struct WaitAsyncJobInner {
    promise_to_resolve: Global<Promise<'static>>,
    join_handle: JoinHandle<Result<(), FutexError>>,
    _has_timeout: bool,
}

#[derive(Debug)]
#[repr(transparent)]
pub(crate) struct WaitAsyncJob(Box<WaitAsyncJobInner>);

impl WaitAsyncJob {
    pub(crate) fn is_finished(&self) -> bool {
        self.0.join_handle.is_finished()
    }

    pub(crate) fn _will_halt(&self) -> bool {
        self.0._has_timeout
    }

    // NOTE: The reason for using `GcScope` here even though we could've gotten
    // away with `NoGcScope` is that this is essentially a trait impl method,
    // but currently without the trait. The job trait will be added eventually
    // and we can get rid of this lint exception.
    #[allow(unknown_lints, can_use_no_gc_scope)]
    pub(crate) fn run<'gc>(self, agent: &mut Agent, gc: GcScope) -> JsResult<'gc, ()> {
        let gc = gc.into_nogc();
        let promise = self.0.promise_to_resolve.take(agent).bind(gc);
        let Ok(result) = self.0.join_handle.join() else {
            // Foreign thread died; we can never resolve.
            return Ok(());
        };
        // a. Perform EnterCriticalSection(WL).
        // b. If WL.[[Waiters]] contains waiterRecord, then
        //         i. Let timeOfJobExecution be the time value (UTC) identifying the current time.
        //         ii. Assert: ℝ(timeOfJobExecution) ≥ waiterRecord.[[TimeoutTime]] (ignoring potential non-monotonicity of time values).
        //         iii. Set waiterRecord.[[Result]] to "timed-out".
        //         iv. Perform RemoveWaiter(WL, waiterRecord).
        //         v. Perform NotifyWaiter(WL, waiterRecord).
        // c. Perform LeaveCriticalSection(WL).
        let promise_capability = PromiseCapability::from_promise(promise, true);
        let result = match result {
            Ok(_) => BUILTIN_STRING_MEMORY.ok.into(),
            Err(FutexError::NotEqual) => BUILTIN_STRING_MEMORY.ok.into(),
            Err(FutexError::Timeout) => BUILTIN_STRING_MEMORY.timed_out.into(),
            Err(FutexError::Unknown) => {
                let error = agent.throw_exception_with_static_message(
                    ExceptionType::Error,
                    "unknown error occurred",
                    gc,
                );
                promise_capability.reject(agent, error.value(), gc);
                return Ok(());
            }
        };
        unwrap_try(promise_capability.try_resolve(agent, result, gc));
        // d. Return unused.
        Ok(())
    }
}

/// ### [25.4.3.15 EnqueueAtomicsWaitAsyncTimeoutJob ( WL, waiterRecord )](https://tc39.es/ecma262/#sec-enqueueatomicswaitasynctimeoutjob)
///
/// The abstract operation EnqueueAtomicsWaitAsyncTimeoutJob takes arguments WL
/// (a WaiterList Record) and waiterRecord (a Waiter Record) and returns
/// unused.
fn enqueue_atomics_wait_async_job<const IS_I64: bool>(
    agent: &mut Agent,
    buffer: SharedDataBlock,
    byte_index_in_buffer: usize,
    v: i64,
    t: u64,
    promise: Global<Promise>,
    gc: NoGcScope,
) {
    // 1. Let timeoutJob be a new Job Abstract Closure with no parameters that
    //    captures WL and waiterRecord and performs the following steps when
    //    called:
    let signal = Arc::new(AtomicBool::new(false));
    let s = signal.clone();
    let handle = thread::spawn(move || {
        let slot = buffer.as_racy_slice().slice_from(byte_index_in_buffer);
        if IS_I64 {
            let v = v as u64;
            // SAFETY: buffer is still live and index was checked.
            let slot = unsafe { slot.as_aligned::<u64>().unwrap_unchecked() };
            s.store(true, std::sync::atomic::Ordering::Release);
            if t == u64::MAX {
                slot.wait(v)
            } else {
                slot.wait_timeout(v, Duration::from_millis(t))
            }
        } else {
            let v = v as i32 as u32;
            // SAFETY: buffer is still live and index was checked.
            let slot = unsafe { slot.as_aligned::<u32>().unwrap_unchecked() };
            s.store(true, std::sync::atomic::Ordering::Release);
            if t == u64::MAX {
                slot.wait(v)
            } else {
                slot.wait_timeout(v, Duration::from_millis(t))
            }
        }
    });
    let wait_async_job = Job {
        realm: Some(agent.current_realm(gc).unbind()),
        inner: InnerJob::WaitAsync(WaitAsyncJob(Box::new(WaitAsyncJobInner {
            promise_to_resolve: promise,
            join_handle: handle,
            _has_timeout: t != u64::MAX,
        }))),
    };
    while !signal.load(std::sync::atomic::Ordering::Acquire) {
        // Wait until the thread has started up and is about to go to sleep.
    }
    // 2. Let now be the time value (UTC) identifying the current time.
    // 3. Let currentRealm be the current Realm Record.
    // 4. Perform HostEnqueueTimeoutJob(timeoutJob, currentRealm, 𝔽(waiterRecord.[[TimeoutTime]]) - now).
    agent.host_hooks.enqueue_generic_job(wait_async_job);
    // 5. Return unused.
}
