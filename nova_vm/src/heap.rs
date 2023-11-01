mod array;
mod array_buffer;
mod bigint;
mod boolean;
mod date;
pub mod element_array;
mod error;
mod function;
mod heap_bits;
mod heap_constants;
mod heap_gc;
pub mod indexes;
mod math;
mod number;
mod object;
mod regexp;
mod string;
mod symbol;

pub(crate) use self::heap_constants::{BuiltinObjectIndexes, WellKnownSymbolIndexes};
use self::{
    array::initialize_array_heap,
    array_buffer::initialize_array_buffer_heap,
    bigint::initialize_bigint_heap,
    boolean::initialize_boolean_heap,
    date::{initialize_date_heap, DateHeapData},
    element_array::{
        ElementArray2Pow10, ElementArray2Pow12, ElementArray2Pow16, ElementArray2Pow24,
        ElementArray2Pow32, ElementArray2Pow4, ElementArray2Pow6, ElementArray2Pow8, ElementArrays,
        ElementsVector,
    },
    error::{initialize_error_heap, ErrorHeapData},
    function::initialize_function_heap,
    heap_constants::{
        FIRST_CONSTRUCTOR_INDEX, LAST_BUILTIN_OBJECT_INDEX, LAST_WELL_KNOWN_SYMBOL_INDEX,
    },
    indexes::{BaseIndex, FunctionIndex, NumberIndex, ObjectIndex, StringIndex},
    math::initialize_math_object,
    number::initialize_number_heap,
    object::{initialize_object_heap, ObjectEntry, PropertyDescriptor},
    regexp::{initialize_regexp_heap, RegExpHeapData},
    string::initialize_string_heap,
    symbol::{initialize_symbol_heap, SymbolHeapData},
};
use crate::ecmascript::{
    builtins::{ArrayBufferHeapData, ArrayHeapData},
    execution::{Environments, Realm, RealmIdentifier},
    types::{
        BigIntHeapData, Function, FunctionHeapData, Number, NumberHeapData, Object, ObjectHeapData,
        PropertyKey, String, StringHeapData, Value,
    },
};
use wtf8::{Wtf8, Wtf8Buf};

#[derive(Debug)]
pub struct Heap<'ctx, 'host> {
    pub realms: Vec<Option<Realm<'ctx, 'host>>>,
    pub environments: Environments,
    /// ElementsArrays is where all element arrays live;
    /// Element arrays are static arrays of Values plus
    /// a HashMap of possible property descriptors.
    pub elements: ElementArrays,
    pub arrays: Vec<Option<ArrayHeapData>>,
    pub array_buffers: Vec<Option<ArrayBufferHeapData>>,
    pub bigints: Vec<Option<BigIntHeapData>>,
    pub errors: Vec<Option<ErrorHeapData>>,
    pub functions: Vec<Option<FunctionHeapData>>,
    pub dates: Vec<Option<DateHeapData>>,
    pub globals: Vec<Value>,
    pub numbers: Vec<Option<NumberHeapData>>,
    pub objects: Vec<Option<ObjectHeapData>>,
    pub regexps: Vec<Option<RegExpHeapData>>,
    pub strings: Vec<Option<StringHeapData>>,
    pub symbols: Vec<Option<SymbolHeapData>>,
}

pub trait CreateHeapData<T, F> {
    /// Creates a [`Value`] from the given data. Allocating the data is **not**
    /// guaranteed.
    fn create(&mut self, data: T) -> F;
}

pub trait GetHeapData<'a, T, F: 'a> {
    fn get(&'a self, id: BaseIndex<T>) -> &'a F;
    fn get_mut(&'a mut self, id: BaseIndex<T>) -> &'a mut F;
}

impl CreateHeapData<f64, Number> for Heap<'_, '_> {
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

macro_rules! impl_heap_data {
    ($table: ident, $in: ty, $out: ty) => {
        impl<'a> GetHeapData<'a, $in, $out> for Heap<'_, '_> {
            fn get(&'a self, id: BaseIndex<$in>) -> &'a $out {
                self.$table.get(id.into_index()).unwrap().as_ref().unwrap()
            }

            fn get_mut(&'a mut self, id: BaseIndex<$in>) -> &'a mut $out {
                self.$table
                    .get_mut(id.into_index())
                    .unwrap()
                    .as_mut()
                    .unwrap()
            }
        }
    };
    ($table: ident, $in: ty, $out: ty, $accessor: ident) => {
        impl<'a> GetHeapData<'a, $in, $out> for Heap<'_, '_> {
            fn get(&'a self, id: BaseIndex<$in>) -> &'a $out {
                &self
                    .$table
                    .get(id.into_index())
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .$accessor
            }

            fn get_mut(&'a mut self, id: BaseIndex<$in>) -> &'a mut $out {
                &mut self
                    .$table
                    .get_mut(id.into_index())
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .$accessor
            }
        }
    };
}

impl_heap_data!(arrays, ArrayHeapData, ArrayHeapData);
impl_heap_data!(array_buffers, ArrayBufferHeapData, ArrayBufferHeapData);
impl_heap_data!(functions, FunctionHeapData, FunctionHeapData);
impl_heap_data!(numbers, NumberHeapData, f64, data);
impl_heap_data!(objects, ObjectHeapData, ObjectHeapData);
impl_heap_data!(strings, StringHeapData, Wtf8Buf, data);
impl_heap_data!(bigints, BigIntHeapData, BigIntHeapData);

impl CreateHeapData<&str, String> for Heap<'_, '_> {
    fn create(&mut self, data: &str) -> String {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            // SAFETY: String couldn't be represented as a SmallString.
            let id = unsafe { self.alloc_string(data) };
            Value::String(id).try_into().unwrap()
        }
    }
}

impl CreateHeapData<FunctionHeapData, Function> for Heap<'_, '_> {
    fn create(&mut self, data: FunctionHeapData) -> Function {
        self.functions.push(Some(data));
        Function::from(FunctionIndex::last(&self.functions))
    }
}

impl CreateHeapData<ObjectHeapData, Object> for Heap<'_, '_> {
    fn create(&mut self, data: ObjectHeapData) -> Object {
        self.objects.push(Some(data));
        Object::Object(ObjectIndex::last(&self.objects))
    }
}

impl<'ctx, 'host> Heap<'ctx, 'host> {
    pub fn new() -> Heap<'ctx, 'host> {
        let mut heap = Heap {
            realms: Vec::with_capacity(1),
            environments: Default::default(),
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
            arrays: Vec::with_capacity(1024),
            array_buffers: Vec::with_capacity(1024),
            bigints: Vec::with_capacity(1024),
            errors: Vec::with_capacity(1024),
            functions: Vec::with_capacity(1024),
            dates: Vec::with_capacity(1024),
            globals: Vec::with_capacity(1024),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            regexps: Vec::with_capacity(1024),
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
        };
        for _ in 0..LAST_WELL_KNOWN_SYMBOL_INDEX + 1 {
            // Initialize well known symbol slots
            heap.symbols.push(None);
        }
        for i in 0..LAST_BUILTIN_OBJECT_INDEX + 1 {
            // Initialize all static slots in heap objects.
            heap.objects.push(None);
            if i >= FIRST_CONSTRUCTOR_INDEX {
                heap.functions.push(None);
            }
        }
        initialize_array_heap(&mut heap);
        initialize_array_buffer_heap(&mut heap);
        initialize_bigint_heap(&mut heap);
        initialize_boolean_heap(&mut heap);
        initialize_date_heap(&mut heap);
        initialize_error_heap(&mut heap);
        initialize_function_heap(&mut heap);
        initialize_math_object(&mut heap);
        initialize_number_heap(&mut heap);
        initialize_object_heap(&mut heap);
        initialize_regexp_heap(&mut heap);
        initialize_string_heap(&mut heap);
        initialize_symbol_heap(&mut heap);

        heap
    }

    pub(crate) fn add_realm(&mut self, realm: Realm<'ctx, 'host>) -> RealmIdentifier<'ctx, 'host> {
        self.realms.push(Some(realm));
        RealmIdentifier::from_usize_index(self.realms.len())
    }

    pub fn get_realm(&self, id: RealmIdentifier<'ctx, 'host>) -> &Realm<'ctx, 'host> {
        self.realms
            .get(id.into_index())
            .expect("RealmIdentifier did not match a Realm")
            .as_ref()
            .expect("RealmIdentifier matched a freed Realm")
    }

    pub fn get_realm_mut(&mut self, id: RealmIdentifier<'ctx, 'host>) -> &mut Realm<'ctx, 'host> {
        self.realms
            .get_mut(id.into_index())
            .expect("RealmIdentifier did not match a Realm")
            .as_mut()
            .expect("RealmIdentifier matched a freed Realm")
    }

    /// Allocate a string onto the Agent heap
    ///
    /// This method will currently iterate through all heap strings to look for
    /// a possible matching string and if found will return its StringIndex
    /// instead of allocating a copy.
    ///
    /// SAFETY: The string being allocated must not be representable as a
    /// SmallString. All SmallStrings must be kept on the stack to ensure that
    /// comparison between heap allocated strings and SmallStrings can be
    /// guaranteed to never equal true.
    pub unsafe fn alloc_string(&mut self, message: &str) -> StringIndex {
        debug_assert!(message.len() > 7 || message.ends_with('\0'));
        let wtf8 = Wtf8::from_str(message);
        let found = self
            .strings
            .iter()
            .position(|opt| opt.as_ref().map_or(false, |data| data.data == wtf8));
        if let Some(idx) = found {
            return StringIndex::from_index(idx);
        }
        let data = StringHeapData::from_str(message);
        let found = self.strings.iter().position(|opt| opt.is_none());
        if let Some(idx) = found {
            self.strings[idx].replace(data);
            StringIndex::from_index(idx)
        } else {
            self.strings.push(Some(data));
            StringIndex::last(&self.strings)
        }
    }

    /// Allocate a 64-bit floating point number onto the Agent heap
    ///
    /// SAFETY: The number being allocated must not be representable
    /// as a SmallInteger or f32. All stack-allocated numbers must be
    /// inequal to any heap-allocated number.
    pub unsafe fn alloc_number(&mut self, number: f64) -> NumberIndex {
        debug_assert!(number.fract() != 0.0 || number as f32 as f64 != number);
        self.numbers.push(Some(number.into()));
        NumberIndex::last(&self.numbers)
    }

    pub fn create_function(
        &mut self,
        name: Value,
        length: u8,
        _uses_arguments: bool,
        // behaviour: Behaviour,
    ) -> FunctionIndex {
        let entries = vec![
            ObjectEntry::new(
                PropertyKey::from_str(self, "length"),
                PropertyDescriptor::roxh(Value::from(length)),
            ),
            ObjectEntry::new(
                PropertyKey::from_str(self, "name"),
                PropertyDescriptor::roxh(name),
            ),
        ];
        let (keys, values): (ElementsVector, ElementsVector) =
            self.elements.create_object_entries(entries);
        let func_object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Some(Object::Object(
                BuiltinObjectIndexes::FunctionPrototypeIndex.into(),
            )),
        };
        self.objects.push(Some(func_object_data));
        let func_data = FunctionHeapData {
            // behaviour,
            // bound: None,
            length,
            object_index: Some(ObjectIndex::last(&self.objects)),
            // uses_arguments,
            // visible: None,
            initial_name: Value::Null,
        };
        let index = FunctionIndex::from_index(self.functions.len());
        self.functions.push(Some(func_data));
        index
    }

    pub fn create_object(&mut self, entries: Vec<ObjectEntry>) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Some(Object::Object(
                BuiltinObjectIndexes::ObjectPrototypeIndex.into(),
            )),
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }

    pub fn create_null_object(&mut self, entries: Vec<ObjectEntry>) -> ObjectIndex {
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

    pub fn create_object_with_prototype(&mut self, prototype: Object) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(vec![]);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Some(prototype),
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }

    pub fn insert_builtin_object(
        &mut self,
        index: BuiltinObjectIndexes,
        extensible: bool,
        prototype: Option<Object>,
        entries: Vec<ObjectEntry>,
    ) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible,
            keys,
            values,
            prototype,
        };
        self.objects[index as usize] = Some(object_data);
        ObjectIndex::last(&self.objects)
    }
}

#[test]
fn init_heap() {
    let heap = Heap::new();
    assert!(heap.objects.len() >= LAST_BUILTIN_OBJECT_INDEX as usize);
    println!("{:#?}", heap);
}
