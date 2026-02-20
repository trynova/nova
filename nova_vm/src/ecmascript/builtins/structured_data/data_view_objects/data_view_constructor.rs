// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ecmascript_atomics::Ordering;

use crate::{
    ecmascript::{
        Agent, AnyDataView, ArgumentsList, BUILTIN_STRING_MEMORY, Behaviour, Builtin,
        BuiltinIntrinsicConstructor, ExceptionType, Function, JsResult, Object, ProtoIntrinsics,
        Realm, String, Value, builders::BuiltinFunctionBuilder, is_fixed_length_array_buffer,
        ordinary_create_from_constructor, require_internal_slot_any_array_buffer, to_index,
    },
    engine::{Bindable, GcScope, Scopable},
    heap::IntrinsicConstructorIndexes,
};

pub(crate) struct DataViewConstructor;
impl Builtin for DataViewConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.DataView;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(Self::constructor);
}
impl BuiltinIntrinsicConstructor for DataViewConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::DataView;
}

/// ### [25.3.2 The DataView Constructor](https://tc39.es/ecma262/#sec-dataview-constructor)
impl DataViewConstructor {
    /// ### [25.3.2.1 DataView ( buffer \[ , byteOffset \[ , byteLength \] \] )](https://tc39.es/ecma262/#sec-dataview-buffer-byteoffset-bytelength)
    fn constructor<'gc>(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList<'_, 'static>,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<'static, Value<'static>> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin DataView constructor without new is forbidden",
                gc.into_nogc(),
            ));
        };
        let new_target = Function::try_from(new_target)
            .unwrap()
            .scope(agent, gc.nogc());

        crate::engine::bind!(let buffer = arguments.get(0), gc);
        crate::engine::bind!(let byte_offset = arguments.get(1), gc);
        let byte_length = arguments.get(2).scope(agent, gc.nogc());

        // 2. Perform ? RequireInternalSlot(buffer, [[ArrayBufferData]]).
        let scoped_buffer = require_internal_slot_any_array_buffer(agent, buffer, gc.nogc())?
            .scope(agent, gc.nogc());

        // 3. Let offset be ? ToIndex(byteOffset).
        let offset = to_index(agent, byte_offset, gc.reborrow())? as usize;

        // 4. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        crate::engine::bind!(let buffer = scoped_buffer.get(agent).local(), gc);
        if buffer.is_detached(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempting to access detached ArrayBuffer",
                gc.into_nogc(),
            ));
        }

        // 5. Let bufferByteLength be ArrayBufferByteLength(buffer, seq-cst).
        let buffer_byte_length = buffer.byte_length(agent, Ordering::SeqCst);

        // 6. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
                gc.into_nogc(),
            ));
        }

        // 7. Let bufferIsFixedLength be IsFixedLengthArrayBuffer(buffer).
        let buffer_is_fixed_length = is_fixed_length_array_buffer(agent, buffer);
        let buffer_is_shared = buffer.is_shared();

        // 8. If byteLength is undefined, then
        crate::engine::bind!(let byte_length = byte_length.get(agent).local(), gc);
        let view_byte_length = if byte_length.is_undefined() {
            // a. If bufferIsFixedLength is true, then
            if buffer_is_fixed_length {
                // i. Let viewByteLength be bufferByteLength - offset.
                Some(buffer_byte_length - offset)
            } else {
                // b. Else,
                // i. Let viewByteLength be auto.
                None
            }
        } else {
            // 9. Else,
            // a. Let viewByteLength be ? ToIndex(byteLength).
            let view_byte_length = to_index(agent, byte_length, gc.reborrow())? as usize;
            // b. If offset + viewByteLength > bufferByteLength, throw a RangeError exception.
            if offset + view_byte_length > buffer_byte_length {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "offset is outside the bounds of the buffer",
                    gc.into_nogc(),
                ));
            }
            Some(view_byte_length)
        };

        // 10. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%DataView.prototype%", « [[DataView]], [[ViewedArrayBuffer]], [[ByteLength]], [[ByteOffset]] »).
        let o = ordinary_create_from_constructor(
            agent,
            new_target.get(agent).local(),
            if buffer_is_shared {
                #[cfg(feature = "shared-array-buffer")]
                {
                    ProtoIntrinsics::SharedDataView
                }
                #[cfg(not(feature = "shared-array-buffer"))]
                {
                    unreachable!()
                }
            } else {
                ProtoIntrinsics::DataView
            },
            gc.reborrow(),
        )?;
        let gc = gc.into_nogc();
        crate::engine::bind!(let o = o, gc);
        crate::engine::bind!(let buffer = scoped_buffer.get(agent).local(), gc);
        // 11. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        if buffer.is_detached(agent) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempting to access detached ArrayBuffer",
                gc,
            ));
        }

        // 12. Set bufferByteLength to ArrayBufferByteLength(buffer, seq-cst).
        let buffer_byte_length = buffer.byte_length(agent, Ordering::SeqCst);

        // 13. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
                gc,
            ));
        }

        // 14. If byteLength is not undefined, then
        if let Some(view_byte_length) = view_byte_length {
            // a. If offset + viewByteLength > bufferByteLength, throw a RangeError exception.
            if offset + view_byte_length > buffer_byte_length {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "offset is outside the bounds of the buffer",
                    gc,
                ));
            }
        }

        let o = AnyDataView::try_from(o).unwrap();

        // 15. Set O.[[ViewedArrayBuffer]] to buffer.
        // 16. Set O.[[ByteLength]] to viewByteLength.
        // 17. Set O.[[ByteOffset]] to offset.
        // SAFETY: Initialising O.
        unsafe { o.initialise_data(agent, buffer, view_byte_length, offset) };

        // 18. Return O.
        Ok(o.into())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: Realm<'static>) {
        let intrinsics = agent.get_realm_record_by_id(realm).intrinsics();
        let data_view_prototype = intrinsics.data_view_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<DataViewConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(data_view_prototype.into())
            .build();
    }
}
