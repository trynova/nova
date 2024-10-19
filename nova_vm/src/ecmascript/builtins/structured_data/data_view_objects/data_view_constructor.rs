// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_index,
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{
            array_buffer::{
                array_buffer_byte_length, is_detached_buffer, is_fixed_length_array_buffer,
                Ordering,
            },
            data_view::{
                data::{DataViewByteLength, DataViewByteOffset},
                DataView,
            },
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
    const NAME: String = BUILTIN_STRING_MEMORY.DataView;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(DataViewConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for DataViewConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::DataView;
}

/// ### [25.3.2 The DataView Constructor](https://tc39.es/ecma262/#sec-dataview-constructor)
impl DataViewConstructor {
    /// ### [25.3.2.1 DataView ( buffer \[ , byteOffset \[ , byteLength \] \] )](https://tc39.es/ecma262/#sec-dataview-buffer-byteoffset-bytelength)
    fn behaviour(
        agent: &mut Agent,
        _this_value: Value,
        arguments: ArgumentsList,
        new_target: Option<Object>,
    ) -> JsResult<Value> {
        // 1. If NewTarget is undefined, throw a TypeError exception.
        let Some(new_target) = new_target else {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "calling a builtin DataView constructor without new is forbidden",
            ));
        };
        let new_target = Function::try_from(new_target).unwrap();

        let buffer = arguments.get(0);
        let byte_offset = arguments.get(1);
        let byte_length = arguments.get(2);

        // 2. Perform ? RequireInternalSlot(buffer, [[ArrayBufferData]]).
        let buffer = require_internal_slot_array_buffer(agent, buffer)?;

        // 3. Let offset be ? ToIndex(byteOffset).
        let offset = to_index(agent, byte_offset)? as usize;

        // 4. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        if is_detached_buffer(agent, buffer) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempting to access detached ArrayBuffer",
            ));
        }

        // 5. Let bufferByteLength be ArrayBufferByteLength(buffer, seq-cst).
        let buffer_byte_length = array_buffer_byte_length(agent, buffer, Ordering::SeqCst) as usize;

        // 6. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
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
            let view_byte_length = to_index(agent, byte_length)? as usize;
            // b. If offset + viewByteLength > bufferByteLength, throw a RangeError exception.
            if offset + view_byte_length > buffer_byte_length {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "offset is outside the bounds of the buffer",
                ));
            }
            Some(view_byte_length)
        };

        // 10. Let O be ? OrdinaryCreateFromConstructor(NewTarget, "%DataView.prototype%", « [[DataView]], [[ViewedArrayBuffer]], [[ByteLength]], [[ByteOffset]] »).
        let o = ordinary_create_from_constructor(agent, new_target, ProtoIntrinsics::DataView)?;

        // 11. If IsDetachedBuffer(buffer) is true, throw a TypeError exception.
        if is_detached_buffer(agent, buffer) {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::TypeError,
                "attempting to access detached ArrayBuffer",
            ));
        }

        // 12. Set bufferByteLength to ArrayBufferByteLength(buffer, seq-cst).
        let buffer_byte_length = array_buffer_byte_length(agent, buffer, Ordering::SeqCst) as usize;

        // 13. If offset > bufferByteLength, throw a RangeError exception.
        if offset > buffer_byte_length {
            return Err(agent.throw_exception_with_static_message(
                ExceptionType::RangeError,
                "offset is outside the bounds of the buffer",
            ));
        }

        // 14. If byteLength is not undefined, then
        if !byte_length.is_undefined() {
            // a. If offset + viewByteLength > bufferByteLength, throw a RangeError exception.
            if offset + view_byte_length.unwrap() > buffer_byte_length {
                return Err(agent.throw_exception_with_static_message(
                    ExceptionType::RangeError,
                    "offset is outside the bounds of the buffer",
                ));
            }
        }

        let o = DataView::try_from(o).unwrap();

        // 15. Set O.[[ViewedArrayBuffer]] to buffer.
        agent[o].viewed_array_buffer = buffer;
        // 16. Set O.[[ByteLength]] to viewByteLength.
        agent[o].byte_length = view_byte_length.into();
        // 17. Set O.[[ByteOffset]] to offset.
        agent[o].byte_offset = offset.into();

        if agent[o].byte_length == DataViewByteLength::heap() {
            agent
                .heap
                .data_view_byte_lengths
                .insert(o, view_byte_length.unwrap());
        }

        if agent[o].byte_offset == DataViewByteOffset::heap() {
            agent.heap.data_view_byte_offsets.insert(o, offset);
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
