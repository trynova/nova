// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::abstract_operations::type_conversion::try_to_index;
use crate::ecmascript::builtins::array_buffer::{
    ViewedArrayBufferByteLength, ViewedArrayBufferByteOffset,
};
use crate::engine::context::GcScope;
use crate::engine::TryResult;
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_index,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            array_buffer::{
                array_buffer_byte_length, is_detached_buffer, is_fixed_length_array_buffer,
                Ordering,
            },
            data_view::DataView,
            ordinary::ordinary_create_from_constructor,
            structured_data::array_buffer_objects::array_buffer_prototype::require_internal_slot_array_buffer,
            ArgumentsList, Behaviour, Builtin, BuiltinIntrinsicConstructor,
        },
        execution::{agent::ExceptionType, Agent, JsResult, ProtoIntrinsics, RealmIdentifier},
        types::{Function, IntoObject, IntoValue, Object, String, Value, BUILTIN_STRING_MEMORY},
    },
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
impl<'gc> DataViewConstructor {
    /// ### [25.3.2.1 DataView ( buffer \[ , byteOffset \[ , byteLength \] \] )](https://tc39.es/ecma262/#sec-dataview-buffer-byteoffset-bytelength)
    fn constructor(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
        mut gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin DataView constructor without new is forbidden",
                gc.nogc(),
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();

        let buffer = arguments.get(0);
        let byte_offset = arguments.get(1);
        let byte_length = arguments.get(2);

        // 2. Perform ? RequireInternalSlot(buffer, [[ArrayBufferData]]).
        let mut buffer = require_internal_slot_array_buffer(agent, buffer, gc.nogc())?;
        let scoped_buffer = buffer.scope(agent, gc.nogc());

        // 3. Let offset be ? ToIndex(byteOffset).
        let offset = if let TryResult::Continue(res) = try_to_index(agent, byte_offset, gc.nogc()) {
            res? as usize
        } else {
            let res = to_index(agent, byte_offset, gc.reborrow())?;
            buffer = scoped_buffer.get(agent).bind(gc.nogc());
            res as usize
        };

        // 4. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        if is_detached_buffer(agent, buffer) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempting to access detached ArrayBuffer",
                gc.nogc(),
            ));
        }

        // 5. Let bufferByteLength be ArrayBufferByteLength(buffer, seq-cst).
        let buffer_byte_length = array_buffer_byte_length(agent, buffer, Ordering::SeqCst);

        // 6. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
                gc.nogc(),
            ));
        }

        // 7. Let bufferIsFixedLength be IsFixedLengthArrayBuffer(buffer).
        let buffer_is_fixed_length = is_fixed_length_array_buffer(agent, buffer);

        // 8. If byteLength is undefined, then
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
                    gc.nogc(),
                ));
            }
            Some(view_byte_length)
        };

        // 10. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%DataView.prototype%", « [[DataView]], [[ViewedArrayBuffer]], [[ByteLength]], [[ByteOffset]] »).
        let o = ordinary_create_from_constructor(
            agent,
            new_target,
            ProtoIntrinsics::DataView,
            gc.reborrow(),
        )?
        .unbind();

        let gc = gc.into_nogc();
        let o = o.bind(gc);
        let buffer = scoped_buffer.get(agent).bind(gc);
        // 11. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        if is_detached_buffer(agent, buffer) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempting to access detached ArrayBuffer",
                gc,
            ));
        }

        // 12. Set bufferByteLength to ArrayBufferByteLength(buffer, seq-cst).
        let buffer_byte_length = array_buffer_byte_length(agent, buffer, Ordering::SeqCst);

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

        let o = DataView::try_from(o).unwrap();

        let heap_data = &mut agent[o];

        // 15. Set O.[[ViewedArrayBuffer]] to buffer.
        heap_data.viewed_array_buffer = buffer.unbind();
        // 16. Set O.[[ByteLength]] to viewByteLength.
        let byte_length = view_byte_length.into();
        heap_data.byte_length = byte_length;
        // 17. Set O.[[ByteOffset]] to offset.
        let byte_offset = offset.into();
        heap_data.byte_offset = byte_offset;

        if byte_length == ViewedArrayBufferByteLength::heap() {
            agent
                .heap
                .data_view_byte_lengths
                .insert(o.unbind(), view_byte_length.unwrap());
        }

        if byte_offset == ViewedArrayBufferByteOffset::heap() {
            agent.heap.data_view_byte_offsets.insert(o.unbind(), offset);
        }

        // 18. Return O.
        Ok(o.into_value())
    }

    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let data_view_prototype = intrinsics.data_view_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<DataViewConstructor>(agent, realm)
            .with_property_capacity(1)
            .with_prototype_property(data_view_prototype.into_object())
            .build();
    }
}
