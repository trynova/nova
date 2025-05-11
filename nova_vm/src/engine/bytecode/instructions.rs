// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use oxc_ast::ast::BindingPattern;
use oxc_syntax::{number::ToJsString, operator::BinaryOperator};

use crate::{
    ecmascript::{execution::Agent, types::String},
    engine::{Scoped, context::NoGcScope},
};

use super::{Executable, IndexType};

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
    /// Push a hole into an array
    ArrayElision,
    /// Performs Await() on the result value, and after resuming, stores the
    /// promise result as the result value.
    Await,
    /// Performs steps 2-4 from the [UnaryExpression ~ Runtime Semantics](https://tc39.es/ecma262/#sec-bitwise-not-operator-runtime-semantics-evaluation).
    BitwiseNot,
    /// Create a catch binding for the given name and populate it with the
    /// stored exception.
    CreateCatchBinding,
    /// Performs CreateUnmappedArgumentsObject() on the arguments list present
    /// in the iterator stack, and stores the created arguments object as the
    /// result value.
    CreateUnmappedArgumentsObject,
    /// Performs CopyDataProperties() with the source being the result value and
    /// the target object being at the top of the stack. The excluded names list
    /// will be empty.
    CopyDataProperties,
    /// Performs CopyDataProperties() into a newly created object and returns it.
    /// The source object will be on the result value, and the excluded names
    /// will be read from the reference stack, with the number of names passed
    /// in an immediate.
    CopyDataPropertiesIntoObject,
    /// Apply the delete operation to the evaluated expression and set it as
    /// the result value.
    Delete,
    /// Call the `eval` function in a direct way.
    ///
    /// If the `eval` identifier points to the current realm's eval intrinsic
    /// function, then it performs a direct eval. Otherwise, it loads the value
    /// that identifier points to, and calls it.
    ///
    /// This instruction has the number of argument values that need to be
    /// popped from the stack (last to first) as an argument.
    DirectEvalCall,
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
    /// property updates and function calls (where `this` comes from the
    /// reference).
    GetValueKeepReference,
    /// Compare the last two values on the stack using the '>' operator rules.
    GreaterThan,
    /// Compare the last two values on the stack using the '>=' operator rules.
    GreaterThanEquals,
    /// Store HasProperty() as the result value.
    HasProperty,
    Increment,
    Decrement,
    /// Store InstanceofOperator() as the result value.
    InstanceofOperator,
    /// Store InstantiateArrowFunctionExpression() as the result value.
    InstantiateArrowFunctionExpression,
    /// Store InstantiateOrdinaryFunctionExpression() as the result value.
    InstantiateOrdinaryFunctionExpression,
    /// Create a class constructor and store it as the result value.
    ///
    /// The class name should be found at the top of the stack.
    /// If the class is a derived class, then the parent constructor should
    /// also be on the stack after the class name.
    ClassDefineConstructor,
    /// Store CreateBuiltinFunction(defaultConstructor, 0, className) as the
    /// result value.
    ClassDefineDefaultConstructor,
    /// Store IsLooselyEqual() as the result value.
    IsLooselyEqual,
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
    /// Store true as the result value if the current result value is an
    /// object.
    IsObject,
    /// Call IsConstructor() on the current result value and store the result
    /// as the result value.
    IsConstructor,
    /// Jump to another instruction by setting the instruction pointer.
    Jump,
    /// Jump to another instruction by setting the instruction pointer
    /// if the current result is falsey.
    JumpIfNot,
    /// Jump to another intrsuction by setting the instruction pointer if the
    /// current result is `true`.
    JumpIfTrue,
    /// Compare the last two values on the stack using the '<' operator rules.
    LessThan,
    /// Compare the last two values on the stack using the '<=' operator rules.
    LessThanEquals,
    /// Load the result value and add it to the stack.
    Load,
    /// Add the result value to the stack, without removing it as the result
    /// value.
    LoadCopy,
    /// Load a constant and add it to the stack.
    LoadConstant,
    /// Swaps the last value in the stack and the result value.
    LoadStoreSwap,
    /// Determine the this value for an upcoming evaluate_call instruction and
    /// add it to the stack.
    LoadThisValue,
    /// Swap the last two values on the stack.
    Swap,
    /// Performs steps 2-4 from the [UnaryExpression ! Runtime Semantics](https://tc39.es/ecma262/#sec-logical-not-operator-runtime-semantics-evaluation).
    LogicalNot,
    /// Store OrdinaryObjectCreate(%Object.prototype%) on the stack.
    ObjectCreate,
    /// Call CreateDataPropertyOrThrow(object, key, value) with value being the
    /// result value, key being the top stack value and object being the second
    /// stack value. The object is not popped from the stack.
    ObjectDefineProperty,
    /// Create and define a method on an object.
    ///
    /// The key is at the top stack value, the object is second on the stack.
    ObjectDefineMethod,
    ObjectDefineGetter,
    ObjectDefineSetter,
    /// Call `object[[SetPrototypeOf]](value)` on the object on the stack using
    /// the current result value as the parameter.
    ObjectSetPrototype,
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
    /// Store a copy of the last value from the stack as the result value.
    StoreCopy,
    /// Store a constant as the result value.
    StoreConstant,
    /// Take N items from the stack and string-concatenate them together.
    StringConcat,
    /// Throw the result value as an exception.
    Throw,
    /// Throw a new Error object as an exception with the result value as the
    /// message.
    ///
    /// The error subtype is determined by an immediate value.
    ThrowError,
    /// Store ToNumber() as the result value.
    ToNumber,
    /// Store ToNumeric() as the result value.
    ToNumeric,
    /// Store ToObject() as the result value.
    ToObject,
    /// Store ToString() as the result value.
    ToString,
    /// Apply the typeof operation to the evaluated expression and set it as
    /// the result value.
    Typeof,
    /// Performs steps 3 and 4 from the [UnaryExpression - Runtime Semantics](https://tc39.es/ecma262/#sec-unary-minus-operator-runtime-semantics-evaluation).
    UnaryMinus,
    /// Performs Yield() on the result value, and after resuming, stores the
    /// value passed to `next()` as the result value.
    Yield,
    /// Perform CreateImmutableBinding in the running execution context's
    /// LexicalEnvironment with an identifier parameter and `true`
    CreateImmutableBinding,
    /// Perform CreateMutableBinding in the running execution context's
    /// LexicalEnvironment with an identifier parameter and `false`
    CreateMutableBinding,
    /// Perform InitializeReferencedBinding with parameters reference (V) and
    /// result (W).
    InitializeReferencedBinding,
    /// Create a new VariableEnvironment and initialize it with variable names
    /// and values from the stack, where each name comes before the value.
    /// The first immediate argument is the number of variables to initialize.
    /// The second immediate is a boolean which is true if LexicalEnvironment
    /// should also be set to this new environment (true in strict mode), or
    /// false if it should be set to a new descendant declarative environment.
    InitializeVariableEnvironment,
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
    /// binding and \[\[FunctionObject]]. This is used for class static
    /// initializers.
    EnterClassStaticElementEnvironment,
    /// Reset the running execution context's LexicalEnvironment to its current
    /// value's \[\[OuterEnv]].
    ExitDeclarativeEnvironment,
    ExitVariableEnvironment,
    /// Begin binding values using destructuring
    BeginSimpleObjectBindingPattern,
    /// Begin binding values using a sync iterator for known repetitions
    BeginSimpleArrayBindingPattern,
    /// In array binding patterns, bind the current result to the given
    /// identifier. In object binding patterns, bind the object's property with
    /// the identifier's name.
    ///
    /// ```js
    /// const { a } = x;
    /// const [a] = x;
    /// ```
    BindingPatternBind,
    /// Bind an object property to an identifier with a different name. The
    /// constant given as the second argument is the property key.
    ///
    /// ```js
    /// const { a: b } = x;
    /// ```
    BindingPatternBindNamed,
    /// Bind all remaining values to given identifier
    ///
    /// ```js
    /// const { a, ...b } = x;
    /// const [a, ...b] = x;
    /// ```
    BindingPatternBindRest,
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
    /// Perform IteratorStepValue on the current iterator and jump to
    /// index if iterator completed.
    IteratorStepValue,
    /// Perform IteratorStepValue on the current iterator, putting the resulting
    /// value on the result value, or undefined if the iterator has completed.
    ///
    /// When the iterator has completed, rather than popping it off the stack,
    /// it sets it to `VmIterator::EmptyIterator` so further reads and closes
    /// aren't observable.
    IteratorStepValueOrUndefined,
    /// Consume the remainder of the iterator, and produce a new array with
    /// those elements. This pops the iterator off the iterator stack.
    IteratorRestIntoArray,
    /// Perform CloseIterator on the current iterator
    IteratorClose,
    /// Perform AsyncCloseIterator on the current iterator
    AsyncIteratorClose,
    /// Perform CloseIterator on the current iterator with the current result
    /// as a thrown value.
    IteratorCloseWithError,
    /// Perform AsyncCloseIterator on the current iterator with the current
    /// result as a thrown value.
    ///
    /// This handling will store the error on the stack and perform an Await if
    /// needed, otherwise it will rethrows the error.
    AsyncIteratorCloseWithError,
    /// Store GetNewTarget() as the result value.
    GetNewTarget,
}

impl Instruction {
    pub fn argument_count(self) -> u8 {
        match self {
            // Number of repetitions and lexical status
            Self::BeginSimpleArrayBindingPattern
            | Self::BindingPatternBindNamed
            | Self::ClassDefineConstructor
            | Self::InitializeVariableEnvironment
            | Self::IteratorStepValue
            | Self::Jump
            | Self::JumpIfNot
            | Self::JumpIfTrue
            | Self::ObjectDefineGetter
            | Self::ObjectDefineMethod
            | Self::ObjectDefineSetter
            | Self::PushExceptionJumpTarget => 2,
            Self::ArrayCreate
            | Self::ArraySetValue
            | Self::BeginSimpleObjectBindingPattern
            | Self::BindingPatternBind
            | Self::BindingPatternBindRest
            | Self::BindingPatternGetValueNamed
            | Self::ClassDefineDefaultConstructor
            | Self::CopyDataPropertiesIntoObject
            | Self::CreateCatchBinding
            | Self::CreateImmutableBinding
            | Self::CreateMutableBinding
            | Self::DirectEvalCall
            | Self::EvaluateCall
            | Self::EvaluateNew
            | Self::EvaluateSuper
            | Self::EvaluatePropertyAccessWithIdentifierKey
            | Self::InstantiateArrowFunctionExpression
            | Self::InstantiateOrdinaryFunctionExpression
            | Self::LoadConstant
            | Self::ResolveBinding
            | Self::StoreConstant
            | Self::StringConcat
            | Self::ThrowError => 1,
            _ => 0,
        }
    }

    pub fn has_double_arg(self) -> bool {
        debug_assert_eq!(self.argument_count(), 2);
        matches!(
            self,
            Self::IteratorStepValue
                | Self::Jump
                | Self::JumpIfNot
                | Self::JumpIfTrue
                | Self::PushExceptionJumpTarget
        )
    }

    pub fn has_constant_index(self) -> bool {
        matches!(
            self,
            Self::LoadConstant
                | Self::StoreConstant
                | Self::BindingPatternBindNamed
                | Self::BindingPatternGetValueNamed
        )
    }

    pub fn has_identifier_index(self) -> bool {
        matches!(
            self,
            Self::CreateCatchBinding
                | Self::EvaluatePropertyAccessWithIdentifierKey
                | Self::ResolveBinding
                | Self::CreateImmutableBinding
                | Self::CreateMutableBinding
                | Self::BindingPatternBind
                | Self::BindingPatternBindNamed
                | Self::BindingPatternBindRest
        )
    }

    pub fn has_function_expression_index(self) -> bool {
        matches!(
            self,
            Self::ClassDefineConstructor
                | Self::InstantiateArrowFunctionExpression
                | Self::InstantiateOrdinaryFunctionExpression
                | Self::ObjectDefineGetter
                | Self::ObjectDefineMethod
                | Self::ObjectDefineSetter
        )
    }

    pub fn has_jump_slot(self) -> bool {
        matches!(
            self,
            Self::Jump
                | Self::JumpIfNot
                | Self::JumpIfTrue
                | Self::PushExceptionJumpTarget
                | Self::IteratorStepValue
        )
    }

    pub fn as_u8(self) -> u8 {
        const {
            // Check statically that Instruction hasn't overflowed u8.
            assert!(std::mem::size_of::<Self>() == std::mem::size_of::<u8>());
            assert!(std::mem::align_of::<Self>() == std::mem::align_of::<u8>());
        }
        unsafe { core::mem::transmute::<Self, u8>(self) }
    }
}

union InstructionArgs {
    none: (),
    single_arg: u16,
    two_args: [u16; 2],
    double_arg: u32,
}

impl core::fmt::Debug for InstructionArgs {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct Instr {
    pub kind: Instruction,
    args: InstructionArgs,
}

impl Instr {
    pub(super) fn consume_instruction(instructions: &[u8], ip: &mut usize) -> Option<Instr> {
        let len = instructions.len();
        let cur_ip = *ip;
        if cur_ip >= len {
            return None;
        }
        *ip += 1;
        let kind =
            Instruction::try_from(instructions[cur_ip]).expect("Invalid bytecode instruction");

        let arg_count = kind.argument_count() as usize;

        let cur_ip = *ip;
        match arg_count {
            0 => Some(Instr::new(kind)),
            1 => {
                let bytes: [u8; 2] = [instructions[cur_ip], instructions[cur_ip + 1]];
                let arg0 = IndexType::from_ne_bytes(bytes);
                *ip += 2;
                Some(Instr::new_with_arg(kind, arg0))
            }
            2 => {
                if kind.has_double_arg() {
                    let mut bytes = [0u8; 4];
                    bytes.copy_from_slice(&instructions[cur_ip..cur_ip + 4]);
                    *ip += 4;
                    let arg = u32::from_ne_bytes(bytes);
                    Some(Instr::new_with_double_arg(kind, arg))
                } else {
                    let mut bytes0 = [0u8; 2];
                    let mut bytes1 = [0u8; 2];
                    bytes0.copy_from_slice(&instructions[cur_ip..cur_ip + 2]);
                    bytes1.copy_from_slice(&instructions[cur_ip + 2..cur_ip + 4]);
                    *ip += 4;
                    let arg0 = IndexType::from_ne_bytes(bytes0);
                    let arg1 = IndexType::from_ne_bytes(bytes1);
                    Some(Instr::new_with_two_args(kind, arg0, arg1))
                }
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn new(kind: Instruction) -> Self {
        Self {
            kind,
            args: InstructionArgs { none: () },
        }
    }

    pub(crate) fn new_with_arg(kind: Instruction, arg: u16) -> Self {
        Self {
            kind,
            args: InstructionArgs { single_arg: arg },
        }
    }

    pub(crate) fn new_with_two_args(kind: Instruction, arg1: u16, arg2: u16) -> Self {
        Self {
            kind,
            args: InstructionArgs {
                two_args: [arg1, arg2],
            },
        }
    }

    pub(crate) fn new_with_double_arg(kind: Instruction, arg: u32) -> Self {
        Self {
            kind,
            args: InstructionArgs { double_arg: arg },
        }
    }

    pub(crate) fn get_first_arg(&self) -> IndexType {
        match self.kind.argument_count() {
            // SAFETY: one u16 arg is guaranteed.
            1 => unsafe { self.args.single_arg },
            2 => {
                assert!(!self.kind.has_double_arg());
                // SAFETY: two u16 args are guaranteed.
                unsafe { self.args.two_args[0] }
            }
            _ => panic!("Instruction does not have an argument"),
        }
    }

    pub(crate) fn get_first_index(&self) -> usize {
        match self.kind.argument_count() {
            // SAFETY: one u16 arg is guaranteed.
            1 => unsafe { self.args.single_arg as usize },
            2 => {
                assert!(!self.kind.has_double_arg());
                // SAFETY: two u16 args are guaranteed.
                unsafe { self.args.two_args[0] as usize }
            }
            _ => panic!("Instruction does not have an argument"),
        }
    }

    pub(crate) fn get_second_index(&self) -> usize {
        match self.kind.argument_count() {
            2 => {
                assert!(!self.kind.has_double_arg());
                // SAFETY: two u16 args are guaranteed.
                unsafe { self.args.two_args[1] as usize }
            }
            _ => panic!("Instruction does not have two argument"),
        }
    }

    pub(crate) fn get_first_bool(&self) -> bool {
        let first_arg = match self.kind.argument_count() {
            // SAFETY: one u16 arg is guaranteed.
            1 => unsafe { self.args.single_arg },
            2 => {
                assert!(!self.kind.has_double_arg());
                // SAFETY: two u16 args are guaranteed.
                unsafe { self.args.two_args[1] }
            }
            _ => panic!("Instruction does not have two argument"),
        };
        if first_arg == 0 || first_arg == 1 {
            first_arg == 1
        } else {
            panic!("First argument was not a boolean")
        }
    }

    pub(crate) fn get_second_bool(&self) -> bool {
        match self.kind.argument_count() {
            2 => {
                assert!(!self.kind.has_double_arg());
                // SAFETY: two u16 args are guaranteed.
                let second_arg = unsafe { self.args.two_args[1] };
                if second_arg == 0 || second_arg == 1 {
                    second_arg == 1
                } else {
                    panic!("Second argument was not a boolean")
                }
            }
            _ => panic!("Instruction does not have two argument"),
        }
    }

    pub(crate) fn get_jump_slot(&self) -> usize {
        assert!(self.kind.has_jump_slot());
        assert_eq!(self.kind.argument_count(), 2);
        // SAFETY: Instruction is a jump.
        let jump = unsafe { self.args.double_arg };
        jump as usize
    }

    pub(crate) fn debug_print(
        &self,
        agent: &mut Agent,
        ip: usize,
        exe: Scoped<Executable>,
        gc: NoGcScope,
    ) {
        match self.kind.argument_count() {
            0 => {
                eprintln!("  {}: {:?}", ip, self.kind);
            }
            1 => {
                // SAFETY: Instruction has a single argument.
                let arg0 = unsafe { self.args.single_arg };
                eprintln!(
                    "  {}: {:?}({})",
                    ip,
                    self.kind,
                    Self::print_single_arg(agent, self.kind, arg0, exe, gc)
                );
            }
            2 => {
                if self.kind.has_jump_slot() {
                    let jump_slot = self.get_jump_slot();
                    eprintln!("  {}: {:?}({})", ip, self.kind, jump_slot);
                } else {
                    // SAFETY: Instruction has two arguments.
                    let [arg0, arg1] = unsafe { self.args.two_args };
                    eprintln!(
                        "  {}: {:?}({})",
                        ip,
                        self.kind,
                        Self::print_two_args(agent, self.kind, arg0, arg1, exe, gc)
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
                format!("{{ length: {}, env: {} }}", arg0, arg1 == 1)
            }
            Instruction::BindingPatternBindNamed => {
                format!(
                    "{{ {}: {} }}",
                    debug_print_constant(agent, exe.clone(), arg1 as usize, gc),
                    debug_print_identifier(agent, exe, arg0 as usize, gc)
                )
            }
            Instruction::ClassDefineConstructor => {
                if arg1 == 1 {
                    "constructor() { super() }".to_string()
                } else {
                    "constructor()".to_string()
                }
            }
            Instruction::InitializeVariableEnvironment => {
                format!("{{ var count: {}, strict: {} }}", arg0, arg1 == 1)
            }
            Instruction::ObjectDefineGetter => "get function() {}".to_string(),
            Instruction::ObjectDefineMethod => "function() {}".to_string(),
            Instruction::ObjectDefineSetter => "set function() {}".to_string(),
            _ => unreachable!(),
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
        format!("\"{}\"", string_constant.as_str(agent))
    } else {
        constant
            .try_string_repr(agent, gc)
            .as_str(agent)
            .to_string()
    }
}

fn debug_print_identifier(
    agent: &Agent,
    exe: Scoped<Executable>,
    index: usize,
    gc: NoGcScope,
) -> std::string::String {
    let identifier = exe.fetch_identifier(agent, index, gc);
    identifier.as_str(agent).to_string()
}

fn debug_print_binding_pattern(b: &BindingPattern) -> std::string::String {
    match &b.kind {
        oxc_ast::ast::BindingPatternKind::BindingIdentifier(b) => b.name.to_string(),
        oxc_ast::ast::BindingPatternKind::ObjectPattern(b) => {
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
        oxc_ast::ast::BindingPatternKind::ArrayPattern(b) => {
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
        oxc_ast::ast::BindingPatternKind::AssignmentPattern(b) => {
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
        oxc_ast::ast::Expression::BigIntLiteral(l) => l.raw.to_string(),
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

impl Iterator for InstructionIter<'_> {
    type Item = (usize, Instr);

    fn next(&mut self) -> Option<Self::Item> {
        let index = self.index;
        let instr = Instr::consume_instruction(self.instructions, &mut self.index)?;

        Some((index, instr))
    }
}

impl TryFrom<u8> for Instruction {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        const ADDITION: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Addition,
            ))
        };
        const BITWISEAND: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::BitwiseAnd,
            ))
        };
        const BITWISEOR: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::BitwiseOR,
            ))
        };
        const BITWISEXOR: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::BitwiseXOR,
            ))
        };
        const DIVISION: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Division,
            ))
        };
        const EQUALITY: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Equality,
            ))
        };
        const EXPONENTIAL: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Exponential,
            ))
        };
        const GREATEREQUALTHAN: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::GreaterEqualThan,
            ))
        };
        const GREATERTHAN_UNUSED: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::GreaterThan,
            ))
        };
        const LESSEQUALTHAN: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::LessEqualThan,
            ))
        };
        const LESSTHAN_UNUSED: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::LessThan,
            ))
        };
        const MULTIPLICATION: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Multiplication,
            ))
        };
        const IN: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::In,
            ))
        };
        const INEQUALITY: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Inequality,
            ))
        };
        const INSTANCEOF: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Instanceof,
            ))
        };
        const REMAINDER: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Remainder,
            ))
        };
        const SHIFTLEFT: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::ShiftLeft,
            ))
        };
        const SHIFTRIGHT: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::ShiftRight,
            ))
        };
        const SHIFTRIGHTZEROFILL: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::ShiftRightZeroFill,
            ))
        };
        const STRICTEQUALITY: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::StrictEquality,
            ))
        };
        const STRICTINEQUALITY: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::StrictInequality,
            ))
        };
        const SUBTRACTION: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Subtraction,
            ))
        };
        const DEBUG: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Debug) };
        const ARRAYCREATE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ArrayCreate) };
        const ARRAYPUSH: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ArrayPush) };
        const ARRAYSETVALUE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ArraySetValue) };
        const ARRAYELISION: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ArrayElision) };
        const AWAIT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Await) };
        const BITWISENOT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::BitwiseNot) };
        const CREATECATCHBINDING: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::CreateCatchBinding) };
        const CREATEUNMAPPEDARGUMENTSOBJECT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::CreateUnmappedArgumentsObject) };
        const COPYDATAPROPERTIES: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::CopyDataProperties) };
        const COPYDATAPROPERTIESINTOOBJECT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::CopyDataPropertiesIntoObject) };
        const DELETE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Delete) };
        const DIRECTEVALCALL: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::DirectEvalCall) };
        const EVALUATECALL: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::EvaluateCall) };
        const EVALUATENEW: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::EvaluateNew) };
        const EVALUATESUPER: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::EvaluateSuper) };
        const EVALUATEPROPERTYACCESSWITHEXPRESSIONKEY: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::EvaluatePropertyAccessWithExpressionKey)
        };
        const EVALUATEPROPERTYACCESSWITHIDENTIFIERKEY: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::EvaluatePropertyAccessWithIdentifierKey)
        };
        const GETVALUE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::GetValue) };
        const GETVALUEKEEPREFERENCE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::GetValueKeepReference) };
        const GREATERTHAN: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::GreaterThan) };
        const GREATERTHANEQUALS: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::GreaterThanEquals) };
        const HASPROPERTY: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::HasProperty) };
        const INCREMENT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Increment) };
        const DECREMENT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Decrement) };
        const INSTANCEOFOPERATOR: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::InstanceofOperator) };
        const INSTANTIATEARROWFUNCTIONEXPRESSION: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::InstantiateArrowFunctionExpression)
        };
        const INSTANTIATEORDINARYFUNCTIONEXPRESSION: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::InstantiateOrdinaryFunctionExpression)
        };
        const CLASSDEFINECONSTRUCTOR: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ClassDefineConstructor) };
        const CLASSDEFINEDEFAULTCONSTRUCTOR: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ClassDefineDefaultConstructor) };
        const ISLOOSELYEQUAL: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IsLooselyEqual) };
        const ISSTRICTLYEQUAL: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IsStrictlyEqual) };
        const ISNULLORUNDEFINED: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IsNullOrUndefined) };
        const ISNULL: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::IsNull) };
        const ISUNDEFINED: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::IsUndefined) };
        const ISOBJECT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::IsObject) };
        const ISCONSTRUCTOR: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IsConstructor) };
        const JUMP: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Jump) };
        const JUMPIFNOT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::JumpIfNot) };
        const JUMPIFTRUE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::JumpIfTrue) };
        const LESSTHAN: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::LessThan) };
        const LESSTHANEQUALS: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::LessThanEquals) };
        const LOAD: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Load) };
        const LOADCOPY: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::LoadCopy) };
        const LOADCONSTANT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::LoadConstant) };
        const LOADSTORESWAP: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::LoadStoreSwap) };
        const LOADTHISVALUE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::LoadThisValue) };
        const SWAP: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Swap) };
        const LOGICALNOT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::LogicalNot) };
        const OBJECTCREATE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ObjectCreate) };
        const OBJECTDEFINEPROPERTY: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ObjectDefineProperty) };
        const OBJECTDEFINEMETHOD: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ObjectDefineMethod) };
        const OBJECTDEFINEGETTER: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ObjectDefineGetter) };
        const OBJECTDEFINESETTER: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ObjectDefineSetter) };
        const OBJECTSETPROTOTYPE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ObjectSetPrototype) };
        const POPEXCEPTIONJUMPTARGET: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::PopExceptionJumpTarget) };
        const POPREFERENCE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::PopReference) };
        const PUSHEXCEPTIONJUMPTARGET: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::PushExceptionJumpTarget) };
        const PUSHREFERENCE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::PushReference) };
        const PUTVALUE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::PutValue) };
        const RESOLVEBINDING: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ResolveBinding) };
        const RESOLVETHISBINDING: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ResolveThisBinding) };
        const RETHROWEXCEPTIONIFANY: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::RethrowExceptionIfAny) };
        const RETURN: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Return) };
        const STORE: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Store) };
        const STORECOPY: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::StoreCopy) };
        const STORECONSTANT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::StoreConstant) };
        const STRINGCONCAT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::StringConcat) };
        const THROW: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Throw) };
        const THROWERROR: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ThrowError) };
        const TONUMBER: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ToNumber) };
        const TONUMERIC: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ToNumeric) };
        const TOOBJECT: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ToObject) };
        const TOSTRING: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::ToString) };
        const TYPEOF: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Typeof) };
        const UNARYMINUS: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::UnaryMinus) };
        const YIELD: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::Yield) };
        const CREATEIMMUTABLEBINDING: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::CreateImmutableBinding) };
        const CREATEMUTABLEBINDING: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::CreateMutableBinding) };
        const INITIALIZEREFERENCEDBINDING: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::InitializeReferencedBinding) };
        const INITIALIZEVARIABLEENVIRONMENT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::InitializeVariableEnvironment) };
        const ENTERDECLARATIVEENVIRONMENT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::EnterDeclarativeEnvironment) };
        const ENTERCLASSSTATICELEMENTENVIRONMENT: u8 = unsafe {
            std::mem::transmute::<_, u8>(Instruction::EnterClassStaticElementEnvironment)
        };
        const EXITDECLARATIVEENVIRONMENT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ExitDeclarativeEnvironment) };
        const EXITVARIABLEENVIRONMENT: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::ExitVariableEnvironment) };
        const BEGINSIMPLEOBJECTBINDINGPATTERN: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BeginSimpleObjectBindingPattern) };
        const BEGINSIMPLEARRAYBINDINGPATTERN: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BeginSimpleArrayBindingPattern) };
        const BINDINGPATTERNBIND: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternBind) };
        const BINDINGPATTERNBINDNAMED: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternBindNamed) };
        const BINDINGPATTERNBINDREST: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternBindRest) };
        const BINDINGPATTERNSKIP: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternSkip) };
        const BINDINGPATTERNGETVALUE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternGetValue) };
        const BINDINGPATTERNGETVALUENAMED: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternGetValueNamed) };
        const BINDINGPATTERNGETRESTVALUE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::BindingPatternGetRestValue) };
        const FINISHBINDINGPATTERN: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::FinishBindingPattern) };
        const ENUMERATEOBJECTPROPERTIES: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::EnumerateObjectProperties) };
        const GETITERATORSYNC: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::GetIteratorSync) };
        const GETITERATORASYNC: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::GetIteratorAsync) };
        const ITERATORSTEPVALUE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IteratorStepValue) };
        const ITERATORSTEPVALUEORUNDEFINED: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IteratorStepValueOrUndefined) };
        const ITERATORRESTINTOARRAY: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IteratorRestIntoArray) };
        const ITERATORCLOSE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IteratorClose) };
        const ASYNCITERATORCLOSE: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::AsyncIteratorClose) };
        const ITERATORCLOSEWITHERROR: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::IteratorCloseWithError) };
        const ASYNCITERATORCLOSEWITHERROR: u8 =
            unsafe { std::mem::transmute::<_, u8>(Instruction::AsyncIteratorCloseWithError) };
        const GETNEWTARGET: u8 = unsafe { std::mem::transmute::<_, u8>(Instruction::GetNewTarget) };
        match value {
            ADDITION => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Addition,
            )),
            BITWISEAND => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::BitwiseAnd,
            )),
            BITWISEOR => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::BitwiseOR,
            )),
            BITWISEXOR => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::BitwiseXOR,
            )),
            DIVISION => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Division,
            )),
            EQUALITY => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Equality,
            )),
            EXPONENTIAL => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Exponential,
            )),
            GREATEREQUALTHAN => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::GreaterEqualThan,
            )),
            GREATERTHAN_UNUSED => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::GreaterThan,
            )),
            LESSEQUALTHAN => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::LessEqualThan,
            )),
            LESSTHAN_UNUSED => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::LessThan,
            )),
            MULTIPLICATION => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Multiplication,
            )),
            IN => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::In,
            )),
            INEQUALITY => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Inequality,
            )),
            INSTANCEOF => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Instanceof,
            )),
            REMAINDER => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Remainder,
            )),
            SHIFTLEFT => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::ShiftLeft,
            )),
            SHIFTRIGHT => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::ShiftRight,
            )),
            SHIFTRIGHTZEROFILL => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::ShiftRightZeroFill,
            )),
            STRICTEQUALITY => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::StrictEquality,
            )),
            STRICTINEQUALITY => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::StrictInequality,
            )),
            SUBTRACTION => Ok(Instruction::ApplyStringOrNumericBinaryOperator(
                BinaryOperator::Subtraction,
            )),
            DEBUG => Ok(Instruction::Debug),
            ARRAYCREATE => Ok(Instruction::ArrayCreate),
            ARRAYPUSH => Ok(Instruction::ArrayPush),
            ARRAYSETVALUE => Ok(Instruction::ArraySetValue),
            ARRAYELISION => Ok(Instruction::ArrayElision),
            AWAIT => Ok(Instruction::Await),
            BITWISENOT => Ok(Instruction::BitwiseNot),
            CREATECATCHBINDING => Ok(Instruction::CreateCatchBinding),
            CREATEUNMAPPEDARGUMENTSOBJECT => Ok(Instruction::CreateUnmappedArgumentsObject),
            COPYDATAPROPERTIES => Ok(Instruction::CopyDataProperties),
            COPYDATAPROPERTIESINTOOBJECT => Ok(Instruction::CopyDataPropertiesIntoObject),
            DELETE => Ok(Instruction::Delete),
            DIRECTEVALCALL => Ok(Instruction::DirectEvalCall),
            EVALUATECALL => Ok(Instruction::EvaluateCall),
            EVALUATENEW => Ok(Instruction::EvaluateNew),
            EVALUATESUPER => Ok(Instruction::EvaluateSuper),
            EVALUATEPROPERTYACCESSWITHEXPRESSIONKEY => {
                Ok(Instruction::EvaluatePropertyAccessWithExpressionKey)
            }
            EVALUATEPROPERTYACCESSWITHIDENTIFIERKEY => {
                Ok(Instruction::EvaluatePropertyAccessWithIdentifierKey)
            }
            GETVALUE => Ok(Instruction::GetValue),
            GETVALUEKEEPREFERENCE => Ok(Instruction::GetValueKeepReference),
            GREATERTHAN => Ok(Instruction::GreaterThan),
            GREATERTHANEQUALS => Ok(Instruction::GreaterThanEquals),
            HASPROPERTY => Ok(Instruction::HasProperty),
            INCREMENT => Ok(Instruction::Increment),
            DECREMENT => Ok(Instruction::Decrement),
            INSTANCEOFOPERATOR => Ok(Instruction::InstanceofOperator),
            INSTANTIATEARROWFUNCTIONEXPRESSION => {
                Ok(Instruction::InstantiateArrowFunctionExpression)
            }
            INSTANTIATEORDINARYFUNCTIONEXPRESSION => {
                Ok(Instruction::InstantiateOrdinaryFunctionExpression)
            }
            CLASSDEFINECONSTRUCTOR => Ok(Instruction::ClassDefineConstructor),
            CLASSDEFINEDEFAULTCONSTRUCTOR => Ok(Instruction::ClassDefineDefaultConstructor),
            ISLOOSELYEQUAL => Ok(Instruction::IsLooselyEqual),
            ISSTRICTLYEQUAL => Ok(Instruction::IsStrictlyEqual),
            ISNULLORUNDEFINED => Ok(Instruction::IsNullOrUndefined),
            ISNULL => Ok(Instruction::IsNull),
            ISUNDEFINED => Ok(Instruction::IsUndefined),
            ISOBJECT => Ok(Instruction::IsObject),
            ISCONSTRUCTOR => Ok(Instruction::IsConstructor),
            JUMP => Ok(Instruction::Jump),
            JUMPIFNOT => Ok(Instruction::JumpIfNot),
            JUMPIFTRUE => Ok(Instruction::JumpIfTrue),
            LESSTHAN => Ok(Instruction::LessThan),
            LESSTHANEQUALS => Ok(Instruction::LessThanEquals),
            LOAD => Ok(Instruction::Load),
            LOADCOPY => Ok(Instruction::LoadCopy),
            LOADCONSTANT => Ok(Instruction::LoadConstant),
            LOADSTORESWAP => Ok(Instruction::LoadStoreSwap),
            LOADTHISVALUE => Ok(Instruction::LoadThisValue),
            SWAP => Ok(Instruction::Swap),
            LOGICALNOT => Ok(Instruction::LogicalNot),
            OBJECTCREATE => Ok(Instruction::ObjectCreate),
            OBJECTDEFINEPROPERTY => Ok(Instruction::ObjectDefineProperty),
            OBJECTDEFINEMETHOD => Ok(Instruction::ObjectDefineMethod),
            OBJECTDEFINEGETTER => Ok(Instruction::ObjectDefineGetter),
            OBJECTDEFINESETTER => Ok(Instruction::ObjectDefineSetter),
            OBJECTSETPROTOTYPE => Ok(Instruction::ObjectSetPrototype),
            POPEXCEPTIONJUMPTARGET => Ok(Instruction::PopExceptionJumpTarget),
            POPREFERENCE => Ok(Instruction::PopReference),
            PUSHEXCEPTIONJUMPTARGET => Ok(Instruction::PushExceptionJumpTarget),
            PUSHREFERENCE => Ok(Instruction::PushReference),
            PUTVALUE => Ok(Instruction::PutValue),
            RESOLVEBINDING => Ok(Instruction::ResolveBinding),
            RESOLVETHISBINDING => Ok(Instruction::ResolveThisBinding),
            RETHROWEXCEPTIONIFANY => Ok(Instruction::RethrowExceptionIfAny),
            RETURN => Ok(Instruction::Return),
            STORE => Ok(Instruction::Store),
            STORECOPY => Ok(Instruction::StoreCopy),
            STORECONSTANT => Ok(Instruction::StoreConstant),
            STRINGCONCAT => Ok(Instruction::StringConcat),
            THROW => Ok(Instruction::Throw),
            THROWERROR => Ok(Instruction::ThrowError),
            TONUMBER => Ok(Instruction::ToNumber),
            TONUMERIC => Ok(Instruction::ToNumeric),
            TOOBJECT => Ok(Instruction::ToObject),
            TOSTRING => Ok(Instruction::ToString),
            TYPEOF => Ok(Instruction::Typeof),
            UNARYMINUS => Ok(Instruction::UnaryMinus),
            YIELD => Ok(Instruction::Yield),
            CREATEIMMUTABLEBINDING => Ok(Instruction::CreateImmutableBinding),
            CREATEMUTABLEBINDING => Ok(Instruction::CreateMutableBinding),
            INITIALIZEREFERENCEDBINDING => Ok(Instruction::InitializeReferencedBinding),
            INITIALIZEVARIABLEENVIRONMENT => Ok(Instruction::InitializeVariableEnvironment),
            ENTERDECLARATIVEENVIRONMENT => Ok(Instruction::EnterDeclarativeEnvironment),
            ENTERCLASSSTATICELEMENTENVIRONMENT => {
                Ok(Instruction::EnterClassStaticElementEnvironment)
            }
            EXITDECLARATIVEENVIRONMENT => Ok(Instruction::ExitDeclarativeEnvironment),
            EXITVARIABLEENVIRONMENT => Ok(Instruction::ExitVariableEnvironment),
            BEGINSIMPLEOBJECTBINDINGPATTERN => Ok(Instruction::BeginSimpleObjectBindingPattern),
            BEGINSIMPLEARRAYBINDINGPATTERN => Ok(Instruction::BeginSimpleArrayBindingPattern),
            BINDINGPATTERNBIND => Ok(Instruction::BindingPatternBind),
            BINDINGPATTERNBINDNAMED => Ok(Instruction::BindingPatternBindNamed),
            BINDINGPATTERNBINDREST => Ok(Instruction::BindingPatternBindRest),
            BINDINGPATTERNSKIP => Ok(Instruction::BindingPatternSkip),
            BINDINGPATTERNGETVALUE => Ok(Instruction::BindingPatternGetValue),
            BINDINGPATTERNGETVALUENAMED => Ok(Instruction::BindingPatternGetValueNamed),
            BINDINGPATTERNGETRESTVALUE => Ok(Instruction::BindingPatternGetRestValue),
            FINISHBINDINGPATTERN => Ok(Instruction::FinishBindingPattern),
            ENUMERATEOBJECTPROPERTIES => Ok(Instruction::EnumerateObjectProperties),
            GETITERATORSYNC => Ok(Instruction::GetIteratorSync),
            GETITERATORASYNC => Ok(Instruction::GetIteratorAsync),
            ITERATORSTEPVALUE => Ok(Instruction::IteratorStepValue),
            ITERATORSTEPVALUEORUNDEFINED => Ok(Instruction::IteratorStepValueOrUndefined),
            ITERATORRESTINTOARRAY => Ok(Instruction::IteratorRestIntoArray),
            ITERATORCLOSE => Ok(Instruction::IteratorClose),
            ASYNCITERATORCLOSE => Ok(Instruction::AsyncIteratorClose),
            ITERATORCLOSEWITHERROR => Ok(Instruction::IteratorCloseWithError),
            ASYNCITERATORCLOSEWITHERROR => Ok(Instruction::AsyncIteratorCloseWithError),
            GETNEWTARGET => Ok(Instruction::GetNewTarget),
            _ => Err(()),
        }
    }
}
