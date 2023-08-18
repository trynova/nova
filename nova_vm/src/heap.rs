mod array;
mod bigint;
mod boolean;
mod date;
mod error;
mod function;
mod heap_constants;
mod math;
mod number;
mod object;
mod regexp;
mod string;
mod symbol;

use self::{
    array::{initialize_array_heap, ArrayHeapData},
    bigint::{initialize_bigint_heap, BigIntHeapData},
    boolean::initialize_boolean_heap,
    date::{initialize_date_heap, DateHeapData},
    error::{initialize_error_heap, ErrorHeapData},
    function::{initialize_function_heap, FunctionHeapData, JsBindingFunction},
    heap_constants::{
        BuiltinObjectIndexes, FIRST_CONSTRUCTOR_INDEX, LAST_BUILTIN_OBJECT_INDEX,
        LAST_WELL_KNOWN_SYMBOL_INDEX,
    },
    math::initialize_math_object,
    number::{initialize_number_heap, NumberHeapData},
    object::{
        initialize_object_heap, ObjectEntry, ObjectHeapData, PropertyDescriptor, PropertyKey,
    },
    regexp::{initialize_regexp_heap, RegExpHeapData},
    string::{initialize_string_heap, StringHeapData},
    symbol::{initialize_symbol_heap, SymbolHeapData},
};
use crate::value::Value;
use wtf8::Wtf8;

#[derive(Debug)]
pub(crate) enum ElementArrayKey {
    /// up to 16 elements
    E4,
    /// up to 64 elements
    E6,
    /// up to 256 elements
    E8,
    /// up to 1024 elements
    E10,
    /// up to 4096 elements
    E12,
    /// up to 65536 elements
    E16,
    /// up to 16777216 elements
    E24,
    /// up to 4294967296 elements
    E32,
}

#[derive(Debug)]
pub(crate) struct ElementsVector {
    elements_index: u32,
    cap: ElementArrayKey,
    len: u32,
}

pub(crate) struct ElementArrays {
    /// up to 16 elements
    e_2_4: Vec<[Option<Value>; usize::pow(2, 4)]>,
    /// up to 64 elements
    e_2_6: Vec<[Option<Value>; usize::pow(2, 6)]>,
    /// up to 256 elements
    e_2_8: Vec<[Option<Value>; usize::pow(2, 8)]>,
    /// up to 1024 elements
    e_2_10: Vec<[Option<Value>; usize::pow(2, 10)]>,
    /// up to 4096 elements
    e_2_12: Vec<[Option<Value>; usize::pow(2, 12)]>,
    /// up to 65536 elements
    e_2_16: Vec<[Option<Value>; usize::pow(2, 16)]>,
    /// up to 16777216 elements
    e_2_24: Vec<[Option<Value>; usize::pow(2, 24)]>,
    /// up to 4294967296 elements
    e_2_32: Vec<[Option<Value>; usize::pow(2, 32)]>,
}

#[derive(Debug)]
pub struct Heap {
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

fn stop_the_world() {}
fn start_the_world() {}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
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
        initialize_object_heap(&mut heap);
        initialize_function_heap(&mut heap);
        initialize_boolean_heap(&mut heap);
        initialize_symbol_heap(&mut heap);
        initialize_error_heap(&mut heap);
        initialize_number_heap(&mut heap);
        initialize_bigint_heap(&mut heap);
        initialize_math_object(&mut heap);
        initialize_date_heap(&mut heap);
        initialize_string_heap(&mut heap);
        initialize_regexp_heap(&mut heap);
        initialize_array_heap(&mut heap);
        // initialize_typedarray_heap(&mut heap);
        // initialize_map_heap(&mut heap);
        // initialize_set_heap(&mut heap);
        // initialize_weak_map_heap(&mut heap);
        // initialize_weak_set_heap(&mut heap);
        // initialize_array_buffer_heap(&mut heap);
        // initialize_shared_array_buffer_heap(&mut heap);
        // initialize_data_view_heap(&mut heap);
        // initialize_json_heap(&mut heap);
        // initialize_atomics_heap(&mut heap);
        // initialize_weak_ref_heap(&mut heap);
        // initialize_finalization_registry_heap(&mut heap);
        // initialize_iterator_heap(&mut heap);
        // initialize_async_iterator_heap(&mut heap);
        // initialize_promise_heap(&mut heap);
        // initialize_generator_function_heap(&mut heap);
        // initialize_async_generator_function_heap(&mut heap);
        // initialize_generator_heap(&mut heap);
        // initialize_async_generator_heap(&mut heap);
        // initialize_async_function_heap(&mut heap);
        // initialize_reflect_heap(&mut heap);
        // initialize_proxy_heap(&mut heap);
        // initialize_module_heap(&mut heap);

        heap
    }

    pub(crate) fn alloc_string(&mut self, message: &str) -> u32 {
        let found = self.strings.iter().position(|opt| {
            opt.as_ref()
                .map_or(false, |data| data.data == Wtf8::from_str(message))
        });
        if let Some(idx) = found {
            return idx as u32;
        }
        let data = StringHeapData::from_str(message);
        let found = self.strings.iter().position(|opt| opt.is_none());
        if let Some(idx) = found {
            self.strings[idx].replace(data);
            idx as u32
        } else {
            self.strings.push(Some(data));
            self.strings.len() as u32
        }
    }

    pub(crate) fn alloc_number(&mut self, number: f64) -> u32 {
        self.numbers.push(Some(NumberHeapData::new(number)));
        self.numbers.len() as u32
    }

    pub(crate) fn create_function(
        &mut self,
        name: Value,
        length: u8,
        uses_arguments: bool,
        binding: JsBindingFunction,
    ) -> u32 {
        let func_object_data = ObjectHeapData {
            _extensible: true,
            entries: vec![
                ObjectEntry::new(
                    PropertyKey::from_str(self, "length"),
                    PropertyDescriptor::roxh(Value::SmiU(length as u32)),
                ),
                ObjectEntry::new(
                    PropertyKey::from_str(self, "name"),
                    PropertyDescriptor::roxh(name),
                ),
            ],
            prototype: PropertyDescriptor::roh(Value::Object(
                BuiltinObjectIndexes::FunctionPrototypeIndex as u32,
            )),
        };
        self.objects.push(Some(func_object_data));
        let func_data = FunctionHeapData {
            binding,
            bound: None,
            length,
            object_index: self.objects.len() as u32,
            uses_arguments,
            visible: None,
        };
        self.functions.push(Some(func_data));
        self.functions.len() as u32
    }

    pub(crate) fn create_object(&mut self, entries: Vec<ObjectEntry>) -> u32 {
        let object_data = ObjectHeapData {
            _extensible: true,
            entries,
            prototype: PropertyDescriptor::roh(Value::Object(
                BuiltinObjectIndexes::ObjectPrototypeIndex as u32,
            )),
        };
        self.objects.push(Some(object_data));
        self.objects.len() as u32
    }

    pub(crate) fn create_null_object(&mut self, entries: Vec<ObjectEntry>) -> u32 {
        let object_data = ObjectHeapData {
            _extensible: true,
            entries,
            prototype: PropertyDescriptor::roh(Value::Null),
        };
        self.objects.push(Some(object_data));
        self.objects.len() as u32
    }
}

#[test]
fn init_heap() {
    let heap = Heap::new();
    assert!(heap.objects.len() >= LAST_BUILTIN_OBJECT_INDEX as usize);
    println!("{:#?}", heap);
}
