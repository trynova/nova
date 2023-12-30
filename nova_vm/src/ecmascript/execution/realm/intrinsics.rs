use crate::{
    ecmascript::types::{Function, Object, OrdinaryObject},
    heap::{
        indexes::{BuiltinFunctionIndex, ObjectIndex},
        BuiltinObjectIndexes,
    },
};

#[derive(Debug, Clone)]
pub struct Intrinsics {
    /// %Array%
    array: BuiltinFunctionIndex,
    /// %Array.prototype%
    array_prototype: ObjectIndex,
    /// %ArrayBuffer%
    array_buffer: BuiltinFunctionIndex,
    /// %ArrayBuffer.prototype%
    array_buffer_prototype: ObjectIndex,
    /// %BigInt%
    big_int: BuiltinFunctionIndex,
    /// %BigInt.prototype%
    big_int_prototype: ObjectIndex,
    /// %Boolean%
    boolean: BuiltinFunctionIndex,
    /// %Boolean.prototype%
    boolean_prototype: ObjectIndex,
    /// %Error%
    error: BuiltinFunctionIndex,
    /// %Error.prototype%
    error_prototype: ObjectIndex,
    /// %eval%
    eval: BuiltinFunctionIndex,
    /// %EvalError%
    eval_error: BuiltinFunctionIndex,
    /// %EvalError.prototype%
    eval_error_prototype: ObjectIndex,
    /// %Function%
    function: BuiltinFunctionIndex,
    /// %Function.prototype%
    ///
    /// NOTE: This is not spec-compliant. Function prototype should
    /// be a function that always returns undefined no matter how
    /// it is called. That's stupid so we do not have that.
    function_prototype: ObjectIndex,
    /// %isFinite%
    is_finite: BuiltinFunctionIndex,
    /// %isNaN%
    is_nan: BuiltinFunctionIndex,
    /// %Math%
    math: ObjectIndex,
    /// %Number%
    number: BuiltinFunctionIndex,
    /// %Number.prototype%
    number_prototype: ObjectIndex,
    /// %Object%
    object: BuiltinFunctionIndex,
    /// %Object.prototype%
    object_prototype: ObjectIndex,
    /// %Object.prototype.toString%
    object_prototype_to_string: BuiltinFunctionIndex,
    /// %RangeError%
    range_error: BuiltinFunctionIndex,
    /// %RangeError.prototype%
    range_error_prototype: ObjectIndex,
    /// %ReferenceError%
    reference_error: BuiltinFunctionIndex,
    /// %ReferenceError.prototype%
    reference_error_prototype: ObjectIndex,
    /// %Reflect%
    reflect: BuiltinFunctionIndex,
    /// %String%
    string: BuiltinFunctionIndex,
    /// %String.prototype%
    string_prototype: ObjectIndex,
    /// %Symbol%
    symbol: BuiltinFunctionIndex,
    /// %Symbol.prototype%
    symbol_prototype: ObjectIndex,
    /// %SyntaxError%
    syntax_error: BuiltinFunctionIndex,
    /// %SyntaxError.prototype%
    syntax_error_prototype: ObjectIndex,
    /// %ThrowTypeError%
    throw_type_error: BuiltinFunctionIndex,
    /// %TypeError%
    type_error: BuiltinFunctionIndex,
    /// %TypeError.prototype%
    type_error_prototype: ObjectIndex,
    /// %URIError%
    uri_error: BuiltinFunctionIndex,
    /// %URIError.prototype%
    uri_error_prototype: ObjectIndex,
}

/// Enumeration of intrinsics intended to be used as the [[Prototype]] value of
/// an object. Used in GetPrototypeFromConstructor.
pub(crate) enum ProtoIntrinsics {
    Array,
    ArrayBuffer,
    BigInt,
    Boolean,
    Error,
    EvalError,
    Function,
    Number,
    Object,
    RangeError,
    ReferenceError,
    String,
    Symbol,
    SyntaxError,
    TypeError,
    UriError,
}

impl Default for Intrinsics {
    fn default() -> Self {
        let array = BuiltinObjectIndexes::ArrayConstructor.into();
        let array_prototype = BuiltinObjectIndexes::ArrayPrototype.into();
        let array_buffer = BuiltinObjectIndexes::ArrayBufferConstructor.into();
        let array_buffer_prototype = BuiltinObjectIndexes::ArrayBufferPrototype.into();
        let big_int = BuiltinObjectIndexes::BigintConstructor.into();
        let big_int_prototype = BuiltinObjectIndexes::BigintPrototype.into();
        let boolean = BuiltinObjectIndexes::BooleanConstructor.into();
        let boolean_prototype = BuiltinObjectIndexes::BooleanPrototype.into();
        let error = BuiltinObjectIndexes::ErrorConstructor.into();
        let error_prototype = BuiltinObjectIndexes::ErrorPrototype.into();
        // TODO: Placeholder.
        let eval = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let eval_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let eval_error_prototype = ObjectIndex::from_u32_index(0);
        let function = BuiltinObjectIndexes::FunctionConstructor.into();
        let function_prototype = BuiltinObjectIndexes::FunctionPrototype.into();
        // TODO: Placeholder.
        let is_finite = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let is_nan = BuiltinFunctionIndex::from_u32_index(0);
        let math = BuiltinObjectIndexes::MathObject.into();
        let number = BuiltinObjectIndexes::NumberConstructor.into();
        let number_prototype = BuiltinObjectIndexes::NumberPrototype.into();
        let object = BuiltinObjectIndexes::ObjectConstructor.into();
        let object_prototype = BuiltinObjectIndexes::ObjectPrototype.into();
        // TODO: Placeholder.
        let object_prototype_to_string = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let range_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let range_error_prototype = ObjectIndex::from_u32_index(0);
        // TODO: Placeholder.
        let reference_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let reference_error_prototype = ObjectIndex::from_u32_index(0);
        let reflect = BuiltinObjectIndexes::ReflectObject.into();
        let string = BuiltinObjectIndexes::StringConstructor.into();
        let string_prototype = BuiltinObjectIndexes::StringPrototype.into();
        let symbol = BuiltinObjectIndexes::SymbolConstructor.into();
        let symbol_prototype = BuiltinObjectIndexes::SymbolPrototype.into();
        // TODO: Placeholder.
        let syntax_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let syntax_error_prototype = ObjectIndex::from_u32_index(0);
        // TODO: Placeholder.
        let throw_type_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let type_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let type_error_prototype = ObjectIndex::from_u32_index(0);
        // TODO: Placeholder.
        let uri_error = BuiltinFunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let uri_error_prototype = ObjectIndex::from_u32_index(0);

        Self {
            array,
            array_prototype,
            array_buffer,
            array_buffer_prototype,
            big_int,
            big_int_prototype,
            boolean,
            boolean_prototype,
            error,
            error_prototype,
            eval,
            eval_error,
            eval_error_prototype,
            function,
            function_prototype,
            is_finite,
            is_nan,
            math,
            number,
            number_prototype,
            object,
            object_prototype,
            object_prototype_to_string,
            range_error,
            range_error_prototype,
            reference_error,
            reference_error_prototype,
            reflect,
            string,
            string_prototype,
            symbol,
            symbol_prototype,
            syntax_error,
            syntax_error_prototype,
            throw_type_error,
            type_error,
            type_error_prototype,
            uri_error,
            uri_error_prototype,
        }
    }
}

impl Intrinsics {
    pub(crate) fn get_intrinsic_default_proto(
        &self,
        intrinsic_default_proto: ProtoIntrinsics,
    ) -> Object {
        match intrinsic_default_proto {
            ProtoIntrinsics::Array => self.array_prototype(),
            ProtoIntrinsics::ArrayBuffer => self.array_buffer_prototype().into(),
            ProtoIntrinsics::BigInt => self.big_int_prototype(),
            ProtoIntrinsics::Boolean => self.boolean_prototype(),
            ProtoIntrinsics::Error => self.error_prototype(),
            ProtoIntrinsics::EvalError => self.eval_error_prototype(),
            ProtoIntrinsics::Function => self.function_prototype(),
            ProtoIntrinsics::Number => self.number_prototype(),
            ProtoIntrinsics::Object => self.object_prototype(),
            ProtoIntrinsics::RangeError => self.range_error_prototype(),
            ProtoIntrinsics::ReferenceError => self.reference_error_prototype(),
            ProtoIntrinsics::String => self.string_prototype(),
            ProtoIntrinsics::Symbol => self.symbol_prototype(),
            ProtoIntrinsics::SyntaxError => self.syntax_error_prototype(),
            ProtoIntrinsics::TypeError => self.type_error_prototype(),
            ProtoIntrinsics::UriError => self.uri_error_prototype(),
        }
    }

    /// %Array%
    pub const fn array(&self) -> Function {
        Function::new_builtin_function(self.array)
    }

    /// %Array.prototype%
    pub const fn array_prototype(&self) -> Object {
        Object::Object(self.array_prototype)
    }

    /// %ArrayBuffer%
    pub const fn array_buffer(&self) -> Function {
        Function::new_builtin_function(self.array_buffer)
    }

    /// %ArrayBuffer.prototype%
    pub const fn array_buffer_prototype(&self) -> OrdinaryObject {
        OrdinaryObject::new(self.array_buffer_prototype)
    }

    /// %BigInt%
    pub const fn big_int(&self) -> Function {
        Function::new_builtin_function(self.big_int)
    }

    /// %BigInt.prototype%
    pub const fn big_int_prototype(&self) -> Object {
        Object::Object(self.big_int_prototype)
    }

    /// %Boolean%
    pub const fn boolean(&self) -> Function {
        Function::new_builtin_function(self.boolean)
    }

    /// %Boolean.prototype%
    pub const fn boolean_prototype(&self) -> Object {
        Object::Object(self.boolean_prototype)
    }

    /// %Error%
    pub const fn error(&self) -> Function {
        Function::new_builtin_function(self.error)
    }

    /// %Error.prototype%
    pub const fn error_prototype(&self) -> Object {
        Object::Object(self.error_prototype)
    }

    /// %eval%
    pub const fn eval(&self) -> Function {
        todo!()
    }

    /// %EvalError%
    pub const fn eval_error(&self) -> Function {
        Function::new_builtin_function(self.eval_error)
    }

    /// %EvalError.prototype%
    pub const fn eval_error_prototype(&self) -> Object {
        todo!()
    }

    /// %Function%
    pub const fn function(&self) -> Function {
        Function::new_builtin_function(self.function)
    }

    /// %Function.prototype%
    pub const fn function_prototype(&self) -> Object {
        Object::Object(self.function_prototype)
    }

    /// %isFinite%
    pub const fn is_finite(&self) -> Function {
        todo!()
    }

    /// %isNaN%
    pub const fn is_nan(&self) -> Function {
        todo!()
    }

    /// %Math%
    pub const fn math(&self) -> Object {
        Object::Object(self.math)
    }

    /// %Number%
    pub const fn number(&self) -> Function {
        Function::new_builtin_function(self.number)
    }

    /// %Number.prototype%
    pub const fn number_prototype(&self) -> Object {
        Object::Object(self.number_prototype)
    }

    /// %Object%
    pub const fn object(&self) -> Function {
        Function::new_builtin_function(self.object)
    }

    /// %Object.prototype%
    pub const fn object_prototype(&self) -> Object {
        Object::Object(self.object_prototype)
    }

    /// %Object.prototype.toString%
    pub const fn object_prototype_to_string(&self) -> Object {
        todo!()
    }

    /// %RangeError%
    pub const fn range_error(&self) -> Object {
        todo!()
    }

    /// %RangeError.prototype%
    pub const fn range_error_prototype(&self) -> Object {
        todo!()
    }

    /// %ReferenceError%
    pub const fn reference_error(&self) -> Object {
        todo!()
    }

    /// %ReferenceError.prototype%
    pub const fn reference_error_prototype(&self) -> Object {
        todo!()
    }

    /// %Reflect%
    pub const fn reflect(&self) -> Object {
        todo!()
    }

    /// %String%
    pub const fn string(&self) -> Function {
        Function::new_builtin_function(self.string)
    }

    /// %String.prototype%
    pub const fn string_prototype(&self) -> Object {
        Object::Object(self.string_prototype)
    }

    /// %Symbol%
    pub const fn symbol(&self) -> Function {
        Function::new_builtin_function(self.symbol)
    }

    /// %Symbol.prototype%
    pub const fn symbol_prototype(&self) -> Object {
        Object::Object(self.symbol_prototype)
    }

    /// %SyntaxError%
    pub const fn syntax_error(&self) -> Object {
        todo!()
    }

    /// %SyntaxError.prototype%
    pub const fn syntax_error_prototype(&self) -> Object {
        todo!()
    }

    /// %ThrowTypeError%
    pub const fn throw_type_error(&self) -> Object {
        todo!()
    }

    /// %TypeError%
    pub const fn type_error(&self) -> Object {
        todo!()
    }

    /// %TypeError.prototype%
    pub const fn type_error_prototype(&self) -> Object {
        todo!()
    }

    /// %URIError%
    pub const fn uri_error(&self) -> Object {
        todo!()
    }

    /// %URIError.prototype%
    pub const fn uri_error_prototype(&self) -> Object {
        todo!()
    }
}
