// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{marker::PhantomData, ptr::NonNull};

use oxc_ast::ast::BindingPattern;
use oxc_syntax::number::ToJsString;

use crate::{
    ecmascript::{Agent, String},
    engine::{Executable, NoGcScope, Scoped, bytecode::bytecode_compiler::IndexType},
};

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Instruction {
    // === HOT INSTRUCTIONS ===
    // = ZERO ARGS =
    /// Load the result value and add it to the stack.
    Load,
    /// Add the result value to the stack, without removing it as the result
    /// value.
    LoadCopy,
    /// Store the last value from the stack as the result value.
    Store,
    /// Pop a value from the stack without storing it anywhere.
    PopStack,
    /// Push the last evaluated reference, if any.
    PushReference,
    /// Pop the last stored reference.
    PopReference,
    /// Store [GetValue()](https://tc39.es/ecma262/#sec-getvalue) as the result
    /// value.
    ///
    /// #### Note
    /// We only call `GetValue` on reference values. This can be statically
    /// analysed from the AST. Non-reference values are already in the result
    /// value so a `GetValue` call would be a no-op.
    GetValue,
    /// Same as GetValue without taking the reference slot. Used for reference
    /// property updates and function calls (where `this` comes from the
    /// reference).
    GetValueKeepReference,
    /// Call PutValue() with the last reference on the reference stack and the
    /// result value.
    PutValue,
    /// Perform EvaluatePropertyAccessWithExpressionKey with the `baseValue` at
    /// the top of the stack and the `propertyNameValue` in result register,
    /// and store the result in the reference register.
    EvaluatePropertyAccessWithExpressionKey,
    /// Perform InitializeReferencedBinding with parameters reference (V) and
    /// result (W).
    InitializeReferencedBinding,
    /// Store ToNumeric() as the result value.
    ToNumeric,
    /// Stop bytecode execution, indicating a return from the current function.
    Return,
    /// Performs Await() on the result value, and after resuming, stores the
    /// promise result as the result value.
    Await,
    /// Performs Yield() on the result value, and after resuming, stores the
    /// value passed to `next()` as the result value.
    Yield,
    /// Take the result value and the top stack value, compare them using
    /// IsStrictlyEqual() and store the result as the result value.
    IsStrictlyEqual,
    /// Store true as the result value if the current result value is null or
    /// undefined, false otherwise.
    IsNullOrUndefined,
    /// Store true as the result value if the current result value is null,
    /// false otherwise.
    IsNull,
    /// Store true as the result value if the current result value is undefined,
    /// false otherwise.
    IsUndefined,

    // = ONE ARG =
    /// Load the result value to the given stack slot.
    PutValueToIndex,
    /// Store a copy of a value from the stack by index as the result value.
    GetValueFromIndex,
    /// Store a constant as the result value.
    StoreConstant,
    /// Store ResolveBinding() in the reference register.
    ResolveBinding,
    /// Store [GetValue()](https://tc39.es/ecma262/#sec-getvalue) as the result
    /// value. This variant caches the property lookup.
    ///
    /// #### Note
    /// We only call `GetValue` on reference values. This can be statically
    /// analysed from the AST. Non-reference values are already in the result
    /// value so a `GetValue` call would be a no-op.
    GetValueWithCache,
    GetValueWithCacheKeepReference,
    /// Same as PutValue but with a cache slot.
    PutValueWithCache,
    /// Perform EvaluatePropertyAccessWithIdentifierKey with the `baseValue` in
    /// the result register and the `propertyNameString` given as the first
    /// immediate argument, and store the result in the reference register.
    EvaluatePropertyAccessWithIdentifierKey,
    /// Perform CreateImmutableBinding in the running execution context's
    /// LexicalEnvironment with an identifier parameter and `true`
    CreateImmutableBinding,
    /// Perform CreateMutableBinding in the running execution context's
    /// LexicalEnvironment with an identifier parameter and `false`
    CreateMutableBinding,

    // = TWO ARGS =
    /// Jump to another instruction by setting the instruction pointer.
    Jump,
    /// Jump to another instruction by setting the instruction pointer
    /// if the current result is falsey.
    JumpIfNot,
    /// Jump to another instruction by setting the instruction pointer if the
    /// current result is `true`.
    JumpIfTrue,

    // === COLD INSTRUCTIONS ===

    // = ZERO ARGS =
    /// Call IsConstructor() on the current result value and store the result
    /// as the result value.
    IsConstructor,
    /// Load the result value, if present, to the top of the stack, replacing
    /// the previous top of the stack value.
    LoadReplace,
    /// Perform UpdateEmpty with the value coming from the top of the stack,
    /// and the Completion Record \[\[Value]] being the result value.
    UpdateEmpty,
    /// Swap the last two values on the stack.
    Swap,
    /// Empty the result register.
    Empty,
    /// Performs steps 2-4 from the [UnaryExpression ! Runtime Semantics](https://tc39.es/ecma262/#sec-logical-not-operator-runtime-semantics-evaluation).
    LogicalNot,
    /// Store ApplyStringOrNumericBinaryOperator() as the result value.
    ApplyAdditionBinaryOperator,
    ApplySubtractionBinaryOperator,
    ApplyMultiplicationBinaryOperator,
    ApplyDivisionBinaryOperator,
    ApplyRemainderBinaryOperator,
    ApplyExponentialBinaryOperator,
    ApplyShiftLeftBinaryOperator,
    ApplyShiftRightBinaryOperator,
    ApplyShiftRightZeroFillBinaryOperator,
    ApplyBitwiseORBinaryOperator,
    ApplyBitwiseXORBinaryOperator,
    ApplyBitwiseAndBinaryOperator,
    /// Push a value into an array
    ArrayPush,
    /// Push a hole into an array
    ArrayElision,
    /// Performs steps 2-4 from the [UnaryExpression ~ Runtime Semantics](https://tc39.es/ecma262/#sec-bitwise-not-operator-runtime-semantics-evaluation).
    BitwiseNot,
    /// Performs CreateUnmappedArgumentsObject() on the arguments list present
    /// in the iterator stack, and stores the created arguments object as the
    /// result value.
    CreateUnmappedArgumentsObject,
    /// Performs CopyDataProperties() with the source being the result value and
    /// the target object being at the top of the stack. The excluded names list
    /// will be empty.
    CopyDataProperties,
    /// Perform MakeSuperPropertyReference with the `propertyKey` in the result
    /// register, and store the result in the reference register.
    MakeSuperPropertyReferenceWithExpressionKey,
    /// Compare the last two values on the stack using the '>' operator rules.
    GreaterThan,
    /// Compare the last two values on the stack using the '>=' operator rules.
    GreaterThanEquals,
    /// Apply the delete operation to the evaluated expression and set it as
    /// the result value.
    Delete,
    /// Store HasProperty() as the result value.
    HasProperty,
    /// Perform PrivateElementFind on the private property reference currently
    /// in the reference register, and convert the result into a boolean.
    HasPrivateElement,
    Increment,
    Decrement,
    /// Store InstanceofOperator() as the result value.
    InstanceofOperator,
    /// Reserves enough room for all of a class instance's PrivateElements
    /// fields in the backing object, and copies all private methods to the
    /// backing object.
    ///
    /// The target object is at the top of the stack; it should be the `this`
    /// value. The target is not popped off the stack.
    ClassInitializePrivateElements,
    /// Store IsLooselyEqual() as the result value.
    IsLooselyEqual,
    /// Compare the last two values on the stack using the '<' operator rules.
    LessThan,
    /// Compare the last two values on the stack using the '<=' operator rules.
    LessThanEquals,
    /// Store OrdinaryObjectCreate(%Object.prototype%) on the stack.
    ObjectCreate,
    /// Call CreateDataPropertyOrThrow(object, key, value) with value being the
    /// result value, key being the top stack value and object being the second
    /// stack value. The object is not popped from the stack.
    ObjectDefineProperty,
    /// Call `object[[SetPrototypeOf]](value)` on the object on the stack using
    /// the current result value as the parameter.
    ObjectSetPrototype,
    /// Pop a jump target for uncaught exceptions
    PopExceptionJumpTarget,
    /// Store ResolveThisBinding() in the result register.
    ResolveThisBinding,
    /// Store a copy of the last value from the stack as the result value.
    StoreCopy,
    /// Throw the result value as an exception.
    Throw,
    /// Store ToNumber() as the result value.
    ToNumber,
    /// Store ToObject() as the result value.
    ToObject,
    /// Apply the typeof operation to the evaluated expression and set it as
    /// the result value.
    Typeof,
    /// Performs steps 3 and 4 from the [UnaryExpression - Runtime Semantics](https://tc39.es/ecma262/#sec-unary-minus-operator-runtime-semantics-evaluation).
    UnaryMinus,
    /// Perform NewDeclarativeEnvironment with the running execution context's
    /// LexicalEnvironment as the only parameter and set it as the running
    /// execution context's LexicalEnvironment.
    ///
    /// #### Note
    /// It is technically against the spec to immediately set the new
    /// environment as the running execution context's LexicalEnvironment. The
    /// spec requires that creation of bindings in the environment is done
    /// first. This is immaterial because creating the bindings cannot fail.
    EnterDeclarativeEnvironment,
    /// Enter a new FunctionEnvironment with the top of the stack as the this
    /// binding and `[[FunctionObject]]`. This is used for class static
    /// initializers.
    EnterClassStaticElementEnvironment,
    /// Reset the running execution context's LexicalEnvironment to its current
    /// value's `[[OuterEnv]]`.
    ExitDeclarativeEnvironment,
    /// Reset the running execution context's VariableEnvironment to its
    /// current value's `[[OuterEnv]]`.
    ExitVariableEnvironment,
    /// Reset the running execution context's PrivateEnvironment to its current
    /// value's `[[OuterPrivateEnvironment]]`.
    ExitPrivateEnvironment,
    /// Skip the next value in an array binding pattern.
    ///
    /// ```js
    /// const [a,,b] = x;
    /// ```
    BindingPatternSkip,
    /// Load the next value in an array binding pattern onto the stack.
    ///
    /// This is used to implement nested binding patterns. The current binding
    /// pattern needs to take the "next step", but instead of binding to an
    /// identifier it is instead saved on the stack and the nested binding
    /// pattern starts its work.
    ///
    /// ```js
    /// const [{ a }] = x;
    /// ```
    BindingPatternGetValue,
    /// Load all remaining values onto the stack
    ///
    /// This is used to implement nested binding patterns in rest elements.
    ///
    /// ```js
    /// const [a, ...[b, c]] = x;
    /// ```
    BindingPatternGetRestValue,
    /// Finish binding values
    ///
    /// This stops
    FinishBindingPattern,
    /// Take the current result and begin iterating it according to
    /// EnumerateObjectProperties.
    EnumerateObjectProperties,
    /// Take the current result and call `GetIterator(result, SYNC)`
    GetIteratorSync,
    /// Take the current result and call `GetIterator(result, ASYNC)`
    GetIteratorAsync,
    /// Perform IteratorStepValue on the current iterator, putting the resulting
    /// value on the result value, or undefined if the iterator has completed.
    ///
    /// When the iterator has completed, rather than popping it off the stack,
    /// it sets it to `VmIterator::EmptyIterator` so further reads and closes
    /// aren't observable.
    IteratorStepValueOrUndefined,
    /// Call the current iterator's `[[NextMethod]]` with the current result
    /// register value as the only parameter.
    IteratorCallNextMethod,
    /// Perform IteratorValue on the current result register value.
    IteratorValue,
    /// Consume the remainder of the iterator, and produce a new array with
    /// those elements. This replaces the iterator with a special exhausted
    /// iterator after use.
    IteratorRestIntoArray,
    /// Perform CloseIterator on the current iterator
    IteratorClose,
    /// Perform AsyncCloseIterator on the current iterator
    AsyncIteratorClose,
    /// Perform CloseIterator on the current iterator with the current result
    /// as a thrown value.
    ///
    /// This will call the `return` method of the current iterator, ignoring
    /// all errors, and then continues with the thrown value still as the
    /// current result.
    IteratorCloseWithError,
    /// Perform AsyncCloseIterator on the current iterator with the current
    /// result as a thrown value.
    ///
    /// This will call the `return` method of the current iterator. If the
    /// method is found and returns a value, then the current result is stored
    /// onto the stack, a special "ignore thrown error and next instruction"
    /// exception jump target handler is installed, and then an await is
    /// performed. If an error is thrown or no method exists, then the current
    /// result is rethrown immediately.
    ///
    /// This instruction should always be followed by the following bytecode
    /// snippet:
    /// ```rust,ignore
    /// // Pop the special exception jump target handler if await didn't throw.
    /// // Note: this is skipped by the special handler if Await did throw.
    /// Instruction::PopExceptionJumpTarget;
    /// // Return the current result into the result register.
    /// Instruction::Store;
    /// // Rethrow the current result.
    /// Instruction::Throw;
    /// ```
    AsyncIteratorCloseWithError,
    /// Pop the current iterator from the iterator stack.
    IteratorPop,
    /// Store GetNewTarget() as the result value.
    GetNewTarget,
    /// Perform EvaluateImportCall with specifier at the top of the stack, and
    /// options (optionally) in the result register. Pops the stack and places
    /// a dynamic import Promise into the result register.
    ImportCall,
    /// Store `import.meta` object as the result value.
    ImportMeta,
    /// Debug instruction
    Debug,

    // = ONE ARG =
    /// Load a constant and add it to the stack.
    LoadConstant,
    /// Store ArrayCreate(0) as the result value.
    ///
    /// This instruction has one immediate argument that is the minimum
    /// number of elements to expect.
    ArrayCreate,
    /// Performs CopyDataProperties() into a newly created object and returns it.
    /// The source object will be on the result value, and the excluded names
    /// will be read from the reference stack, with the number of names passed
    /// in an immediate.
    CopyDataPropertiesIntoObject,
    /// Store EvaluateCall() as the result value.
    ///
    /// This instruction has the number of argument values that need to be
    /// popped from the stack (last to first) as an argument, and finally the
    /// function to call.
    EvaluateCall,
    /// Store EvaluateNew() as the result value.
    ///
    /// This instruction has the number of argument values that need to be
    /// popped from the stack (last to first) as an argument, the value on the
    /// stack afterwards is the constructor to call.
    EvaluateNew,
    /// Store SuperCall() as the result value.
    ///
    /// This instruction has the number of argument values that need to be
    /// popped from the stack (last to first) as an argument.
    EvaluateSuper,
    /// Call the `eval` function in a direct way.
    ///
    /// If the `eval` identifier points to the current realm's eval intrinsic
    /// function, then it performs a direct eval. Otherwise, it loads the value
    /// that identifier points to, and calls it.
    ///
    /// This instruction has the number of argument values that need to be
    /// popped from the stack (last to first) as an argument.
    DirectEvalCall,
    /// Perform MakePrivateReference with the `baseValue` in the result
    /// register and the `privateIdentifier` given as the first immediate
    /// argument, and store the result in the reference register.
    MakePrivateReference,
    /// Perform MakeSuperPropertyReference with the `propertyKey` given as the
    /// first immediate argument, and store the result in the reference
    /// register.
    MakeSuperPropertyReferenceWithIdentifierKey,
    /// Store InstantiateArrowFunctionExpression() as the result value.
    InstantiateArrowFunctionExpression,
    /// Store InstantiateOrdinaryFunctionExpression() as the result value.
    InstantiateOrdinaryFunctionExpression,
    /// Store CreateBuiltinFunction(defaultConstructor, 0, className) as the
    /// result value.
    ClassDefineDefaultConstructor,
    /// Put the current result value at the next PrivateName's slot in the
    /// target object. The PrivateName is calculated based on the offset
    /// provided as an immediate, and the current PrivateEnvironment.
    ///
    /// The target object is at the top of the stack. the target is not popped
    /// off the stack.
    ClassInitializePrivateValue,
    /// Store a new as the result Object created with the given shape, with its
    /// properties coming from the stack.
    ObjectCreateWithShape,
    /// Truncate the runtime stack to the given depth.
    TruncateStack,
    /// Take N items from the stack and string-concatenate them together.
    StringConcat,
    /// Throw a new Error object as an exception with the result value as the
    /// message.
    ///
    /// The error subtype is determined by an immediate value.
    ThrowError,
    /// Perform NewPrivateEnvironment with the running execution context's
    /// PrivateEnvironment and enter it.
    ///
    /// The number of private names in the environment is given
    EnterPrivateEnvironment,
    /// Begin binding values using destructuring
    BeginSimpleObjectBindingPattern,
    /// In array binding patterns, bind the current result to the given
    /// identifier. In object binding patterns, bind the object's property with
    /// the identifier's name.
    ///
    /// ```js
    /// const { a } = x;
    /// const [a] = x;
    /// ```
    BindingPatternBind,
    /// Bind all remaining values to given identifier.
    ///
    /// ```js
    /// const { a, ...b } = x;
    /// const [a, ...b] = x;
    /// ```
    BindingPatternBindRest,
    /// Bind all remaining values to a given stack index.
    ///
    /// ```js
    /// const { a, ...b } = x;
    /// const [a, ...b] = x;
    /// ```
    BindingPatternBindRestToIndex,
    /// Load the value of the property with the given name in an object binding
    /// pattern onto the stack. The name is passed as a constant argument.
    ///
    /// This is used to implement nested binding patterns. The current binding
    /// pattern needs to take the "next step", but instead of binding to an
    /// identifier it is instead saved on the stack and the nested binding
    /// pattern starts its work.
    ///
    /// ```js
    /// const { a: [b] } = x;
    /// ```
    BindingPatternGetValueNamed,
    /// Throw a TypeError if the result register does not contain an Object.
    ///
    /// The error message is provided as an identifier.
    VerifyIsObject,

    // = TWO ARGS =
    /// Begin binding values using a sync iterator for known repetitions
    BeginSimpleArrayBindingPattern,
    /// Bind an object property to an identifier with a different name. The
    /// constant given as the second argument is the property key.
    ///
    /// ```js
    /// const { a: b } = x;
    /// ```
    BindingPatternBindNamed,
    /// Bind an object property to a stack variable. The constant given as the
    /// second argument is the stack slot.
    BindingPatternBindToIndex,
    /// Create a class constructor and store it as the result value.
    ///
    /// The class name should be found at the top of the stack.
    /// If the class is a derived class, then the parent constructor should
    /// also be on the stack after the class name.
    ClassDefineConstructor,
    /// Define a private method on class constructor or instances.
    ///
    /// The target object is at the top or second from the top of the stack,
    /// and the method's PrivateName's `[[Description]]` String is the
    /// current result value, the method's function expression is provided as
    /// the first immediate, and the method's getter/setter metadata is
    /// provided as the second immediate. The last bit in the metadata is the
    /// Get flag, the second last bit is the Set flag, and the third last bit
    /// is the Static flag.
    ClassDefinePrivateMethod,
    /// Define a private property field on class constructor or instances.
    ///
    /// The field's PrivateName's `[[Description]]` String is provided as an
    /// identifier, and the staticness of the field is provided as an
    /// immediate.
    ClassDefinePrivateProperty,
    /// Create a new VariableEnvironment and initialize it with variable names
    /// and values from the stack, where each name comes before the value.
    /// The first immediate argument is the number of variables to initialize.
    /// The second immediate is a boolean which is true if LexicalEnvironment
    /// should also be set to this new environment (true in strict mode), or
    /// false if it should be set to a new descendant declarative environment.
    InitializeVariableEnvironment,
    /// Perform IteratorStepValue on the current iterator and jump to
    /// index if iterator completed.
    IteratorStepValue,
    /// Verify that the current result register value contains an object, perform IteratorComplete on it, and if the
    /// result is `true` then perform IteratorValue on it and jump to the
    /// provided instruction.
    IteratorComplete,
    /// Perform `? GetMethod(iterator, "return")` on the current iterator and
    /// call the result if it is not undefined. If the result is undefined,
    /// jump to the provided instruction.
    IteratorThrow,
    /// Perform `? GetMethod(iterator, "throw")` on the current iterator and
    /// call the result if it is not undefined. If the result is undefined,
    /// jump to the provided instruction.
    IteratorReturn,
    /// Create and define a method on an object.
    ///
    /// The key is at the top stack value, the object is second on the stack.
    ObjectDefineMethod,
    ObjectDefineGetter,
    ObjectDefineSetter,
    /// Push a jump target for uncaught exceptions
    PushExceptionJumpTarget,
    /// Store ResolveBinding() in the reference register using a property
    /// lookup cache.
    ///
    /// # Safety
    ///
    /// If this is no longer the last item, adjust [`Instruction::MAX`][Instruction::MAX].
    ResolveBindingWithCache,
}

impl Instruction {
    pub(crate) const MAX: Self = Self::ResolveBindingWithCache;

    /// Returns true if this instruction is a terminal instruction where
    /// control flow cannot continue to the next instruction.
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Jump | Self::Return | Self::Throw | Self::ThrowError
        )
    }

    #[inline]
    pub const fn argument_count(self) -> u8 {
        match self {
            // Number of repetitions and lexical status
            Self::BeginSimpleArrayBindingPattern
            | Self::BindingPatternBindNamed
            | Self::BindingPatternBindToIndex
            | Self::ClassDefineConstructor
            | Self::ClassDefinePrivateMethod
            | Self::ClassDefinePrivateProperty
            | Self::InitializeVariableEnvironment
            | Self::IteratorStepValue
            | Self::IteratorComplete
            | Self::IteratorThrow
            | Self::IteratorReturn
            | Self::Jump
            | Self::JumpIfNot
            | Self::JumpIfTrue
            | Self::ObjectDefineGetter
            | Self::ObjectDefineMethod
            | Self::ObjectDefineSetter
            | Self::PushExceptionJumpTarget
            | Self::ResolveBindingWithCache => 2,
            Self::PutValueToIndex
            | Self::GetValueFromIndex
            | Self::ArrayCreate
            | Self::BeginSimpleObjectBindingPattern
            | Self::BindingPatternBind
            | Self::BindingPatternBindRest
            | Self::BindingPatternBindRestToIndex
            | Self::BindingPatternGetValueNamed
            | Self::ClassDefineDefaultConstructor
            | Self::ClassInitializePrivateValue
            | Self::CopyDataPropertiesIntoObject
            | Self::CreateImmutableBinding
            | Self::CreateMutableBinding
            | Self::DirectEvalCall
            | Self::EnterPrivateEnvironment
            | Self::EvaluateCall
            | Self::EvaluateNew
            | Self::EvaluatePropertyAccessWithIdentifierKey
            | Self::EvaluateSuper
            | Self::GetValueWithCache
            | Self::GetValueWithCacheKeepReference
            | Self::InstantiateArrowFunctionExpression
            | Self::InstantiateOrdinaryFunctionExpression
            | Self::LoadConstant
            | Self::MakePrivateReference
            | Self::MakeSuperPropertyReferenceWithIdentifierKey
            | Self::ObjectCreateWithShape
            | Self::TruncateStack
            | Self::PutValueWithCache
            | Self::ResolveBinding
            | Self::StoreConstant
            | Self::StringConcat
            | Self::ThrowError
            | Self::VerifyIsObject => 1,
            _ => 0,
        }
    }

    pub const fn has_cache_index(self) -> bool {
        matches!(
            self,
            Self::GetValueWithCache
                | Self::GetValueWithCacheKeepReference
                | Self::PutValueWithCache
                | Self::ResolveBindingWithCache
        )
    }

    pub const fn has_constant_index(self) -> bool {
        matches!(
            self,
            Self::BindingPatternBindNamed
                | Self::BindingPatternBindToIndex
                | Self::BindingPatternGetValueNamed
                | Self::LoadConstant
                | Self::StoreConstant
        )
    }

    pub const fn has_shape_index(self) -> bool {
        matches!(self, Self::ObjectCreateWithShape)
    }

    pub const fn has_identifier_index(self) -> bool {
        let base_matches = matches!(
            self,
            Self::BindingPatternBind
                | Self::BindingPatternBindNamed
                | Self::BindingPatternBindRest
                | Self::ClassDefinePrivateMethod
                | Self::ClassDefinePrivateProperty
                | Self::CreateImmutableBinding
                | Self::CreateMutableBinding
                | Self::EvaluatePropertyAccessWithIdentifierKey
                | Self::MakePrivateReference
                | Self::MakeSuperPropertyReferenceWithIdentifierKey
                | Self::ResolveBinding
                | Self::ResolveBindingWithCache
                | Self::VerifyIsObject
        );

        #[cfg(feature = "typescript")]
        let typescript_matches = false;
        #[cfg(not(feature = "typescript"))]
        let typescript_matches = false;

        base_matches || typescript_matches
    }

    pub const fn has_function_expression_index(self) -> bool {
        matches!(
            self,
            Self::ClassDefineConstructor
                | Self::ClassDefinePrivateMethod
                | Self::InstantiateArrowFunctionExpression
                | Self::InstantiateOrdinaryFunctionExpression
                | Self::ObjectDefineGetter
                | Self::ObjectDefineMethod
                | Self::ObjectDefineSetter
        )
    }

    pub const fn has_jump_slot(self) -> bool {
        matches!(
            self,
            Self::IteratorComplete
                | Self::IteratorThrow
                | Self::IteratorReturn
                | Self::Jump
                | Self::JumpIfNot
                | Self::JumpIfTrue
                | Self::PushExceptionJumpTarget
                | Self::IteratorStepValue
        )
    }

    pub const fn as_u8(self) -> u8 {
        // SAFETY: Transmute checks that Self is same size as u8.
        unsafe { core::mem::transmute::<Self, u8>(self) }
    }
}

#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub(crate) struct Instr<'a> {
    ptr: NonNull<Instruction>,
    _lt: PhantomData<&'a [u8]>,
}

impl<'a> Instr<'a> {
    pub(super) fn consume_instruction(instructions: &'a [u8], ip: &mut usize) -> Option<Self> {
        let Some(kind_at) = instructions.get(*ip) else {
            return None;
        };
        let Ok(kind) = Instruction::try_from(*kind_at) else {
            panic_invalid_instruction()
        };
        // SAFETY: we checked above the kind is Ok.
        let ptr = NonNull::from_ref(kind_at).cast::<Instruction>();

        let byte_count = match kind.argument_count() {
            value @ (0 | 1 | 2) => (value * 2) as usize,
            _ => panic_invalid_instruction(),
        };
        *ip += byte_count + 1;
        // If we went past the instructions length (instead of
        // one-over-the-edge) then we have over-indexed.
        if *ip > instructions.len() {
            panic_invalid_instruction()
        }
        Some(Instr {
            ptr,
            _lt: PhantomData,
        })
    }

    pub(crate) fn kind(self) -> &'a Instruction {
        // SAFETY: Instr creation has checked that the pointer is valid.
        unsafe { self.ptr.as_ref() }
    }

    /// Get a reference to the first 16-bit argument.
    ///
    /// # Safety
    ///
    /// The kind must have at least one argument.
    unsafe fn first_arg_ref(self) -> &'a [u8; 2] {
        // SAFETY: caller guarantees we have initialised data in the buffer,
        // Instr guarantees that the reference lifetime is valid.
        unsafe { self.ptr.cast::<u8>().add(1).cast::<[u8; 2]>().as_ref() }
    }

    /// Get a reference to the first 16-bit argument.
    ///
    /// # Safety
    ///
    /// The kind must have at least two arguments.
    unsafe fn second_arg_ref(self) -> &'a [u8; 2] {
        // SAFETY: caller guarantees we have initialised data in the buffer,
        // Instr guarantees that the reference lifetime is valid.
        unsafe {
            self.ptr
                .cast::<u8>()
                .add(1)
                .cast::<[u8; 2]>()
                .add(1)
                .as_ref()
        }
    }

    /// Get a reference to a 32-bit double argument.
    ///
    /// # Safety
    ///
    /// The kind must have at least two arguments.
    unsafe fn double_arg_ref(self) -> &'a [u8; 4] {
        // SAFETY: caller guarantees we have initialised data in the buffer,
        // Instr guarantees that the reference lifetime is valid.
        unsafe { self.ptr.cast::<u8>().add(1).cast::<[u8; 4]>().as_ref() }
    }

    pub(crate) fn get_first_arg(&self) -> IndexType {
        // SAFETY: argument count checked in consume_instruction.
        IndexType::from_ne_bytes(unsafe { *self.first_arg_ref() })
    }

    pub(crate) fn get_first_index(&self) -> usize {
        self.get_first_arg() as usize
    }

    fn get_second_arg(&self) -> IndexType {
        // SAFETY: argument count checked in consume_instruction.
        IndexType::from_ne_bytes(unsafe { *self.second_arg_ref() })
    }

    pub(crate) fn get_second_index(&self) -> usize {
        self.get_second_arg() as usize
    }

    pub(crate) fn get_first_bool(&self) -> bool {
        let first_arg = self.get_first_arg();
        if first_arg == 0 || first_arg == 1 {
            first_arg == 1
        } else {
            panic_first_argument_not_boolean()
        }
    }

    pub(crate) fn get_second_bool(&self) -> bool {
        let second_arg = self.get_second_arg();
        if second_arg == 0 || second_arg == 1 {
            second_arg == 1
        } else {
            panic_second_argument_not_boolean()
        }
    }

    pub(crate) fn get_jump_slot(&self) -> usize {
        debug_assert!(self.kind().has_jump_slot());
        // SAFETY: Argument count checked.
        u32::from_le_bytes(unsafe { *self.double_arg_ref() }) as usize
    }

    pub(crate) fn debug_print(
        &self,
        agent: &mut Agent,
        ip: usize,
        exe: Scoped<Executable>,
        gc: NoGcScope,
    ) {
        match self.kind().argument_count() {
            0 => {
                eprintln!("  {}: {:?}", ip, self.kind());
            }
            1 => {
                let arg0 = self.get_first_arg();
                eprintln!(
                    "  {}: {:?}({})",
                    ip,
                    self.kind(),
                    Self::print_single_arg(agent, *self.kind(), arg0, exe, gc)
                );
            }
            2 => {
                if self.kind().has_jump_slot() {
                    let jump_slot = self.get_jump_slot();
                    eprintln!("  {}: {:?}({})", ip, self.kind(), jump_slot);
                } else {
                    let arg0 = self.get_first_arg();
                    let arg1 = self.get_second_arg();
                    eprintln!(
                        "  {}: {:?}({})",
                        ip,
                        self.kind(),
                        Self::print_two_args(agent, *self.kind(), arg0, arg1, exe, gc)
                    );
                }
            }
            _ => unreachable!(),
        }
    }

    fn print_single_arg(
        agent: &mut Agent,
        kind: Instruction,
        arg: IndexType,
        exe: Scoped<Executable>,
        gc: NoGcScope,
    ) -> std::string::String {
        let index = arg as usize;
        debug_assert!(kind.argument_count() == 1);
        if kind.has_constant_index() {
            debug_print_constant(agent, exe, index, gc)
        } else if kind.has_jump_slot() {
            arg.to_string()
        } else if kind.has_identifier_index() {
            debug_print_identifier(agent, exe, index, gc)
        } else if kind.has_function_expression_index() {
            if kind == Instruction::InstantiateArrowFunctionExpression {
                let expr = exe.fetch_arrow_function_expression(agent, index);
                let arrow_fn = expr.expression.get();
                format!(
                    "({}) => {{}}",
                    arrow_fn
                        .params
                        .iter_bindings()
                        .map(debug_print_binding_pattern)
                        .collect::<Vec<std::string::String>>()
                        .join(", ")
                )
            } else {
                let expr = exe.fetch_function_expression(agent, index, gc);
                let normal_fn = expr.expression.get();
                format!(
                    "function {}({})",
                    normal_fn.name().map_or("anonymous", |a| a.as_str()),
                    normal_fn
                        .params
                        .iter_bindings()
                        .map(debug_print_binding_pattern)
                        .collect::<Vec<std::string::String>>()
                        .join(", ")
                )
            }
        } else if kind == Instruction::ClassDefineDefaultConstructor {
            if exe.fetch_class_initializer_bytecode(agent, index, gc).1 {
                "{ super() }".to_string()
            } else {
                "{}".to_string()
            }
        } else {
            // Immediate
            arg.to_string()
        }
    }

    fn print_two_args(
        agent: &mut Agent,
        kind: Instruction,
        arg0: IndexType,
        arg1: IndexType,
        exe: Scoped<Executable>,
        gc: NoGcScope,
    ) -> std::string::String {
        match kind {
            Instruction::BeginSimpleArrayBindingPattern => {
                let arg1_equals = arg1 == 1;
                format!("{{ length: {arg0}, env: {arg1_equals} }}")
            }
            Instruction::BindingPatternBindNamed => {
                format!(
                    "{{ {}: {} }}",
                    debug_print_constant(agent, exe.clone(), arg1 as usize, gc),
                    debug_print_identifier(agent, exe, arg0 as usize, gc)
                )
            }
            Instruction::BindingPatternBindToIndex => {
                format!(
                    "{{ {}: stack[{}] }}",
                    debug_print_constant(agent, exe, arg1 as usize, gc),
                    arg0,
                )
            }
            Instruction::ClassDefineConstructor => {
                if arg1 == 1 {
                    "constructor() { super() }".to_string()
                } else {
                    "constructor()".to_string()
                }
            }
            Instruction::ClassDefinePrivateMethod => {
                let is_static = arg1 & 0b100 == 0b100;
                let static_prefix = if is_static { "static " } else { "" };
                let is_get = arg1 & 0b1 == 0b1;
                let is_set = arg1 & 0b10 == 0b10;
                let accessor_prefix = if is_get {
                    "get "
                } else if is_set {
                    "set "
                } else {
                    ""
                };
                format!("{static_prefix}{accessor_prefix} #function() {{}}")
            }
            Instruction::ClassDefinePrivateProperty => {
                let key = debug_print_identifier(agent, exe, arg0 as usize, gc);
                let is_static = arg1 != 0;
                let static_prefix = if is_static { "static " } else { "" };
                format!("{static_prefix}#{key}")
            }
            Instruction::InitializeVariableEnvironment => {
                let arg1_equals = arg1 == 1;
                format!("{{ var count: {arg0}, strict: {arg1_equals} }}")
            }
            Instruction::ObjectDefineGetter => "get function() {}".to_string(),
            Instruction::ObjectDefineMethod => "function() {}".to_string(),
            Instruction::ObjectDefineSetter => "set function() {}".to_string(),
            Instruction::ResolveBindingWithCache => {
                let key = debug_print_identifier(agent, exe, arg0 as usize, gc);
                format!("{key}, {arg1}")
            }
            _ => unreachable!("{kind:?}"),
        }
    }
}

fn debug_print_constant(
    agent: &mut Agent,
    exe: Scoped<Executable>,
    index: usize,
    gc: NoGcScope,
) -> std::string::String {
    let constant = exe.fetch_constant(agent, index, gc);
    if let Ok(string_constant) = String::try_from(constant) {
        format!("\"{}\"", string_constant.to_string_lossy_(agent))
    } else {
        constant
            .try_string_repr(agent, gc)
            .to_string_lossy_(agent)
            .to_string()
    }
}

fn debug_print_identifier(
    agent: &Agent,
    exe: Scoped<Executable>,
    index: usize,
    gc: NoGcScope,
) -> std::string::String {
    let identifier = exe.fetch_property_key(agent, index, gc);
    identifier.as_display(agent).to_string()
}

fn debug_print_binding_pattern(b: &BindingPattern) -> std::string::String {
    match b {
        oxc_ast::ast::BindingPattern::BindingIdentifier(b) => b.name.to_string(),
        oxc_ast::ast::BindingPattern::ObjectPattern(b) => {
            let mut prop_strings = b
                .properties
                .iter()
                .map(|b| {
                    format!(
                        "{}: {}",
                        debug_print_property_key(&b.key),
                        debug_print_binding_pattern(&b.value)
                    )
                })
                .collect::<Vec<std::string::String>>();
            if let Some(rest) = &b.rest {
                prop_strings.push(format!(
                    "...{}",
                    debug_print_binding_pattern(&rest.argument)
                ));
            }
            format!("{{ {} }}", prop_strings.join(", "))
        }
        oxc_ast::ast::BindingPattern::ArrayPattern(b) => {
            let mut elem_strings = b
                .elements
                .iter()
                .map(|b| {
                    if let Some(b) = b {
                        debug_print_binding_pattern(b)
                    } else {
                        "".to_string()
                    }
                })
                .collect::<Vec<std::string::String>>();
            if let Some(rest) = &b.rest {
                elem_strings.push(format!(
                    "...{}",
                    debug_print_binding_pattern(&rest.argument)
                ));
            }
            format!("{{ {} }}", elem_strings.join(", "))
        }
        oxc_ast::ast::BindingPattern::AssignmentPattern(b) => {
            format!(
                "{} = {}",
                debug_print_binding_pattern(&b.left),
                debug_print_expression(&b.right)
            )
        }
    }
}

fn debug_print_property_key<'a>(pk: &'a oxc_ast::ast::PropertyKey) -> &'a str {
    match pk {
        oxc_ast::ast::PropertyKey::StaticIdentifier(n) => n.name.as_str(),
        oxc_ast::ast::PropertyKey::PrivateIdentifier(p) => p.name.as_str(),
        _ => "[computed]",
    }
}

fn debug_print_expression(expr: &oxc_ast::ast::Expression) -> std::string::String {
    match expr {
        oxc_ast::ast::Expression::BooleanLiteral(l) => l.value.to_string(),
        oxc_ast::ast::Expression::NullLiteral(_) => "null".to_string(),
        oxc_ast::ast::Expression::NumericLiteral(l) => l.value.to_js_string(),
        oxc_ast::ast::Expression::BigIntLiteral(l) => l.raw.as_ref().unwrap().to_string(),
        oxc_ast::ast::Expression::RegExpLiteral(l) => l.raw.as_ref().unwrap().to_string(),
        oxc_ast::ast::Expression::StringLiteral(l) => l.raw.as_ref().unwrap().to_string(),
        oxc_ast::ast::Expression::TemplateLiteral(_) => "`...`".to_string(),
        _ => "[computed]".to_string(),
    }
}

#[derive(Debug)]
pub(crate) struct InstructionIter<'a> {
    instructions: &'a [u8],
    pub(crate) index: usize,
}

impl<'a> InstructionIter<'a> {
    pub(crate) fn new(instructions: &'a [u8]) -> Self {
        Self {
            instructions,
            index: 0,
        }
    }
}

impl<'a> Iterator for InstructionIter<'a> {
    type Item = (usize, Instr<'a>);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        let instr = Instr::consume_instruction(self.instructions, &mut self.index)?;

        Some((index, instr))
    }
}

impl TryFrom<u8> for Instruction {
    type Error = ();

    #[inline]
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > Instruction::MAX.as_u8() {
            Err(())
        } else {
            // SAFETY: Checked that we're within Instruction::Debug.
            Ok(unsafe { core::mem::transmute::<u8, Instruction>(value) })
        }
    }
}

// These panics are marked as cold and moved into their own non-inlineable
// functions, they should be called very infrequently and by doing this
// eliminates the setup and teardown of stack frames keeping the hot-paths
// as hot as possible.
//
// For reference, on my ARM64 machine it reduced the assembly instructions
// needed for `get_first_arg` from 34 all the way down to 14 and showing
// a minor but noticeable performance increase when running the "richards"
// benchmark.

#[cold]
#[inline(never)]
fn panic_invalid_instruction() -> ! {
    panic!("Invalid bytecode instruction")
}

#[cold]
#[inline(never)]
fn panic_first_argument_not_boolean() -> ! {
    panic!("First argument was not a boolean")
}

#[cold]
#[inline(never)]
fn panic_second_argument_not_boolean() -> ! {
    panic!("Second argument was not a boolean")
}
