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
use self::indexes::{DateIndex, ErrorIndex};
pub(crate) use self::object_entry::{ObjectEntry, ObjectEntryPropertyDescriptor};
use self::{
    element_array::{
        ElementArray2Pow10, ElementArray2Pow12, ElementArray2Pow16, ElementArray2Pow24,
        ElementArray2Pow32, ElementArray2Pow4, ElementArray2Pow6, ElementArray2Pow8, ElementArrays,
    },
    indexes::{
        BaseIndex, BigIntIndex, BoundFunctionIndex, BuiltinFunctionIndex, ECMAScriptFunctionIndex,
        NumberIndex, ObjectIndex, StringIndex,
    },
};
use crate::ecmascript::{
    builtins::{
        date::{data::DateHeapData, Date},
        error::{Error, ErrorHeapData},
        regexp::RegExpHeapData,
    },
    types::BUILTIN_STRINGS_LIST,
};
use crate::ecmascript::{
    builtins::{ArrayBufferHeapData, ArrayHeapData, BuiltinFunction},
    execution::{Environments, Realm, RealmIdentifier},
    scripts_and_modules::{
        module::{Module, ModuleIdentifier},
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
    pub modules: Vec<Option<Module>>,
    pub realms: Vec<Option<Realm>>,
    pub scripts: Vec<Option<Script>>,
    pub environments: Environments,
    /// ElementsArrays is where all element arrays live;
    /// Element arrays are static arrays of Values plus
    /// a HashMap of possible property descriptors.
    pub elements: ElementArrays,
    pub arrays: Vec<Option<ArrayHeapData>>,
    pub array_buffers: Vec<Option<ArrayBufferHeapData>>,
    pub bigints: Vec<Option<BigIntHeapData>>,
    pub errors: Vec<Option<ErrorHeapData>>,
    pub bound_functions: Vec<Option<BoundFunctionHeapData>>,
    pub builtin_functions: Vec<Option<BuiltinFunctionHeapData>>,
    pub ecmascript_functions: Vec<Option<ECMAScriptFunctionHeapData>>,
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

macro_rules! impl_heap_data {
    ($table: ident, $in: ty, $out: ty) => {
        impl<'a> GetHeapData<'a, $in, $out> for Heap {
            fn get(&'a self, id: BaseIndex<$in>) -> &'a $out {
                self.$table
                    .get(id.into_index())
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex for Heap::get ({id:?}): Index is out of bounds");
                    })
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex for Heap::get ({id:?}): No item at index");
                    })
            }

            fn get_mut(&'a mut self, id: BaseIndex<$in>) -> &'a mut $out {
                self.$table
                    .get_mut(id.into_index())
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex Heap::get_mut ({id:?}): Index is out of bounds");
                    })
                    .as_mut()
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex Heap::get_mut ({id:?}): No item at index");
                    })
            }
        }
    };
    ($table: ident, $in: ty, $out: ty, $accessor: ident) => {
        impl<'a> GetHeapData<'a, $in, $out> for Heap {
            fn get(&'a self, id: BaseIndex<$in>) -> &'a $out {
                &self
                    .$table
                    .get(id.into_index())
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex Heap::get ({id:?}): Index is out of bounds")
                    })
                    .as_ref()
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex Heap::get ({id:?}): No item at index")
                    })
                    .$accessor
            }

            fn get_mut(&'a mut self, id: BaseIndex<$in>) -> &'a mut $out {
                &mut self
                    .$table
                    .get_mut(id.into_index())
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex Heap::get_mut ({id:?}): Index is out of bounds",)
                    })
                    .as_mut()
                    .unwrap_or_else(|| {
                        panic!("Invalid HeapIndex Heap::get_mut ({id:?}): No item at index",)
                    })
                    .$accessor
            }
        }
    };
}

impl_heap_data!(arrays, ArrayHeapData, ArrayHeapData);
impl_heap_data!(array_buffers, ArrayBufferHeapData, ArrayBufferHeapData);
impl_heap_data!(dates, DateHeapData, DateHeapData);
impl_heap_data!(errors, ErrorHeapData, ErrorHeapData);
impl_heap_data!(
    bound_functions,
    BoundFunctionHeapData,
    BoundFunctionHeapData
);
impl_heap_data!(
    builtin_functions,
    BuiltinFunctionHeapData,
    BuiltinFunctionHeapData
);
impl_heap_data!(
    ecmascript_functions,
    ECMAScriptFunctionHeapData,
    ECMAScriptFunctionHeapData
);
impl_heap_data!(numbers, NumberHeapData, f64, data);
impl_heap_data!(objects, ObjectHeapData, ObjectHeapData);
impl_heap_data!(strings, StringHeapData, StringHeapData);
impl_heap_data!(symbols, SymbolHeapData, SymbolHeapData);
impl_heap_data!(bigints, BigIntHeapData, BigIntHeapData);

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

impl CreateHeapData<ObjectHeapData, Object> for Heap {
    fn create(&mut self, data: ObjectHeapData) -> Object {
        self.objects.push(Some(data));
        Object::Object(ObjectIndex::last(&self.objects))
    }
}

impl CreateHeapData<BigIntHeapData, BigInt> for Heap {
    fn create(&mut self, data: BigIntHeapData) -> BigInt {
        self.bigints.push(Some(data));
        BigInt::BigInt(BigIntIndex::last(&self.bigints))
    }
}

impl Heap {
    pub fn new() -> Heap {
        let mut heap = Heap {
            modules: vec![],
            realms: Vec::with_capacity(1),
            scripts: Vec::with_capacity(1),
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
            bound_functions: Vec::with_capacity(256),
            builtin_functions: Vec::with_capacity(1024),
            ecmascript_functions: Vec::with_capacity(1024),
            dates: Vec::with_capacity(1024),
            globals: Vec::with_capacity(1024),
            numbers: Vec::with_capacity(1024),
            objects: Vec::with_capacity(1024),
            regexps: Vec::with_capacity(1024),
            strings: Vec::with_capacity(1024),
            symbols: Vec::with_capacity(1024),
        };

        heap.strings.extend_from_slice(
            &BUILTIN_STRINGS_LIST
                .map(|builtin_string| Some(StringHeapData::from_static_str(builtin_string))),
        );

        heap
    }

    pub(crate) fn add_module(&mut self, module: Module) -> ModuleIdentifier {
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

    pub(crate) fn get_module(&self, id: ModuleIdentifier) -> &Module {
        self.modules
            .get(id.into_index())
            .expect("ModuleIdentifier did not match a Module")
            .as_ref()
            .expect("ModuleIdentifier matched a freed Module")
    }

    pub(crate) fn get_module_mut(&mut self, id: ModuleIdentifier) -> &mut Module {
        self.modules
            .get_mut(id.into_index())
            .expect("ModuleIdentifier did not match a Module")
            .as_mut()
            .expect("ModuleIdentifier matched a freed Module")
    }

    pub(crate) fn get_realm(&self, id: RealmIdentifier) -> &Realm {
        self.realms
            .get(id.into_index())
            .expect("RealmIdentifier did not match a Realm")
            .as_ref()
            .expect("RealmIdentifier matched a freed Realm")
    }

    pub(crate) fn get_realm_mut(&mut self, id: RealmIdentifier) -> &mut Realm {
        self.realms
            .get_mut(id.into_index())
            .expect("RealmIdentifier did not match a Realm")
            .as_mut()
            .expect("RealmIdentifier matched a freed Realm")
    }

    pub(crate) fn get_script(&self, id: ScriptIdentifier) -> &Script {
        self.scripts
            .get(id.into_index())
            .expect("ScriptIdentifier did not match a Script")
            .as_ref()
            .expect("ScriptIdentifier matched a freed Script")
    }

    pub(crate) fn get_script_mut(&mut self, id: ScriptIdentifier) -> &mut Script {
        self.scripts
            .get_mut(id.into_index())
            .expect("ScriptIdentifier did not match a Script")
            .as_mut()
            .expect("ScriptIdentifier matched a freed Script")
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

    pub(crate) fn create_null_object(&mut self, entries: Vec<ObjectEntry>) -> ObjectIndex {
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
        entries: Vec<ObjectEntry>,
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
