use crate::{
    ecmascript::{
        abstract_operations::type_conversion::{to_big_int, to_index, to_number},
        builtins::{
            array_buffer::{
                array_buffer_byte_length, get_value_from_buffer, is_fixed_length_array_buffer,
                set_value_in_buffer, Ordering,
            },
            structured_data::data_view_objects::data_view_prototype::require_internal_slot_data_view,
        },
        execution::{agent::ExceptionType, Agent, JsResult},
        types::{IntoNumeric, Numeric, Value, Viewable},
    },
    engine::context::GcScope,
};

use super::DataView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ByteLength(pub usize);

impl ByteLength {
    pub const fn value(value: usize) -> Self {
        Self(value)
    }

    pub const fn detached() -> Self {
        Self(usize::MAX)
    }

    pub fn is_detached(&self) -> bool {
        *self == Self::detached()
    }
}

/// ### [25.3.1.1 DataView With Buffer Witness Records](https://tc39.es/ecma262/#sec-dataview-with-buffer-witness-records)
///
/// A DataView With Buffer Witness Record is a Record value used to encapsulate
/// a DataView along with a cached byte length of the viewed buffer. It is used
/// to help ensure there is a single shared memory read event of the byte
/// length data block when the viewed buffer is a growable SharedArrayBuffers.
#[derive(Debug, Clone)]
pub(crate) struct DataViewWithBufferWitnessRecord {
    /// ### [\[\[Object\]\]](https://tc39.es/ecma262/#table-dataview-with-buffer-witness-record-fields)
    object: DataView,
    /// ### [\[\[CachedBufferByteLength\]\]](https://tc39.es/ecma262/#table-dataview-with-buffer-witness-record-fields)
    cached_buffer_byte_length: ByteLength,
}

/// ### [25.3.1.2 MakeDataViewWithBufferWitnessRecord ( obj, order )](https://tc39.es/ecma262/#sec-makedataviewwithbufferwitnessrecord)
///
/// The abstract operation MakeDataViewWithBufferWitnessRecord takes arguments
/// obj (a DataView) and order (seq-cst or unordered) and returns a DataView
/// With Buffer Witness Record.
pub(crate) fn make_data_view_with_buffer_witness_record(
    agent: &Agent,
    obj: DataView,
    order: Ordering,
) -> DataViewWithBufferWitnessRecord {
    let buffer = obj.get_viewed_array_buffer(agent);
    let byte_length = if buffer.is_detached(agent) {
        ByteLength::detached()
    } else {
        ByteLength::value(array_buffer_byte_length(agent, buffer, order))
    };
    DataViewWithBufferWitnessRecord {
        object: obj,
        cached_buffer_byte_length: byte_length,
    }
}

/// ### [25.3.1.3 GetViewByteLength ( viewRecord )](https://tc39.es/ecma262/#sec-getviewbytelength)
///
/// The abstract operation GetViewByteLength takes argument viewRecord
/// (a DataView With Buffer Witness Record) and returns a non-negative integer.
pub(crate) fn get_view_byte_length(
    agent: &Agent,
    view_record: &DataViewWithBufferWitnessRecord,
) -> usize {
    // 1. Assert: IsViewOutOfBounds(viewRecord) is false.
    assert!(!is_view_out_of_bounds(agent, view_record));

    // 2. Let view be viewRecord.[[Object]].
    let view = view_record.object;

    // 3. If view.[[ByteLength]] is not auto, return view.[[ByteLength]].
    if let Some(byte_length) = view.byte_length(agent) {
        return byte_length;
    }

    // NOTE: This assert seems to not be guarding anything important, so it's
    // debug only. See https://github.com/trynova/nova/pull/447#discussion_r1805708906
    // 4. Assert: IsFixedLengthArrayBuffer(view.[[ViewedArrayBuffer]]) is false.
    debug_assert!(!is_fixed_length_array_buffer(
        agent,
        view.get_viewed_array_buffer(agent)
    ));

    // 5. Let byteOffset be view.[[ByteOffset]].
    let byte_offset = view.byte_offset(agent);

    // 6. Let byteLength be viewRecord.[[CachedBufferByteLength]].
    // 7. Assert: byteLength is not detached.
    assert!(!view_record.cached_buffer_byte_length.is_detached());
    let byte_length = view_record.cached_buffer_byte_length.0;

    // 8. Return byteLength - byteOffset.
    byte_length - byte_offset
}

/// ### [25.3.1.4 IsViewOutOfBounds ( viewRecord )](https://tc39.es/ecma262/#sec-isviewoutofbounds)
///
/// The abstract operation IsViewOutOfBounds takes argument viewRecord
/// (a DataView With Buffer Witness Record) and returns a Boolean.
pub(crate) fn is_view_out_of_bounds(
    agent: &Agent,
    view_record: &DataViewWithBufferWitnessRecord,
) -> bool {
    // 1. Let view be viewRecord.[[Object]].
    let view = view_record.object;
    let ab = view.get_viewed_array_buffer(agent);

    // 2. Let bufferByteLength be viewRecord.[[CachedBufferByteLength]].
    let buffer_byte_length = view_record.cached_buffer_byte_length;

    // 3. Assert: IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is true if and only if bufferByteLength is detached.
    assert!(ab.is_detached(agent) == buffer_byte_length.is_detached());

    // 4. If bufferByteLength is detached, return true.
    if buffer_byte_length.is_detached() {
        return true;
    }
    let buffer_byte_length = buffer_byte_length.0;

    // 5. Let byteOffsetStart be view.[[ByteOffset]].
    let byte_offset_start = view.byte_offset(agent);

    // 6. If view.[[ByteLength]] is auto, then
    let byte_offset_end = if let Some(byte_length) = view.byte_length(agent) {
        // 7. Else,
        // a. Let byteOffsetEnd be byteOffsetStart + view.[[ByteLength]].
        byte_offset_start + byte_length
    } else {
        // a. Let byteOffsetEnd be bufferByteLength.
        buffer_byte_length
    };

    // 8. If byteOffsetStart > bufferByteLength or byteOffsetEnd > bufferByteLength, return true.
    if byte_offset_start > buffer_byte_length || byte_offset_end > buffer_byte_length {
        return true;
    }

    // 9. NOTE: 0-length DataViews are not considered out-of-bounds.
    // 10. Return false.
    false
}

/// ### [25.3.1.5 GetViewValue ( view, requestIndex, isLittleEndian, type )](https://tc39.es/ecma262/#sec-getviewvalue)
///
/// The abstract operation GetViewValue takes arguments view (an ECMAScript
/// language value), requestIndex (an ECMAScript language value), isLittleEndian
/// (an ECMAScript language value), and type (a TypedArray element type) and
/// returns either a normal completion containing either a Number or a BigInt,
/// or a throw completion. It is used by functions on DataView instances to
/// retrieve values from the view's buffer.
pub(crate) fn get_view_value<'gc, T: Viewable>(
    agent: &mut Agent,
    view: Value,
    request_index: Value,
    // 4. Set isLittleEndian to ToBoolean(isLittleEndian).
    is_little_endian: bool,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<Numeric<'gc>> {
    // 1. Perform ? RequireInternalSlot(view, [[DataView]]).
    // 2. Assert: view has a [[ViewedArrayBuffer]] internal slot.
    let view = require_internal_slot_data_view(agent, view, gc.nogc())?;

    // 3. Let getIndex be ? ToIndex(requestIndex).
    let get_index = to_index(agent, request_index, gc.reborrow())? as usize;
    // No GC is possible beyond this point.
    let gc = gc.into_nogc();
    // 5. Let viewOffset be view.[[ByteOffset]].
    let view_offset = view.byte_offset(agent);

    // 6. Let viewRecord be MakeDataViewWithBufferWitnessRecord(view, unordered).
    let view_record = make_data_view_with_buffer_witness_record(agent, view, Ordering::Unordered);

    // 7. NOTE: Bounds checking is not a synchronizing operation when view's backing buffer is a growable SharedArrayBuffer.
    // 8. If IsViewOutOfBounds(viewRecord) is true, throw a TypeError exception.
    if is_view_out_of_bounds(agent, &view_record) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "DataView is out of bounds",
            gc,
        ));
    }

    // 9. Let viewSize be GetViewByteLength(viewRecord).
    let view_size = get_view_byte_length(agent, &view_record);

    // 10. Let elementSize be the Element Size value specified in Table 69 for Element Type type.
    let element_size = size_of::<T>();

    // 11. If getIndex + elementSize > viewSize, throw a RangeError exception.
    if get_index + element_size > view_size {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Index out of bounds",
            gc,
        ));
    }

    // 12. Let bufferIndex be getIndex + viewOffset.
    let buffer_index = get_index + view_offset;

    // 13. Return GetValueFromBuffer(view.[[ViewedArrayBuffer]], bufferIndex, type, false, unordered, isLittleEndian).
    Ok(get_value_from_buffer::<T>(
        agent,
        view.get_viewed_array_buffer(agent),
        buffer_index,
        false,
        Ordering::Unordered,
        Some(is_little_endian),
        gc,
    ))
}

/// ### [25.3.1.6 SetViewValue ( view, requestIndex, isLittleEndian, type, value )](https://tc39.es/ecma262/#sec-setviewvalue)
///
/// The abstract operation SetViewValue takes arguments view (an ECMAScript
/// language value), requestIndex (an ECMAScript language value), isLittleEndian
/// (an ECMAScript language value), type (a TypedArray element type), and value
/// (an ECMAScript language value) and returns either a normal completion
/// containing undefined or a throw completion. It is used by functions on
/// DataView instances to store values into the view's buffer.
pub(crate) fn set_view_value<T: Viewable>(
    agent: &mut Agent,
    view: Value,
    request_index: Value,
    // 6. Set isLittleEndian to ToBoolean(isLittleEndian).
    is_little_endian: bool,
    value: Value,
    mut gc: GcScope<'_, '_>,
) -> JsResult<Value> {
    // 1. Perform ? RequireInternalSlot(view, [[DataView]]).
    // 2. Assert: view has a [[ViewedArrayBuffer]] internal slot.
    let view = require_internal_slot_data_view(agent, view, gc.nogc())?;

    // 3. Let getIndex be ? ToIndex(requestIndex).
    let get_index = to_index(agent, request_index, gc.reborrow())? as usize;

    // 4. If IsBigIntElementType(type) is true, let numberValue be ? ToBigInt(value).
    let number_value = if T::IS_BIGINT {
        to_big_int(agent, value, gc.reborrow())?.into_numeric()
    } else {
        // 5. Otherwise, let numberValue be ? ToNumber(value).
        to_number(agent, value, gc.reborrow())?.into_numeric()
    };

    // 7. Let viewOffset be view.[[ByteOffset]].
    let view_offset = view.byte_offset(agent);

    // 8. Let viewRecord be MakeDataViewWithBufferWitnessRecord(view, unordered).
    let view_record = make_data_view_with_buffer_witness_record(agent, view, Ordering::Unordered);

    // 9. NOTE: Bounds checking is not a synchronizing operation when view's backing buffer is a growable SharedArrayBuffer.
    // 10. If IsViewOutOfBounds(viewRecord) is true, throw a TypeError exception.
    if is_view_out_of_bounds(agent, &view_record) {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "DataView is out of bounds",
            gc.nogc(),
        ));
    }

    // 11. Let viewSize be GetViewByteLength(viewRecord).
    let view_size = get_view_byte_length(agent, &view_record);

    // 12. Let elementSize be the Element Size value specified in Table 69 for Element Type type.
    let element_size = size_of::<T>();
    // 13. If getIndex + elementSize > viewSize, throw a RangeError exception.
    if get_index + element_size > view_size {
        return Err(agent.throw_exception_with_static_message(
            ExceptionType::RangeError,
            "Index out of bounds",
            gc.nogc(),
        ));
    }

    // 14. Let bufferIndex be getIndex + viewOffset.
    let buffer_index = get_index + view_offset;

    // 15. Perform SetValueInBuffer(view.[[ViewedArrayBuffer]], bufferIndex, type, numberValue, false, unordered, isLittleEndian).
    set_value_in_buffer::<T>(
        agent,
        view.get_viewed_array_buffer(agent),
        buffer_index,
        number_value,
        false,
        Ordering::Unordered,
        Some(is_little_endian),
    );

    // 16. Return undefined.
    Ok(Value::Undefined)
}
