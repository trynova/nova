use crate::{
    ecmascript::types::{Function, Object},
    heap::{
        indexes::{FunctionIndex, ObjectIndex},
        BuiltinObjectIndexes,
    },
};

#[derive(Debug, Clone)]
pub struct Intrinsics {
    /// %Array%
    array: FunctionIndex,
    /// %Array.prototype%
    array_prototype: ObjectIndex,
    /// %BigInt%
    big_int: FunctionIndex,
    /// %BigInt.prototype%
    big_int_prototype: ObjectIndex,
    /// %Boolean%
    boolean: FunctionIndex,
    /// %Boolean.prototype%
    boolean_prototype: ObjectIndex,
    /// %Error%
    error: FunctionIndex,
    /// %Error.prototype%
    error_prototype: ObjectIndex,
    /// %eval%
    eval: FunctionIndex,
    /// %EvalError%
    eval_error: FunctionIndex,
    /// %EvalError.prototype%
    eval_error_prototype: ObjectIndex,
    /// %Function%
    function: FunctionIndex,
    /// %Function.prototype%
    ///
    /// NOTE: This is not spec-compliant. Function prototype should
    /// be a function that always returns undefined no matter how
    /// it is called. That's stupid so we do not have that.
    function_prototype: ObjectIndex,
    /// %isFinite%
    is_finite: FunctionIndex,
    /// %isNaN%
    is_nan: FunctionIndex,
    /// %Math%
    math: ObjectIndex,
    /// %Number%
    number: FunctionIndex,
    /// %Number.prototype%
    number_prototype: ObjectIndex,
    /// %Object%
    object: FunctionIndex,
    /// %Object.prototype%
    object_prototype: ObjectIndex,
    /// %Object.prototype.toString%
    object_prototype_to_string: FunctionIndex,
    /// %RangeError%
    range_error: FunctionIndex,
    /// %RangeError.prototype%
    range_error_prototype: ObjectIndex,
    /// %ReferenceError%
    reference_error: FunctionIndex,
    /// %ReferenceError.prototype%
    reference_error_prototype: ObjectIndex,
    /// %Reflect%
    reflect: FunctionIndex,
    /// %String%
    string: FunctionIndex,
    /// %String.prototype%
    string_prototype: ObjectIndex,
    /// %Symbol%
    symbol: FunctionIndex,
    /// %Symbol.prototype%
    symbol_prototype: ObjectIndex,
    /// %SyntaxError%
    syntax_error: FunctionIndex,
    /// %SyntaxError.prototype%
    syntax_error_prototype: ObjectIndex,
    /// %ThrowTypeError%
    throw_type_error: FunctionIndex,
    /// %TypeError%
    type_error: FunctionIndex,
    /// %TypeError.prototype%
    type_error_prototype: ObjectIndex,
    /// %URIError%
    uri_error: FunctionIndex,
    /// %URIError.prototype%
    uri_error_prototype: ObjectIndex,
}

impl Default for Intrinsics {
    fn default() -> Self {
        let array = BuiltinObjectIndexes::ArrayConstructorIndex.into();
        let array_prototype = BuiltinObjectIndexes::ArrayPrototypeIndex.into();
        let big_int = BuiltinObjectIndexes::BigintConstructorIndex.into();
        let big_int_prototype = BuiltinObjectIndexes::BigintPrototypeIndex.into();
        let boolean = BuiltinObjectIndexes::BooleanConstructorIndex.into();
        let boolean_prototype = BuiltinObjectIndexes::BooleanPrototypeIndex.into();
        let error = BuiltinObjectIndexes::ErrorConstructorIndex.into();
        let error_prototype = BuiltinObjectIndexes::ErrorPrototypeIndex.into();
        // TODO: Placeholder.
        let eval = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let eval_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let eval_error_prototype = ObjectIndex::from_u32_index(0);
        let function = BuiltinObjectIndexes::FunctionConstructorIndex.into();
        let function_prototype = BuiltinObjectIndexes::FunctionPrototypeIndex.into();
        // TODO: Placeholder.
        let is_finite = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let is_nan = FunctionIndex::from_u32_index(0);
        let math = BuiltinObjectIndexes::MathObjectIndex.into();
        let number = BuiltinObjectIndexes::NumberConstructorIndex.into();
        let number_prototype = BuiltinObjectIndexes::NumberPrototypeIndex.into();
        let object = BuiltinObjectIndexes::ObjectConstructorIndex.into();
        let object_prototype = BuiltinObjectIndexes::ObjectPrototypeIndex.into();
        // TODO: Placeholder.
        let object_prototype_to_string = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let range_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let range_error_prototype = ObjectIndex::from_u32_index(0);
        // TODO: Placeholder.
        let reference_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let reference_error_prototype = ObjectIndex::from_u32_index(0);
        let reflect = BuiltinObjectIndexes::ReflectObjectIndex.into();
        let string = BuiltinObjectIndexes::StringConstructorIndex.into();
        let string_prototype = BuiltinObjectIndexes::StringPrototypeIndex.into();
        let symbol = BuiltinObjectIndexes::SymbolConstructorIndex.into();
        let symbol_prototype = BuiltinObjectIndexes::SymbolPrototypeIndex.into();
        // TODO: Placeholder.
        let syntax_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let syntax_error_prototype = ObjectIndex::from_u32_index(0);
        // TODO: Placeholder.
        let throw_type_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let type_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let type_error_prototype = ObjectIndex::from_u32_index(0);
        // TODO: Placeholder.
        let uri_error = FunctionIndex::from_u32_index(0);
        // TODO: Placeholder.
        let uri_error_prototype = ObjectIndex::from_u32_index(0);

        Self {
            array,
            array_prototype,
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
    /// %Array%
    pub const fn array(&self) -> Function {
        Function(self.array)
    }

    /// %Array.prototype%
    pub const fn array_prototype(&self) -> Object {
        Object::Object(self.array_prototype)
    }

    /// %BigInt%
    pub const fn big_int(&self) -> Function {
        Function(self.big_int)
    }

    /// %BigInt.prototype%
    pub const fn big_int_prototype(&self) -> Object {
        Object::Object(self.big_int_prototype)
    }

    /// %Boolean%
    pub const fn boolean(&self) -> Function {
        Function(self.boolean)
    }

    /// %Boolean.prototype%
    pub const fn boolean_prototype(&self) -> Object {
        Object::Object(self.boolean_prototype)
    }

    /// %Error%
    pub const fn error(&self) -> Function {
        Function(self.error)
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
        Function(self.eval_error)
    }

    /// %EvalError.prototype%
    pub const fn eval_error_prototype(&self) -> Object {
        todo!()
    }

    /// %Function%
    pub const fn function(&self) -> Function {
        Function(self.function)
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
        Function(self.number)
    }

    /// %Number.prototype%
    pub const fn number_prototype(&self) -> Object {
        Object::Object(self.number_prototype)
    }

    /// %Object%
    pub const fn object(&self) -> Function {
        Function(self.object)
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
        Function(self.string)
    }

    /// %String.prototype%
    pub const fn string_prototype(&self) -> Object {
        Object::Object(self.string_prototype)
    }

    /// %Symbol%
    pub const fn symbol(&self) -> Function {
        Function(self.symbol)
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
