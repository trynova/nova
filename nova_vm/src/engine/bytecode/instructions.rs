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
            | Self::ObjectDefineGetter
            | Self::ObjectDefineMethod
            | Self::ObjectDefineSetter => 2,
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
            | Self::IteratorStepValue
            | Self::Jump
            | Self::JumpIfNot
            | Self::JumpIfTrue
            | Self::LoadConstant
            | Self::PushExceptionJumpTarget
            | Self::ResolveBinding
            | Self::StoreConstant
            | Self::StringConcat
            | Self::ThrowError => 1,
            _ => 0,
        }
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
            |Self::ClassDefineConstructor| Self::InstantiateArrowFunctionExpression
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
        unsafe { core::mem::transmute::<Self, u8>(self) }
    }
}

#[derive(Debug)]
pub(crate) struct Instr {
    pub kind: Instruction,
    pub args: [Option<IndexType>; 2],
}

impl Instr {
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
                let arg0 = self.args.first().unwrap().unwrap();
                eprintln!(
                    "  {}: {:?}({})",
                    ip,
                    self.kind,
                    Self::print_single_arg(agent, self.kind, arg0, exe, gc)
                );
            }
            2 => {
                let arg0 = self.args.first().unwrap().unwrap();
                let arg1 = self.args.last().unwrap().unwrap();
                eprintln!(
                    "  {}: {:?}({})",
                    ip,
                    self.kind,
                    Self::print_two_args(agent, self.kind, arg0, arg1, exe, gc)
                );
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
        let index0 = arg0 as usize;
        let index1 = arg1 as usize;
        match kind {
            Instruction::BeginSimpleArrayBindingPattern => {
                format!("{{ length: {}, env: {} }}", arg0, arg1 == 1)
            }
            Instruction::BindingPatternBindNamed => {
                format!(
                    "{{ {}: {} }}",
                    debug_print_constant(agent, exe.clone(), index1, gc),
                    debug_print_identifier(agent, exe, index0, gc)
                )
            }
            Instruction::ClassDefineConstructor => {
                if index1 == 1 {
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
        if self.index >= self.instructions.len() {
            return None;
        }
        let index = self.index;

        let kind: Instruction = unsafe { core::mem::transmute(self.instructions[self.index]) };
        self.index += 1;

        let mut args: [Option<IndexType>; 2] = [None, None];

        for item in args.iter_mut().take(kind.argument_count() as usize) {
            let length = self.instructions[self.index..].len();
            if length >= 2 {
                let bytes = IndexType::from_ne_bytes(unsafe {
                    *core::mem::transmute::<*const u8, *const [u8; 2]>(
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
