use crate::ecmascript::types::{OrdinaryObject, Value};

#[derive(Debug, Clone, Default)]
pub struct MapHeapData {
    pub(crate) object_index: Option<OrdinaryObject>,
    // TODO: This isn't even close to a hashmap; HashMap won't allow inserting
    // Value as key; f32 isn't hashable. And our f64s are found on the Heap and
    // require fetching; What we actually should do is more like:
    // pub(crate) map: HashMap<ValueHash, u32>
    // pub(crate) key_values: ParallelVec<Option<Value>, Option<Value>>
    // ValueHash is created using a Value.hash(agent) function and connects to
    // an index; the index points to a key and value in parallel vector / Vec2.
    // Note that empty slots are deleted values in the ParallelVec.
    pub(crate) keys: Vec<Value>,
    pub(crate) values: Vec<Value>,
    // TODO: When an non-terminal (start or end) iterator exists for the Map,
    // the items in the map cannot be compacted.
    // pub(crate) observed: bool;
}
