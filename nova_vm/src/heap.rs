mod array;
mod bigint;
mod boolean;
mod date;
mod element_array;
mod error;
mod function;
mod heap_bits;
mod heap_constants;
mod heap_gc;
pub(crate) mod indexes;
mod math;
mod number;
mod object;
mod regexp;
mod string;
mod symbol;

pub(crate) use self::heap_constants::BuiltinObjectIndexes;

use self::{
    array::{initialize_array_heap, ArrayHeapData},
    bigint::{initialize_bigint_heap, BigIntHeapData},
    boolean::initialize_boolean_heap,
    date::{initialize_date_heap, DateHeapData},
    element_array::{
        ElementArray2Pow10, ElementArray2Pow12, ElementArray2Pow16, ElementArray2Pow24,
        ElementArray2Pow32, ElementArray2Pow4, ElementArray2Pow6, ElementArray2Pow8, ElementArrays,
        ElementsVector,
    },
    error::{initialize_error_heap, ErrorHeapData},
    function::{initialize_function_heap, FunctionHeapData},
    heap_constants::{
        FIRST_CONSTRUCTOR_INDEX, LAST_BUILTIN_OBJECT_INDEX, LAST_WELL_KNOWN_SYMBOL_INDEX,
    },
    indexes::{BaseIndex, FunctionIndex, NumberIndex, ObjectIndex, StringIndex},
    math::initialize_math_object,
    number::{initialize_number_heap, NumberHeapData},
    object::{
        initialize_object_heap, ObjectEntry, ObjectHeapData, PropertyDescriptor, PropertyKey,
    },
    regexp::{initialize_regexp_heap, RegExpHeapData},
    string::{initialize_string_heap, StringHeapData},
    symbol::{initialize_symbol_heap, SymbolHeapData},
};
use crate::types::{Function, Number, Object, String, Value};
use wtf8::Wtf8;

#[derive(Debug)]
pub struct Heap {
    /// ElementsArrays is where all element arrays live;
    /// Element arrays are static arrays of Values plus
    /// a HashMap of possible property descriptors.
    pub(crate) elements: ElementArrays,
    pub(crate) arrays: Vec<Option<ArrayHeapData>>,
    pub(crate) bigints: Vec<Option<BigIntHeapData>>,
    pub(crate) errors: Vec<Option<ErrorHeapData>>,
    pub(crate) functions: Vec<Option<FunctionHeapData>>,
    pub(crate) dates: Vec<Option<DateHeapData>>,
    pub(crate) globals: Vec<Value>,
    pub(crate) numbers: Vec<Option<NumberHeapData>>,
    pub(crate) objects: Vec<Option<ObjectHeapData>>,
    pub(crate) regexps: Vec<Option<RegExpHeapData>>,
    pub(crate) strings: Vec<Option<StringHeapData>>,
    pub(crate) symbols: Vec<Option<SymbolHeapData>>,
}

pub trait CreateHeapData<T, F> {
    /// Creates a [`Value`] from the given data. Allocating the data is **not**
    /// guaranteed.
    fn create(&mut self, data: T) -> F;
}

pub trait GetHeapData<'a, T, F: 'a> {
    fn get(&'a self, handle: BaseIndex<T>) -> F;
}

impl CreateHeapData<f64, Number> for Heap {
    fn create(&mut self, data: f64) -> Number {
        if let Ok(value) = Value::try_from(data) {
            Number::new(value)
        } else if data as f32 as f64 == data {
            Number::new(Value::Float(data as f32))
        } else {
            let id = self.alloc_number(data);
            Value::Number(id).try_into().unwrap()
        }
    }
}

impl<'a> GetHeapData<'a, NumberHeapData, f64> for Heap {
    fn get(&'a self, id: NumberIndex) -> f64 {
        self.numbers
            .get(id.into_index())
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .data
    }
}

impl CreateHeapData<&str, String> for Heap {
    fn create(&mut self, data: &str) -> String {
        if let Ok(value) = String::try_from(data) {
            value
        } else {
            let id = self.alloc_string(data);
            Value::String(id).try_into().unwrap()
        }
    }
}

impl<'a> GetHeapData<'a, StringHeapData, &'a Wtf8> for Heap {
    fn get(&'a self, id: StringIndex) -> &'a Wtf8 {
        let data = self
            .strings
            .get(id.into_index())
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap();
        &data.data.slice(0, data.data.len())
    }
}

impl CreateHeapData<FunctionHeapData, Function> for Heap {
    fn create(&mut self, data: FunctionHeapData) -> Function {
        self.functions.push(Some(data));
        Function::new(Value::Function(FunctionIndex::last(&self.functions)))
    }
}

impl<'a> GetHeapData<'a, FunctionHeapData, &'a FunctionHeapData> for Heap {
    fn get(&'a self, id: FunctionIndex) -> &'a FunctionHeapData {
        self.functions
            .get(id.into_index())
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
    }
}

impl CreateHeapData<ObjectHeapData, Object> for Heap {
    fn create(&mut self, data: ObjectHeapData) -> Object {
        self.objects.push(Some(data));
        Object::new(Value::Object(ObjectIndex::last(&self.objects)))
    }
}

impl<'a> GetHeapData<'a, ObjectHeapData, &'a ObjectHeapData> for Heap {
    fn get(&'a self, id: ObjectIndex) -> &'a ObjectHeapData {
        self.objects
            .get(id.into_index())
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
    }
}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
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

    pub(crate) fn alloc_string(&mut self, message: &str) -> StringIndex {
        let found = self.strings.iter().position(|opt| {
            opt.as_ref()
                .map_or(false, |data| data.data == Wtf8::from_str(message))
        });
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

    pub(crate) fn alloc_number(&mut self, number: f64) -> NumberIndex {
        self.numbers.push(Some(NumberHeapData::new(number)));
        NumberIndex::last(&self.numbers)
    }

    pub(crate) fn create_function(
        &mut self,
        name: Value,
        length: u8,
        uses_arguments: bool,
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
            prototype: Value::Object(BuiltinObjectIndexes::FunctionPrototypeIndex.into()),
        };
        self.objects.push(Some(func_object_data));
        let func_data = FunctionHeapData {
            // behaviour,
            // bound: None,
            length,
            object_index: ObjectIndex::last(&self.objects),
            // uses_arguments,
            // visible: None,
            initial_name: Value::Null,
        };
        let index = FunctionIndex::from_index(self.functions.len());
        self.functions.push(Some(func_data));
        index
    }

    pub(crate) fn create_object(&mut self, entries: Vec<ObjectEntry>) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Value::Object(BuiltinObjectIndexes::ObjectPrototypeIndex.into()),
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }

    pub(crate) fn create_null_object(&mut self, entries: Vec<ObjectEntry>) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(entries);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype: Value::Null,
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }

    pub(crate) fn create_object_with_prototype(&mut self, prototype: Value) -> ObjectIndex {
        let (keys, values) = self.elements.create_object_entries(vec![]);
        let object_data = ObjectHeapData {
            extensible: true,
            keys,
            values,
            prototype,
        };
        self.objects.push(Some(object_data));
        ObjectIndex::last(&self.objects)
    }

    pub(crate) fn insert_builtin_object(
        &mut self,
        index: BuiltinObjectIndexes,
        extensible: bool,
        prototype: Value,
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
