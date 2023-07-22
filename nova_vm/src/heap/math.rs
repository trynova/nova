use crate::value::{JsResult, Value};

use super::{
    heap_constants::WellKnownSymbolIndexes,
    object::{ObjectEntry, PropertyDescriptor, PropertyKey},
    Heap,
};

pub(super) fn initialize_math_object(heap: &mut Heap) {
    let e = Value::from_f64(heap, std::f64::consts::E);
    let ln10 = Value::from_f64(heap, std::f64::consts::LN_10);
    let ln2 = Value::from_f64(heap, std::f64::consts::LN_2);
    let log10e = Value::from_f64(heap, std::f64::consts::LOG10_E);
    let log2e = Value::from_f64(heap, std::f64::consts::LOG2_E);
    let pi = Value::from_f64(heap, std::f64::consts::PI);
    let sqrt1_2 = Value::from_f64(heap, std::f64::consts::FRAC_1_SQRT_2);
    let sqrt2 = Value::from_f64(heap, std::f64::consts::SQRT_2);
    let abs = ObjectEntry::new_prototype_function_entry(heap, "abs", 1, false, math_todo);
    let acos = ObjectEntry::new_prototype_function_entry(heap, "acos", 1, false, math_todo);
    let acosh = ObjectEntry::new_prototype_function_entry(heap, "acosh", 1, false, math_todo);
    let asin = ObjectEntry::new_prototype_function_entry(heap, "asin", 1, false, math_todo);
    let asinh = ObjectEntry::new_prototype_function_entry(heap, "asinh", 1, false, math_todo);
    let atan = ObjectEntry::new_prototype_function_entry(heap, "atan", 1, false, math_todo);
    let atanh = ObjectEntry::new_prototype_function_entry(heap, "atanh", 1, false, math_todo);
    let atan2 = ObjectEntry::new_prototype_function_entry(heap, "atan2", 2, false, math_todo);
    let cbrt = ObjectEntry::new_prototype_function_entry(heap, "cbrt", 1, false, math_todo);
    let ceil = ObjectEntry::new_prototype_function_entry(heap, "ceil", 1, false, math_todo);
    let clz32 = ObjectEntry::new_prototype_function_entry(heap, "clz32", 1, false, math_todo);
    let cos = ObjectEntry::new_prototype_function_entry(heap, "cos", 1, false, math_todo);
    let cosh = ObjectEntry::new_prototype_function_entry(heap, "cosh", 1, false, math_todo);
    let exp = ObjectEntry::new_prototype_function_entry(heap, "exp", 1, false, math_todo);
    let expm1 = ObjectEntry::new_prototype_function_entry(heap, "expm1", 1, false, math_todo);
    let floor = ObjectEntry::new_prototype_function_entry(heap, "floor", 1, false, math_todo);
    let fround = ObjectEntry::new_prototype_function_entry(heap, "fround", 1, false, math_todo);
    let hypot = ObjectEntry::new_prototype_function_entry(heap, "hypot", 2, true, math_todo);
    let imul = ObjectEntry::new_prototype_function_entry(heap, "imul", 2, false, math_todo);
    let log = ObjectEntry::new_prototype_function_entry(heap, "log", 1, false, math_todo);
    let log1p = ObjectEntry::new_prototype_function_entry(heap, "log1p", 1, false, math_todo);
    let log10 = ObjectEntry::new_prototype_function_entry(heap, "log10", 1, false, math_todo);
    let log2 = ObjectEntry::new_prototype_function_entry(heap, "log2", 1, false, math_todo);
    let max = ObjectEntry::new_prototype_function_entry(heap, "max", 2, true, math_todo);
    let min = ObjectEntry::new_prototype_function_entry(heap, "min", 2, true, math_todo);
    let pow = ObjectEntry::new_prototype_function_entry(heap, "pow", 2, false, math_todo);
    let random = ObjectEntry::new_prototype_function_entry(heap, "random", 0, false, math_todo);
    let round = ObjectEntry::new_prototype_function_entry(heap, "round", 1, false, math_todo);
    let sign = ObjectEntry::new_prototype_function_entry(heap, "sign", 1, false, math_todo);
    let sin = ObjectEntry::new_prototype_function_entry(heap, "sin", 1, false, math_todo);
    let sinh = ObjectEntry::new_prototype_function_entry(heap, "sinh", 1, false, math_todo);
    let sqrt = ObjectEntry::new_prototype_function_entry(heap, "sqrt", 1, false, math_todo);
    let tan = ObjectEntry::new_prototype_function_entry(heap, "tan", 1, false, math_todo);
    let tanh = ObjectEntry::new_prototype_function_entry(heap, "tanh", 1, false, math_todo);
    let trunc = ObjectEntry::new_prototype_function_entry(heap, "trunc", 1, false, math_todo);
    let entries = vec![
        ObjectEntry::new_frozen_entry(heap, "E", e),
        ObjectEntry::new_frozen_entry(heap, "LN10", ln10),
        ObjectEntry::new_frozen_entry(heap, "LN2", ln2),
        ObjectEntry::new_frozen_entry(heap, "LOG10E", log10e),
        ObjectEntry::new_frozen_entry(heap, "LOG2E", log2e),
        ObjectEntry::new_frozen_entry(heap, "PI", pi),
        ObjectEntry::new_frozen_entry(heap, "SQRT1_2", sqrt1_2),
        ObjectEntry::new_frozen_entry(heap, "SQRT2", sqrt2),
        ObjectEntry::new(
            PropertyKey::Symbol(WellKnownSymbolIndexes::ToStringTag as u32),
            PropertyDescriptor::roxh(Value::new_string(heap, "Math")),
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
