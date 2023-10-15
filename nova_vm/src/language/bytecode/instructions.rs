use super::IndexType;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
#[non_exhaustive]
pub enum Instruction {
    /// Store ApplyStringOrNumericBinaryOperator() as the result value.
    ApplyStringOrNumericBinaryOperator,
    /// Store ArrayCreate(0) as the result value.
    ArrayCreate,
    /// Set an array's value at the given index.
    ArraySetValue,
    /// Set the length property of an array to the given index.
    ArraySetLength,
    /// Apply bitwise NOT to the last value on the stack and store it as the result value.
    BitwiseNot,
    /// Create a catch binding for the given name and populate it with the stored exception.
    CreateCatchBinding,
    /// Apply the delete operation to the evaluated expression and set it as the result value.
    Delete,
    /// Store EvaluateCall() as the result value.
    /// This instruction has the number of argument values that need to be popped from the stack
    /// (last to first) as an argument, the values on the stack afterwards are the this value and
    /// lastly the function to call.
    EvaluateCall,
    /// Store EvaluateNew() as the result value.
    /// This instruction has the number of argument values that need to be popped from the stack
    /// (last to first) as an argument, the value on the stack afterwards is the constructor to
    /// call.
    EvaluateNew,
    /// Store EvaluatePropertyAccessWithExpressionKey() as the result value.
    EvaluatePropertyAccessWithExpressionKey,
    /// Store EvaluatePropertyAccessWithIdentifierKey() as the result value.
    EvaluatePropertyAccessWithIdentifierKey,
    /// Store GetValue() as the result value.
    GetValue,
    /// Compare the last two values on the stack using the '>' operator rules.
    GreaterThan,
    /// Compare the last two values on the stack using the '>=' operator rules.
    GreaterThanEquals,
    /// Store HasProperty() as the result value.
    HasProperty,
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
    /// Jump to one of two other instructions depending on whether the last value on the stack is
    /// truthy or not.
    JumpConditional,
    /// Compare the last two values on the stack using the '<' operator rules.
    LessThan,
    /// Compare the last two values on the stack using the '<=' operator rules.
    LessThanEquals,
    /// Load the result value and add it to the stack.
    Load,
    /// Load a constant and add it to the stack.
    LoadConstant,
    /// Determine the this value for an upcoming evaluate_call instruction and add it to the stack.
    LoadThisValue,
    /// Apply logical NOT to the last value on the stack and store it as the result value.
    LogicalNot,
    /// Store OrdinaryObjectCreate(%Object.prototype%) as the result value.
    ObjectCreate,
    /// Set an object's property to the key/value pair from the last two values on the stack.
    ObjectSetProperty,
    /// Pop a jump target for uncaught exceptions
    PopExceptionJumpTarget,
    /// Pop the last stored reference.
    PopReference,
    /// Push a jump target for uncaught exceptions
    PushExceptionJumpTarget,
    /// Push the last evaluated reference, if any.
    PushReference,
    /// Call PutValue() with the last reference on the reference stack and the result value.
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
    /// Apply the typeof operation to the evaluated expression and set it as the result value.
    Typeof,
    /// Store Number::unaryMinus() / BigInt::unaryMinus() as the result value.
    UnaryMinus,
}

impl Instruction {
    pub fn argument_count(self) -> u8 {
        return match self {
            Self::EvaluateCall
            | Self::EvaluatePropertyAccessWithIdentifierKey
            | Self::JumpConditional
            | Self::ResolveBinding => 2,
            Self::ApplyStringOrNumericBinaryOperator
            | Self::ArraySetLength
            | Self::ArraySetValue
            | Self::CreateCatchBinding
            | Self::EvaluateNew
            | Self::EvaluatePropertyAccessWithExpressionKey
            | Self::InstantiateArrowFunctionExpression
            | Self::InstantiateOrdinaryFunctionExpression
            | Self::Jump
            | Self::LoadConstant
            | Self::PushExceptionJumpTarget
            | Self::StoreConstant => 1,
            _ => 0,
        };
    }

    pub fn has_constant_index(self) -> bool {
        return match self {
            Self::LoadConstant | Self::StoreConstant => true,
            _ => false,
        };
    }

    pub fn has_identifier_index(self) -> bool {
        return match self {
            Self::CreateCatchBinding
            | Self::EvaluatePropertyAccessWithIdentifierKey
            | Self::ResolveBinding => true,
            _ => false,
        };
    }

    pub fn has_function_expression_index(self) -> bool {
        return match self {
            Self::InstantiateArrowFunctionExpression
            | Self::InstantiateOrdinaryFunctionExpression => true,
            _ => false,
        };
    }
}

#[derive(Debug)]
pub struct Instr {
    pub kind: Instruction,
    pub args: [Option<IndexType>; 2],
}

#[derive(Debug)]
pub struct InstructionIter<'a> {
    instructions: &'a [Instruction],
    index: usize,
}

impl<'a> InstructionIter<'a> {
    pub fn new(instructions: &'a [Instruction]) -> Self {
        Self {
            instructions,
            index: 0,
        }
    }
}

impl Iterator for InstructionIter<'_> {
    type Item = Instr;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.instructions.len() {
            return None;
        }

        let kind = self.instructions[self.index];
        self.index += 1;

        let mut args: [Option<IndexType>; 2] = [None, None];

        for i in 0..kind.argument_count() as usize {
            let bytes: &[IndexType] =
                unsafe { std::mem::transmute(&self.instructions[self.index..]) };
            self.index += 2;
            args[i] = Some(bytes[0]);
        }

        Some(Instr { kind, args })
    }
}
