use super::{
    heap_constants::WellKnownSymbolIndexes,
    object::{ObjectEntry, PropertyDescriptor},
    CreateHeapData, Heap,
};
use crate::ecmascript::{
    execution::JsResult,
    types::{PropertyKey, Value},
};

pub(super) fn initialize_math_object(heap: &mut Heap) {
    let e = heap.create(std::f64::consts::E);
    let ln10 = heap.create(std::f64::consts::LN_10);
    let ln2 = heap.create(std::f64::consts::LN_2);
    let log10e = heap.create(std::f64::consts::LOG10_E);
    let log2e = heap.create(std::f64::consts::LOG2_E);
    let pi = heap.create(std::f64::consts::PI);
    let sqrt1_2 = heap.create(std::f64::consts::FRAC_1_SQRT_2);
    let sqrt2 = heap.create(std::f64::consts::SQRT_2);
    let abs = ObjectEntry::new_prototype_function_entry(heap, "abs", 1, false);
    let acos = ObjectEntry::new_prototype_function_entry(heap, "acos", 1, false);
    let acosh = ObjectEntry::new_prototype_function_entry(heap, "acosh", 1, false);
    let asin = ObjectEntry::new_prototype_function_entry(heap, "asin", 1, false);
    let asinh = ObjectEntry::new_prototype_function_entry(heap, "asinh", 1, false);
    let atan = ObjectEntry::new_prototype_function_entry(heap, "atan", 1, false);
    let atanh = ObjectEntry::new_prototype_function_entry(heap, "atanh", 1, false);
    let atan2 = ObjectEntry::new_prototype_function_entry(heap, "atan2", 2, false);
    let cbrt = ObjectEntry::new_prototype_function_entry(heap, "cbrt", 1, false);
    let ceil = ObjectEntry::new_prototype_function_entry(heap, "ceil", 1, false);
    let clz32 = ObjectEntry::new_prototype_function_entry(heap, "clz32", 1, false);
    let cos = ObjectEntry::new_prototype_function_entry(heap, "cos", 1, false);
    let cosh = ObjectEntry::new_prototype_function_entry(heap, "cosh", 1, false);
    let exp = ObjectEntry::new_prototype_function_entry(heap, "exp", 1, false);
    let expm1 = ObjectEntry::new_prototype_function_entry(heap, "expm1", 1, false);
    let floor = ObjectEntry::new_prototype_function_entry(heap, "floor", 1, false);
    let fround = ObjectEntry::new_prototype_function_entry(heap, "fround", 1, false);
    let hypot = ObjectEntry::new_prototype_function_entry(heap, "hypot", 2, true);
    let imul = ObjectEntry::new_prototype_function_entry(heap, "imul", 2, false);
    let log = ObjectEntry::new_prototype_function_entry(heap, "log", 1, false);
    let log1p = ObjectEntry::new_prototype_function_entry(heap, "log1p", 1, false);
    let log10 = ObjectEntry::new_prototype_function_entry(heap, "log10", 1, false);
    let log2 = ObjectEntry::new_prototype_function_entry(heap, "log2", 1, false);
    let max = ObjectEntry::new_prototype_function_entry(heap, "max", 2, true);
    let min = ObjectEntry::new_prototype_function_entry(heap, "min", 2, true);
    let pow = ObjectEntry::new_prototype_function_entry(heap, "pow", 2, false);
    let random = ObjectEntry::new_prototype_function_entry(heap, "random", 0, false);
    let round = ObjectEntry::new_prototype_function_entry(heap, "round", 1, false);
    let sign = ObjectEntry::new_prototype_function_entry(heap, "sign", 1, false);
    let sin = ObjectEntry::new_prototype_function_entry(heap, "sin", 1, false);
    let sinh = ObjectEntry::new_prototype_function_entry(heap, "sinh", 1, false);
    let sqrt = ObjectEntry::new_prototype_function_entry(heap, "sqrt", 1, false);
    let tan = ObjectEntry::new_prototype_function_entry(heap, "tan", 1, false);
    let tanh = ObjectEntry::new_prototype_function_entry(heap, "tanh", 1, false);
    let trunc = ObjectEntry::new_prototype_function_entry(heap, "trunc", 1, false);
    let entries = vec![
        ObjectEntry::new_frozen_entry(heap, "E", e.into()),
        ObjectEntry::new_frozen_entry(heap, "LN10", ln10.into()),
        ObjectEntry::new_frozen_entry(heap, "LN2", ln2.into()),
        ObjectEntry::new_frozen_entry(heap, "LOG10E", log10e.into()),
        ObjectEntry::new_frozen_entry(heap, "LOG2E", log2e.into()),
        ObjectEntry::new_frozen_entry(heap, "PI", pi.into()),
        ObjectEntry::new_frozen_entry(heap, "SQRT1_2", sqrt1_2.into()),
        ObjectEntry::new_frozen_entry(heap, "SQRT2", sqrt2.into()),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag.into()),
            PropertyDescriptor::roxh(Value::from_str(heap, "Math")),
        ),
        abs,
        acos,
        acosh,
        asin,
        asinh,
        atan,
        atanh,
        atan2,
        cbrt,
        ceil,
        clz32,
        cos,
        cosh,
        exp,
        expm1,
        floor,
        fround,
        hypot,
        imul,
        log,
        log1p,
        log10,
        log2,
        max,
        min,
        pow,
        random,
        round,
        sign,
        sin,
        sinh,
        sqrt,
        tan,
        tanh,
        trunc,
    ];
    let _ = heap.create_object(entries);
}

fn math_todo(_heap: &mut Heap, _this: Value, _args: &[Value]) -> JsResult<Value> {
    todo!();
}
