pub mod element_array;
mod heap_bits;
mod heap_constants;
mod heap_gc;
pub mod indexes;
mod object_entry;
mod regexp;

pub(crate) use self::heap_constants::{
    intrinsic_function_count, intrinsic_object_count, IntrinsicConstructorIndexes,
    IntrinsicFunctionIndexes, IntrinsicObjectIndexes, WellKnownSymbolIndexes,
};
#[cfg(test)]
pub(crate) use self::heap_constants::{
    LAST_INTRINSIC_CONSTRUCTOR_INDEX, LAST_INTRINSIC_FUNCTION_INDEX, LAST_INTRINSIC_OBJECT_INDEX,
    LAST_WELL_KNOWN_SYMBOL_INDEX,
};
use self::indexes::{
    ArrayBufferIndex, ArrayIndex, DataViewIndex, DateIndex, ErrorIndex, FinalizationRegistryIndex,
    MapIndex, PrimitiveObjectIndex, PromiseIndex, RegExpIndex, SetIndex, SharedArrayBufferIndex,
    TypedArrayIndex, WeakMapIndex, WeakRefIndex, WeakSetIndex,
};
pub(crate) use self::object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor};
use self::{
    element_array::{
        ElementArray2Pow10, ElementArray2Pow12, ElementArray2Pow16, ElementArray2Pow24,
        ElementArray2Pow32, ElementArray2Pow4, ElementArray2Pow6, ElementArray2Pow8, ElementArrays,
    },
    indexes::{
        BigIntIndex, BoundFunctionIndex, BuiltinFunctionIndex, ECMAScriptFunctionIndex,
        NumberIndex, ObjectIndex, StringIndex,
    },
};
use crate::ecmascript::{
    builtins::{
        control_abstraction_objects::promise_objects::promise_abstract_operations::{promise_capability_records::PromiseCapabilityRecord, promise_reaction_records::PromiseReactionRecord, PromiseRejectFunctionHeapData}, data_view::{data::DataViewHeapData, DataView}, date::{data::DateHeapData, Date}, embedder_object::data::EmbedderObjectHeapData, error::{Error, ErrorHeapData}, finalization_registry::{data::FinalizationRegistryHeapData, FinalizationRegistry}, map::{data::MapHeapData, Map}, module::data::ModuleHeapData, primitive_objects::PrimitiveObjectHeapData, promise::data::PromiseHeapData, proxy::data::ProxyHeapData, regexp::RegExpHeapData, set::{data::SetHeapData, Set}, shared_array_buffer::{data::SharedArrayBufferHeapData, SharedArrayBuffer}, typed_array::{data::TypedArrayHeapData, TypedArray}, weak_map::{data::WeakMapHeapData, WeakMap}, weak_ref::{data::WeakRefHeapData, WeakRef}, weak_set::{data::WeakSetHeapData, WeakSet}, Array, ArrayBuffer
    },
    types::{AbstractClosureHeapData, BUILTIN_STRINGS_LIST},
};
use crate::ecmascript::{
    builtins::{ArrayBufferHeapData, ArrayHeapData, BuiltinFunction},
    execution::{Environments, Realm, RealmIdentifier},
    scripts_and_modules::{
        module::ModuleIdentifier,
        script::{Script, ScriptIdentifier},
    },
    types::{
        BigInt, BigIntHeapData, BoundFunctionHeapData, BuiltinFunctionHeapData,
        ECMAScriptFunctionHeapData, Function, Number, NumberHeapData, Object, ObjectHeapData,
        String, StringHeapData, SymbolHeapData, Value,
    },
};

#[derive(Debug)]
pub struct Heap {
    pub array_buffers: Vec<Option<ArrayBufferHeapData>>,
    pub arrays: Vec<Option<ArrayHeapData>>,
    pub bigints: Vec<Option<BigIntHeapData>>,
    pub bound_functions: Vec<Option<BoundFunctionHeapData>>,
    pub abstract_closures: Vec<Option<AbstractClosureHeapData>>,
    pub builtin_functions: Vec<Option<BuiltinFunctionHeapData>>,
    pub data_views: Vec<Option<DataViewHeapData>>,
    pub dates: Vec<Option<DateHeapData>>,
    pub ecmascript_functions: Vec<Option<ECMAScriptFunctionHeapData>>,
    /// ElementsArrays is where all element arrays live;
    /// Element arrays are static arrays of Values plus
    /// a HashMap of possible property descriptors.
    pub elements: ElementArrays,
    pub embedder_objects: Vec<Option<EmbedderObjectHeapData>>,
    pub environments: Environments,
    pub errors: Vec<Option<ErrorHeapData>>,
    pub finalization_registrys: Vec<Option<FinalizationRegistryHeapData>>,
    pub globals: Vec<Value>,
    pub maps: Vec<Option<MapHeapData>>,
    pub modules: Vec<Option<ModuleHeapData>>,
    pub numbers: Vec<Option<NumberHeapData>>,
    pub objects: Vec<Option<ObjectHeapData>>,
    pub primitive_objects: Vec<Option<PrimitiveObjectHeapData>>,
    pub promise_capability_records: Vec<Option<PromiseCapabilityRecord>>,
    pub promise_reaction_records: Vec<Option<PromiseReactionRecord>>,
    pub promise_reject_functions: Vec<Option<PromiseRejectFunctionHeapData>>,
    pub promises: Vec<Option<PromiseHeapData>>,
    pub proxys: Vec<Option<ProxyHeapData>>,
    pub realms: Vec<Option<Realm>>,
    pub regexps: Vec<Option<RegExpHeapData>>,
    pub scripts: Vec<Option<Script>>,
    pub sets: Vec<Option<SetHeapData>>,
    pub shared_array_buffers: Vec<Option<SharedArrayBufferHeapData>>,
    pub strings: Vec<Option<StringHeapData>>,
    pub symbols: Vec<Option<SymbolHeapData>>,
    pub typed_arrays: Vec<Option<TypedArrayHeapData>>,
    pub weak_maps: Vec<Option<WeakMapHeapData>>,
    pub weak_refs: Vec<Option<WeakRefHeapData>>,
    pub weak_sets: Vec<Option<WeakSetHeapData>>,
}

pub trait CreateHeapData<T, F> {
    /// Creates a [`Value`] from the given data. Allocating the data is **not**
    /// guaranteed.
    fn create(&mut self, data: T) -> F;
}

impl CreateHeapData<f64, Number> for Heap {
    fn create(&mut self, data: f64) -> Number {
        // NOTE: This function cannot currently be implemented
        // directly using `Number::from_f64` as it takes an Agent
        // parameter that we do not have access to here.
        if let Ok(value) = Number::try_from(data) {
            value
        } else {
            // SAFETY: Number was not representable as a
            // stack-allocated Number.
            let id = unsafe { self.alloc_number(data) };
            Number::Number(id)
        }
    }
}

impl CreateHeapData<&str, String> for Heap {
    fn create(&mut self, data: &str) -> String {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { self.alloc_str(data) }
        }
    }
}

impl CreateHeapData<std::string::String, String> for Heap {
    fn create(&mut self, data: std::string::String) -> String {
        if let Ok(value) = String::try_from(data.as_str()) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            unsafe { self.alloc_string(data) }
        }
    }
}

impl CreateHeapData<ArrayHeapData, Array> for Heap {
    fn create(&mut self, data: ArrayHeapData) -> Array {
        self.arrays.push(Some(data));
        Array::from(ArrayIndex::last(&self.arrays))
    }
}

impl CreateHeapData<ArrayBufferHeapData, ArrayBuffer> for Heap {
    fn create(&mut self, data: ArrayBufferHeapData) -> ArrayBuffer {
        self.array_buffers.push(Some(data));
        ArrayBuffer::from(ArrayBufferIndex::last(&self.array_buffers))
    }
}

impl CreateHeapData<BigIntHeapData, BigInt> for Heap {
    fn create(&mut self, data: BigIntHeapData) -> BigInt {
        self.bigints.push(Some(data));
        BigInt::BigInt(BigIntIndex::last(&self.bigints))
    }
}

impl CreateHeapData<DataViewHeapData, DataView> for Heap {
    fn create(&mut self, data: DataViewHeapData) -> DataView {
        self.data_views.push(Some(data));
        DataView::from(DataViewIndex::last(&self.data_views))
    }
}

impl CreateHeapData<DateHeapData, Date> for Heap {
    fn create(&mut self, data: DateHeapData) -> Date {
        self.dates.push(Some(data));
        Date::from(DateIndex::last(&self.dates))
    }
}

impl CreateHeapData<ErrorHeapData, Error> for Heap {
    fn create(&mut self, data: ErrorHeapData) -> Error {
        self.errors.push(Some(data));
        Error::from(ErrorIndex::last(&self.errors))
    }
}

impl CreateHeapData<FinalizationRegistryHeapData, FinalizationRegistry> for Heap {
    fn create(&mut self, data: FinalizationRegistryHeapData) -> FinalizationRegistry {
        self.finalization_registrys.push(Some(data));
        FinalizationRegistry(FinalizationRegistryIndex::last(
            &self.finalization_registrys,
        ))
    }
}

impl CreateHeapData<BoundFunctionHeapData, Function> for Heap {
    fn create(&mut self, data: BoundFunctionHeapData) -> Function {
        self.bound_functions.push(Some(data));
        Function::from(BoundFunctionIndex::last(&self.bound_functions))
    }
}

impl CreateHeapData<BuiltinFunctionHeapData, BuiltinFunction> for Heap {
    fn create(&mut self, data: BuiltinFunctionHeapData) -> BuiltinFunction {
        self.builtin_functions.push(Some(data));
        BuiltinFunctionIndex::last(&self.builtin_functions).into()
    }
}

impl CreateHeapData<ECMAScriptFunctionHeapData, Function> for Heap {
    fn create(&mut self, data: ECMAScriptFunctionHeapData) -> Function {
        self.ecmascript_functions.push(Some(data));
        Function::from(ECMAScriptFunctionIndex::last(&self.ecmascript_functions))
    }
}

impl CreateHeapData<MapHeapData, Map> for Heap {
    fn create(&mut self, data: MapHeapData) -> Map {
        self.maps.push(Some(data));
        Map(MapIndex::last(&self.maps))
    }
}

impl CreateHeapData<ObjectHeapData, Object> for Heap {
    fn create(&mut self, data: ObjectHeapData) -> Object {
        self.objects.push(Some(data));
        Object::Object(ObjectIndex::last(&self.objects))
    }
}

impl CreateHeapData<PrimitiveObjectHeapData, Object> for Heap {
    fn create(&mut self, data: PrimitiveObjectHeapData) -> Object {
        self.primitive_objects.push(Some(data));
        Object::PrimitiveObject(PrimitiveObjectIndex::last(&self.primitive_objects))
    }
}

impl CreateHeapData<PromiseHeapData, Object> for Heap {
    fn create(&mut self, data: PromiseHeapData) -> Object {
        self.promises.push(Some(data));
        Object::Promise(PromiseIndex::last(&self.promises))
    }
}

impl CreateHeapData<RegExpHeapData, Object> for Heap {
    fn create(&mut self, data: RegExpHeapData) -> Object {
        self.regexps.push(Some(data));
        Object::RegExp(RegExpIndex::last(&self.regexps))
    }
}

impl CreateHeapData<SetHeapData, Set> for Heap {
    fn create(&mut self, data: SetHeapData) -> Set {
        self.sets.push(Some(data));
        Set(SetIndex::last(&self.sets))
    }
}

impl CreateHeapData<SharedArrayBufferHeapData, SharedArrayBuffer> for Heap {
    fn create(&mut self, data: SharedArrayBufferHeapData) -> SharedArrayBuffer {
        self.shared_array_buffers.push(Some(data));
        SharedArrayBuffer(SharedArrayBufferIndex::last(&self.shared_array_buffers))
    }
}

impl CreateHeapData<TypedArrayHeapData, TypedArray> for Heap {
    fn create(&mut self, data: TypedArrayHeapData) -> TypedArray {
        self.typed_arrays.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        TypedArray::Uint8Array(TypedArrayIndex::last(&self.typed_arrays))
    }
}

impl CreateHeapData<WeakMapHeapData, WeakMap> for Heap {
    fn create(&mut self, data: WeakMapHeapData) -> WeakMap {
        self.weak_maps.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakMap(WeakMapIndex::last(&self.weak_maps))
    }
}

impl CreateHeapData<WeakRefHeapData, WeakRef> for Heap {
    fn create(&mut self, data: WeakRefHeapData) -> WeakRef {
        self.weak_refs.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakRef(WeakRefIndex::last(&self.weak_refs))
    }
}

impl CreateHeapData<WeakSetHeapData, WeakSet> for Heap {
    fn create(&mut self, data: WeakSetHeapData) -> WeakSet {
        self.weak_sets.push(Some(data));
        // TODO: The type should be checked based on data or something equally stupid
        WeakSet(WeakSetIndex::last(&self.weak_sets))
    }
}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
            abstract_closures: Vec::with_capacity(0),
            array_buffers: Vec::with_capacity(1024),
            arrays: Vec::with_capacity(1024),
            bigints: Vec::with_capacity(1024),
            bound_functions: Vec::with_capacity(256),
            builtin_functions: Vec::with_capacity(1024),
            data_views: Vec::with_capacity(0),
            dates: Vec::with_capacity(1024),
            ecmascript_functions: Vec::with_capacity(1024),
            elements: ElementArrays {
                e2pow4: ElementArray2Pow4::with_capacity(1024),
                e2pow6: ElementArray2Pow6::with_capacity(1024),
                e2pow8: ElementArray2Pow8::default(),
                e2pow10: ElementArray2Pow10::default(),
                e2pow12: ElementArray2Pow12::default(),
                e2pow16: ElementArray2Pow16::default(),
                e2pow24: ElementArray2Pow24::default(),
                e2pow32: ElementArray2Pow32::default(),
            },
            embedder_objects: Vec::with_capacity(0),
            environments: Default::default(),
            errors: Vec::with_capacity(1024),
            finalization_registrys: Vec::with_capacity(0),
            globals: Vec::with_capacity(1024),
            maps: Vec::with_capacity(128),
            modules: Vec::with_capacity(0),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            primitive_objects: Vec::with_capacity(0),
            promise_capability_records: Vec::with_capacity(0),
            promise_reaction_records: Vec::with_capacity(0),
            promise_reject_functions: Vec::with_capacity(0),
            promises: Vec::with_capacity(0),
            proxys: Vec::with_capacity(0),
            realms: Vec::with_capacity(1),
            regexps: Vec::with_capacity(1024),
            scripts: Vec::with_capacity(1),
            sets: Vec::with_capacity(128),
            shared_array_buffers: Vec::with_capacity(0),
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
            typed_arrays: Vec::with_capacity(0),
            weak_maps: Vec::with_capacity(0),
            weak_refs: Vec::with_capacity(0),
            weak_sets: Vec::with_capacity(0),
        };

        heap.strings.extend_from_slice(
            &BUILTIN_STRINGS_LIST
                .map(|builtin_string| Some(StringHeapData::from_static_str(builtin_string))),
        );

        heap
    }

    pub(crate) fn add_module(&mut self, module: ModuleHeapData) -> ModuleIdentifier {
        self.modules.push(Some(module));
        ModuleIdentifier::last(&self.modules)
    }

    pub(crate) fn add_realm(&mut self, realm: Realm) -> RealmIdentifier {
        self.realms.push(Some(realm));
        RealmIdentifier::last(&self.realms)
    }

    pub(crate) fn add_script(&mut self, script: Script) -> ScriptIdentifier {
        self.scripts.push(Some(script));
        ScriptIdentifier::last(&self.scripts)
    }

    /// Allocate a string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its StringIndex
    /// instead of allocating a copy.
    ///
    /// # Safety
    ///
    /// The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    pub(crate) unsafe fn alloc_str(&mut self, message: &str) -> String {
        let found = self.find_equal_string(message);
        if let Some(idx) = found {
            return idx;
        }
        let data = StringHeapData::from_str(message);
        self.strings.push(Some(data));
        StringIndex::last(&self.strings).into()
    }

    /// Allocate a static string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its StringIndex
    /// instead of allocating a copy.
    ///
    /// # Safety
    ///
    /// The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    unsafe fn alloc_string(&mut self, message: std::string::String) -> String {
        let found = self.find_equal_string(message.as_str());
        if let Some(idx) = found {
            return idx;
        }
        let data = StringHeapData::from_string(message);
        self.strings.push(Some(data));
        StringIndex::last(&self.strings).into()
    }

    /// Allocate a static string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its StringIndex
    /// instead of allocating a copy.
    ///
    /// # Safety
    ///
    /// The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    pub(crate) unsafe fn alloc_static_str(&mut self, message: &'static str) -> String {
        let found = self.find_equal_string(message);
        if let Some(idx) = found {
            return idx;
        }
        let data = StringHeapData::from_static_str(message);
        self.strings.push(Some(data));
        StringIndex::last(&self.strings).into()
    }

    fn find_equal_string(&self, message: &str) -> Option<String> {
        debug_assert!(message.len() > 7 || message.ends_with('\0'));
        self.strings
            .iter()
            .position(|opt| opt.as_ref().map_or(false, |data| data.as_str() == message))
            .map(|found_index| StringIndex::from_index(found_index).into())
    }

    /// Allocate a 64-bit floating point number onto the Agent heap
    ///
    /// # Safety
    ///
    /// The number being allocated must not be representable
    /// as a SmallInteger or f32. All stack-allocated numbers must be
    /// inequal to any heap-allocated number.
    pub unsafe fn alloc_number(&mut self, number: f64) -> NumberIndex {
        debug_assert!(number.fract() != 0.0 || number as f32 as f64 != number);
        self.numbers.push(Some(number.into()));
        NumberIndex::last(&self.numbers)
    }

    pub(crate) fn create_null_object(&mut self, entries: &[ObjectEntry]) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: None,
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }

    pub(crate) fn create_object_with_prototype(
        &mut self,
        prototype: Object,
        entries: &[ObjectEntry],
    ) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Some(prototype),
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }
}

impl Default for Heap {
    fn default() -> Self {
        Self::new()
    }
}

#[test]
fn init_heap() {
    let heap = Heap::new();
    println!("{:#?}", heap);
}
