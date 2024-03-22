use oxc_syntax::operator::BinaryOperator;

use super::IndexType;

/// ## Notes
///
/// - This is inspired by and/or copied from Kiesel engine:
///   Copyright (c) 2023-2024 Linus Groh
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Debug,
    /// Store ApplyStringOrNumericBinaryOperator() as the result value.
    ApplyStringOrNumericBinaryOperator(BinaryOperator),
    /// Store ArrayCreate(0) as the result value.
    ///
    /// This instruction has one immediate argument that is the minimum
    /// number of elements to expect.
    ArrayCreate,
    /// Push a value into an array
    ArrayPush,
    /// Set an array's value at the given index.
    ArraySetValue,
    /// Set the length property of an array to the given index.
    ArraySetLength,
    /// Performs steps 2-4 from the [UnaryExpression ~ Runtime Semantics](https://tc39.es/ecma262/#sec-bitwise-not-operator-runtime-semantics-evaluation).
    BitwiseNot,
    /// Create a catch binding for the given name and populate it with the
    /// stored exception.
    CreateCatchBinding,
    /// Apply the delete operation to the evaluated expression and set it as
    /// the result value.
    Delete,
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
    /// stack afterwards is the constructor to
    /// call.
    EvaluateNew,
    /// Store EvaluatePropertyAccessWithExpressionKey() as the result value.
    EvaluatePropertyAccessWithExpressionKey,
    /// Store EvaluatePropertyAccessWithIdentifierKey() as the result value.
    EvaluatePropertyAccessWithIdentifierKey,
    /// Store [GetValue()](https://tc39.es/ecma262/#sec-getvalue) as the result
    /// value.
    ///
    /// #### Note
    /// We only call `GetValue` on reference values. This can be statically
    /// analysed from the AST. Non-reference values are already in the result
    /// value so a `GetValue` call would be a no-op.
    GetValue,
    /// Same as GetValue without taking the reference slot. Used for reference
    /// property updates.
    GetValueKeepReference,
    /// Compare the last two values on the stack using the '>' operator rules.
    GreaterThan,
    /// Compare the last two values on the stack using the '>=' operator rules.
    GreaterThanEquals,
    /// Store HasProperty() as the result value.
    HasProperty,
    Increment,
    /// Store InstanceofOperator() as the result value.
    InstanceofOperator,
    /// Store InstantiateArrowFunctionExpression() as the result value.
    InstantiateArrowFunctionExpression,
    /// Store InstantiateOrdinaryFunctionExpression() as the result value.
    InstantiateOrdinaryFunctionExpression,
    /// Store IsLooselyEqual() as the result value.
    IsLooselyEqual,
    /// Store IsStrictlyEqual() as the result value.
    IsStrictlyEqual,
    /// Jump to another instruction by setting the instruction pointer.
    Jump,
    /// Jump to another instruction by setting the instruction pointer
    /// if the current result is falsey.
    JumpIfNot,
    /// Compare the last two values on the stack using the '<' operator rules.
    LessThan,
    /// Compare the last two values on the stack using the '<=' operator rules.
    LessThanEquals,
    /// Load the result value and add it to the stack.
    Load,
    /// Load a constant and add it to the stack.
    LoadConstant,
    /// Determine the this value for an upcoming evaluate_call instruction and
    /// add it to the stack.
    LoadThisValue,
    /// Performs steps 2-4 from the [UnaryExpression ! Runtime Semantics](https://tc39.es/ecma262/#sec-logical-not-operator-runtime-semantics-evaluation).
    LogicalNot,
    /// Store OrdinaryObjectCreate(%Object.prototype%) on the stack.
    ObjectCreate,
    /// Set an object's property to the key/value pair from the last two values
    /// on the stack.
    ObjectSetProperty,
    /// Pop a jump target for uncaught exceptions
    PopExceptionJumpTarget,
    /// Pop the last stored reference.
    PopReference,
    /// Push a jump target for uncaught exceptions
    PushExceptionJumpTarget,
    /// Push the last evaluated reference, if any.
    PushReference,
    /// Call PutValue() with the last reference on the reference stack and the
    /// result value.
    PutValue,
    /// Store ResolveBinding() as the result value.
    ResolveBinding,
    /// Store ResolveThisBinding() as the result value.
    ResolveThisBinding,
    /// Rethrow the stored exception, if any.
    RethrowExceptionIfAny,
    /// Stop bytecode execution, indicating a return from the current function.
    Return,
    /// Store the last value from the stack as the result value.
    Store,
    /// Store a constant as the result value.
    StoreConstant,
    /// Throw the last value from the stack as an exception.
    Throw,
    /// Store ToNumber() as the result value.
    ToNumber,
    /// Store ToNumeric() as the result value.
    ToNumeric,
    /// Apply the typeof operation to the evaluated expression and set it as
    /// the result value.
    Typeof,
    /// Performs steps 3 and 4 from the [UnaryExpression - Runtime Semantics](https://tc39.es/ecma262/#sec-unary-minus-operator-runtime-semantics-evaluation).
    UnaryMinus,
    /// Perform CreateImmutableBinding in the running execution context's
    /// LexicalEnvironment with an identifier parameter and `true`
    CreateImmutableBinding,
    /// Perform CreateMutableBinding in the running execution context's
    /// LexicalEnvironment with an identifier parameter and `false`
    CreateMutableBinding,
    /// Perform InitializeReferencedBinding with parameters reference (V) and
    /// result (W).
    InitializeReferencedBinding,
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
    /// Reset the running execution context's LexicalEnvironment to its current
    /// value's \[\[OuterEnv]].
    ExitDeclarativeEnvironment,
}

impl Instruction {
    pub fn argument_count(self) -> u8 {
        match self {
            Self::ArrayCreate
            | Self::ArraySetLength
            | Self::ArraySetValue
            | Self::CreateCatchBinding
            | Self::EvaluateCall
            | Self::EvaluateNew
            | Self::EvaluatePropertyAccessWithIdentifierKey
            | Self::InstantiateArrowFunctionExpression
            | Self::InstantiateOrdinaryFunctionExpression
            | Self::Jump
            | Self::JumpIfNot
            | Self::LoadConstant
            | Self::PushExceptionJumpTarget
            | Self::StoreConstant
            | Self::ResolveBinding
            | Self::CreateImmutableBinding
            | Self::CreateMutableBinding => 1,
            _ => 0,
        }
    }

    pub fn has_constant_index(self) -> bool {
        matches!(self, Self::LoadConstant | Self::StoreConstant)
    }

    pub fn has_identifier_index(self) -> bool {
        matches!(
            self,
            Self::CreateCatchBinding
                | Self::EvaluatePropertyAccessWithIdentifierKey
                | Self::ResolveBinding
                | Self::CreateImmutableBinding
                | Self::CreateMutableBinding
        )
    }

    pub fn has_function_expression_index(self) -> bool {
        matches!(
            self,
            Self::InstantiateArrowFunctionExpression | Self::InstantiateOrdinaryFunctionExpression
        )
    }

    pub fn has_jump_slot(self) -> bool {
        matches!(self, Self::Jump | Self::JumpIfNot)
    }

    pub fn as_u8(self) -> u8 {
        unsafe { std::mem::transmute::<Self, u8>(self) }
    }
}

#[derive(Debug)]
pub(crate) struct Instr {
    pub kind: Instruction,
    pub args: [Option<IndexType>; 2],
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

impl Iterator for InstructionIter<'_> {
    type Item = (usize, Instr);

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.instructions.len() {
            return None;
        }
        let index = self.index;

        let kind: Instruction = unsafe { std::mem::transmute(self.instructions[self.index]) };
        self.index += 1;

        let mut args: [Option<IndexType>; 2] = [None, None];

        for item in args.iter_mut().take(kind.argument_count() as usize) {
            let length = self.instructions[self.index..].len();
            if length >= 2 {
                let bytes = IndexType::from_ne_bytes(unsafe {
                    *std::mem::transmute::<*const u8, *const [u8; 2]>(
                        self.instructions[self.index..].as_ptr(),
                    )
                });
                self.index += 2;
                *item = Some(bytes);
            } else {
                self.index += 1;
                *item = None;
            }
        }

        Some((index, Instr { kind, args }))
    }
}
