use crate::ecmascript::{
    builtins::array_buffer::{array_buffer_byte_length, is_fixed_length_array_buffer, Ordering},
    execution::Agent,
};

use super::DataView;

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
    cached_buffer_byte_length: Option<usize>,
}

/// [25.3.1.2 MakeDataViewWithBufferWitnessRecord ( obj, order )](https://tc39.es/ecma262/#sec-makedataviewwithbufferwitnessrecord)
///
/// The abstract operation MakeDataViewWithBufferWitnessRecord takes arguments
/// obj (a DataView) and order (seq-cst or unordered) and returns a DataView
/// With Buffer Witness Record.
pub(crate) fn make_data_view_with_buffer_witness_record(
    agent: &mut Agent,
    obj: DataView,
    order: Ordering,
) -> DataViewWithBufferWitnessRecord {
    let buffer = obj.get_viewed_array_buffer(agent);
    let byte_length = if buffer.is_detached(agent) {
        None
    } else {
        Some(array_buffer_byte_length(agent, buffer, order) as usize)
    };
    DataViewWithBufferWitnessRecord {
        object: obj,
        cached_buffer_byte_length: byte_length,
    }
}

/// [25.3.1.3 GetViewByteLength ( viewRecord )](https://tc39.es/ecma262/#sec-getviewbytelength)
///
/// The abstract operation GetViewByteLength takes argument viewRecord
/// (a DataView With Buffer Witness Record) and returns a non-negative integer.
pub(crate) fn get_view_byte_length(
    agent: &mut Agent,
    view_record: &DataViewWithBufferWitnessRecord,
) -> i64 {
    // 1. Assert: IsViewOutOfBounds(viewRecord) is false.
    assert!(!is_view_out_of_bounds(agent, view_record));

    // 2. Let view be viewRecord.[[Object]].
    let view = view_record.object;

    // 3. If view.[[ByteLength]] is not auto, return view.[[ByteLength]].
    if let Some(byte_length) = view.byte_length(agent) {
        return byte_length as i64;
    }

    // 4. Assert: IsFixedLengthArrayBuffer(view.[[ViewedArrayBuffer]]) is false.
    assert!(!is_fixed_length_array_buffer(
        agent,
        view.get_viewed_array_buffer(agent)
    ));

    // 5. Let byteOffset be view.[[ByteOffset]].
    let byte_offset = view.byte_offset(agent);

    // 6. Let byteLength be viewRecord.[[CachedBufferByteLength]].
    // 7. Assert: byteLength is not detached.
    let Some(byte_length) = view_record.cached_buffer_byte_length else {
        unreachable!();
    };

    // 8. Return byteLength - byteOffset.
    (byte_length - byte_offset) as i64
}

/// [25.3.1.4 IsViewOutOfBounds ( viewRecord )](https://tc39.es/ecma262/#sec-isviewoutofbounds)
///
/// The abstract operation IsViewOutOfBounds takes argument viewRecord
/// (a DataView With Buffer Witness Record) and returns a Boolean.
pub(crate) fn is_view_out_of_bounds(
    agent: &mut Agent,
    view_record: &DataViewWithBufferWitnessRecord,
) -> bool {
    // 1. Let view be viewRecord.[[Object]].
    let view = view_record.object;
    let ab = view.get_viewed_array_buffer(agent);

    // 2. Let bufferByteLength be viewRecord.[[CachedBufferByteLength]].
    let buffer_byte_length = view_record.cached_buffer_byte_length;

    // 3. Assert: IsDetachedBuffer(view.[[ViewedArrayBuffer]]) is true if and only if bufferByteLength is detached.
    assert!(ab.is_detached(agent) == buffer_byte_length.is_none());

    // 4. If bufferByteLength is detached, return true.
    if buffer_byte_length.is_none() {
        return true;
    }

    // 5. Let byteOffsetStart be view.[[ByteOffset]].
    let byte_offset_start = view.byte_offset(agent);

    // 6. If view.[[ByteLength]] is auto, then
    let byte_offset_end = if let Some(byte_length) = view.byte_length(agent) {
        // 7. Else,
        // a. Let byteOffsetEnd be byteOffsetStart + view.[[ByteLength]].
        byte_offset_start + byte_length
    } else {
        // a. Let byteOffsetEnd be bufferByteLength.
        buffer_byte_length.unwrap()
    };

    // 8. If byteOffsetStart > bufferByteLength or byteOffsetEnd > bufferByteLength, return true.
    if byte_offset_start > buffer_byte_length.unwrap()
        || byte_offset_end > buffer_byte_length.unwrap()
    {
        return true;
    }

    // 9. NOTE: 0-length DataViews are not considered out-of-bounds.
    // 10. Return false.
    false
}
