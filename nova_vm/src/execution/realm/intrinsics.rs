use crate::{
    heap::{BuiltinObjectIndexes, Handle},
    types::Object,
};

// TODO: We should probably consider lazily loading intrinsics. This would
//       contain a mutable reference to [`Realm`] and be created via a
//       `Realm::intrinsic()` method to guarantee safety.

pub struct Intrinsics;

impl Intrinsics {
    /// %Array%
    pub const fn array() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ArrayConstructorIndex as u32,
        ))
    }

    /// %Array.prototype%
    pub const fn array_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ArrayPrototypeIndex as u32,
        ))
    }

    /// %BigInt%
    pub const fn big_int() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::BigintConstructorIndex as u32,
        ))
    }

    /// %BigInt.prototype%
    pub const fn big_int_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::BigintPrototypeIndex as u32,
        ))
    }

    /// %Boolean%
    pub const fn boolean() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::BooleanConstructorIndex as u32,
        ))
    }

    /// %Boolean.prototype%
    pub const fn boolean_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::BooleanPrototypeIndex as u32,
        ))
    }

    /// %Error%
    pub const fn error() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ErrorConstructorIndex as u32,
        ))
    }

    /// %Error.prototype%
    pub const fn error_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ErrorPrototypeIndex as u32,
        ))
    }

    /// %eval%
    pub const fn eval() -> Object {
        todo!()
    }

    /// %EvalError%
    pub const fn eval_error() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ArrayConstructorIndex as u32,
        ))
    }

    /// %EvalError.prototype%
    pub const fn eval_error_prototype() -> Object {
        todo!()
    }

    /// %Function%
    pub const fn function() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::FunctionConstructorIndex as u32,
        ))
    }

    /// %Function.prototype%
    pub const fn function_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::FunctionPrototypeIndex as u32,
        ))
    }

    /// %isFinite%
    pub const fn is_finite() -> Object {
        todo!()
    }

    /// %isNaN%
    pub const fn is_nan() -> Object {
        todo!()
    }

    /// %Math%
    pub const fn math() -> Object {
        Object::Object(Handle::new(BuiltinObjectIndexes::MathObjectIndex as u32))
    }

    /// %Number%
    pub const fn number() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::NumberConstructorIndex as u32,
        ))
    }

    /// %Number.prototype%
    pub const fn number_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::NumberPrototypeIndex as u32,
        ))
    }

    /// %Object%
    pub const fn object() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ObjectConstructorIndex as u32,
        ))
    }

    /// %Object.prototype%
    pub const fn object_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::ObjectPrototypeIndex as u32,
        ))
    }

    /// %Object.prototype.toString%
    pub const fn object_prototype_to_string() -> Object {
        todo!()
    }

    /// %RangeError%
    pub const fn range_error() -> Object {
        todo!()
    }

    /// %RangeError.prototype%
    pub const fn range_error_prototype() -> Object {
        todo!()
    }

    /// %ReferenceError%
    pub const fn reference_error() -> Object {
        todo!()
    }

    /// %ReferenceError.prototype%
    pub const fn reference_error_prototype() -> Object {
        todo!()
    }

    /// %Reflect%
    pub const fn reflect() -> Object {
        todo!()
    }

    /// %String%
    pub const fn string() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::StringConstructorIndex as u32,
        ))
    }

    /// %String.prototype%
    pub const fn string_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::StringPrototypeIndex as u32,
        ))
    }

    /// %Symbol%
    pub const fn symbol() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::SymbolConstructorIndex as u32,
        ))
    }

    /// %Symbol.prototype%
    pub const fn symbol_prototype() -> Object {
        Object::Object(Handle::new(
            BuiltinObjectIndexes::SymbolPrototypeIndex as u32,
        ))
    }

    /// %SyntaxError%
    pub const fn syntax_error() -> Object {
        todo!()
    }

    /// %SyntaxError.prototype%
    pub const fn syntax_error_prototype() -> Object {
        todo!()
    }

    /// %ThrowTypeError%
    pub const fn throw_type_error() -> Object {
        todo!()
    }

    /// %TypeError%
    pub const fn type_error() -> Object {
        todo!()
    }

    /// %TypeError.prototype%
    pub const fn type_error_prototype() -> Object {
        todo!()
    }

    /// %URIError%
    pub const fn uri_error() -> Object {
        todo!()
    }

    /// %URIError.prototype%
    pub const fn uri_error_prototype() -> Object {
        todo!()
    }
}
