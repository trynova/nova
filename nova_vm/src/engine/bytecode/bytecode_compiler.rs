// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod assignment;
mod block_declaration_instantiation;
mod class_definition_evaluation;
mod compile_context;
mod executable_context;
mod exports;
mod finaliser_stack;
mod for_in_of_statement;
mod function_declaration_instantiation;
mod labelled_statement;
mod template_literals;
mod with_statement;

use std::{convert::Infallible, ops::ControlFlow};

use super::{FunctionExpression, Instruction, SendableRef, executable::ArrowFunctionExpression};
#[cfg(feature = "typescript")]
use crate::{ecmascript::builtins::ordinary::shape::ObjectShapeRecord, heap::CreateHeapData};
use crate::{
    ecmascript::{
        abstract_operations::type_conversion::to_property_key_simple,
        builtins::ordinary::shape::ObjectShape,
        execution::{Agent, agent::ExceptionType},
        types::{IntoObject, Primitive, PropertyKey},
    },
    engine::context::{Bindable, NoGcScope},
};
use crate::{
    ecmascript::{
        syntax_directed_operations::{
            function_definitions::ContainsExpression,
            scope_analysis::{LexicallyScopedDeclaration, LexicallyScopedDeclarations},
        },
        types::{BUILTIN_STRING_MEMORY, BigInt, IntoValue, Number, String, Value},
    },
    engine::bytecode::bytecode_compiler::compile_context::StackValue,
};
pub(crate) use compile_context::{
    CompileContext, CompileEvaluation, CompileLabelledEvaluation, GeneratorKind, IndexType,
    JumpIndex, NamedEvaluationParameter,
};
use num_traits::Num;
use oxc_ast::ast;
use oxc_ecmascript::BoundNames;
use oxc_semantic::{NodeId, ScopeFlags, SymbolFlags};
use oxc_syntax::operator::{BinaryOperator, UnaryOperator};
use template_literals::get_template_object;
use wtf8::{CodePoint, Wtf8Buf};

/// Defines the compiled output of a place expression. Place expressions define
/// a location in memory, instead of a concrete value. Examples are:
///
/// ```javascript
/// foo; // foo environment variable
/// expr.bar; // bar member of some expression
/// ```
///
/// Place expressions can appear on both the right and left side of assignment
/// expressions. On the left side they're eventually used as an input to
/// PutValue, while on the right hand side they're eventually used as an input
/// to GetValue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Place<'s, 'gc> {
    /// A variable on the stack. The variable data is stored on the VM stack
    /// instead of being in the environment. Stack-slot variables never produce
    /// references.
    Stack {
        name: String<'gc>,
        stack_slot: u32,
        mutable: bool,
    },
    /// A variable in the environment. The variable data is stored in the
    /// declarative environment's hash map and accessed through a reference.
    Env { name: String<'gc> },
    /// A variable in the global environment. The variable data is stored in
    /// the global object's properties and accessed through a reference.
    Global { name: String<'gc> },
    /// A member property. The name may or may not be known. Member references
    /// always produce references.
    Member { name: Option<PropertyKey<'gc>> },
    /// A variable on the stack that is uninitialised at the point of reference.
    /// Using the reference will throw a ReferenceError.
    TemporalDeadZone { name: &'s str },
}

impl<'s, 'gc> Place<'s, 'gc> {
    fn identifier(&self) -> Option<String<'gc>> {
        match self {
            Place::Stack { name, .. } | Place::Env { name } | Place::Global { name } => Some(*name),
            Place::Member { name } => name.and_then(|n| {
                match n {
                    PropertyKey::SmallString(s) => Some(String::SmallString(s)),
                    PropertyKey::String(s) => Some(String::String(s)),
                    // TODO: we probably want to convert integers to strings.
                    _ => None,
                }
            }),
            Place::TemporalDeadZone { .. } => unreachable!(),
        }
    }

    /// Returns true if the place has a Reference on the reference stack
    /// associated with it.
    #[inline]
    fn has_reference(&self) -> bool {
        matches!(
            self,
            Self::Env { .. } | Self::Global { .. } | Self::Member { .. }
        )
    }

    fn initialise_referenced_binding_to_undefined(&self, ctx: &mut CompileContext) {
        match self {
            Place::Env { .. } | Place::Global { .. } | Place::Member { .. } => {
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                ctx.add_instruction(Instruction::InitializeReferencedBinding);
            }
            Place::Stack { .. } => {
                // Note: stack variables are initialised to undefined
                // automatically.
            }
            Place::TemporalDeadZone { .. } => {
                // Stack variables being initialised should never resolve to
                // TDZ.
                unreachable!();
            }
        }
    }

    fn initialise_referenced_binding(&self, ctx: &mut CompileContext, value: ValueOutput) {
        match self {
            Place::Env { .. } | Place::Global { .. } | Place::Member { .. } => {
                ctx.add_instruction(Instruction::InitializeReferencedBinding);
            }
            Place::Stack { stack_slot, .. } => {
                if value == ValueOutput::Literal(Primitive::Undefined) {
                    // Note: stack variables are initialised to undefined
                    // automatically.
                    return;
                }
                ctx.add_instruction_with_immediate(
                    Instruction::PutValueToIndex,
                    *stack_slot as usize,
                );
            }
            Place::TemporalDeadZone { .. } => {
                // Stack variables being initialised should never resolve to
                // TDZ.
                unreachable!();
            }
        }
    }

    fn get_value(
        &self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    ) -> Result<ValueOutput<'static>, ExpressionError> {
        match self {
            Place::Stack { stack_slot, .. } => {
                // Variable is stored on the stack. Caching doesn't help here.
                ctx.add_instruction_with_immediate(
                    Instruction::GetValueFromIndex,
                    *stack_slot as usize,
                );
                Ok(ValueOutput::Value)
            }
            Place::Global { name } => {
                // Variable is stored in the global environment. Caching helps with
                // these accesses.
                let cache = ctx.create_property_lookup_cache(name.to_property_key());
                ctx.add_instruction_with_cache(Instruction::GetValueWithCache, cache);
                Ok(ValueOutput::Value)
            }
            Place::Member { name: Some(name) } => {
                // Property access. Caching helps with these.
                let cache = ctx.create_property_lookup_cache(*name);
                ctx.add_instruction_with_cache(Instruction::GetValueWithCache, cache);
                Ok(ValueOutput::Value)
            }
            Place::Member { .. } | Place::Env { .. } => {
                // Variable is stored in the environment or we don't know the
                // property name at compile time. Caching doesn't help with these.
                ctx.add_instruction(Instruction::GetValue);
                Ok(ValueOutput::Value)
            }
            Place::TemporalDeadZone { name } => {
                let message =
                    format!("can't access lexical declaration '{name}' before initialization");
                let message = ctx.create_string_from_owned(message);
                ctx.add_instruction_with_constant(Instruction::StoreConstant, message);
                ctx.add_instruction_with_immediate(
                    Instruction::ThrowError,
                    ExceptionType::ReferenceError as usize,
                );
                Err(ExpressionError::Error)
            }
        }
    }

    fn get_value_keep_reference(
        &self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    ) -> Result<ValueOutput<'static>, ExpressionError> {
        match self {
            Self::Stack { stack_slot, .. } => {
                // Variable is stored on the stack. Caching doesn't help here and
                // we never have a reference to keep here.
                ctx.add_instruction_with_immediate(
                    Instruction::GetValueFromIndex,
                    *stack_slot as usize,
                );
                Ok(ValueOutput::Value)
            }
            Self::Global { name } => {
                // Variable is stored in the global environment. Caching helps with
                // these accesses.
                let cache = ctx.create_property_lookup_cache(name.to_property_key());
                ctx.add_instruction_with_cache(Instruction::GetValueWithCacheKeepReference, cache);
                Ok(ValueOutput::Value)
            }
            Self::Member { name: Some(name) } => {
                // Property access. Caching helps with these.
                let cache = ctx.create_property_lookup_cache(*name);
                ctx.add_instruction_with_cache(Instruction::GetValueWithCacheKeepReference, cache);
                Ok(ValueOutput::Value)
            }
            Self::Member { .. } | Self::Env { .. } => {
                // Variable is stored in the environment or we don't know the
                // property name at compile time. Caching doesn't help with these.
                ctx.add_instruction(Instruction::GetValueKeepReference);
                Ok(ValueOutput::Value)
            }
            Self::TemporalDeadZone { name } => {
                let message =
                    format!("can't access lexical declaration '{name}' before initialization");
                let message = ctx.create_string_from_owned(message);
                ctx.add_instruction_with_constant(Instruction::StoreConstant, message);
                ctx.add_instruction_with_immediate(
                    Instruction::ThrowError,
                    ExceptionType::ReferenceError as usize,
                );
                Err(ExpressionError::Error)
            }
        }
    }

    #[inline]
    fn get_value_maybe_keep_reference(
        &self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
        keep_reference: bool,
    ) -> Result<ValueOutput<'static>, ExpressionError> {
        if keep_reference {
            self.get_value_keep_reference(ctx)
        } else {
            self.get_value(ctx)
        }
    }

    fn put_value(
        &self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
        _value: ValueOutput,
    ) -> Result<(), ExpressionError> {
        // Note: _value is currently unused but may be used in the future to
        // perform optimisations.
        match self {
            Self::Stack {
                stack_slot,
                mutable,
                name,
            } => {
                if !mutable {
                    let message = format!(
                        "can't mutate const declaration '{}' before initialization",
                        name.as_str(ctx.get_agent()).unwrap()
                    );
                    let message = ctx.create_string_from_owned(message);
                    ctx.add_instruction_with_constant(Instruction::StoreConstant, message);
                    ctx.add_instruction_with_immediate(
                        Instruction::ThrowError,
                        ExceptionType::ReferenceError as usize,
                    );
                    Err(ExpressionError::Error)
                } else {
                    ctx.add_instruction_with_immediate(
                        Instruction::PutValueToIndex,
                        *stack_slot as usize,
                    );
                    Ok(())
                }
            }
            Self::Global { name } => {
                let cache = ctx.create_property_lookup_cache(name.to_property_key());
                ctx.add_instruction_with_cache(Instruction::PutValueWithCache, cache);
                Ok(())
            }
            Self::Member { name: Some(name) } => {
                let cache = ctx.create_property_lookup_cache(*name);
                ctx.add_instruction_with_cache(Instruction::PutValueWithCache, cache);
                Ok(())
            }
            Self::Member { .. } | Self::Env { .. } => {
                ctx.add_instruction(Instruction::PutValue);
                Ok(())
            }
            Self::TemporalDeadZone { name } => {
                let message =
                    format!("can't access lexical declaration '{name}' before initialization");
                let message = ctx.create_string_from_owned(message);
                ctx.add_instruction_with_constant(Instruction::StoreConstant, message);
                ctx.add_instruction_with_immediate(
                    Instruction::ThrowError,
                    ExceptionType::ReferenceError as usize,
                );
                Err(ExpressionError::Error)
            }
        }
    }

    fn delete(
        self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    ) -> Result<ValueOutput<'static>, ExpressionError> {
        match self {
            Self::Stack { .. } => {
                // Delete on a stack variable is only allowed for lexical
                // declarations, as they always return `false`.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, false);
                Ok(false.into())
            }
            Self::Global { .. } | Self::Member { .. } | Self::Env { .. } => {
                ctx.add_instruction(Instruction::Delete);
                // Can return `true` or `false`.
                Ok(ValueOutput::Value)
            }
            Self::TemporalDeadZone { name } => {
                let message =
                    format!("can't access lexical declaration '{name}' before initialization");
                let message = ctx.create_string_from_owned(message);
                ctx.add_instruction_with_constant(Instruction::StoreConstant, message);
                ctx.add_instruction_with_immediate(
                    Instruction::ThrowError,
                    ExceptionType::ReferenceError as usize,
                );
                Err(ExpressionError::Error)
            }
        }
    }
}

impl<'gc> From<PropertyKey<'gc>> for Place<'_, 'gc> {
    #[inline]
    fn from(name: PropertyKey<'gc>) -> Self {
        Self::Member { name: Some(name) }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum ValueOutput<'gc> {
    /// Expression evaluates to some unknown value.
    Value,
    /// Expression evaluates to a known literal value.
    Literal(Primitive<'gc>),
}

impl<'gc> ValueOutput<'gc> {
    fn to_expression_key(self) -> Place<'static, 'gc> {
        match self {
            Self::Value => Place::Member { name: None },
            Self::Literal(p) => match p {
                Primitive::Undefined => BUILTIN_STRING_MEMORY.undefined.to_property_key().into(),
                Primitive::Null => BUILTIN_STRING_MEMORY.null.to_property_key().into(),
                Primitive::Boolean(true) => BUILTIN_STRING_MEMORY.r#true.to_property_key().into(),
                Primitive::Boolean(false) => BUILTIN_STRING_MEMORY.r#false.to_property_key().into(),
                Primitive::String(s) => PropertyKey::String(s).into(),
                Primitive::SmallString(s) => PropertyKey::SmallString(s).into(),
                // Other members don't benefit from caching anyway.
                _ => Place::Member { name: None },
            },
        }
    }
}

impl<'gc, T> From<T> for ValueOutput<'gc>
where
    T: 'gc + Into<Primitive<'gc>>,
{
    #[inline]
    fn from(value: T) -> Self {
        Self::Literal(value.into())
    }
}

fn combine_value_results<'gc>(
    a: Result<ValueOutput<'gc>, ExpressionError>,
    b: Result<ValueOutput<'gc>, ExpressionError>,
) -> Result<ValueOutput<'gc>, ExpressionError> {
    match (a, b) {
        // If two branches unconditionally error, the combination
        // unconditionally errors.
        (Err(err), Err(_)) => Err(err),
        // If two branches evaluate to the same literal, the combination
        // unconditionally evaluates to that literal.
        (Ok(ValueOutput::Literal(a)), Ok(ValueOutput::Literal(b))) if a == b => {
            Ok(ValueOutput::Literal(a))
        }
        // If one branch unconditionally errors and the other does not, the only
        // possible value is the one from the successful one.
        (Ok(v), Err(_)) | (Err(_), Ok(v)) => Ok(v),
        // Otherwise we just end up in unknown value land.
        _ => Ok(ValueOutput::Value),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum PlaceOrValue<'s, 'gc> {
    /// Expression evaluates to a value.
    Value(ValueOutput<'gc>),
    /// Expression evaluates to a place.
    Place(Place<'s, 'gc>),
}

impl<'gc, T> From<T> for PlaceOrValue<'static, 'gc>
where
    T: 'gc + Into<ValueOutput<'gc>>,
{
    #[inline]
    fn from(value: T) -> Self {
        Self::Value(value.into())
    }
}

impl<'s, 'gc> From<Place<'s, 'gc>> for PlaceOrValue<'s, 'gc> {
    #[inline]
    fn from(value: Place<'s, 'gc>) -> Self {
        Self::Place(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub(crate) enum ExpressionError {
    /// Expression evaluates to an abrupt throw.
    Error,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum StatementContinue<'gc> {
    /// Statement evaluates to some unknown value.
    Value,
    /// Statement evaluates to a known literal value.
    Literal(Primitive<'gc>),
    /// Statement evaluates to EMPTY (coerces to undefined).
    Empty,
}

impl<'gc> From<ValueOutput<'gc>> for StatementContinue<'gc> {
    #[inline]
    fn from(value: ValueOutput<'gc>) -> Self {
        match value {
            ValueOutput::Value => Self::Value,
            ValueOutput::Literal(lit) => Self::Literal(lit),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[must_use]
pub(crate) enum StatementBreak {
    /// Statement evaluates to an abrupt throw.
    Error,
    /// Statement evaluates to an abrupt return.
    Return,
    /// Statement evaluates to an abrupt break.
    Break,
    /// Statement evaluates to an abrupt continue.
    Continue,
}

impl From<ExpressionError> for StatementBreak {
    #[inline]
    fn from(value: ExpressionError) -> StatementBreak {
        match value {
            ExpressionError::Error => Self::Error,
        }
    }
}

pub(crate) type StatementResult<'gc> = ControlFlow<StatementBreak, StatementContinue<'gc>>;

#[inline]
pub(super) fn value_result_to_statement_result<'gc>(
    result: Result<ValueOutput<'gc>, ExpressionError>,
) -> StatementResult<'gc> {
    match result {
        Ok(v) => ControlFlow::Continue(v.into()),
        Err(e) => ControlFlow::Break(e.into()),
    }
}

impl<'gc> From<StatementContinue<'gc>> for StatementResult<'gc> {
    #[inline]
    fn from(value: StatementContinue<'gc>) -> StatementResult<'gc> {
        Self::Continue(value)
    }
}

impl From<StatementBreak> for StatementResult<'static> {
    #[inline]
    fn from(value: StatementBreak) -> StatementResult<'static> {
        Self::Break(value)
    }
}

impl<'s, 'gc> PlaceOrValue<'s, 'gc> {
    #[inline]
    fn is_stack_variable(&self) -> bool {
        matches!(self, PlaceOrValue::Place(Place::Stack { .. }))
    }

    /// Returns true if the expression has a Reference on the reference stack
    /// associated with it.
    #[inline]
    fn has_reference(&self) -> bool {
        match self {
            PlaceOrValue::Place(place) => place.has_reference(),
            _ => false,
        }
    }

    fn get_value(
        self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    ) -> Result<ValueOutput<'gc>, ExpressionError> {
        match self {
            Self::Place(place) => {
                place.get_value(ctx)?;
                // After evaluating the GetValue we return an unknown Value.
                Ok(ValueOutput::Value)
            }
            // No GetValue needed.
            Self::Value(value) => Ok(value),
        }
    }

    fn get_value_keep_reference(
        self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    ) -> Result<ValueOutput<'gc>, ExpressionError> {
        match self {
            Self::Place(place) => {
                place.get_value_keep_reference(ctx)?;
                // After evaluating the GetValue we return an unknown Value.
                Ok(ValueOutput::Value)
            }
            // No GetValue needed.
            Self::Value(value) => Ok(value),
        }
    }

    fn delete(
        self,
        ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    ) -> Result<ValueOutput<'gc>, ExpressionError> {
        match self {
            PlaceOrValue::Place(place) => place.delete(ctx),
            _ => {
                // 2. If ref is not a Reference Record, return true.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, true);
                Ok(true.into())
            }
        }
    }
}

impl<'a, 's, 'gc, 'scope, T: CompileEvaluation<'a, 's, 'gc, 'scope>>
    CompileLabelledEvaluation<'a, 's, 'gc, 'scope> for T
{
    type Output = ();

    fn compile_labelled(
        &'s self,
        _label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'a, 's, 'gc, 'scope>,
    ) {
        self.compile(ctx);
    }
}

fn variable_escapes_scope(
    ctx: &CompileContext,
    identifier: &oxc_ast::ast::BindingIdentifier,
) -> bool {
    let agent = ctx.get_agent();
    let sc = ctx.get_source_code();
    let scoping = sc.get_scoping(agent);
    let nodes = sc.get_nodes(agent);
    let s = identifier.symbol_id();
    if !scoping.symbol_redeclarations(s).is_empty() {
        // Redeclarations are a pain to deal with.
        return true;
    }
    let decl_scope = scoping.symbol_scope_id(s);
    if scoping.scope_flags(decl_scope).contains_direct_eval() {
        return true;
    }
    let decl_id = scoping.symbol_declaration(s);
    let symbol_flags = scoping.symbol_flags(s);
    let is_lexical = symbol_flags.intersects(SymbolFlags::BlockScopedVariable);
    let is_class = symbol_flags.is_class();
    for reference in scoping.get_resolved_references(s) {
        let ref_id = reference.node_id();
        if !is_lexical
            && nodes
                .get_node(ref_id)
                .kind()
                .as_unary_expression()
                .is_some_and(|expr| expr.operator.is_delete())
        {
            // Deleting non-lexical references has effects outside of the
            // immediate scope and is thus considered escaping.
            return true;
        }
        let mut scope = nodes.get_node(ref_id).scope_id();
        while scope != decl_scope {
            let flags = scoping.scope_flags(scope);
            if flags.is_var() || flags.contains_direct_eval() || flags.is_with() {
                return true;
            }
            let Some(s) = scoping.scope_parent_id(scope) else {
                panic!("reference in a different scope?")
            };
            scope = s;
        }
        // Classes can refer to themselves both during their creation and in
        // field declarations that escape creation time. We need to check for
        // them.
        if is_class {
            let mut node = ref_id;
            while decl_id < node {
                node = nodes.parent_id(node);
            }
            if decl_id == node {
                // Self-referential class declaration.
                return true;
            }
        }
    }
    false
}

pub(crate) fn is_reference(expression: &ast::Expression) -> bool {
    matches!(
        expression.get_inner_expression(),
        ast::Expression::Identifier(_)
            | ast::Expression::ComputedMemberExpression(_)
            | ast::Expression::StaticMemberExpression(_)
            | ast::Expression::PrivateFieldExpression(_)
    )
}

pub(crate) fn is_boolean_literal_true(expression: &ast::Expression) -> bool {
    matches!(expression.get_inner_expression(), ast::Expression::BooleanLiteral(lit) if lit.value)
}

pub(crate) fn is_boolean_literal_false(expression: &ast::Expression) -> bool {
    matches!(expression.get_inner_expression(), ast::Expression::BooleanLiteral(lit) if !lit.value)
}

fn is_chain_expression(expression: &ast::Expression) -> bool {
    matches!(
        expression.get_inner_expression(),
        ast::Expression::ChainExpression(_)
    )
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::NumericLiteral<'s> {
    type Output = Primitive<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let constant = ctx.create_number(self.value);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
        constant.into()
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BooleanLiteral {
    type Output = Primitive<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        ctx.add_instruction_with_constant(Instruction::StoreConstant, self.value);
        self.value.into()
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BigIntLiteral<'s> {
    type Output = BigInt<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // Drop out the trailing 'n' from BigInt literals.
        let raw_str = self
            .raw
            .as_ref()
            .expect("BigInt literal should have raw text")
            .as_str();
        let last_index = raw_str.len() - 1;
        let (literal, radix) = match self.base {
            oxc_syntax::number::BigintBase::Decimal => (&raw_str[..last_index], 10),
            oxc_syntax::number::BigintBase::Binary => (&raw_str[2..last_index], 2),
            oxc_syntax::number::BigintBase::Octal => (&raw_str[2..last_index], 8),
            oxc_syntax::number::BigintBase::Hex => (&raw_str[2..last_index], 16),
        };
        let constant = ctx.create_bigint(literal, radix);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
        constant
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::NullLiteral {
    type Output = Primitive<'static>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Null);
        Primitive::Null
    }
}

pub(crate) fn string_literal_to_wtf8<'a>(
    agent: &mut Agent,
    string: &ast::StringLiteral,
    gc: NoGcScope<'a, '_>,
) -> String<'a> {
    if string.lone_surrogates {
        let mut buf = Wtf8Buf::with_capacity(string.value.len());
        let mut str = string.value.as_str();
        while let Some(replacement_character_index) = str.find("\u{FFFD}") {
            // Lone surrogates are encoded as \u{FFFD}XXXX and \u{FFFD}
            // itself is encoded as \u{FFFD}fffd: hence the fact that we
            // found a replacement character means that we're guaranteed to
            // have 7 bytes ahead of the replacement character index: 3 for
            // the replacement character itself, 4 for the encoded bytes.

            let (preceding, following) = str.split_at(replacement_character_index);
            let (encoded_surrogate, rest) = following.split_at(7);

            // First copy our preceding slice into the buffer.
            if !preceding.is_empty() {
                // SAFETY: we're working within our search buffer.
                buf.push_str(preceding);
            }
            // Drop the replacement character from our str slice.
            str = rest;
            // Then split off the encoded bytes.
            let encoded_bytes: &[u8; 7] = encoded_surrogate.as_bytes().first_chunk().unwrap();
            fn char_code_to_u16(char_code: u8) -> u16 {
                if char_code >= 97 {
                    // 'a'..'f'
                    (char_code - 87) as u16
                } else {
                    // '0'..'9'
                    (char_code - 48) as u16
                }
            }
            let value = (char_code_to_u16(encoded_bytes[3]) << 12)
                + (char_code_to_u16(encoded_bytes[4]) << 8)
                + (char_code_to_u16(encoded_bytes[5]) << 4)
                + char_code_to_u16(encoded_bytes[6]);
            // SAFETY: Value cannot be larger than 0xFFFF.
            let code_point = unsafe { CodePoint::from_u32_unchecked(value as u32) };
            buf.push(code_point);
        }
        if !str.is_empty() {
            buf.push_str(str);
        }
        String::from_wtf8_buf(agent, buf, gc)
    } else {
        String::from_str(agent, string.value.as_str(), gc)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::StringLiteral<'s> {
    type Output = Primitive<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let (agent, gc) = ctx.get_agent_and_gc();
        let constant = string_literal_to_wtf8(agent, self, gc);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
        constant.into()
    }
}

enum VariableKind {
    /// Stored on the stack, not accessible by name at all.
    Stack { stack_slot: u32, mutable: bool },
    /// Reference to a stack variable in the temporal dead zone of a lexical
    /// declaration. The referrer should throw an error in the bytecode and
    /// skip any further work.
    TemporalDeadZone,
    /// Stored in an environment.
    Local,
    /// Found in the global scope.
    Global,
}

impl VariableKind {
    fn compile<'s, 'gc>(
        self,
        ctx: &mut CompileContext<'_, 's, 'gc, '_>,
        name: &'s str,
    ) -> Place<'s, 'gc> {
        match self {
            VariableKind::Stack {
                stack_slot,
                mutable,
            } => {
                let name = ctx.create_string(name);
                // variable on the stack
                Place::Stack {
                    name,
                    stack_slot,
                    mutable,
                }
            }
            VariableKind::TemporalDeadZone => Place::TemporalDeadZone { name },
            VariableKind::Local => {
                let name = ctx.create_string(name);
                // Local variable: property name caching is not useful here.
                ctx.add_instruction_with_identifier(
                    Instruction::ResolveBinding,
                    name.to_property_key(),
                );
                Place::Env { name }
            }
            VariableKind::Global => {
                let name = ctx.create_string(name);
                // Global variable: property name caching is useful here.
                let cache = ctx.create_property_lookup_cache(name.to_property_key());
                ctx.add_instruction_with_identifier_and_cache(
                    Instruction::ResolveBindingWithCache,
                    name,
                    cache,
                );
                Place::Global { name }
            }
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::IdentifierReference<'s> {
    type Output = Place<'s, 'gc>;
    /// Compile a reference TO a variable. This is used to read or write to a
    /// variable.
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let kind = if let Some(id) = self.reference_id.get() {
            let source_code = ctx.get_source_code();
            let scoping = source_code.get_scoping(ctx.get_agent());
            let reference = scoping.get_reference(id);
            if let Some(s) = reference.symbol_id() {
                // SymbolId means we might be a global, local, or a stack
                // variable.
                let symbol_flags = scoping.symbol_flags(s);
                let mutable = !symbol_flags.is_const_variable();
                if let Some(stack_slot) = ctx.get_variable_stack_index(s) {
                    // We're a stack variable.
                    let nodes = source_code.get_nodes(ctx.get_agent());
                    let ref_id = reference.node_id();
                    let decl_id = scoping.symbol_declaration(s);
                    if decl_id == ref_id {
                        // Reference should never be the declaration itself.
                        unreachable!();
                    }
                    let is_lexical = symbol_flags.intersects(SymbolFlags::BlockScopedVariable);
                    // We might be in the temporal dead-zone.
                    if is_lexical && ref_id < decl_id
                        || !is_lexical && is_parameter_tdz(nodes, decl_id, ref_id)
                    {
                        // Reference before initialization: this is TDZ.
                        VariableKind::TemporalDeadZone
                    } else if nodes.get_node(decl_id).scope_id() == scoping.symbol_scope_id(s) {
                        // If the node comes after the declaration and is in the
                        // same scope, it's still possible for it to be in the
                        // TDZ if it is itself within the declaration
                        // expression. To detect this, we iterate parent nodes
                        // until we find one that is equal to or before the
                        // declaration. If we found the declaration this way,
                        // then this is TDZ.
                        let mut node = ref_id;
                        while decl_id < node {
                            node = nodes.parent_id(node);
                        }
                        if decl_id == node {
                            // Self-referential declaration.
                            VariableKind::TemporalDeadZone
                        } else {
                            VariableKind::Stack {
                                stack_slot,
                                mutable,
                            }
                        }
                    } else {
                        VariableKind::Stack {
                            stack_slot,
                            mutable,
                        }
                    }
                } else {
                    let scope_id = scoping.symbol_scope_id(s);
                    let scope_flags = scoping.scope_flags(scope_id);
                    // Functions declarations and variables defined at the top
                    // level scope end up in the globalThis; we want a property
                    // lookup cache for those.
                    if scope_flags.contains(ScopeFlags::Top)
                        && (symbol_flags.contains(SymbolFlags::FunctionScopedVariable)
                            | symbol_flags.contains(SymbolFlags::Function))
                    {
                        VariableKind::Global
                    } else {
                        VariableKind::Local
                    }
                }
            } else {
                // No SymbolId means this must be a global name.
                VariableKind::Global
            }
        } else {
            // No reference at all: global I guess?
            VariableKind::Global
        };
        kind.compile(ctx, &self.name)
    }
}

/// Formal parameter lists also have a temporal dead-zone; when the list does
/// not contain duplicates (which we consider always escaping), any reference to
/// later parameters from earlier parameters' default expressions is in a TDZ.
fn is_parameter_tdz(nodes: &oxc_semantic::AstNodes, decl_id: NodeId, ref_id: NodeId) -> bool {
    let decl_parent_id = nodes.parent_id(decl_id);
    let oxc_ast::AstKind::FormalParameters(_) = nodes.get_node(decl_parent_id).kind() else {
        // If the declaration isn't a formal parameter, then this cannot be in
        // TDZ.
        return false;
    };
    // Reference points to a formal parameter: this means that we might be in
    // its TDZ.
    if ref_id < decl_id {
        // If our reference is before the formal parameter then we definitely
        // are in the TDZ.
        return true;
    }
    // If our reference comes after the formal parameter we might still be in
    // the TDZ but a followup check tests for that.
    false
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BindingIdentifier<'s> {
    type Output = Place<'s, 'gc>;
    /// Compile variable binding. This is used to create a variable.
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let kind = {
            let s = self.symbol_id();
            if let Some(stack_slot) = ctx.get_variable_stack_index(s) {
                // We're a stack variable declaration.
                VariableKind::Stack {
                    stack_slot,
                    // Variable declarations can always mutate the stack slot.
                    mutable: true,
                }
            } else {
                let source_code = ctx.get_source_code();
                let scoping = source_code.get_scoping(ctx.get_agent());
                let scope_id = scoping.symbol_scope_id(s);
                let scope_flags = scoping.scope_flags(scope_id);
                let symbol_flags = scoping.symbol_flags(s);
                // Functions declarations and variables defined at the top
                // level scope end up in the globalThis; we want a property
                // lookup cache for those.
                if scope_flags.contains(ScopeFlags::Top)
                    && (symbol_flags.contains(SymbolFlags::FunctionScopedVariable)
                        | symbol_flags.contains(SymbolFlags::Function))
                {
                    VariableKind::Global
                } else {
                    VariableKind::Local
                }
            }
        };
        kind.compile(ctx, &self.name)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::IdentifierName<'s> {
    type Output = Place<'s, 'gc>;

    /// Property name in member expressions etc. Has nothing to do with `foo`
    /// in `let foo` unlike type documentation states.
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let identifier = ctx.create_string(self.name.as_str());
        ctx.add_instruction_with_identifier(
            Instruction::EvaluatePropertyAccessWithIdentifierKey,
            identifier.to_property_key(),
        );
        identifier.to_property_key().into()
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::UnaryExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    /// # ['a 13.5 Unary Operators](https://tc39.es/ecma262/#sec-unary-operators)
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        match self.operator {
            // 13.5.5 Unary - Operator
            // https://tc39.es/ecma262/#sec-unary-minus-operator-runtime-semantics-evaluation
            // UnaryExpression : - UnaryExpression
            UnaryOperator::UnaryNegation => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                self.argument.compile(ctx)?.get_value(ctx)?;
                ctx.add_instruction(Instruction::ToNumeric);

                // 3. If oldValue is a Number, then
                //    a. Return Number::unaryMinus(oldValue).
                // 4. Else,
                //    a. Assert: oldValue is a BigInt.
                //    b. Return BigInt::unaryMinus(oldValue).
                ctx.add_instruction(Instruction::UnaryMinus);
                Ok(ValueOutput::Value)
            }
            // 13.5.4 Unary + Operator
            // https://tc39.es/ecma262/#sec-unary-plus-operator
            // UnaryExpression : + UnaryExpression
            UnaryOperator::UnaryPlus => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                // 2. Return ? ToNumber(? GetValue(expr)).
                self.argument.compile(ctx)?.get_value(ctx)?;
                ctx.add_instruction(Instruction::ToNumber);
                Ok(ValueOutput::Value)
            }
            // 13.5.6 Unary ! Operator
            // https://tc39.es/ecma262/#sec-logical-not-operator-runtime-semantics-evaluation
            // UnaryExpression : ! UnaryExpression
            UnaryOperator::LogicalNot => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                // 2. Let oldValue be ToBoolean(? GetValue(expr)).
                self.argument.compile(ctx)?.get_value(ctx)?;
                // 3. If oldValue is true, return false.
                // 4. Return true.
                ctx.add_instruction(Instruction::LogicalNot);
                Ok(ValueOutput::Value)
            }
            // 13.5.7 Unary ~ Operator
            // https://tc39.es/ecma262/#sec-bitwise-not-operator-runtime-semantics-evaluation
            // UnaryExpression : ~ UnaryExpression
            UnaryOperator::BitwiseNot => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                // 2. Let oldValue be ? ToNumeric(? GetValue(expr)).
                self.argument.compile(ctx)?.get_value(ctx)?;
                ctx.add_instruction(Instruction::ToNumeric);

                // 3. If oldValue is a Number, then
                //    a. Return Number::bitwiseNOT(oldValue).
                // 4. Else,
                //    a. Assert: oldValue is a BigInt.
                //    b. Return BigInt::bitwiseNOT(oldValue).
                ctx.add_instruction(Instruction::BitwiseNot);
                Ok(ValueOutput::Value)
            }
            // 13.5.3 The typeof Operator
            // UnaryExpression : typeof UnaryExpression
            UnaryOperator::Typeof => {
                // 1. Let val be ? Evaluation of UnaryExpression.
                let val = self.argument.compile(ctx)?;
                if val.is_stack_variable() {
                    // Stack variables would normally be references but as
                    // they have no Reference (and are known to be resolvable),
                    // we call GetValue directly.
                    val.get_value(ctx)?;
                }
                // 3. Set val to ? GetValue(val).
                ctx.add_instruction(Instruction::Typeof);
                Ok(ValueOutput::Value)
            }
            // 13.5.2 The void operator
            // UnaryExpression : void UnaryExpression
            UnaryOperator::Void => {
                // 1. Let expr be ? Evaluation of UnaryExpression.
                // NOTE: GetValue must be called even though its value is not used because it may have observable side-effects.
                // 2. Perform ? GetValue(expr).
                if !self.argument.is_literal() {
                    self.argument.compile(ctx)?.get_value(ctx)?;
                }
                // 3. Return undefined.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                Ok(ValueOutput::Value)
            }
            // 13.5.1 The delete operator
            // https://tc39.es/ecma262/#sec-delete-operator-runtime-semantics-evaluation
            // UnaryExpression : delete UnaryExpression
            UnaryOperator::Delete => {
                // Let ref be ? Evaluation of UnaryExpression.
                self.argument.compile(ctx)?.delete(ctx)
            }
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BinaryExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let lref be ? Evaluation of leftOperand.
        let lref = self.left.compile(ctx)?;
        // 2. Let lval be ? GetValue(lref).
        let _lval = lref.get_value(ctx)?;
        let lval_on_stack = ctx.load_to_stack();

        // 3. Let rref be ? Evaluation of rightOperand.
        let rref = self.right.compile(ctx);
        // 4. Let rval be ? GetValue(rref).
        let rval = rref.and_then(|r| r.get_value(ctx));

        if let Err(err) = rval {
            lval_on_stack.forget(ctx);
            return Err(err);
        }

        let op_text = match self.operator {
            BinaryOperator::LessThan => Instruction::LessThan,
            BinaryOperator::LessEqualThan => Instruction::LessThanEquals,
            BinaryOperator::GreaterThan => Instruction::GreaterThan,
            BinaryOperator::GreaterEqualThan => Instruction::GreaterThanEquals,
            BinaryOperator::StrictEquality => Instruction::IsStrictlyEqual,
            BinaryOperator::StrictInequality => {
                ctx.add_instruction(Instruction::IsStrictlyEqual);
                Instruction::LogicalNot
            }
            BinaryOperator::Equality => Instruction::IsLooselyEqual,
            BinaryOperator::Inequality => {
                ctx.add_instruction(Instruction::IsLooselyEqual);
                Instruction::LogicalNot
            }
            BinaryOperator::In => Instruction::HasProperty,
            BinaryOperator::Instanceof => Instruction::InstanceofOperator,
            BinaryOperator::Addition => Instruction::ApplyAdditionBinaryOperator,
            BinaryOperator::Subtraction => Instruction::ApplySubtractionBinaryOperator,
            BinaryOperator::Multiplication => Instruction::ApplyMultiplicationBinaryOperator,
            BinaryOperator::Division => Instruction::ApplyDivisionBinaryOperator,
            BinaryOperator::Remainder => Instruction::ApplyRemainderBinaryOperator,
            BinaryOperator::Exponential => Instruction::ApplyExponentialBinaryOperator,
            BinaryOperator::ShiftLeft => Instruction::ApplyShiftLeftBinaryOperator,
            BinaryOperator::ShiftRight => Instruction::ApplyShiftRightBinaryOperator,
            BinaryOperator::ShiftRightZeroFill => {
                Instruction::ApplyShiftRightZeroFillBinaryOperator
            }
            BinaryOperator::BitwiseOR => Instruction::ApplyBitwiseORBinaryOperator,
            BinaryOperator::BitwiseXOR => Instruction::ApplyBitwiseXORBinaryOperator,
            BinaryOperator::BitwiseAnd => Instruction::ApplyBitwiseAndBinaryOperator,
        };
        // 5. Return ? ApplyStringOrNumericBinaryOperator(lval, opText, rval).
        lval_on_stack.forget(ctx);
        ctx.add_instruction(op_text);
        Ok(ValueOutput::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::LogicalExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let lref = self.left.compile(ctx)?;
        let lval = lref.get_value(ctx)?;

        // We store the left value on the stack, because we'll need to restore
        // it later.
        let lval_copy = ctx.load_copy_to_stack();

        match self.operator {
            oxc_syntax::operator::LogicalOperator::Or => {
                ctx.add_instruction(Instruction::LogicalNot);
            }
            oxc_syntax::operator::LogicalOperator::And => {}
            oxc_syntax::operator::LogicalOperator::Coalesce => {
                ctx.add_instruction(Instruction::IsNullOrUndefined);
            }
        }
        let jump_to_return_left = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);

        // We're returning the right expression, so we discard the left value
        // at the top of the stack.
        lval_copy.pop(ctx);

        let rref = self.right.compile(ctx);
        let rval = rref.and_then(|r| r.get_value(ctx));

        let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::Jump);

        ctx.set_jump_target_here(jump_to_return_left);
        // Return the result of the left expression.
        let lval_copy = ctx.mark_stack_value();
        lval_copy.store(ctx);
        ctx.set_jump_target_here(jump_to_end);
        combine_value_results(Ok(lval), rval)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ParenthesizedExpression<'s>
{
    type Output = Result<PlaceOrValue<'s, 'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        self.expression.compile(ctx)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ArrowFunctionExpression<'s>
{
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // CompileContext holds a name identifier for us if this is NamedEvaluation.
        let identifier = ctx.name_identifier.take();
        ctx.add_arrow_function_expression(ArrowFunctionExpression {
            expression: SendableRef::new(unsafe {
                core::mem::transmute::<
                    &ast::ArrowFunctionExpression<'_>,
                    &'static ast::ArrowFunctionExpression<'static>,
                >(self)
            }),
            identifier,
        });
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::Function<'s> {
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // CompileContext holds a name identifier for us if this is NamedEvaluation.
        let identifier = ctx.name_identifier.take();
        ctx.add_instruction_with_function_expression(
            Instruction::InstantiateOrdinaryFunctionExpression,
            FunctionExpression {
                expression: SendableRef::new(unsafe {
                    core::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                        self,
                    )
                }),
                identifier,
                compiled_bytecode: None,
            },
        );
    }
}

fn create_object_with_shape<'s, 'gc>(
    expr: &'s ast::ObjectExpression<'s>,
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
) -> Result<ValueOutput<'gc>, ExpressionError> {
    let proto_prop = expr.properties.iter().find(|prop| {
        let ast::ObjectPropertyKind::ObjectProperty(prop) = prop else {
            unreachable!()
        };
        prop.key.is_specific_static_name("__proto__")
            && prop.kind == ast::PropertyKind::Init
            && !prop.shorthand
    });
    let prototype = if let Some(proto_prop) = proto_prop {
        let ast::ObjectPropertyKind::ObjectProperty(proto_prop) = proto_prop else {
            unreachable!()
        };
        if proto_prop.value.is_null() {
            None
        } else {
            Some(
                ctx.get_agent()
                    .current_realm_record()
                    .intrinsics()
                    .object_prototype()
                    .into_object(),
            )
        }
    } else {
        Some(
            ctx.get_agent()
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .into_object(),
        )
    };
    let mut shape = ObjectShape::get_shape_for_prototype(ctx.get_agent_mut(), prototype);
    let mut prop_values_on_stack: Vec<StackValue> = Vec::with_capacity(expr.properties.len());
    for prop in expr.properties.iter() {
        let ast::ObjectPropertyKind::ObjectProperty(prop) = prop else {
            unreachable!()
        };
        if !prop.shorthand && prop.key.is_specific_static_name("__proto__") {
            continue;
        }
        let ast::PropertyKey::StaticIdentifier(id) = &prop.key else {
            unreachable!()
        };
        let identifier = ctx.create_property_key(&id.name);
        shape = shape
            .get_child_shape(ctx.get_agent_mut(), identifier)
            .expect("Should perform GC here");
        if is_anonymous_function_definition(&prop.value) {
            ctx.add_instruction_with_constant(Instruction::StoreConstant, identifier);
            ctx.name_identifier = Some(NamedEvaluationParameter::Result);
        }
        if let Err(err) = prop.value.compile(ctx).and_then(|r| r.get_value(ctx)) {
            for prop_on_stack in prop_values_on_stack {
                prop_on_stack.forget(ctx);
            }
            return Err(err);
        }

        prop_values_on_stack.push(ctx.load_to_stack());
    }
    // ObjectCreateWithShape consumes the props from the stack.
    for prop_on_stack in prop_values_on_stack {
        prop_on_stack.forget(ctx);
    }
    ctx.add_instruction_with_shape(Instruction::ObjectCreateWithShape, shape);
    Ok(ValueOutput::Value)
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ObjectExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if !self.properties.is_empty()
            && self.properties.iter().all(|prop| {
                !prop.is_spread() && {
                    let ast::ObjectPropertyKind::ObjectProperty(prop) = prop else {
                        unreachable!()
                    };
                    prop.kind == ast::PropertyKind::Init
                        && !prop.method
                        && prop.key.is_identifier()
                        && if prop.key.is_specific_static_name("__proto__") && !prop.shorthand {
                            prop.value.is_null_or_undefined()
                        } else {
                            true
                        }
                }
            })
        {
            let mut dedup_keys = self
                .properties
                .iter()
                .map(|prop| {
                    let ast::ObjectPropertyKind::ObjectProperty(prop) = prop else {
                        unreachable!()
                    };
                    let ast::PropertyKey::StaticIdentifier(key) = &prop.key else {
                        unreachable!()
                    };
                    key.name.as_str()
                })
                .collect::<Vec<_>>();
            dedup_keys.sort();
            dedup_keys.dedup();
            // Check that there are no duplicates.
            if dedup_keys.len() == self.properties.len() {
                // Can create Object Shape beforehand and calculate
                return create_object_with_shape(self, ctx);
            }
        }
        // TODO: Consider preparing the properties onto the stack and creating
        // the object with a known size.
        ctx.add_instruction(Instruction::ObjectCreate);
        let obj = ctx.mark_stack_value();
        for property in self.properties.iter() {
            match property {
                ast::ObjectPropertyKind::ObjectProperty(prop) => {
                    let mut is_proto_setter = false;
                    match &prop.key {
                        // It shouldn't be possible for objects to be created
                        // with private identifiers as keys.
                        ast::PropertyKey::PrivateIdentifier(_) => unreachable!(),
                        ast::PropertyKey::StaticIdentifier(id) => {
                            if id.name == "__proto__" {
                                if prop.kind == ast::PropertyKind::Init && !prop.shorthand {
                                    // If property key is "__proto__" then we
                                    // should dispatch a SetPrototype instruction.
                                    is_proto_setter = true;
                                } else {
                                    ctx.add_instruction_with_constant(
                                        Instruction::StoreConstant,
                                        BUILTIN_STRING_MEMORY.__proto__,
                                    );
                                }
                            } else {
                                let identifier = ctx.create_property_key(&id.name);
                                ctx.add_instruction_with_constant(
                                    Instruction::StoreConstant,
                                    identifier,
                                );
                            }
                        }
                        _ => {
                            let prop_key = prop.key.as_expression().unwrap();
                            if is_reference(prop_key) {
                                assert!(!is_proto_setter);
                            }
                            if let Err(err) = prop_key.compile(ctx).and_then(|r| r.get_value(ctx)) {
                                obj.forget(ctx);
                                return Err(err);
                            }
                        }
                    }
                    match prop.kind {
                        ast::PropertyKind::Init => {
                            if is_proto_setter {
                                if let Err(err) =
                                    prop.value.compile(ctx).and_then(|r| r.get_value(ctx))
                                {
                                    obj.forget(ctx);
                                    return Err(err);
                                }
                                // 7. If isProtoSetter is true, then
                                // a. If propValue is an Object or propValue is null, then
                                //     i. Perform ! object.[[SetPrototypeOf]](propValue).
                                // b. Return unused.
                                ctx.add_instruction(Instruction::ObjectSetPrototype);
                            } else if prop.method {
                                let ast::Expression::FunctionExpression(value) = &prop.value else {
                                    unreachable!()
                                };
                                let identifier = if is_anonymous_function_definition(&prop.value) {
                                    Some(NamedEvaluationParameter::Stack)
                                } else {
                                    None
                                };
                                // Note: not load_copy_to_stack as this is
                                // immediately consumed
                                ctx.add_instruction(Instruction::Load);
                                ctx.add_instruction_with_function_expression_and_immediate(
                                    Instruction::ObjectDefineMethod,
                                    FunctionExpression {
                                        expression: SendableRef::new(unsafe {
                                            core::mem::transmute::<
                                                &ast::Function<'_>,
                                                &'static ast::Function<'static>,
                                            >(value)
                                        }),
                                        identifier,
                                        compiled_bytecode: None,
                                    },
                                    // enumerable: true,
                                    true.into(),
                                );
                            } else {
                                if is_anonymous_function_definition(&prop.value) {
                                    ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
                                }
                                let key_copy = ctx.load_to_stack();
                                let result = prop.value.compile(ctx).and_then(|r| r.get_value(ctx));
                                // Note: key copy is either forgotten on stack
                                // and gets cleaned up by try-catch if result is
                                // Err, or is consumed by ObjectDefineProperty.
                                key_copy.forget(ctx);
                                if let Err(err) = result {
                                    obj.forget(ctx);
                                    return Err(err);
                                }
                                ctx.add_instruction(Instruction::ObjectDefineProperty);
                            }
                        }
                        ast::PropertyKind::Get | ast::PropertyKind::Set => {
                            // Note: no load_copy_to_stack as this is
                            // immediately consumed.
                            ctx.add_instruction(Instruction::Load);
                            let is_get = prop.kind == ast::PropertyKind::Get;
                            let ast::Expression::FunctionExpression(function_expression) =
                                &prop.value
                            else {
                                unreachable!()
                            };
                            ctx.add_instruction_with_function_expression_and_immediate(
                                if is_get {
                                    Instruction::ObjectDefineGetter
                                } else {
                                    Instruction::ObjectDefineSetter
                                },
                                FunctionExpression {
                                    expression: SendableRef::new(unsafe {
                                        core::mem::transmute::<
                                            &ast::Function<'_>,
                                            &'static ast::Function<'static>,
                                        >(
                                            function_expression
                                        )
                                    }),
                                    identifier: None,
                                    compiled_bytecode: None,
                                },
                                // enumerable: true,
                                true.into(),
                            );
                        }
                    }
                }
                ast::ObjectPropertyKind::SpreadProperty(spread) => {
                    if let Err(err) = spread.argument.compile(ctx).and_then(|r| r.get_value(ctx)) {
                        obj.forget(ctx);
                        return Err(err);
                    }
                    ctx.add_instruction(Instruction::CopyDataProperties);
                }
            }
        }
        // 3. Return obj
        obj.store(ctx);
        Ok(ValueOutput::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ArrayExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let elements_min_count = self.elements.len();
        ctx.add_instruction_with_immediate(Instruction::ArrayCreate, elements_min_count);
        if self.elements.is_empty() {
            return Ok(ValueOutput::Value);
        }
        let array_on_stack = ctx.load_to_stack();
        let try_catch_block = if self
            .elements
            .iter()
            .all(|e| e.is_elision() || e.as_expression().is_some_and(|e| e.is_literal()))
        {
            // Note: if all elements are elisions or literals, then the
            // whole ArrayExpression is infallible.
            None
        } else {
            Some(ctx.enter_try_catch_block())
        };
        let mut jumps_to_pop_iterator = vec![];
        let mut err = None;
        for ele in &self.elements {
            match ele {
                ast::ArrayExpressionElement::SpreadElement(spread) => {
                    if let Err(e) = spread.argument.compile(ctx).and_then(|s| s.get_value(ctx)) {
                        err = Some(e);
                        break;
                    }
                    let sync_iterator = ctx.push_sync_iterator();

                    let iteration_start = ctx.get_jump_index_to_here();
                    let iteration_end =
                        ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue);
                    ctx.add_instruction(Instruction::ArrayPush);
                    ctx.add_jump_instruction_to_index(Instruction::Jump, iteration_start);
                    ctx.set_jump_target_here(iteration_end);
                    jumps_to_pop_iterator.push(sync_iterator.exit(ctx));
                }
                ast::ArrayExpressionElement::Elision(_) => {
                    ctx.add_instruction(Instruction::ArrayElision);
                }
                _ => {
                    let expression = ele.to_expression();
                    if let Err(e) = expression.compile(ctx).and_then(|s| s.get_value(ctx)) {
                        err = Some(e);
                        break;
                    }
                    ctx.add_instruction(Instruction::ArrayPush);
                }
            }
        }
        if let Some(try_catch_block) = try_catch_block {
            // Note: if our ArrayExpression is fallible, then we need to
            // compile our catch block here and (unfortunately) also jump over
            // it as well.
            let jump_to_update_empty = try_catch_block.exit(ctx);
            let jump_over_catch = ctx.add_instruction_with_jump_slot(Instruction::Jump);
            // ## Catch block
            if !jumps_to_pop_iterator.is_empty() {
                for jump in jumps_to_pop_iterator {
                    ctx.set_jump_target_here(jump);
                }
                // Rest iterator threw an error: pop the jump_to_update_empty
                // exception handler and the failing iterator off their stacks.
                // Note: IteratorPop is infallible, so we can pop here safely.
                ctx.add_instruction(Instruction::PopExceptionJumpTarget);
                ctx.add_instruction(Instruction::IteratorPop);
            }
            ctx.set_jump_target_here(jump_to_update_empty);
            // Note: we use UpdateEmpty to pop the Array off the stack here,
            // since the result register is always non-empty in throw paths.
            ctx.add_instruction(Instruction::UpdateEmpty);
            ctx.add_instruction(Instruction::Throw);
            ctx.set_jump_target_here(jump_over_catch);
        } else {
            // If we have an infallible loop, it cannot contain a spread
            // element.
            debug_assert!(jumps_to_pop_iterator.is_empty());
        }
        array_on_stack.store(ctx);
        if let Some(err) = err {
            Err(err)
        } else {
            Ok(ValueOutput::Value)
        }
    }
}

fn compile_arguments<'s>(
    arguments: &'s [ast::Argument<'s>],
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) -> Result<usize, ExpressionError> {
    let mut static_unwind_try_catch_blocks = if arguments.len() == 1
        && arguments.first().unwrap().is_expression()
        || arguments
            .iter()
            .all(|arg| arg.as_expression().is_some_and(|expr| expr.is_literal()))
    {
        // If we have just one non-spread argument, or all parameters are
        // literals (have no side-effects whatsoever) then we know the
        // arguments compilation is infallible (or fails with no items pushed
        // onto the stack), and we don't need a try-catch block here.
        None
    } else {
        // We'll need at most IndexType::MAX unwind sites.
        Some(Vec::with_capacity(
            arguments.len().min(IndexType::MAX as usize),
        ))
    };
    let mut try_catch_block = None;
    let mut jump_to_iterator_pop = None;
    // If the arguments don't contain the spread operator, then we can know the
    // number of arguments at compile-time and we can pass it as an argument to
    // the call instruction.
    // Otherwise, the first time we find a spread operator, we need to start
    // tracking the number of arguments in the compiled bytecode. We store this
    // number in the result value, and we pass u16::MAX to the call instruction.
    let mut known_num_arguments = Some(0 as IndexType);

    let mut err = None;
    for argument in arguments {
        // If known_num_arguments is None, the stack contains the number of
        // arguments, followed by the arguments.
        if let ast::Argument::SpreadElement(spread) = argument {
            if let Some(num_arguments) = known_num_arguments.take() {
                ctx.add_instruction_with_constant(Instruction::LoadConstant, num_arguments);
                try_catch_block = Some(ctx.enter_try_catch_block());
            }

            if let Err(e) = spread.argument.compile(ctx).and_then(|s| s.get_value(ctx)) {
                err = Some(e);
                break;
            };
            let iterator = ctx.push_sync_iterator();

            let iteration_start = ctx.get_jump_index_to_here();
            let iteration_end = ctx.add_instruction_with_jump_slot(Instruction::IteratorStepValue);
            // result: value; stack: [num, ...args]
            ctx.add_instruction(Instruction::LoadStoreSwap);
            // result: num; stack: [value, ...args]
            ctx.add_instruction(Instruction::Increment);
            // result: num + 1; stack: [value, ...args]
            ctx.add_instruction(Instruction::Load);
            // stack: [num + 1, value, ...args]
            ctx.add_jump_instruction_to_index(Instruction::Jump, iteration_start);
            ctx.set_jump_target_here(iteration_end);
            jump_to_iterator_pop = Some(iterator.exit(ctx));
        } else {
            let expression = argument.to_expression();

            if let Err(e) = expression.compile(ctx).and_then(|s| s.get_value(ctx)) {
                err = Some(e);
                break;
            }
            if let Some(num_arguments) = known_num_arguments.as_mut() {
                ctx.add_instruction(Instruction::Load);
                // stack: [value, ...args]

                if *num_arguments < IndexType::MAX - 1 {
                    // If we know the number of arguments statically and we need
                    // unwinding, then we need to push something to the static
                    // unwinding jumps here as we've loaded one extra value to
                    // the stack.
                    *num_arguments += 1;
                    if let Some(jumps_to_static_unwind) = static_unwind_try_catch_blocks.as_mut() {
                        // If the next argument is a literal, then we won't
                        // need a catch handler for it.
                        let next_index = *num_arguments as usize;
                        if let Some(next_argument) = arguments.get(next_index) {
                            // Next argument exists; we might need a catch
                            // handler.
                            if next_argument
                                .as_expression()
                                .is_some_and(|e| e.is_literal())
                            {
                                // Next argument is a literal: it doesn't need
                                // catch but a subsequent arg might, and it
                                // needs to know how many values we pushed onto
                                // the stack. Hence, a None is pushed here.
                                jumps_to_static_unwind.push(None);
                            } else {
                                // Next argument isn't a literal; needs catch.
                                jumps_to_static_unwind.push(Some(ctx.enter_try_catch_block()));
                            }
                        }
                    }
                } else {
                    // If we overflow, we switch to tracking the number on the
                    // result value.
                    debug_assert_eq!(*num_arguments, IndexType::MAX - 1);
                    known_num_arguments = None;
                    ctx.add_instruction_with_constant(
                        Instruction::LoadConstant,
                        Value::from(IndexType::MAX),
                    );
                    try_catch_block = Some(ctx.enter_try_catch_block());
                    // stack: [num + 1, value, ...args]
                }
            } else {
                // result: value; stack: [num, ...args]
                ctx.add_instruction(Instruction::LoadStoreSwap);
                // result: num; stack: [value, ...args]
                ctx.add_instruction(Instruction::Increment);
                // result: num + 1; stack: [value, ...args]
                ctx.add_instruction(Instruction::Load);
                // stack: [num + 1, value, ...args]
            }
        }
    }

    let result = if let Some(num_arguments) = known_num_arguments {
        assert_ne!(num_arguments, IndexType::MAX);
        num_arguments as usize
    } else {
        // stack: [num, ...args]
        ctx.add_instruction(Instruction::Store);
        // result: num; stack: [...args]
        IndexType::MAX as usize
    };

    // Exit our try-catch blocks.
    let jumps_to_static_unwind =
        static_unwind_try_catch_blocks.map(|static_unwind_try_catch_blocks| {
            static_unwind_try_catch_blocks
                .into_iter()
                .map(|e| e.map(|e| e.exit(ctx)))
                .collect::<Vec<_>>()
        });
    let jump_to_dynamic_unwind = try_catch_block.map(|b| b.exit(ctx));

    if let Some(mut jumps_to_static_unwind) = jumps_to_static_unwind {
        let jump_over_catch = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        // ## Catch block
        if let Some(jump_to_iterator_pop) = jump_to_iterator_pop {
            debug_assert!(jump_to_dynamic_unwind.is_some());
            ctx.set_jump_target_here(jump_to_iterator_pop);
            // Arguments spread threw an error: we need to pop the
            // jump_to_dynamic_unwind exception handler, pop the iterator
            // stack, and then continue into the jump_to_dynamic_unwind
            // catch block.
            ctx.add_instruction(Instruction::PopExceptionJumpTarget);
            ctx.add_instruction(Instruction::IteratorPop);
        }
        if let Some(jump_to_dynamic_unwind) = jump_to_dynamic_unwind {
            ctx.set_jump_target_here(jump_to_dynamic_unwind);
            let error = ctx.mark_stack_value();
            // When we enter the catch block with a dynamic number of
            // arguments, our stack situation looks like this:
            // result: error; stack: [num, ...args]
            // We need to remove our statically known exception jump targets
            // and then pop off the dynamic number of arguments from the stack.
            // Finally, we of course need to rethrow our error.
            for e in jumps_to_static_unwind.iter() {
                // Pop all the static exception targets.
                if e.is_some() {
                    ctx.add_instruction(Instruction::PopExceptionJumpTarget);
                }
            }
            // result: error; stack: [num, ...args]
            ctx.add_instruction(Instruction::LoadStoreSwap);

            let continue_stack_unwind = ctx.get_jump_index_to_here();
            // result: num; stack: [error, ...args]
            let num_copy = ctx.load_copy_to_stack();
            // result: num; stack: [num, error, ...args]
            let finish_stack_unwind = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
            // result: None; stack: [num, error, ...args]
            num_copy.store(ctx);
            // result: num; stack: [error, ...args]
            ctx.add_instruction(Instruction::Decrement);
            // result: num - 1; stack: [error, ...args]
            ctx.add_instruction(Instruction::Swap);
            // result: num - 1; stack: [args[0], error, ...args[1..]]
            ctx.add_instruction(Instruction::UpdateEmpty);
            // result: num - 1; stack: [error, ...args[1..]]
            ctx.add_jump_instruction_to_index(Instruction::Jump, continue_stack_unwind);

            // === BREAK HERE - CONTROL FLOW NEVER PASSES THROUGH HERE ===
            ctx.set_jump_target_here(finish_stack_unwind);
            // result: None; stack: [num, error]
            let num_copy = ctx.mark_stack_value();
            num_copy.pop(ctx);
            error.store(ctx);
            // result: error; stack: []
            ctx.add_instruction(Instruction::Throw);
        }
        // Here is the static unwind logic: here we know exactly how many items
        // we've pushed to the stack (and when we threw an error). Each static
        // unwind jump target should thus drop one argument from stack and, if
        // it is not the first one, pop the next exception target.
        // result: error; stack: [...args]
        let mut is_first = true;
        while let Some(jump_to_static_unwind) = jumps_to_static_unwind.pop() {
            if let Some(jump_to_static_unwind) = jump_to_static_unwind {
                if !is_first {
                    // Pop this jump target the stack if we're not the first one.
                    // This is needed for fall-through cases.
                    ctx.add_instruction(Instruction::PopExceptionJumpTarget);
                }
                is_first = false;
                ctx.set_jump_target_here(jump_to_static_unwind);
            }
            // Note: it's possible that jump_to_static_unwind entries are None,
            // meaning that the argument was infallible. In that case we're
            // only interested in popping the value off the stack, but that
            // also is only needed if a previous exception jump target already
            // existed. eg. `foo(a, b, 1, 2, 3)` can only ever need to pop off
            // `a`, whereas `foo(a, 1, 2, 3, b)` may only ever need to pop off
            // `a, 1, 2, 3`, and `foo(a, 1, 2, b, 3, c)` may need to pop off
            // either `a, 1, 2`, or `a, 1, 2, b, 3`.
            if !is_first {
                // result: error; stack: [args[0], ...args[1..]]
                ctx.add_instruction(Instruction::UpdateEmpty);
                // result: error; stack: [...args[1..]]
            }
        }
        if is_first {
            // If we made it through the static unwind bit without encountering
            // a single JumpIndex, it means that all statically knowable
            // parameters are infallible or fail on an empty stack: This means
            // we don't need a rethrow as this location is unreachable.
            debug_assert!(ctx.is_unreachable());
        } else {
            // Now it is finally time to rethrow our original error!
            ctx.add_instruction(Instruction::Throw);
        }
        ctx.set_jump_target_here(jump_over_catch);
    } else {
        // If we have no need for a stack-unwind catch block, we should have no
        // need for an iterator pop or dynamic unwind either.
        debug_assert!(jump_to_iterator_pop.is_none());
        debug_assert!(jump_to_dynamic_unwind.is_none());
    }
    if let Some(err) = err {
        Err(err)
    } else {
        Ok(result)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::CallExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if !self.optional
            && let ast::Expression::Identifier(ident) = &self.callee
            && ident.name == "eval"
        {
            // Direct eval(...)
            let num_arguments = compile_arguments(&self.arguments, ctx)?;
            ctx.add_instruction_with_immediate(Instruction::DirectEvalCall, num_arguments);
            return Ok(ValueOutput::Value);
        } else if matches!(self.callee, ast::Expression::Super(_)) {
            // super(...)
            let num_arguments = compile_arguments(&self.arguments, ctx)?;
            ctx.add_instruction_with_immediate(Instruction::EvaluateSuper, num_arguments);
            return Ok(ValueOutput::Value);
        }
        // 1. Let ref be ? Evaluation of CallExpression.
        ctx.is_call_optional_chain_this = is_chain_expression(&self.callee);
        let r#ref = self.callee.compile(ctx)?;
        // Optimization: If we know arguments is empty, we don't need to
        // worry about arguments evaluation clobbering our function's this
        // reference.
        let need_pop_reference = r#ref.has_reference() && !self.arguments.is_empty();
        // 2. Let func be ? GetValue(ref).
        let _func = r#ref.get_value_keep_reference(ctx)?;
        if need_pop_reference {
            ctx.add_instruction(Instruction::PushReference);
        }

        let func_on_stack = if self.optional {
            // Optional Chains

            // Load copy of func to stack.
            let func_copy = ctx.load_copy_to_stack();
            // 3. If func is either undefined or null, then
            ctx.add_instruction(Instruction::IsNullOrUndefined);
            // a. Return undefined

            // To return undefined we jump over the rest of the call handling.
            let jump_over_call = if need_pop_reference {
                // If we need to pop the reference stack, then we must do it
                // here before we go to the nullish case handling.
                // Note the inverted jump condition here!
                let jump_to_call = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                // Now we're in our local nullish case handling.
                // First we pop our reference.
                ctx.add_instruction(Instruction::PopReference);
                // And now we're ready to jump over the call.
                let jump_over_call = ctx.add_instruction_with_jump_slot(Instruction::Jump);
                // But if we're jumping to call then we need to land here.
                ctx.set_jump_target_here(jump_to_call);
                jump_over_call
            } else {
                ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue)
            };
            // Register our jump slot to the chain nullish case handling.
            ctx.optional_chains.as_mut().unwrap().push(jump_over_call);
            func_copy
        } else {
            ctx.load_to_stack()
        };
        // If we're in an optional chain, we need to pluck it out while we're
        // compiling the parameters: They do not join our chain.
        let optional_chain = ctx.optional_chains.take();
        let num_arguments = compile_arguments(&self.arguments, ctx);
        // After we're done with compiling parameters we go back into the chain.
        if let Some(optional_chain) = optional_chain {
            ctx.optional_chains.replace(optional_chain);
        }
        // Note: func on stack is forgotten either by an error being thrown and
        // gets cleaned up by a try-catch, or is consumed by EvaluateCall.
        func_on_stack.forget(ctx);

        let num_arguments = num_arguments?;

        if need_pop_reference {
            ctx.add_instruction(Instruction::PopReference);
        }
        ctx.add_instruction_with_immediate(Instruction::EvaluateCall, num_arguments);
        Ok(ValueOutput::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::NewExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        self.callee.compile(ctx)?.get_value(ctx)?;
        let func_on_stack = ctx.load_to_stack();

        let num_arguments = compile_arguments(&self.arguments, ctx);
        // Note: func on stack gets dropped by try-catch or consumed by
        // EvaluateNew.
        func_on_stack.forget(ctx);
        ctx.add_instruction_with_immediate(Instruction::EvaluateNew, num_arguments?);
        Ok(ValueOutput::Value)
    }
}

/// Compile the baseReference part of a member expression with possible
/// optional chaining.
///
/// ```text
/// 1. Let baseReference be ? Evaluation of MemberExpression.
/// 2. Let baseValue be ? GetValue(baseReference).
/// 3. If baseValue is either undefined or null, then
///     a. Return undefined.
/// 4. Return ? ChainEvaluation of OptionalChain with arguments baseValue and baseReference.
/// ```
///
/// After this call, if optional chaining isn't present then the base value is
/// in the result register. If optional chaining is present, then the base
/// value is at the top of the stack.
fn compile_optional_base_reference<'s, 'gc>(
    object: &'s ast::Expression<'s>,
    is_optional: bool,
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
) -> Result<ValueOutput<'gc>, ExpressionError> {
    // 1. Let baseReference be ? Evaluation of MemberExpression.
    // 2. Let baseValue be ? GetValue(baseReference).
    let base_value = object.compile(ctx)?.get_value(ctx)?;

    if is_optional {
        // Optional Chains

        // Load copy of baseValue to stack.
        ctx.add_instruction(Instruction::LoadCopy);
        // 3. If baseValue is either undefined or null, then
        ctx.add_instruction(Instruction::IsNullOrUndefined);
        // a. Return undefined

        // To return undefined we jump over the property access.
        let jump_over_property_access = ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

        // Register our jump slot to the chain nullish case handling.
        ctx.optional_chains
            .as_mut()
            .unwrap()
            .push(jump_over_property_access);
    }
    Ok(base_value)
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ComputedMemberExpression<'s>
{
    type Output = Result<Place<'s, 'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if self.object.is_super() {
            // super[expression]
            let output = self.expression.compile(ctx)?.get_value(ctx)?;
            if let ValueOutput::Literal(literal) = output {
                let (agent, gc) = ctx.get_agent_and_gc();
                if let Some(identifier) = to_property_key_simple(agent, literal, gc) {
                    ctx.add_instruction_with_identifier(
                        Instruction::MakeSuperPropertyReferenceWithIdentifierKey,
                        identifier,
                    );
                    return Ok(identifier.into());
                }
            }
            ctx.add_instruction(Instruction::MakeSuperPropertyReferenceWithExpressionKey);
            return Ok(Place::Member { name: None });
        }
        compile_optional_base_reference(&self.object, self.optional, ctx)?;
        // If we do not have optional chaining present it means that base value
        // is currently in the result slot. We need to store it on the stack.
        if !self.optional {
            ctx.add_instruction(Instruction::Load);
        }
        let base_value_on_stack = ctx.mark_stack_value();

        // If we're in an optional chain, we need to pluck it out while we're
        // compiling the member expression: They do not join our chain.
        let optional_chain = ctx.optional_chains.take();
        // 1. Let baseReference be ? Evaluation of expression.
        // 2. Let baseValue be ? GetValue(baseReference).
        let output = self.expression.compile(ctx).and_then(|r| r.get_value(ctx));
        // After we're done with compiling the member expression we go back
        // into the chain.
        if let Some(optional_chain) = optional_chain {
            ctx.optional_chains.replace(optional_chain);
        }

        let output = match output {
            Ok(o) => o,
            Err(err) => {
                base_value_on_stack.forget(ctx);
                return Err(err);
            }
        };

        if let ValueOutput::Literal(literal) = output {
            let (agent, gc) = ctx.get_agent_and_gc();
            if let Some(identifier) = to_property_key_simple(agent, literal, gc) {
                base_value_on_stack.store(ctx);
                // 4. Return ? EvaluatePropertyAccessWithExpressionKey(baseValue, Expression, strict).
                ctx.add_instruction_with_identifier(
                    Instruction::EvaluatePropertyAccessWithIdentifierKey,
                    identifier,
                );
                return Ok(identifier.into());
            }
        }
        // 4. Return ? EvaluatePropertyAccessWithExpressionKey(baseValue, Expression, strict).
        base_value_on_stack.forget(ctx);
        ctx.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
        Ok(Place::Member { name: None })
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::StaticMemberExpression<'s>
{
    type Output = Result<Place<'s, 'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if self.object.is_super() {
            // super.property
            let identifier = ctx.create_string(self.property.name.as_str());
            ctx.add_instruction_with_identifier(
                Instruction::MakeSuperPropertyReferenceWithIdentifierKey,
                identifier.to_property_key(),
            );
            return Ok(identifier.to_property_key().into());
        }
        compile_optional_base_reference(&self.object, self.optional, ctx)?;
        // If we are in an optional chain then result will be on the top of the
        // stack. We need to pop it into the register slot in that case.
        if self.optional {
            ctx.add_instruction(Instruction::Store);
        }

        // 4. Return EvaluatePropertyAccessWithIdentifierKey(baseValue, IdentifierName, strict).
        Ok(self.property.compile(ctx))
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::PrivateFieldExpression<'s>
{
    type Output = Result<Place<'static, 'static>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        compile_optional_base_reference(&self.object, self.optional, ctx)?;
        // If we are in an optional chain then result will be on the top of the
        // stack. We need to pop it into the register slot in that case.
        if self.optional {
            ctx.add_instruction(Instruction::Store);
        }

        //  MemberExpression : MemberExpression . PrivateIdentifier
        // 3. Let fieldNameString be the StringValue of PrivateIdentifier.
        // 4. Return MakePrivateReference(baseValue, fieldNameString).

        // 4. Return EvaluatePropertyAccessWithIdentifierKey(baseValue, IdentifierName, strict).
        let identifier = ctx.create_string(&self.field.name);
        ctx.add_instruction_with_identifier(
            Instruction::MakePrivateReference,
            identifier.to_property_key(),
        );
        Ok(Place::Member { name: None })
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::AwaitExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let exprRef be ? Evaluation of UnaryExpression.
        // 2. Let value be ? GetValue(exprRef).
        let value = self.argument.compile(ctx)?.get_value(ctx)?;
        // 3. Return ? Await(value).
        ctx.add_instruction(Instruction::Await);
        Ok(value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ChainExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // It's possible that we're compiling a ChainExpression inside a call
        // that is itself in a ChainExpression. We will drop into the previous
        // chain in this case.
        let installed_own_chains = if ctx.optional_chains.is_none() {
            // We prepare for at least two chains to exist. One chain is often
            // enough but two is a bit safer. Three is rare.
            ctx.optional_chains.replace(Vec::with_capacity(2));
            true
        } else {
            false
        };
        let result = match &self.expression {
            ast::ChainElement::CallExpression(expr) => expr.compile(ctx),
            ast::ChainElement::ComputedMemberExpression(expr) => {
                let place = expr.compile(ctx);
                let result = place.and_then(|p| {
                    p.get_value_maybe_keep_reference(ctx, ctx.is_call_optional_chain_this)
                });
                ctx.is_call_optional_chain_this = false;
                result
            }
            ast::ChainElement::StaticMemberExpression(expr) => {
                let place = expr.compile(ctx);
                let result = place.and_then(|p| {
                    p.get_value_maybe_keep_reference(ctx, ctx.is_call_optional_chain_this)
                });
                ctx.is_call_optional_chain_this = false;
                result
            }
            ast::ChainElement::PrivateFieldExpression(expr) => {
                let place = expr.compile(ctx);
                let result = place.and_then(|p| {
                    p.get_value_maybe_keep_reference(ctx, ctx.is_call_optional_chain_this)
                });
                ctx.is_call_optional_chain_this = false;
                result
            }
            #[cfg(feature = "typescript")]
            ast::ChainElement::TSNonNullExpression(expr) => {
                let result = expr.expression.compile(ctx).and_then(|r| match r {
                    PlaceOrValue::Value(r) => Ok(r),
                    PlaceOrValue::Place(place) => {
                        place.get_value_maybe_keep_reference(ctx, ctx.is_call_optional_chain_this)
                    }
                });
                ctx.is_call_optional_chain_this = false;
                result
            }
            #[cfg(not(feature = "typescript"))]
            ast::ChainElement::TSNonNullExpression(_) => unreachable!(),
        };
        // If chain succeeded, we come here and should jump over the nullish
        // case handling.
        if installed_own_chains {
            let own_chains = ctx.optional_chains.take().unwrap();
            if !own_chains.is_empty() {
                let jump_over_return_undefined =
                    ctx.add_instruction_with_jump_slot(Instruction::Jump);
                for jump_to_return_undefined in own_chains {
                    ctx.set_jump_target_here(jump_to_return_undefined);
                }
                // All optional chains come here with a copy of their null or
                // undefined baseValue on the stack. Pop it off.
                ctx.add_instruction(Instruction::Store);
                // Replace any possible null with undefined.
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                ctx.set_jump_target_here(jump_over_return_undefined);
                // Note: because we have incoming jumps, it's possible for us to
                // return any result.
                Ok(ValueOutput::Value)
            } else {
                // If we have no incoming jumps, then the expression result
                // rules.
                result
            }
        } else {
            // If we're just a link in a chain, then our own result is our final
            // word.
            result
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::ConditionalExpression<'s>
{
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    /// # ['a 13.14 Conditional Operator ( ? : )](https://tc39.es/ecma262/#sec-conditional-operator)
    /// ### [13.14.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-conditional-operator-runtime-semantics-evaluation)
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let lref be ? Evaluation of ShortCircuitExpression.
        // 2. Let lval be ToBoolean(? GetValue(lref)).
        let _lval = self.test.compile(ctx)?.get_value(ctx)?;
        // Jump over first AssignmentExpression (consequent) if test fails.
        // Note: JumpIfNot performs ToBoolean from above step.
        let jump_to_second = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
        // 3. If lval is true, then
        // a. Let trueRef be ? Evaluation of the first AssignmentExpression.
        // b. Return ? GetValue(trueRef).
        let true_ref = self.consequent.compile(ctx).and_then(|c| c.get_value(ctx));
        // Jump over second AssignmentExpression (alternate).
        let jump_over_second = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        // 4. Else,
        ctx.set_jump_target_here(jump_to_second);
        // a. Let falseRef be ? Evaluation of the second AssignmentExpression.
        // b. Return ? GetValue(falseRef).
        let false_ref = self.alternate.compile(ctx).and_then(|c| c.get_value(ctx));
        ctx.set_jump_target_here(jump_over_second);
        combine_value_results(true_ref, false_ref)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ImportExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    /// ### [13.3.10.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-import-call-runtime-semantics-evaluation)
    ///
    /// ```text
    /// ImportCall : import ( AssignmentExpression , (opt) )
    ///
    /// 1. Return ? EvaluateImportCall(AssignmentExpression).
    /// ```
    ///
    /// ```text
    /// ImportCall : import ( AssignmentExpression , AssignmentExpression , (opt) )
    ///
    /// 1. Return ? EvaluateImportCall(the first AssignmentExpression, the second AssignmentExpression).
    /// ```
    ///
    /// ### [13.3.10.2 EvaluateImportCall ( specifierExpression \[ , optionsExpression \] )](https://tc39.es/ecma262/#sec-evaluate-import-call)
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // Note: referrer cannot change dynamically, so we don't need to get it
        // right here and now; we'll defer that to after all the other steps.
        // 3. Let specifierRef be ? Evaluation of specifierExpression.
        let specifier_ref = self.source.compile(ctx)?;
        // 4. Let specifier be ? GetValue(specifierRef).
        let _specifier = specifier_ref.get_value(ctx)?;
        // Note: no load_to_stack as we ImportCall consumes it immediately
        // if we don't have options.
        ctx.add_instruction(Instruction::Load);
        // 5. If optionsExpression is present, then
        if let Some(options) = &self.options {
            let specifier_on_stack = ctx.mark_stack_value();
            // a. Let optionsRef be ? Evaluation of optionsExpression.
            // b. Let options be ? GetValue(optionsRef).
            let options = options.compile(ctx).and_then(|r| r.get_value(ctx));
            specifier_on_stack.forget(ctx);
            options?;
        }
        // 6. Else,
        // a. Let options be undefined.
        // Note: we don't store an undefined constant; the ImportCall
        // instruction can take care of that.
        ctx.add_instruction(Instruction::ImportCall);
        Ok(ValueOutput::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::MetaProperty<'s> {
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if self.meta.name == "new" && self.property.name == "target" {
            ctx.add_instruction(Instruction::GetNewTarget);
        } else if self.meta.name == "import" && self.property.name == "meta" {
            ctx.add_instruction(Instruction::ImportMeta);
        } else {
            unreachable!()
        };
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::PrivateInExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    /// ### [13.10.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-relational-operators-runtime-semantics-evaluation)
    /// ###  RelationalExpression : PrivateIdentifier in ShiftExpression
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let privateIdentifier be the StringValue of PrivateIdentifier.
        let private_identifier = ctx.create_string(&self.left.name);
        // 2. Let rRef be ? Evaluation of ShiftExpression.
        // 3. Let rVal be ? GetValue(rRef).
        let _rval = self.right.compile(ctx)?.get_value(ctx)?;
        // 4. If rVal is not an Object, throw a TypeError exception.
        // 5. Let privateEnv be the running execution context's PrivateEnvironment.
        // 6. Assert: privateEnv is not null.
        // 7. Let privateName be ResolvePrivateIdentifier(privateEnv, privateIdentifier).
        ctx.add_instruction_with_identifier(
            Instruction::MakePrivateReference,
            private_identifier.to_property_key(),
        );
        // 8. If PrivateElementFind(rVal, privateName) is not empty, return true.
        // 9. Return false.
        ctx.add_instruction(Instruction::HasPrivateElement);
        Ok(ValueOutput::Value)
    }
}
#[cfg(feature = "regexp")]
impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::RegExpLiteral<'s> {
    type Output = ();
    /// ### [13.2.7.3 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-regular-expression-literals-runtime-semantics-evaluation)
    ///
    /// ```text
    /// PrimaryExpression : RegularExpressionLiteral
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let pattern be CodePointsToString(BodyText of RegularExpressionLiteral).
        // 2. Let flags be CodePointsToString(FlagText of RegularExpressionLiteral).

        let pattern = self.regex.pattern.text.as_str();
        // 3. Return ! RegExpCreate(pattern, flags).
        let regexp = ctx.create_regexp(pattern, self.regex.flags);
        ctx.add_instruction_with_constant(Instruction::StoreConstant, regexp);
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::SequenceExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    /// ### [13.16.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-comma-operator-runtime-semantics-evaluation)
    ///
    /// ```text
    /// Expression : Expression , AssignmentExpression
    /// ```
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let lRef be ? Evaluation of Expression.
        // 2. Perform ? GetValue(lRef).
        // 3. Let rRef be ? Evaluation of AssignmentExpression.
        // 4. Return ? GetValue(rRef).
        let (last, rest) = self.expressions.split_last().unwrap();
        for expr in rest {
            if expr.is_literal() {
                // Literals do not have observable side-effects when compiled,
                // we can skip these when they're not the last expression.
                continue;
            }
            // NOTE: GetValue must be called even though its value is not used
            // because it may have observable side-effects.
            expr.compile(ctx)?.get_value(ctx)?;
        }
        last.compile(ctx)?.get_value(ctx)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::Super {
    type Output = ();
    #[inline(always)]
    fn compile(&'s self, _ctx: &mut CompileContext<'_, 's, '_, '_>) -> Self::Output {
        // There's no work to be done here.
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope>
    for ast::TaggedTemplateExpression<'s>
{
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    /// ### [13.3.11 Tagged Templates](https://tc39.es/ecma262/#sec-tagged-templates)
    ///
    /// > NOTE: A tagged template is a function call where the arguments of the
    /// > call are derived from a TemplateLiteral (13.2.8). The actual
    /// > arguments include a template object (13.2.8.4) and the values
    /// > produced by evaluating the expressions embedded within the
    /// > TemplateLiteral.
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) -> Self::Output {
        //  MemberExpression : MemberExpression TemplateLiteral
        //  CallExpression : CallExpression TemplateLiteral

        // 1. Let tagRef be ? Evaluation of MemberExpression/CallExpression.
        let tag_ref = self.tag.compile(ctx)?;
        // 2. Let tagFunc be ? GetValue(tagRef).
        let _tag_func = tag_ref.get_value_keep_reference(ctx)?;
        let need_pop_reference =
            tag_ref.has_reference() && !self.quasi.is_no_substitution_template();
        if need_pop_reference {
            ctx.add_instruction(Instruction::PushReference);
        }
        // Load tagFunc to the stack.
        // TÄHÄN JÄIT
        let tag_func_on_stack = ctx.load_to_stack();

        // 3. Let thisCall be this MemberExpression.
        // 4. Let tailCall be IsInTailPosition(thisCall).
        // 5. Return ? EvaluateCall(tagFunc, tagRef, TemplateLiteral, tailCall).
        //    3. Let argList be ? ArgumentListEvaluation of arguments.

        // ### 13.3.8.1 Runtime Semantics: ArgumentListEvaluation

        //  TemplateLiteral : NoSubstitutionTemplate
        let mut num_arguments = 0;
        if self.quasi.is_no_substitution_template() {
            // 1. Let templateLiteral be this TemplateLiteral.
            // 2. Let siteObj be GetTemplateObject(templateLiteral).
            let (agent, gc) = ctx.get_agent_and_gc();
            let site_obj = get_template_object(agent, &self.quasi, gc);
            // 3. Return « siteObj ».
            ctx.add_instruction_with_constant(Instruction::LoadConstant, site_obj);
            num_arguments += 1;
        } else {
            // TemplateLiteral : SubstitutionTemplate

            // 1. Let templateLiteral be this TemplateLiteral.
            // 2. Let siteObj be GetTemplateObject(templateLiteral).
            let (agent, gc) = ctx.get_agent_and_gc();
            let site_obj = get_template_object(agent, &self.quasi, gc);
            ctx.add_instruction_with_constant(Instruction::LoadConstant, site_obj);
            num_arguments += 1;
            // 3. Let remaining be ? ArgumentListEvaluation of SubstitutionTemplate.
            // 4. Return the list-concatenation of « siteObj » and remaining.

            // SubstitutionTemplate : TemplateHead Expression TemplateSpans
            for expression in self.quasi.expressions.iter() {
                // 1. Let firstSubRef be ? Evaluation of Expression.
                // 2. Let firstSub be ? GetValue(firstSubRef).
                expression.compile(ctx)?.get_value(ctx)?;
                // 3. Let restSub be ? SubstitutionEvaluation of TemplateSpans.
                ctx.add_instruction(Instruction::Load);
                num_arguments += 1;
                // 4. Assert: restSub is a possibly empty List.
                // 5. Return the list-concatenation of « firstSub » and restSub.
            }
        }
        if need_pop_reference {
            ctx.add_instruction(Instruction::PopReference);
        }
        ctx.add_instruction_with_immediate(Instruction::EvaluateCall, num_arguments);
        Ok(ValueOutput::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::TemplateLiteral<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if let Some(quasi) = self.single_quasi() {
            let constant = ctx.create_string(&quasi);
            ctx.add_instruction_with_constant(Instruction::StoreConstant, constant);
            Ok(constant.into())
        } else {
            let mut count = 0;
            let mut quasis = self.quasis.as_slice();
            let mut expressions = self.expressions.as_slice();
            while let Some((head, rest)) = quasis.split_first() {
                quasis = rest;
                // 1. Let head be the TV of TemplateHead as defined in 12.9.6.
                let head = ctx.create_string(head.value.cooked.as_ref().unwrap().as_str());
                ctx.add_instruction_with_constant(Instruction::LoadConstant, head);
                count += 1;
                if let Some((expression, rest)) = expressions.split_first() {
                    expressions = rest;
                    // 2. Let subRef be ? Evaluation of Expression.
                    // 3. Let sub be ? GetValue(subRef).
                    expression.compile(ctx)?.get_value(ctx)?;
                    // 4. Let middle be ? ToString(sub).
                    // Note: This is done by StringConcat.
                    ctx.add_instruction(Instruction::Load);
                    count += 1;
                }
                // 5. Let tail be ? Evaluation of TemplateSpans.
            }
            // 6. Return the string-concatenation of head, middle, and tail.
            ctx.add_instruction_with_immediate(Instruction::StringConcat, count);
            Ok(ValueOutput::Value)
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ThisExpression {
    type Output = ();
    #[inline]
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        ctx.add_instruction(Instruction::ResolveThisBinding);
    }
}

/// ### [15.5.5 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-generator-function-definitions-runtime-semantics-evaluation)
///
/// ### YieldExpression : yield * AssignmentExpression
fn compile_delegate_yield_expression<'s>(
    expr: &'s ast::YieldExpression<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) -> Result<(), ExpressionError> {
    let assignment_expression = expr
        .argument
        .as_ref()
        .expect("Unhandled SyntaxError: yield * requires an argument");
    // 1. Let generatorKind be GetGeneratorKind().
    let generator_kind_is_async = ctx.is_async_generator();
    // 2. Assert: generatorKind is either sync or async.
    // 3. Let exprRef be ? Evaluation of AssignmentExpression.
    // 4. Let value be ? GetValue(exprRef).
    let _value = assignment_expression.compile(ctx)?.get_value(ctx)?;
    // 5. Let iteratorRecord be ? GetIterator(value, generatorKind).
    // If a ? throw happens after this, we need to pop the iterator before
    // allowing the error to continue onwards.
    let iterator = if generator_kind_is_async {
        ctx.push_async_iterator()
    } else {
        ctx.push_sync_iterator()
    };
    // 6. Let iterator be iteratorRecord.[[Iterator]].
    // 7. Let received be NormalCompletion(undefined).
    ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    let jump_over_repeat = ctx.add_instruction_with_jump_slot(Instruction::Jump);
    // 8. Repeat,
    let jump_to_repeat = ctx.get_jump_index_to_here();
    // We should be +1 try-catch block here.
    // NOTE: this here is the last part of the normal completion handling.
    ctx.add_instruction(Instruction::PopExceptionJumpTarget);
    // We should be +0 try-catch block here.
    ctx.set_jump_target_here(jump_over_repeat);
    // a. If received is a normal completion, then
    let (inner_result_yield_label, inner_result_handling_label, try_catch_block, jump_to_end) = {
        // ### Normal result handling
        // i. Let innerResult be ? Call(
        //        iteratorRecord.[[NextMethod]],
        //        iteratorRecord.[[Iterator]],
        //        « received.[[Value]] »
        //    ).
        ctx.add_instruction(Instruction::IteratorCallNextMethod);
        // We should be +0 try-catch block here.
        let inner_result_handling_label = ctx.get_jump_index_to_here();
        if generator_kind_is_async {
            // ii. If generatorKind is async, set innerResult to
            //     ? Await(innerResult).
            ctx.add_instruction(Instruction::Await);
        }
        let jump_to_end = ctx.add_instruction_with_jump_slot(Instruction::IteratorComplete);
        // iii. If innerResult is not an Object, throw a TypeError exception.
        // iv. Let done be ? IteratorComplete(innerResult).
        // v. If done is true, then
        //     1. Return ? IteratorValue(innerResult).

        let inner_result_yield_label = ctx.get_jump_index_to_here();
        // vi. If generatorKind is async,
        if generator_kind_is_async {
            // set received to Completion(
            //     AsyncGeneratorYield(? IteratorValue(innerResult))
            // ).
            ctx.add_instruction(Instruction::IteratorValue);
        }
        // +1
        let try_catch_block = ctx.enter_try_catch_block();
        // We should be +1 try-catch block here.
        // vii. Else, set received to Completion(GeneratorYield(innerResult)).
        ctx.add_instruction(Instruction::Yield);
        // Note: generators can be resumed with a Return instruction. For those
        // cases we need to generate Return handling here.
        ctx.add_jump_instruction_to_index(Instruction::Jump, jump_to_repeat);
        // Note: We need to observe the index here as the Yield above makes
        // this instruction pointer reachable even by jumping over the above
        // Jump.
        let _ = ctx.get_jump_index_to_here();
        (
            inner_result_yield_label,
            inner_result_handling_label,
            try_catch_block,
            jump_to_end,
        )
    };
    // c. Else, i. Assert: received is a return completion.
    let jump_to_throw_result_handling = {
        // ### Return result handling
        // We should be +1 try-catch block here.
        // +0
        let jump_to_throw_result_handling = try_catch_block.exit(ctx);
        let jump_over_return_call = ctx.add_instruction_with_jump_slot(Instruction::IteratorReturn);
        // ii. Let return be ? GetMethod(iterator, "return").
        // iii. If return is undefined, then ... (jump over return call)
        // iv. Let innerReturnResult be
        //     ? Call(return, iterator, « received.[[Value]] »).
        // v. If generatorKind is async,
        if generator_kind_is_async {
            // set innerReturnResult to ? Await(innerReturnResult).
            ctx.add_instruction(Instruction::Await);
        }
        // vi. If innerReturnResult is not an Object, throw a TypeError exception.
        // vii. Let done be ? IteratorComplete(innerReturnResult).
        // viii. If done is true, then
        //     1. Set value to ? IteratorValue(innerReturnResult).
        //     2. Return ReturnCompletion(value).
        let jump_to_return = ctx.add_instruction_with_jump_slot(Instruction::IteratorComplete);
        // ix. If generatorKind is async,
        //     set received to Completion(
        //         AsyncGeneratorYield(? IteratorValue(innerReturnResult))
        //     ).
        // x. Else, set received to
        //    Completion(GeneratorYield(innerReturnResult)).
        // Note: the above steps are a repeat of steps vi. and vii. from normal
        // completion handling, so we jump there to reduce duplication.
        ctx.add_jump_instruction_to_index(Instruction::Jump, inner_result_yield_label);

        // We should be +0 try-catch block here.
        ctx.set_jump_target_here(jump_over_return_call);
        // 1. Set value to received.[[Value]].
        ctx.set_jump_target_here(jump_to_return);
        // 2. If generatorKind is async, then
        // a. Set value to ? Await(value).
        // Note: compile_return performs await on value in async generators.
        // 3. Return ReturnCompletion(value).
        ctx.compile_return(true);
        jump_to_throw_result_handling
    };
    // b. Else if received is a throw completion, then
    let jump_to_iterator_pop = {
        // ### Throw result handling
        // We should be +0 try-catch block here.
        ctx.set_jump_target_here(jump_to_throw_result_handling);
        // b. Else if received is a throw completion, then
        // i. Let throw be ? GetMethod(iterator, "throw").
        let jump_over_throw_call = ctx.add_instruction_with_jump_slot(Instruction::IteratorThrow);
        // ii. If throw is not undefined, then
        // 1. Let innerResult be ? Call(throw, iterator, « received.[[Value]] »).
        // 2. If generatorKind is async,
        //    set innerResult to ? Await(innerResult).
        // 3. NOTE: Exceptions from the inner iterator throw method are
        //    propagated. Normal completions from an inner throw method are
        //    processed similarly to an inner next.
        // => we jump to normal inner result handling
        ctx.add_jump_instruction_to_index(Instruction::Jump, inner_result_handling_label);
        // 4. If innerResult is not an Object, throw a TypeError exception.
        // 5. Let done be ? IteratorComplete(innerResult).
        // 6. If done is true, then
        //    a. Return ? IteratorValue(innerResult).
        // 7. If generatorKind is async,
        //    set received to Completion(
        //        AsyncGeneratorYield(? IteratorValue(innerResult))
        //    ).
        // 8. Else, set received to Completion(GeneratorYield(innerResult)).

        // iii. Else,
        // We should be +0 try-catch block here.
        ctx.set_jump_target_here(jump_over_throw_call);
        // 1. NOTE: If iterator does not have a throw method, this throw is
        //    going to terminate the yield* loop. But first we need to give
        //    iterator a chance to clean up.
        // 2. Let closeCompletion be NormalCompletion(empty).
        // 3. If generatorKind is async,
        if generator_kind_is_async {
            // perform ? AsyncIteratorClose(iteratorRecord, closeCompletion).
            ctx.add_instruction(Instruction::AsyncIteratorClose);
            // If async iterator close returned a Value, then it'll push the previous
            // result value into the stack and perform an implicit Await.
            // We should verify that the result of the await is an object, and then
            // return the original result.
            let error_message = ctx.create_string("iterator.return() returned a non-object value");
            ctx.add_instruction_with_identifier(
                Instruction::VerifyIsObject,
                error_message.to_property_key(),
            );
            ctx.add_instruction(Instruction::Store);
        } else {
            // 4. Else, perform ? IteratorClose(iteratorRecord, closeCompletion).
            ctx.add_instruction(Instruction::IteratorClose);
        }
        // Pop the overall catch block and pop the iterator.
        let jump_to_iterator_pop = iterator.exit(ctx);
        // 5. NOTE: The next step throws a TypeError to indicate that there was
        //    a yield* protocol violation: iterator does not have a throw
        //    method.
        // 6. Throw a TypeError exception.
        let error_message = ctx.create_string("iterator does not have a throw method");
        ctx.add_instruction_with_constant(Instruction::StoreConstant, error_message);
        ctx.add_instruction_with_immediate(
            Instruction::ThrowError,
            ExceptionType::TypeError as usize,
        );
        jump_to_iterator_pop
    };

    {
        // Overall catch block to pop the iterator and rethrow.
        ctx.set_jump_target_here(jump_to_iterator_pop);
        ctx.add_instruction(Instruction::IteratorPop);
        ctx.add_instruction(Instruction::Throw);
    }

    // We should be +0 try-catch block here.
    ctx.set_jump_target_here(jump_to_end);
    ctx.add_instruction(Instruction::PopExceptionJumpTarget);
    ctx.add_instruction(Instruction::IteratorPop);
    Ok(())
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::YieldExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;

    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if self.delegate {
            compile_delegate_yield_expression(self, ctx)?;
            return Ok(ValueOutput::Value);
        }
        let _value = if let Some(arg) = &self.argument {
            // YieldExpression : yield AssignmentExpression
            // 1. Let exprRef be ? Evaluation of AssignmentExpression.
            // 2. Let value be ? GetValue(exprRef).
            arg.compile(ctx)?.get_value(ctx)?
        } else {
            // YieldExpression : yield
            // 1. Return ? Yield(undefined).
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            Primitive::Undefined.into()
        };
        // 3. Return ? Yield(value).
        // ### 27.5.3.7 Yield ( value )
        // 1. Let generatorKind be GetGeneratorKind().
        let generator_kind_is_async = ctx.is_async_generator();
        // 2. If generatorKind is async, return ? AsyncGeneratorYield(? Await(value)).
        if generator_kind_is_async {
            ctx.add_instruction(Instruction::Await);
        } else {
            // 3. Otherwise, return ? GeneratorYield(CreateIteratorResultObject(value, false)).
            compile_create_iterator_result_object(ctx, false);
        }
        ctx.add_instruction(Instruction::Yield);
        // Note: generators can be resumed with a Return instruction. For those
        // cases we need to generate Return handling here.
        let jump_over_return = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        ctx.compile_return(true);
        ctx.set_jump_target_here(jump_over_return);
        Ok(ValueOutput::Value)
    }
}

fn compile_create_iterator_result_object(ctx: &mut CompileContext, done: bool) {
    let (agent, gc) = ctx.get_agent_and_gc();
    let prototype = agent
        .current_realm_record()
        .intrinsics()
        .object_prototype()
        .bind(gc);
    let shape = ObjectShape::get_shape_for_prototype(agent, Some(prototype.into_object()))
        .get_child_shape(agent, BUILTIN_STRING_MEMORY.value.to_property_key())
        .expect("Should perform GC here")
        .get_child_shape(agent, BUILTIN_STRING_MEMORY.done.to_property_key())
        .expect("Should perform GC here");
    ctx.add_instruction(Instruction::Load);
    ctx.add_instruction_with_constant(Instruction::LoadConstant, done);
    ctx.add_instruction_with_shape(Instruction::ObjectCreateWithShape, shape);
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::Expression<'s> {
    type Output = Result<PlaceOrValue<'s, 'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        match self {
            ast::Expression::ArrayExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::ArrowFunctionExpression(x) => {
                x.compile(ctx);
                Ok(ValueOutput::Value.into())
            }
            ast::Expression::AssignmentExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::AwaitExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::BigIntLiteral(x) => Ok(x.compile(ctx).into()),
            ast::Expression::BinaryExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::BooleanLiteral(x) => Ok(x.compile(ctx).into()),
            ast::Expression::CallExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::ChainExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::ClassExpression(x) => {
                x.compile(ctx)?;
                Ok(ValueOutput::Value.into())
            }
            ast::Expression::ComputedMemberExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::ConditionalExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::FunctionExpression(x) => {
                x.compile(ctx);
                Ok(ValueOutput::Value.into())
            }
            ast::Expression::Identifier(x) => Ok(x.compile(ctx).into()),
            ast::Expression::ImportExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::LogicalExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::MetaProperty(x) => {
                x.compile(ctx);
                Ok(ValueOutput::Value.into())
            }
            ast::Expression::NewExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::NullLiteral(x) => Ok(x.compile(ctx).into()),
            ast::Expression::NumericLiteral(x) => Ok(x.compile(ctx).into()),
            ast::Expression::ObjectExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::ParenthesizedExpression(x) => x.compile(ctx),
            ast::Expression::PrivateFieldExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::PrivateInExpression(x) => x.compile(ctx).map(Into::into),
            #[cfg(feature = "regexp")]
            ast::Expression::RegExpLiteral(x) => {
                x.compile(ctx);
                Ok(ValueOutput::Value.into())
            }
            #[cfg(not(feature = "regexp"))]
            ast::Expression::RegExpLiteral(_) => unreachable!(),
            ast::Expression::SequenceExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::StaticMemberExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::StringLiteral(x) => Ok(x.compile(ctx).into()),
            ast::Expression::Super(x) => {
                x.compile(ctx);
                Ok(ValueOutput::Value.into())
            }
            ast::Expression::TaggedTemplateExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::TemplateLiteral(x) => x.compile(ctx).map(Into::into),
            ast::Expression::ThisExpression(x) => {
                x.compile(ctx);
                Ok(ValueOutput::Value.into())
            }
            ast::Expression::UnaryExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::UpdateExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::YieldExpression(x) => x.compile(ctx).map(Into::into),
            ast::Expression::V8IntrinsicExpression(_) => todo!(),
            #[cfg(feature = "typescript")]
            ast::Expression::TSAsExpression(x) => x.expression.compile(ctx),
            #[cfg(feature = "typescript")]
            ast::Expression::TSSatisfiesExpression(x) => x.expression.compile(ctx),
            #[cfg(feature = "typescript")]
            ast::Expression::TSNonNullExpression(x) => x.expression.compile(ctx),
            #[cfg(feature = "typescript")]
            ast::Expression::TSTypeAssertion(x) => x.expression.compile(ctx),
            #[cfg(feature = "typescript")]
            ast::Expression::TSInstantiationExpression(x) => x.expression.compile(ctx),
            ast::Expression::JSXElement(_) | ast::Expression::JSXFragment(_) => unreachable!(),
            #[cfg(not(feature = "typescript"))]
            ast::Expression::TSTypeAssertion(_)
            | ast::Expression::TSInstantiationExpression(_)
            | ast::Expression::TSAsExpression(_)
            | ast::Expression::TSNonNullExpression(_)
            | ast::Expression::TSSatisfiesExpression(_) => {
                unreachable!()
            }
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::UpdateExpression<'s> {
    type Output = Result<ValueOutput<'gc>, ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let lref = match &self.argument {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(x) => x.compile(ctx),
            ast::SimpleAssignmentTarget::ComputedMemberExpression(x) => x.compile(ctx)?,
            ast::SimpleAssignmentTarget::PrivateFieldExpression(x) => x.compile(ctx)?,
            ast::SimpleAssignmentTarget::StaticMemberExpression(x) => x.compile(ctx)?,
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSAsExpression(x) => match x.expression.compile(ctx)? {
                PlaceOrValue::Place(pk) => pk,
                _ => unreachable!(),
            },
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSNonNullExpression(x) => {
                match x.expression.compile(ctx)? {
                    PlaceOrValue::Place(pk) => pk,
                    _ => unreachable!(),
                }
            }
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSSatisfiesExpression(x) => {
                match x.expression.compile(ctx)? {
                    PlaceOrValue::Place(pk) => pk,
                    _ => unreachable!(),
                }
            }
            #[cfg(feature = "typescript")]
            ast::SimpleAssignmentTarget::TSTypeAssertion(x) => match x.expression.compile(ctx)? {
                PlaceOrValue::Place(pk) => pk,
                _ => unreachable!(),
            },
            #[cfg(not(feature = "typescript"))]
            ast::SimpleAssignmentTarget::TSNonNullExpression(_)
            | ast::SimpleAssignmentTarget::TSSatisfiesExpression(_)
            | ast::SimpleAssignmentTarget::TSAsExpression(_)
            | ast::SimpleAssignmentTarget::TSTypeAssertion(_) => unreachable!(),
        };
        lref.get_value_keep_reference(ctx)?;
        ctx.add_instruction(Instruction::ToNumeric);
        let value_on_stack = if !self.prefix {
            // The return value of postfix increment/decrement is the value
            // after ToNumeric.
            Some(ctx.load_copy_to_stack())
        } else {
            None
        };
        match self.operator {
            oxc_syntax::operator::UpdateOperator::Increment => {
                ctx.add_instruction(Instruction::Increment);
            }
            oxc_syntax::operator::UpdateOperator::Decrement => {
                ctx.add_instruction(Instruction::Decrement);
            }
        }
        let value_on_stack = value_on_stack.unwrap_or_else(|| ctx.load_copy_to_stack());
        let result = lref.put_value(ctx, ValueOutput::Value);
        value_on_stack.store(ctx);
        result.map(|_| ValueOutput::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ExpressionStatement<'s> {
    type Output = StatementResult<'gc>;
    /// # ['a 14.5.1 Runtime Semantics: Evaluation](https://tc39.es/ecma262/#sec-expression-statement-runtime-semantics-evaluation)
    /// `ExpressionStatement : Expression ;`
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let exprRef be ? Evaluation of Expression.
        // 2. Return ? GetValue(exprRef).
        value_result_to_statement_result(
            self.expression.compile(ctx).and_then(|r| r.get_value(ctx)),
        )
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ReturnStatement<'s> {
    type Output = StatementBreak;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if let Some(expr) = &self.argument {
            if let Err(err) = expr.compile(ctx).and_then(|r| r.get_value(ctx)) {
                return err.into();
            }
        } else {
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
        }
        ctx.compile_return(self.argument.is_some());
        StatementBreak::Return
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::IfStatement<'s> {
    type Output = StatementResult<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // 1. Let exprRef be ? Evaluation of Expression.
        // 2. Let exprValue be ToBoolean(? GetValue(exprRef)).
        value_result_to_statement_result(self.test.compile(ctx).and_then(|r| r.get_value(ctx)))?;
        // 3. If exprValue is true, then
        let jump_to_else = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
        let st = ctx.enter_if_statement();
        // a. Let stmtCompletion be Completion(Evaluation of the first Statement).
        let consequent_stmt_completion = self.consequent.compile(ctx);
        st.exit(ctx, false);
        // 4. Else,
        let jump_over_else = if consequent_stmt_completion.is_continue() {
            Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
        } else {
            None
        };
        if let Some(alternate) = &self.alternate {
            ctx.set_jump_target_here(jump_to_else);
            // a. Let stmtCompletion be Completion(Evaluation of the second Statement).
            let st = ctx.enter_if_statement();
            let _stmt_completion = alternate.compile(ctx);
            st.exit(ctx, false);
        } else {
            ctx.set_jump_target_here(jump_to_else);
            // 3. If exprValue is false, then
            // a. Return undefined.
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            // 5. Return ? UpdateEmpty(stmtCompletion, undefined).
        }
        if let Some(jump_over_else) = jump_over_else {
            ctx.set_jump_target_here(jump_over_else);
        }
        ControlFlow::Continue(StatementContinue::Value)
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ArrayPattern<'s> {
    type Output = Result<(), ExpressionError>;
    /// ### [8.6.2 Runtime Semantics: BindingInitialization](https://tc39.es/ecma262/#sec-runtime-semantics-bindinginitialization)
    /// ### BindingPattern : ArrayBindingPattern
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) -> Self::Output {
        if self.elements.is_empty() && self.rest.is_none() {
            // ArrayAssignmentPattern : [ ]
            // 1. Let iteratorRecord be ? GetIterator(value, sync).
            // 2. Return ? IteratorClose(iteratorRecord, NormalCompletion(unused)).
            let iterator = ctx.push_sync_iterator();
            ctx.add_instruction(Instruction::IteratorClose);
            let jump_to_catch = iterator.exit(ctx);
            let jump_over_catch = ctx.add_instruction_with_jump_slot(Instruction::Jump);
            {
                // Catch block
                ctx.set_jump_target_here(jump_to_catch);
                ctx.add_instruction(Instruction::IteratorPop);
                ctx.add_instruction(Instruction::Throw);
            }
            ctx.set_jump_target_here(jump_over_catch);
            return Ok(());
        }

        // 1. Let iteratorRecord be ? GetIterator(value, sync).
        let iterator = ctx.push_sync_iterator();
        // 2. Let result be Completion(IteratorBindingInitialization of
        //    ArrayBindingPattern with arguments iteratorRecord and
        //    environment).
        let result = if !self.contains_expression() {
            simple_array_pattern(
                ctx,
                self.elements.iter().map(Option::as_ref),
                self.rest.as_deref(),
                self.elements.len(),
                ctx.lexical_binding_state,
            );
            Ok(())
        } else {
            complex_array_pattern(
                ctx,
                self.elements.iter().map(Option::as_ref),
                self.rest.as_deref(),
                ctx.lexical_binding_state,
            )
        };
        // 3. If iteratorRecord.[[Done]] is false, return
        //    ? IteratorClose(iteratorRecord, result).
        // Note: simple array binding handles IteratorClose at runtime, while
        // complex array binding injects it on its own. We don't need to do
        // anything special here.
        let jump_to_catch = iterator.exit(ctx);
        let jump_over_catch_and_exit = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        {
            // catch handling, we have to call IteratorClose with the error,
            // then pop the iterator and rethrow our error.
            ctx.set_jump_target_here(jump_to_catch);
            ctx.add_instruction(Instruction::IteratorCloseWithError);
            ctx.add_instruction(Instruction::IteratorPop);
            ctx.add_instruction(Instruction::Throw);
        }
        ctx.set_jump_target_here(jump_over_catch_and_exit);
        // 4. Return ? result.
        result
    }
}

fn simple_array_pattern<'s, I>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    elements: I,
    rest: Option<&'s ast::BindingRestElement<'s>>,
    num_elements: usize,
    has_environment: bool,
) where
    I: Iterator<Item = Option<&'s ast::BindingPattern<'s>>>,
{
    let lexical_binding_state = ctx.lexical_binding_state;
    ctx.lexical_binding_state = has_environment;
    ctx.add_instruction_with_immediate_and_immediate(
        Instruction::BeginSimpleArrayBindingPattern,
        num_elements,
        has_environment.into(),
    );

    for ele in elements {
        let Some(ele) = ele else {
            ctx.add_instruction(Instruction::BindingPatternSkip);
            continue;
        };
        match &ele.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                let identifier_string = ctx.create_string(identifier.name.as_str());
                if let Some(stack_slot) = ctx.get_variable_stack_index(identifier.symbol_id()) {
                    ctx.add_instruction_with_immediate_and_constant(
                        Instruction::BindingPatternBindToIndex,
                        stack_slot as usize,
                        identifier_string,
                    );
                } else {
                    ctx.add_instruction_with_identifier(
                        Instruction::BindingPatternBind,
                        identifier_string.to_property_key(),
                    )
                }
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetValue);
                simple_object_pattern(pattern, ctx, has_environment);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetValue);
                simple_array_pattern(
                    ctx,
                    pattern.elements.iter().map(Option::as_ref),
                    pattern.rest.as_deref(),
                    pattern.elements.len(),
                    has_environment,
                );
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    }

    if let Some(rest) = rest {
        match &rest.argument.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                if let Some(stack_slot) = ctx.get_variable_stack_index(identifier.symbol_id()) {
                    ctx.add_instruction_with_immediate(
                        Instruction::BindingPatternBindRestToIndex,
                        stack_slot as usize,
                    );
                } else {
                    let identifier_string = ctx.create_string(identifier.name.as_str());
                    ctx.add_instruction_with_identifier(
                        Instruction::BindingPatternBindRest,
                        identifier_string.to_property_key(),
                    );
                }
            }
            ast::BindingPatternKind::ObjectPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetRestValue);
                simple_object_pattern(pattern, ctx, has_environment);
            }
            ast::BindingPatternKind::ArrayPattern(pattern) => {
                ctx.add_instruction(Instruction::BindingPatternGetRestValue);
                simple_array_pattern(
                    ctx,
                    pattern.elements.iter().map(Option::as_ref),
                    pattern.rest.as_deref(),
                    pattern.elements.len(),
                    has_environment,
                );
            }
            ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
        }
    } else {
        ctx.add_instruction(Instruction::FinishBindingPattern);
    }
    ctx.lexical_binding_state = lexical_binding_state;
}

fn check_result_is_undefined(ctx: &mut CompileContext) -> JumpIndex {
    // Run the initializer if the result value is undefined.
    ctx.add_instruction(Instruction::LoadCopy);
    ctx.add_instruction(Instruction::IsUndefined);
    let jump_slot = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
    // Drop the undefined result value and run initializer.
    ctx.add_instruction(Instruction::Store);
    jump_slot
}

/// ### [8.6.3 Runtime Semantics: IteratorBindingInitialization](https://tc39.es/ecma262/#sec-runtime-semantics-iteratorbindinginitialization)
fn complex_array_pattern<'s, I>(
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    elements: I,
    rest: Option<&'s ast::BindingRestElement<'s>>,
    has_environment: bool,
) -> Result<(), ExpressionError>
where
    I: Iterator<Item = Option<&'s ast::BindingPattern<'s>>>,
{
    let lexical_binding_state = ctx.lexical_binding_state;
    ctx.lexical_binding_state = has_environment;
    let result = 'iter: {
        for ele in elements {
            ctx.add_instruction(Instruction::IteratorStepValueOrUndefined);

            let Some(ele) = ele else {
                continue;
            };

            if let Err(err) = ele.compile(ctx) {
                break 'iter Err(err);
            }
        }

        if let Some(rest) = rest {
            ctx.add_instruction(Instruction::IteratorRestIntoArray);
            if let Err(err) = rest.argument.compile(ctx) {
                break 'iter Err(err);
            }
        } else {
            ctx.add_instruction(Instruction::IteratorClose);
        }
        Ok(())
    };
    ctx.lexical_binding_state = lexical_binding_state;
    result
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ObjectPattern<'s> {
    type Output = Result<(), ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if !self.contains_expression() {
            simple_object_pattern(self, ctx, ctx.lexical_binding_state);
        } else {
            complex_object_pattern(self, ctx, ctx.lexical_binding_state)?;
        }
        Ok(())
    }
}

fn simple_object_pattern<'s>(
    pattern: &'s ast::ObjectPattern<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    has_environment: bool,
) {
    let lexical_binding_state = ctx.lexical_binding_state;
    ctx.lexical_binding_state = has_environment;
    ctx.add_instruction_with_immediate(
        Instruction::BeginSimpleObjectBindingPattern,
        has_environment.into(),
    );

    for ele in &pattern.properties {
        if ele.shorthand {
            debug_assert!(
                matches!(&ele.key, ast::PropertyKey::StaticIdentifier(_))
                    && matches!(
                        &ele.value.kind,
                        ast::BindingPatternKind::BindingIdentifier(_)
                    )
            );
            let ast::BindingPatternKind::BindingIdentifier(identifier) = &ele.value.kind else {
                unreachable!()
            };
            let identifier = identifier.as_ref();
            let identifier_string = ctx.create_string(identifier.name.as_str());
            if let Some(stack_slot) = ctx.get_variable_stack_index(identifier.symbol_id()) {
                ctx.add_instruction_with_immediate_and_constant(
                    Instruction::BindingPatternBindToIndex,
                    stack_slot as usize,
                    identifier_string,
                );
            } else {
                ctx.add_instruction_with_identifier(
                    Instruction::BindingPatternBind,
                    identifier_string.to_property_key(),
                );
            }
        } else {
            let key_string = match &ele.key {
                ast::PropertyKey::StaticIdentifier(identifier) => {
                    // SAFETY: We'll use this value as a PropertyKey directly later.
                    unsafe {
                        ctx.create_property_key(&identifier.name)
                            .into_value_unchecked()
                    }
                }
                ast::PropertyKey::NumericLiteral(literal) => {
                    if let Ok(Number::Integer(integer)) = Number::try_from(literal.value) {
                        // Literal is an integer, just drop it in as a
                        // PropertyKey integer directly.
                        Value::Integer(integer)
                    } else {
                        // Literal is a float: it needs to be converted into a
                        // String.
                        let mut buffer = ryu_js::Buffer::new();
                        ctx.create_string(buffer.format(literal.value)).into_value()
                    }
                }
                ast::PropertyKey::StringLiteral(literal) => {
                    // SAFETY: We'll use this value as a PropertyKey directly later.
                    unsafe {
                        ctx.create_property_key(&literal.value)
                            .into_value_unchecked()
                    }
                }
                ast::PropertyKey::BigIntLiteral(lit) => {
                    // Drop out the trailing 'n' from BigInt literals.
                    let raw_str = lit
                        .raw
                        .as_ref()
                        .expect("BigInt literal should have raw text")
                        .as_str();
                    let last_index = raw_str.len() - 1;
                    let (literal, radix) = match lit.base {
                        oxc_syntax::number::BigintBase::Decimal => (&raw_str[..last_index], 10),
                        oxc_syntax::number::BigintBase::Binary => (&raw_str[2..last_index], 2),
                        oxc_syntax::number::BigintBase::Octal => (&raw_str[2..last_index], 8),
                        oxc_syntax::number::BigintBase::Hex => (&raw_str[2..last_index], 16),
                    };
                    if let Ok(result) = i64::from_str_radix(literal, radix) {
                        if let Ok(number) = Number::try_from(result) {
                            number.into_value()
                        } else {
                            ctx.create_string_from_owned(result.to_string())
                                .into_value()
                        }
                    } else {
                        let string = num_bigint::BigInt::from_str_radix(literal, radix)
                            .unwrap()
                            .to_string();
                        ctx.create_string_from_owned(string).into_value()
                    }
                }
                _ => unreachable!(),
            };

            match &ele.value.kind {
                ast::BindingPatternKind::BindingIdentifier(identifier) => {
                    let value_identifier_string = ctx.create_string(identifier.name.as_str());
                    if let Some(stack_slot) = ctx.get_variable_stack_index(identifier.symbol_id()) {
                        ctx.add_instruction_with_immediate_and_constant(
                            Instruction::BindingPatternBindToIndex,
                            stack_slot as usize,
                            key_string,
                        );
                    } else {
                        ctx.add_instruction_with_identifier_and_constant(
                            Instruction::BindingPatternBindNamed,
                            value_identifier_string,
                            key_string,
                        )
                    }
                }
                ast::BindingPatternKind::ObjectPattern(pattern) => {
                    ctx.add_instruction_with_constant(
                        Instruction::BindingPatternGetValueNamed,
                        key_string,
                    );
                    simple_object_pattern(pattern, ctx, has_environment);
                }
                ast::BindingPatternKind::ArrayPattern(pattern) => {
                    ctx.add_instruction_with_constant(
                        Instruction::BindingPatternGetValueNamed,
                        key_string,
                    );
                    simple_array_pattern(
                        ctx,
                        pattern.elements.iter().map(Option::as_ref),
                        pattern.rest.as_deref(),
                        pattern.elements.len(),
                        has_environment,
                    );
                }
                ast::BindingPatternKind::AssignmentPattern(_) => unreachable!(),
            }
        }
    }

    if let Some(rest) = &pattern.rest {
        match &rest.argument.kind {
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                if let Some(stack_slot) = ctx.get_variable_stack_index(identifier.symbol_id()) {
                    ctx.add_instruction_with_immediate(
                        Instruction::BindingPatternBindRestToIndex,
                        stack_slot as usize,
                    );
                } else {
                    let identifier_string = ctx.create_string(identifier.name.as_str());
                    ctx.add_instruction_with_identifier(
                        Instruction::BindingPatternBindRest,
                        identifier_string.to_property_key(),
                    );
                }
            }
            _ => unreachable!(),
        }
    } else {
        ctx.add_instruction(Instruction::FinishBindingPattern);
    }
    ctx.lexical_binding_state = lexical_binding_state;
}

fn complex_object_pattern<'s>(
    object_pattern: &'s ast::ObjectPattern<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
    has_environment: bool,
) -> Result<(), ExpressionError> {
    let lexical_binding_state = ctx.lexical_binding_state;
    ctx.lexical_binding_state = has_environment;
    // 8.6.2 Runtime Semantics: BindingInitialization
    // BindingPattern : ObjectBindingPattern
    // 1. Perform ? RequireObjectCoercible(value).
    // NOTE: RequireObjectCoercible throws in the same cases as ToObject, and
    // other operations later on (such as GetV) also perform ToObject, so we
    // convert to an object early.
    ctx.add_instruction(Instruction::ToObject);
    ctx.add_instruction(Instruction::Load);

    let result = 'iter: {
        for property in &object_pattern.properties {
            let place = match &property.key {
                ast::PropertyKey::StaticIdentifier(identifier) => {
                    // Make a copy of the baseValue in the result register;
                    // EvaluatePropertyAccessWithIdentifierKey uses it.
                    ctx.add_instruction(Instruction::StoreCopy);
                    identifier.compile(ctx)
                }
                // Note: private field aren't valid in this context.
                ast::PropertyKey::PrivateIdentifier(_) => unreachable!(),
                _ => {
                    // Make a copy of the baseValue on the stack;
                    // EvaluatePropertyAccessWithExpressionKey pops the stack.
                    ctx.add_instruction(Instruction::StoreCopy);
                    ctx.add_instruction(Instruction::Load);
                    let expr = property.key.to_expression();
                    let output = expr.compile(ctx)?.get_value(ctx)?;
                    ctx.add_instruction(Instruction::EvaluatePropertyAccessWithExpressionKey);
                    output.to_expression_key()
                }
            };
            place.get_value_maybe_keep_reference(ctx, object_pattern.rest.is_some())?;
            if object_pattern.rest.is_some() {
                assert!(place.has_reference());
                ctx.add_instruction(Instruction::PushReference);
            }

            if let Err(err) = property.value.compile(ctx) {
                break 'iter Err(err);
            };
        }

        // Don't keep the object on the stack.
        ctx.add_instruction(Instruction::Store);

        if let Some(rest) = &object_pattern.rest {
            let ast::BindingPatternKind::BindingIdentifier(identifier) = &rest.argument.kind else {
                unreachable!()
            };

            // We have kept the references for all of the properties read in the
            // reference stack, so we can now use them to exclude those
            // properties from the rest object.
            ctx.add_instruction_with_immediate(
                Instruction::CopyDataPropertiesIntoObject,
                object_pattern.properties.len(),
            );
            let value = ValueOutput::Value;

            let place = identifier.compile(ctx);
            if !has_environment {
                if let Err(err) = place.put_value(ctx, value) {
                    break 'iter Err(err);
                }
            } else {
                place.initialise_referenced_binding(ctx, value);
            }
        }
        Ok(())
    };
    ctx.lexical_binding_state = lexical_binding_state;
    result
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BindingPattern<'s> {
    type Output = Result<(), ExpressionError>;
    /// ### [8.6.2 Runtime Semantics: BindingInitialization](https://tc39.es/ecma262/#sec-runtime-semantics-bindinginitialization)
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        match &self.kind {
            // ### BindingIdentifier : Identifier
            // ### BindingIdentifier : yield
            // ### BindingIdentifier : await
            ast::BindingPatternKind::BindingIdentifier(identifier) => {
                // 1. Let name be the StringValue of Identifier.
                // 2. Return ? InitializeBoundName(name, value, environment).
                let place = identifier.compile(ctx);
                let value = ValueOutput::Value;

                // ### 8.6.2.1 InitializeBoundName ( name, value, environment )
                // 1. If environment is not undefined, then
                if ctx.lexical_binding_state {
                    // a. Perform ! environment.InitializeBinding(name, value).
                    // b. Return unused.
                    place.initialise_referenced_binding(ctx, value);
                    Ok(())
                } else {
                    // 2. Else,
                    // a. Let lhs be ? ResolveBinding(name).
                    // b. Return ? PutValue(lhs, value).
                    place.put_value(ctx, value)
                }
            }
            // ### BindingPattern : ObjectBindingPattern
            ast::BindingPatternKind::ObjectPattern(object_binding_pattern) => {
                object_binding_pattern.compile(ctx)
            }
            // ### BindingPattern : ArrayBindingPattern
            ast::BindingPatternKind::ArrayPattern(array_binding_pattern) => {
                array_binding_pattern.compile(ctx)
            }
            // ### SingleNameBinding : BindingIdentifier Initializer
            // ### BindingElement : BindingPattern Initializer
            ast::BindingPatternKind::AssignmentPattern(pattern) => {
                match &pattern.left.kind {
                    // ### SingleNameBinding : BindingIdentifier Initializer
                    //
                    // * function (a = 1) {}
                    // * [a = 1]
                    ast::BindingPatternKind::BindingIdentifier(binding_identifier) => {
                        // 1. Let bindingId be the StringValue of BindingIdentifier.
                        // 2. Let lhs be ? ResolveBinding(bindingId, environment).
                        let lhs = binding_identifier.compile(ctx);
                        // Note: v is already in the result register after
                        // IteratorStepValueOrUndefined above.
                        // 3. Let v be undefined.
                        // 4. If iteratorRecord.[[Done]] is false, then
                        //         a. Let next be ? IteratorStepValue(iteratorRecord).
                        //         b. If next is not done, then
                        //                 i. Set v to next.
                        // 5. If Initializer is present and v is undefined, then
                        let jump_over_initializer = check_result_is_undefined(ctx);
                        if is_anonymous_function_definition(&pattern.right) {
                            // a. If IsAnonymousFunctionDefinition(Initializer) is
                            //    true, then
                            // i. Set v to ? NamedEvaluation of Initializer with
                            //    argument bindingId.
                            ctx.add_instruction_with_constant(
                                Instruction::StoreConstant,
                                lhs.identifier().unwrap(),
                            );
                            ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                        }
                        let do_push_reference = lhs.has_reference() && !pattern.right.is_literal();
                        if do_push_reference {
                            ctx.add_instruction(Instruction::PushReference);
                        }
                        // b. Else,
                        // i. Let defaultValue be ? Evaluation of Initializer.
                        let default_value = pattern.right.compile(ctx);
                        // ii. Set v to ? GetValue(defaultValue).
                        // Note: no early exit as this is not an unconditional
                        // branch.
                        let v = default_value
                            .and_then(|dv| dv.get_value(ctx))
                            .unwrap_or(ValueOutput::Value);
                        if do_push_reference {
                            ctx.add_instruction(Instruction::PopReference);
                        }
                        ctx.name_identifier = None;
                        ctx.add_instruction(Instruction::Load);
                        ctx.set_jump_target_here(jump_over_initializer);
                        ctx.add_instruction(Instruction::Store);
                        // 6. If environment is undefined,
                        if !ctx.lexical_binding_state {
                            // return ? PutValue(lhs, v).
                            lhs.put_value(ctx, v)
                        } else {
                            // 7. Return ? InitializeReferencedBinding(lhs, v).
                            lhs.initialise_referenced_binding(ctx, v);
                            Ok(())
                        }
                    }
                    // ### BindingElement : BindingPattern Initializer
                    //
                    // * function ({} = 1)
                    // * [{} = 1]
                    // * function ([] = 1)
                    // * [[] = 1]
                    _ => {
                        // Note: v is already in the result register after
                        // IteratorStepValueOrUndefined above.
                        // 1. Let v be undefined.
                        // 2. If iteratorRecord.[[Done]] is false, then
                        //         a. Let next be ? IteratorStepValue(iteratorRecord).
                        //         b. If next is not done, then
                        //                 i. Set v to next.
                        // 3. If Initializer is present and v is undefined, then
                        let jump_over_initializer = check_result_is_undefined(ctx);
                        // a. Let defaultValue be ? Evaluation of Initializer.
                        let default_value = pattern.right.compile(ctx);
                        // b. Set v to ? GetValue(defaultValue).
                        // Note: no early exit as this is not an unconditional
                        // branch.
                        let _v = default_value
                            .and_then(|dv| dv.get_value(ctx))
                            .unwrap_or(ValueOutput::Value);
                        ctx.add_instruction(Instruction::Load);
                        ctx.set_jump_target_here(jump_over_initializer);
                        ctx.add_instruction(Instruction::Store);
                        // 4. Return ? BindingInitialization of BindingPattern with
                        //    arguments v and environment.
                        pattern.left.compile(ctx)
                    }
                }
            }
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::VariableDeclaration<'s> {
    type Output = Result<(), ExpressionError>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // If this is a declare statement, it's a TypeScript ambient declaration
        // and should not generate any runtime code, similar to type declarations
        #[cfg(feature = "typescript")]
        if self.declare {
            return Ok(());
        }

        match self.kind {
            // VariableStatement : var VariableDeclarationList ;
            ast::VariableDeclarationKind::Var => {
                for decl in &self.declarations {
                    // VariableDeclaration : BindingIdentifier
                    let Some(init) = &decl.init else {
                        // 1. Return EMPTY.
                        continue;
                    };
                    // VariableDeclaration : BindingIdentifier Initializer

                    let ast::BindingPatternKind::BindingIdentifier(identifier) = &decl.id.kind
                    else {
                        //  VariableDeclaration : BindingPattern Initializer
                        // 1. Let rhs be ? Evaluation of Initializer.
                        // 2. Let rval be ? GetValue(rhs).
                        init.compile(ctx)?.get_value(ctx)?;
                        // 3. Return ? BindingInitialization of BidingPattern with arguments rval and undefined.
                        let lexical_binding_state = ctx.lexical_binding_state;
                        ctx.lexical_binding_state = false;
                        let result = decl.id.compile(ctx);
                        ctx.lexical_binding_state = lexical_binding_state;
                        result?;
                        continue;
                    };

                    // 1. Let bindingId be StringValue of BindingIdentifier.
                    // 2. Let lhs be ? ResolveBinding(bindingId).
                    let lhs = identifier.compile(ctx);

                    let push_reference = lhs.has_reference() && !init.is_literal();
                    if push_reference {
                        ctx.add_instruction(Instruction::PushReference);
                    }

                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    if let Some(binding_id) = lhs.identifier()
                        && is_anonymous_function_definition(init)
                    {
                        ctx.add_instruction_with_constant(Instruction::StoreConstant, binding_id);
                        // a. Let value be ? NamedEvaluation of Initializer with argument StackId.
                        ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                        // 4. Else,
                    }
                    // a. Let rhs be ? Evaluation of Initializer.
                    let rhs = init.compile(ctx)?;
                    // b. Let value be ? GetValue(rhs).
                    let value = rhs.get_value(ctx)?;
                    // 5. Perform ? PutValue(lhs, value).
                    if push_reference {
                        ctx.add_instruction(Instruction::PopReference);
                    }
                    lhs.put_value(ctx, value)?;

                    // 6. Return EMPTY.
                }
            }
            ast::VariableDeclarationKind::Let | ast::VariableDeclarationKind::Const => {
                for decl in &self.declarations {
                    let ast::BindingPatternKind::BindingIdentifier(identifier) = &decl.id.kind
                    else {
                        let init = decl.init.as_ref().unwrap();

                        //  LexicalBinding : BindingPattern Initializer
                        // 1. Let rhs be ? Evaluation of Initializer.
                        let rhs = init.compile(ctx)?;
                        // 2. Let value be ? GetValue(rhs).
                        let _value = rhs.get_value(ctx)?;
                        // 3. Let env be the running execution context's LexicalEnvironment.
                        // 4. Return ? BindingInitialization of BindingPattern with arguments value and env.
                        let lexical_binding_state = ctx.lexical_binding_state;
                        ctx.lexical_binding_state = true;
                        let result = decl.id.compile(ctx);
                        ctx.lexical_binding_state = lexical_binding_state;
                        result?;
                        continue;
                    };

                    // 1. Let lhs be ! ResolveBinding(StringValue of BindingIdentifier).
                    let lhs = identifier.compile(ctx);

                    let Some(init) = &decl.init else {
                        // LexicalBinding : BindingIdentifier
                        // 2. Perform ! InitializeReferencedBinding(lhs, undefined).
                        lhs.initialise_referenced_binding_to_undefined(ctx);
                        // 3. Return empty.
                        continue;
                    };

                    let do_push_reference = lhs.has_reference() && !init.is_literal();
                    //  LexicalBinding : BindingIdentifier Initializer
                    if do_push_reference {
                        ctx.add_instruction(Instruction::PushReference);
                    }
                    // 3. If IsAnonymousFunctionDefinition(Initializer) is true, then
                    if is_anonymous_function_definition(init) {
                        // a. Let value be ? NamedEvaluation of Initializer with argument bindingId.
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            lhs.identifier().unwrap(),
                        );
                        ctx.name_identifier = Some(NamedEvaluationParameter::Result);
                        // 4. Else,
                        // a. Let rhs be ? Evaluation of Initializer.
                    }
                    // b. Let value be ? GetValue(rhs).
                    let value = init.compile(ctx)?.get_value(ctx)?;
                    if do_push_reference {
                        ctx.add_instruction(Instruction::PopReference);
                    }

                    // 5. Perform ! InitializeReferencedBinding(lhs, value).
                    lhs.initialise_referenced_binding(ctx, value);
                    // 6. Return empty.
                }
            }
            ast::VariableDeclarationKind::Using => todo!(),
            ast::VariableDeclarationKind::AwaitUsing => todo!(),
        }
        Ok(())
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BlockStatement<'s> {
    type Output = StatementResult<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if self.body.is_empty() {
            // Block : {}
            // 1. Return EMPTY.
            return ControlFlow::Continue(StatementContinue::Empty);
        }
        block_declaration_instantiation::instantiation(ctx, self, |ctx| {
            let mut result = StatementContinue::Empty;
            for ele in &self.body {
                result = ele.compile(ctx)?;
            }
            ControlFlow::Continue(result)
        })
    }
}

impl<'a, 's, 'gc, 'scope> CompileLabelledEvaluation<'a, 's, 'gc, 'scope> for ast::ForStatement<'s> {
    type Output = StatementResult<'gc>;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'a, 's, 'gc, 'scope>,
    ) -> Self::Output {
        let mut per_iteration_lets: Vec<String<'_>> = vec![];
        let mut loop_env = None;

        let result = if let Some(init) = &self.init {
            match init {
                ast::ForStatementInit::VariableDeclaration(init) => {
                    if init.kind.is_lexical() {
                        // 1. Let oldEnv be the running execution context's LexicalEnvironment.
                        // 2. Let loopEnv be NewDeclarativeEnvironment(oldEnv).
                        loop_env = Some(ctx.enter_lexical_scope());
                        // 3. Let isConst be IsConstantDeclaration of LexicalDeclaration.
                        let is_const = init.kind.is_const();
                        // 4. Let boundNames be the BoundNames of LexicalDeclaration.
                        // 5. For each element dn of boundNames, do
                        // a. If isConst is true, then
                        if is_const {
                            init.bound_names(&mut |dn| {
                                // i. Perform ! loopEnv.CreateImmutableBinding(dn, true).
                                let identifier = ctx.create_string(dn.name.as_str());
                                ctx.add_instruction_with_identifier(
                                    Instruction::CreateImmutableBinding,
                                    identifier.to_property_key(),
                                )
                            });
                        } else {
                            // b. Else,
                            // i. Perform ! loopEnv.CreateMutableBinding(dn, false).
                            init.bound_names(&mut |dn| {
                                let identifier = ctx.create_string(dn.name.as_str());
                                // 9. If isConst is false, let perIterationLets
                                // be boundNames; otherwise let perIterationLets
                                // be a new empty List.
                                per_iteration_lets.push(identifier);
                                ctx.add_instruction_with_identifier(
                                    Instruction::CreateMutableBinding,
                                    identifier.to_property_key(),
                                )
                            });
                        }
                        // 6. Set the running execution context's LexicalEnvironment to loopEnv.
                    }
                    init.compile(ctx)
                }
                _ => {
                    let expr = init.as_expression().unwrap();
                    expr.compile(ctx).and_then(|r| r.get_value(ctx)).map(|_| ())
                }
            }
        } else {
            Ok(())
        };

        if let Err(err) = result {
            if let Some(loop_env) = loop_env {
                // Lexical binding loops have an extra declarative environment
                // that we need to exit from once we exit the loop.
                loop_env.exit(ctx);
            }
            return ControlFlow::Break(err.into());
        }
        // 2. Perform ? CreatePerIterationEnvironment(perIterationBindings).
        let create_per_iteration_env = !per_iteration_lets.is_empty();

        // 2. Perform ? CreatePerIterationEnvironment(perIterationBindings).
        if create_per_iteration_env {
            create_per_iteration_environment(ctx, &per_iteration_lets);
        }

        // 1. Let V be undefined.
        let v = ctx.push_stack_loop_result();
        // 3. Repeat,
        let l = ctx.enter_loop(label_set.cloned());
        let jump_over_continue = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        let continue_label = ctx.get_jump_index_to_here();
        // Note: to save one Jump in continue cases, the LoopContinues work is
        // here.
        // d. If result.[[Value]] is not empty, set V to result.[[Value]].
        ctx.add_instruction(Instruction::LoadReplace);
        // e. Perform ? CreatePerIterationEnvironment(perIterationBindings).
        if create_per_iteration_env {
            create_per_iteration_environment(ctx, &per_iteration_lets);
        }
        // f. If increment is not empty, then
        if let Some(update) = &self.update {
            // i. Let incRef be ? Evaluation of increment.
            let inc_ref = update.compile(ctx);
            // ii. Perform ? GetValue(incRef).
            // Note: no early exit, as this path is not guaranteed to run.
            let _ = inc_ref.and_then(|r| r.get_value(ctx));
        }

        ctx.set_jump_target_here(jump_over_continue);

        // a. If test is not empty, then
        let test_result = if let Some(test) = &self.test {
            // i. Let testRef be ? Evaluation of test.
            let test_ref = test.compile(ctx);
            // ii. Let testValue be ? GetValue(testRef).
            let test_value = test_ref.and_then(|r| r.get_value(ctx));
            // iii. If ToBoolean(testValue) is false, return V.
            // jump over consequent if test fails
            test_value.map(|_| Some(ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot)))
        } else {
            Ok(None)
        };

        let result = if let Err(err) = test_result.as_ref() {
            ControlFlow::Break((*err).into())
        } else {
            // b. Let result be Completion(Evaluation of stmt).
            let result = self.body.compile(ctx);
            if result.is_continue() {
                ctx.add_jump_instruction_to_index(Instruction::Jump, continue_label.clone());
            }
            // We cannot know that the loop completes abruptly even if the body
            // says so. Continues and so forth can exit the loop.
            ControlFlow::Continue(StatementContinue::Value)
        };
        // c. If LoopContinues(result, labelSet) is false,
        //    return ? UpdateEmpty(result, V).
        // d. If result.[[Value]] is not empty, set V to result.[[Value]].
        // e. Perform ? CreatePerIterationEnvironment(perIterationBindings).
        // f. If increment is not empty, then

        {
            // ## Catch block
            ctx.set_jump_target_here(l.on_abrupt_exit());
            // Error was thrown: this means loop continues is false:
            // > c. If LoopContinues(result, labelSet) is false,
            // >    return ? UpdateEmpty(result, V).
            ctx.add_instruction(Instruction::UpdateEmpty);
            ctx.add_instruction(Instruction::Throw);
        }

        // iii. If ToBoolean(testValue) is false, return V.
        if let Ok(Some(end_jump)) = test_result {
            ctx.set_jump_target_here(end_jump);
        }
        // Note: exit_loop performs UpdateEmpty; if we jumped here from test
        // failure then result is currently empty and UpdateEmpty will pop V
        // into the result register.
        l.exit(ctx, continue_label);
        v.exit(ctx);

        if let Some(loop_env) = loop_env {
            // Lexical binding loops have an extra declarative environment that
            // we need to exit from once we exit the loop.
            loop_env.exit(ctx);
        }
        // c. If LoopContinues(result, labelSet) is false,
        //    return ? UpdateEmpty(result, V).
        result
    }
}

fn create_per_iteration_environment<'gc>(
    ctx: &mut CompileContext<'_, '_, 'gc, '_>,
    per_iteration_lets: &[String<'gc>],
) {
    if per_iteration_lets.len() == 1 {
        // NOTE: Optimization for the usual case of a single let
        // binding. We do not need to push and pop from the stack
        // in this case but can use the result register directly.
        // There are rather easy further optimizations available as
        // well around creating a sibling environment directly,
        // creating an initialized mutable binding directly, and
        // importantly: The whole loop environment is unnecessary
        // if the loop contains no closures (that capture the
        // per-iteration lets).

        let binding = *per_iteration_lets.first().unwrap();
        // Get value of binding from lastIterationEnv.
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, binding.to_property_key());
        ctx.add_instruction(Instruction::GetValue);
        // Note: here we do not use exit & enter lexical
        // environment helpers as we'd just immediately exit again.
        ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
        ctx.add_instruction_with_identifier(
            Instruction::CreateMutableBinding,
            binding.to_property_key(),
        );
        ctx.add_instruction_with_identifier(Instruction::ResolveBinding, binding.to_property_key());
        ctx.add_instruction(Instruction::InitializeReferencedBinding);
    } else {
        for bn in per_iteration_lets {
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, bn.to_property_key());
            ctx.add_instruction(Instruction::GetValue);
            ctx.add_instruction(Instruction::Load);
        }
        // Note: here we do not use exit & enter lexical
        // environment helpers as we'd just immediately exit again.
        ctx.add_instruction(Instruction::ExitDeclarativeEnvironment);
        ctx.add_instruction(Instruction::EnterDeclarativeEnvironment);
        for bn in per_iteration_lets.iter().rev() {
            ctx.add_instruction_with_identifier(
                Instruction::CreateMutableBinding,
                bn.to_property_key(),
            );
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, bn.to_property_key());
            ctx.add_instruction(Instruction::Store);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }
    }
}

impl<'a, 's, 'gc, 'scope> CompileLabelledEvaluation<'a, 's, 'gc, 'scope>
    for ast::SwitchStatement<'s>
{
    type Output = StatementResult<'gc>;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    ) -> Self::Output {
        // 1. Let exprRef be ? Evaluation of Expression.
        // 2. Let switchValue be ? GetValue(exprRef).
        value_result_to_statement_result(
            self.discriminant
                .compile(ctx)
                .and_then(|r| r.get_value(ctx)),
        )?;
        ctx.add_instruction(Instruction::Load);
        if self.cases.is_empty() {
            // CaseBlock : { }
            // 1. Return undefined.
            ctx.add_instruction_with_constant(Instruction::LoadConstant, Value::Undefined);
            return ControlFlow::Continue(StatementContinue::Literal(Primitive::Undefined));
        }
        let switch = ctx.enter_switch(label_set.cloned());
        // 3. Let oldEnv be the running execution context's LexicalEnvironment.
        // 4. Let blockEnv be NewDeclarativeEnvironment(oldEnv).
        // 5. Perform BlockDeclarationInstantiation(CaseBlock, blockEnv).
        // 6. Set the running execution context's LexicalEnvironment to blockEnv.
        let r = block_declaration_instantiation::instantiation(ctx, self, |ctx| {
            // 7. Let R be Completion(CaseBlockEvaluation of CaseBlock with argument switchValue).
            let mut has_default = false;
            let mut jump_indexes = Vec::with_capacity(self.cases.len());
            for case in &self.cases {
                let Some(test) = &case.test else {
                    // Default case test does not care about the write order: After
                    // all other cases have been tested, default will be entered if
                    // no other was entered previously. The placement of the
                    // default case only matters for fall-through behaviour.
                    has_default = true;
                    continue;
                };
                // Duplicate the switchValue on the stack. One will remain, one is
                // used by the IsStrictlyEqual
                ctx.add_instruction(Instruction::StoreCopy);
                ctx.add_instruction(Instruction::Load);
                // 2. Let exprRef be ? Evaluation of the Expression of C.
                let expr_ref = test.compile(ctx);
                // 3. Let clauseSelector be ? GetValue(exprRef).
                let clause_selector = expr_ref.and_then(|r| r.get_value(ctx));
                if clause_selector.is_ok() {
                    // 4. Return IsStrictlyEqual(input, clauseSelector).
                    ctx.add_instruction(Instruction::IsStrictlyEqual);
                    // b. If found is true then [evaluate case]
                    jump_indexes.push(Some(
                        ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue),
                    ));
                } else {
                    jump_indexes.push(None);
                }
            }

            let jump_to_end = if has_default {
                // 10. If foundInB is true, return V.
                // 11. Let defaultR be Completion(Evaluation of DefaultClause).
                jump_indexes.push(Some(ctx.add_instruction_with_jump_slot(Instruction::Jump)));
                None
            } else {
                Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
            };

            let mut index = 0;
            let mut prev_result = ControlFlow::Continue(StatementContinue::Empty);
            for (i, case) in self.cases.iter().enumerate() {
                let fallthrough_jump = if i != 0 {
                    // OPTIMISATION: if previous case ended with a break or an
                    // otherwise terminal instruction, we don't need a fallthrough
                    // jump at the beginning of the next case.
                    if prev_result.is_break() {
                        None
                    } else {
                        Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
                    }
                } else {
                    None
                };
                // Jump from IsStrictlyEqual comparison to here.
                let jump_index = if case.test.is_some() {
                    let jump_index = jump_indexes.get(index).unwrap();
                    index += 1;
                    jump_index
                } else {
                    // Default case! The jump index is last in the Vec.
                    jump_indexes.last().unwrap()
                };
                if let Some(jump_index) = jump_index {
                    ctx.set_jump_target_here(jump_index.clone());
                }

                // 1. Let V be undefined.
                // Pop the switchValue from the stack.
                ctx.add_instruction(Instruction::Store);
                // And override it with undefined
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);

                if let Some(fallthrough_jump) = fallthrough_jump {
                    ctx.set_jump_target_here(fallthrough_jump);
                }

                // Reset the previous result for every case.
                prev_result = ControlFlow::Continue(StatementContinue::Empty);

                // i. Let R be Completion(Evaluation of C).
                for ele in &case.consequent {
                    if prev_result.is_break() {
                        // Stop looping over statements if we found a break.
                        break;
                    }
                    prev_result = ele.compile(ctx);
                }
                // ii. If R.[[Value]] is not empty, set V to R.[[Value]].
                // if !ctx.is_unreachable() {
                //     ctx.add_instruction(Instruction::LoadReplace);
                // }
            }

            if let Some(jump_to_end) = jump_to_end {
                ctx.set_jump_target_here(jump_to_end);
            }
            ControlFlow::Continue(StatementContinue::Value)
        });

        switch.exit(ctx);
        // iii. If R is an abrupt completion, return ? UpdateEmpty(R, V).
        // ctx.add_instruction(Instruction::UpdateEmpty);
        // 9. Return R.
        r
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ThrowStatement<'s> {
    type Output = ExpressionError;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        self.argument
            .compile(ctx)
            .and_then(|arg| arg.get_value(ctx))
            .and_then(|_| {
                ctx.add_instruction(Instruction::Throw);
                Result::<Infallible, ExpressionError>::Err(ExpressionError::Error)
            })
            .unwrap_err()
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::TryStatement<'s> {
    type Output = StatementResult<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        ctx.add_instruction(Instruction::Empty);
        let try_finally_block = self
            .finalizer
            .is_some()
            .then(|| ctx.enter_try_finally_block());
        let try_catch_block = self.handler.is_some().then(|| ctx.enter_try_catch_block());

        // 1. Let B be Completion(Evaluation of Block).
        let b = self.block.compile(ctx);
        // 2. If B is a throw completion,
        let jump_over_catch_blocks = if let Some(catch_clause) = &self.handler {
            let jump_to_catch = try_catch_block.unwrap().exit(ctx);
            // OPTIMISATION: If the end of the try-block is unreachable, we
            // don't need a jump over the catch blocks.
            let jump_over_catch_blocks = if b.is_continue() {
                Some(ctx.add_instruction_with_jump_slot(Instruction::Jump))
            } else {
                None
            };
            ctx.set_jump_target_here(jump_to_catch);

            // let C be Completion(CatchClauseEvaluation of Catch with argument B.[[Value]]).
            let _c = catch_clause_evaluation(catch_clause, ctx);
            // 9. Return ? B.
            jump_over_catch_blocks
        } else {
            // 3. Else, let C be B.
            assert!(try_catch_block.is_none());
            None
        };
        if let Some(finalizer) = &self.finalizer {
            try_finally_block
                .unwrap()
                .exit(ctx, finalizer, jump_over_catch_blocks);
        } else if let Some(jump_over_catch_blocks) = jump_over_catch_blocks {
            // If we have a catch block following the normal execution but no
            // finally block then we'll have to handle the jump out ourselves.
            ctx.set_jump_target_here(jump_over_catch_blocks);
        }
        if !ctx.is_unreachable() {
            // 4. Return ? UpdateEmpty(C, undefined).
            ctx.add_instruction_with_constant(Instruction::LoadConstant, Value::Undefined);
            ctx.add_instruction(Instruction::UpdateEmpty);
        }
        ControlFlow::Continue(StatementContinue::Value)
    }
}

fn catch_clause_evaluation<'s, 'gc>(
    catch_clause: &'s ast::CatchClause<'s>,
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
) -> StatementResult<'gc> {
    // 14.15.2 Runtime Semantics: CatchClauseEvaluation
    let catch_env = if let Some(exception_param) = &catch_clause.param {
        // 1. Let oldEnv be the running execution context's LexicalEnvironment.
        // 2. Let catchEnv be NewDeclarativeEnvironment(oldEnv).
        // 4. Set the running execution context's LexicalEnvironment to catchEnv.
        // Note: We skip the declarative environment if there is no catch
        // param as it's not observable.
        let catch_env = ctx.enter_lexical_scope();

        // 3. For each element argName of the BoundNames of CatchParameter, do
        // a. Perform ! catchEnv.CreateMutableBinding(argName, false).
        exception_param.pattern.bound_names(&mut |arg_name| {
            let arg_name = ctx.create_string(arg_name.name.as_str());
            ctx.add_instruction_with_identifier(
                Instruction::CreateMutableBinding,
                arg_name.to_property_key(),
            );
        });
        // 5. Let status be Completion(BindingInitialization of
        //    CatchParameter with arguments thrownValue and catchEnv).
        let lexical_binding_state = ctx.lexical_binding_state;
        ctx.lexical_binding_state = true;
        let status = exception_param.pattern.compile(ctx);
        ctx.lexical_binding_state = lexical_binding_state;
        // 6. If status is an abrupt completion, then
        if let Err(status) = status {
            // a. Set the running execution context's LexicalEnvironment to
            //    oldEnv.
            catch_env.exit(ctx);
            // b. Return ? status.
            return ControlFlow::Break(status.into());
        }
        Some(catch_env)
    } else {
        None
    };
    // 7. Let B be Completion(Evaluation of Block).
    let b = catch_clause.body.compile(ctx);
    // 8. Set the running execution context's LexicalEnvironment to oldEnv.
    if let Some(catch_env) = catch_env {
        catch_env.exit(ctx);
    }
    // 9. Return ? B.
    b
}

impl<'a, 's, 'gc, 'scope> CompileLabelledEvaluation<'a, 's, 'gc, 'scope>
    for ast::WhileStatement<'s>
{
    type Output = StatementResult<'gc>;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) -> Self::Output {
        // 1. Let V be undefined.
        let v = ctx.push_stack_loop_result();
        // 2. Repeat
        let l = ctx.enter_loop(label_set.cloned());
        let jump_over_continue = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        let continue_label = ctx.get_jump_index_to_here();
        // f. If stmtResult.[[Value]] is not EMPTY, set V to
        //    stmtResult.[[Value]].
        ctx.add_instruction(Instruction::LoadReplace);
        ctx.set_jump_target_here(jump_over_continue);

        // a. Let exprRef be ? Evaluation of Expression.
        // OPTIMISATION: while(true) loops are pretty common, skip the test.
        let expr_result = if !is_boolean_literal_true(&self.test) {
            // b. Let exprValue be ? GetValue(exprRef).
            let expr_value = self.test.compile(ctx).and_then(|r| r.get_value(ctx));

            // c. If ToBoolean(exprValue) is false, return V.
            // jump over loop jump if test fails
            expr_value.map(|_| Some(ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot)))
        } else {
            Ok(None)
        };

        // d. Let stmtResult be Completion(Evaluation of Statement).
        let stmt_result = if let Err(err) = expr_result {
            ControlFlow::Break(err.into())
        } else {
            let stmt_result = self.body.compile(ctx);
            if stmt_result.is_continue() {
                ctx.add_jump_instruction_to_index(Instruction::Jump, continue_label.clone());
            }
            // We cannot know how the loop ends.
            ControlFlow::Continue(StatementContinue::Value)
        };
        {
            // ## Catch block
            ctx.set_jump_target_here(l.on_abrupt_exit());
            // Error was thrown: this means loop continues is false:
            // > c. If LoopContinues(result, labelSet) is false,
            // >    return ? UpdateEmpty(result, V).
            ctx.add_instruction(Instruction::UpdateEmpty);
            ctx.add_instruction(Instruction::Throw);
        }
        // f. If stmtResult.[[Value]] is not EMPTY, set V to
        //    stmtResult.[[Value]].

        // c. If ToBoolean(exprValue) is false, return V.
        if let Ok(Some(end_jump)) = expr_result {
            ctx.set_jump_target_here(end_jump);
        }
        // Note: exit_loop performs UpdateEmpty; if we jumped here from test
        // failure then result is currently empty and UpdateEmpty will pop V
        // into the result register.
        l.exit(ctx, continue_label);
        v.exit(ctx);

        stmt_result
    }
}

impl<'a, 's, 'gc, 'scope> CompileLabelledEvaluation<'a, 's, 'gc, 'scope>
    for ast::DoWhileStatement<'s>
{
    type Output = StatementResult<'gc>;

    fn compile_labelled(
        &'s self,
        label_set: Option<&mut Vec<&'s ast::LabelIdentifier<'s>>>,
        ctx: &mut CompileContext<'_, 's, '_, '_>,
    ) -> Self::Output {
        // 1. Let V be undefined.
        let v = ctx.push_stack_loop_result();
        // 2. Repeat,
        let l = ctx.enter_loop(label_set.cloned());
        let jump_over_continue = ctx.add_instruction_with_jump_slot(Instruction::Jump);
        // Note: to save one Jump in continue cases, the LoopContinues work is
        // here.
        // c. If stmtResult.[[Value]] is not empty, set V to
        //    stmtResult.[[Value]].
        let continue_label = ctx.get_jump_index_to_here();
        ctx.add_instruction(Instruction::LoadReplace);
        let expr_result = if is_boolean_literal_true(&self.test) {
            // OPTIMISATION: do {} while(true) loops are still somewhat common,
            // skip the test.
            // f. If ToBoolean(exprValue) is false, return V.
            Ok(None)
        } else if is_boolean_literal_false(&self.test) {
            // OPTIMISATION: do {} while(false) loops appear in tests; this is
            // a dumb optimisation: continue can never return to the beginning
            // of the loop.
            // f. If ToBoolean(exprValue) is false, return V.
            Ok(Some(ctx.add_instruction_with_jump_slot(Instruction::Jump)))
        } else {
            // d. Let exprRef be ? Evaluation of Expression.
            // e. Let exprValue be ? GetValue(exprRef).
            let expr_value = self.test.compile(ctx).and_then(|r| r.get_value(ctx));

            // f. If ToBoolean(exprValue) is false, return V.
            expr_value.map(|_| Some(ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot)))
        };

        let stmt_result = if let Err(err) = expr_result {
            ControlFlow::Break(err.into())
        } else {
            ctx.set_jump_target_here(jump_over_continue);
            // a. Let stmtResult be Completion(Evaluation of Statement).
            let stmt_result = self.body.compile(ctx);
            // b. If LoopContinues(stmtResult, labelSet) is false,
            //    return ? UpdateEmpty(stmtResult, V).
            if stmt_result.is_continue() {
                ctx.add_jump_instruction_to_index(Instruction::Jump, continue_label.clone());
            }
            // We cannot know how the loop ends.
            ControlFlow::Continue(StatementContinue::Value)
        };

        {
            // ## Catch block
            ctx.set_jump_target_here(l.on_abrupt_exit());
            // Error was thrown: this means loop continues is false:
            // > b. If LoopContinues(stmtResult, labelSet) is false,
            // >    return ? UpdateEmpty(stmtResult, V).
            ctx.add_instruction(Instruction::UpdateEmpty);
            ctx.add_instruction(Instruction::Throw);
        }
        // f. If ToBoolean(exprValue) is false, return V.
        if let Ok(Some(jump_to_end)) = expr_result {
            ctx.set_jump_target_here(jump_to_end.clone());
        }
        // Note: exit_loop performs UpdateEmpty; if we jumped here from test
        // failure then result is currently empty and UpdateEmpty will pop V
        // into the result register.
        l.exit(ctx, continue_label);
        v.exit(ctx);
        stmt_result
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::BreakStatement<'s> {
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        ctx.compile_break(self.label.as_ref());
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::ContinueStatement<'s> {
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        ctx.compile_continue(self.label.as_ref());
    }
}

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::Statement<'s> {
    type Output = StatementResult<'gc>;
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        if ctx.is_unreachable() {
            // OPTIMISATION: If the previous statement was terminal, then later
            // statements cannot be executed and do not need to be compiled.
            return StatementBreak::Return.into();
        }
        match self {
            Self::ExpressionStatement(x) => x.compile(ctx),
            Self::ReturnStatement(x) => x.compile(ctx).into(),
            Self::IfStatement(x) => x.compile(ctx),
            Self::VariableDeclaration(x) => {
                if let Err(err) = x.compile(ctx) {
                    ControlFlow::Break(err.into())
                } else {
                    // 6. Return EMPTY.
                    ControlFlow::Continue(StatementContinue::Empty)
                }
            }
            Self::FunctionDeclaration(_) => {
                // Note: Function declaration statements are always hoisted.
                // There is no work left to do here.
                ControlFlow::Continue(StatementContinue::Empty)
            }
            Self::BlockStatement(x) => x.compile(ctx),
            Self::EmptyStatement(_) => ControlFlow::Continue(StatementContinue::Empty),
            Self::ForStatement(x) => x.compile_labelled(None, ctx),
            Self::ThrowStatement(x) => ControlFlow::Break(x.compile(ctx).into()),
            Self::TryStatement(x) => x.compile(ctx),
            Self::BreakStatement(statement) => {
                statement.compile(ctx);
                ControlFlow::Break(StatementBreak::Break)
            }
            Self::ContinueStatement(statement) => {
                statement.compile(ctx);
                ControlFlow::Break(StatementBreak::Continue)
            }
            Self::DebuggerStatement(_) => {
                ctx.add_instruction(Instruction::Debug);
                ControlFlow::Continue(StatementContinue::Empty)
            }
            Self::DoWhileStatement(statement) => statement.compile_labelled(None, ctx),
            Self::ForInStatement(statement) => statement.compile_labelled(None, ctx),
            Self::ForOfStatement(statement) => statement.compile_labelled(None, ctx),
            Self::LabeledStatement(statement) => statement.compile_labelled(None, ctx),
            Self::SwitchStatement(statement) => statement.compile_labelled(None, ctx),
            Self::WhileStatement(statement) => statement.compile_labelled(None, ctx),
            Self::WithStatement(st) => st.compile(ctx),
            Self::ClassDeclaration(x) => {
                // If this is a declare statement, it's a TypeScript ambient declaration
                // and should not generate any runtime code, similar to type declarations
                #[cfg(feature = "typescript")]
                if x.declare {
                    return ControlFlow::Continue(StatementContinue::Empty);
                }
                if let Err(err) = x.compile(ctx) {
                    ControlFlow::Break(err.into())
                } else {
                    ControlFlow::Continue(StatementContinue::Value)
                }
            }
            Self::ImportDeclaration(_) => {
                // Note: Import declarations do not perform any runtime work.
                ControlFlow::Continue(StatementContinue::Empty)
            }
            Self::ExportAllDeclaration(x) => {
                x.compile(ctx);
                ControlFlow::Continue(StatementContinue::Empty)
            }
            Self::ExportDefaultDeclaration(x) => {
                x.compile(ctx)?;
                ControlFlow::Continue(StatementContinue::Empty)
            }
            Self::ExportNamedDeclaration(x) => {
                x.compile(ctx)?;
                ControlFlow::Continue(StatementContinue::Empty)
            }
            #[cfg(feature = "typescript")]
            Self::TSEnumDeclaration(x) => {
                x.compile(ctx);
                ControlFlow::Continue(StatementContinue::Empty)
            }
            #[cfg(feature = "typescript")]
            Self::TSTypeAliasDeclaration(_)
            | Self::TSInterfaceDeclaration(_)
            | Self::TSModuleDeclaration(_)
            | Self::TSGlobalDeclaration(_) => ControlFlow::Continue(StatementContinue::Empty),
            #[cfg(not(feature = "typescript"))]
            Self::TSTypeAliasDeclaration(_)
            | Self::TSInterfaceDeclaration(_)
            | Self::TSModuleDeclaration(_)
            | Self::TSEnumDeclaration(_)
            | Self::TSGlobalDeclaration(_) => {
                unreachable!()
            }
            // TODO: Implement TypeScript-specific statement compilation
            Self::TSExportAssignment(_)
            | Self::TSImportEqualsDeclaration(_)
            | Self::TSNamespaceExportDeclaration(_) => {
                unreachable!()
            }
        }
    }
}

fn is_anonymous_function_definition(expression: &ast::Expression) -> bool {
    match expression {
        ast::Expression::ArrowFunctionExpression(_) => true,
        ast::Expression::FunctionExpression(f) => f.id.is_none(),
        ast::Expression::ClassExpression(f) => f.id.is_none(),
        ast::Expression::ParenthesizedExpression(x) => {
            is_anonymous_function_definition(&x.expression)
        }
        _ => false,
    }
}

#[cfg(feature = "typescript")]
impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::TSEnumDeclaration<'s> {
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        // TODO: Check if this is a const enum when the field is available
        // For now, treat all enums as regular enums

        // 1. Create the enum name binding
        let enum_name = self.id.name.as_str();
        let enum_identifier = ctx.create_string(enum_name);
        ctx.add_instruction_with_identifier(
            Instruction::ResolveBinding,
            enum_identifier.to_property_key(),
        );

        // 2. Analyze enum properties to determine if we can use ObjectCreateWithShape
        let mut is_numeric_enum = true;
        let mut has_computed_members = false;

        // First pass: check if all members are simple (no computed expressions)
        for member in self.body.members.iter() {
            if let Some(ref initializer) = member.initializer {
                match initializer {
                    ast::Expression::StringLiteral(_) => {
                        is_numeric_enum = false;
                    }
                    ast::Expression::NumericLiteral(_) => {}
                    _ => {
                        // Computed expression
                        is_numeric_enum = false;
                        has_computed_members = true;
                        break;
                    }
                }
            }
        }

        // If we have computed members, fall back to the original property-by-property approach
        if has_computed_members {
            compile_enum_with_computed_members(self, ctx);
            return;
        }

        // 3. Create object shape with all enum member keys directly
        let prototype = Some(
            ctx.get_agent()
                .current_realm_record()
                .intrinsics()
                .object_prototype()
                .into_object(),
        );

        // Collect all property keys upfront for intrinsic shape creation
        let mut property_keys = Vec::new();

        // Add forward mapping keys
        for member in self.body.members.iter() {
            let member_name = match &member.id {
                ast::TSEnumMemberName::Identifier(ident) => ident.name.as_str(),
                _ => "unknown",
            };
            let identifier = ctx.create_property_key(member_name);
            property_keys.push(identifier);
        }

        // Add reverse mapping keys for numeric enums
        if is_numeric_enum {
            let mut current_numeric_value = 0f64;
            for member in self.body.members.iter() {
                let reverse_key_value =
                    if let Some(ast::Expression::NumericLiteral(num_lit)) = &member.initializer {
                        current_numeric_value = num_lit.value + 1.0;
                        num_lit.value
                    } else {
                        let value = current_numeric_value;
                        current_numeric_value += 1.0;
                        value
                    };

                let reverse_key = ctx.create_property_key(&reverse_key_value.to_string());
                property_keys.push(reverse_key);
            }
        }

        // Create intrinsic shape directly with all properties in one shot
        let properties_count = property_keys.len();
        let agent = ctx.get_agent_mut();
        let (cap, index) = agent
            .heap
            .elements
            .allocate_keys_with_capacity(properties_count)
            .expect("Should perform GC here");
        let cap = cap.make_intrinsic();

        let keys_memory = agent.heap.elements.get_keys_uninit_raw(cap, index);
        for (slot, key) in keys_memory.iter_mut().zip(property_keys.iter()) {
            *slot = Some(key.unbind());
        }

        let shape = agent.heap.create(ObjectShapeRecord::create(
            prototype,
            index,
            cap,
            properties_count,
        ));

        // 4. Compile values in correct order (matching the shape)
        let mut current_numeric_value = 0f64;

        // Compile forward mapping values
        for member in self.body.members.iter() {
            if let Some(ref initializer) = member.initializer {
                match initializer {
                    ast::Expression::StringLiteral(string_lit) => {
                        let string_value = ctx.create_string(string_lit.value.as_str());
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            string_value.into_value(),
                        );
                    }
                    ast::Expression::NumericLiteral(num_lit) => {
                        let number_value = ctx.create_number(num_lit.value);
                        ctx.add_instruction_with_constant(
                            Instruction::StoreConstant,
                            number_value.into_value(),
                        );
                        current_numeric_value = num_lit.value + 1.0;
                    }
                    _ => unreachable!("Computed members should have been filtered out"),
                }
            } else {
                let number_value = ctx.create_number(current_numeric_value);
                ctx.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    number_value.into_value(),
                );
                current_numeric_value += 1.0;
            }
            ctx.add_instruction(Instruction::Load);
        }

        // Compile reverse mapping values for numeric enums
        if is_numeric_enum {
            for member in self.body.members.iter() {
                let member_name = match &member.id {
                    ast::TSEnumMemberName::Identifier(ident) => ident.name.as_str(),
                    _ => "unknown",
                };
                let name_string = ctx.create_string(member_name);
                ctx.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    name_string.into_value(),
                );
                ctx.add_instruction(Instruction::Load);
            }
        }

        // 5. Create object with pre-computed shape
        ctx.add_instruction_with_shape(Instruction::ObjectCreateWithShape, shape);

        // 6. Initialize the binding with the completed enum object
        ctx.add_instruction(Instruction::InitializeReferencedBinding);
    }
}

#[cfg(feature = "typescript")]
fn compile_enum_with_computed_members<'s>(
    enum_decl: &'s ast::TSEnumDeclaration<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // Fallback to original implementation for enums with computed members
    ctx.add_instruction(Instruction::ObjectCreate);

    let mut current_numeric_value = 0f64;
    let mut is_numeric_enum = true;

    for member in enum_decl.body.members.iter() {
        let member_name = match &member.id {
            ast::TSEnumMemberName::Identifier(ident) => ident.name.as_str(),
            _ => "unknown",
        };

        // Push member name as property key onto stack
        let member_string = ctx.create_string(member_name);
        ctx.add_instruction_with_constant(Instruction::LoadConstant, member_string.into_value());

        // Determine the value for this enum member
        if let Some(ref initializer) = member.initializer {
            match initializer {
                ast::Expression::StringLiteral(string_lit) => {
                    is_numeric_enum = false;
                    let string_value = ctx.create_string(string_lit.value.as_str());
                    ctx.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        string_value.into_value(),
                    );
                }
                ast::Expression::NumericLiteral(num_lit) => {
                    let number_value = ctx.create_number(num_lit.value);
                    ctx.add_instruction_with_constant(
                        Instruction::StoreConstant,
                        number_value.into_value(),
                    );
                    current_numeric_value = num_lit.value + 1.0;
                }
                _ => {
                    is_numeric_enum = false;
                    let _ = initializer.compile(ctx).and_then(|r| r.get_value(ctx));
                }
            }
        } else {
            let number_value = ctx.create_number(current_numeric_value);
            ctx.add_instruction_with_constant(
                Instruction::StoreConstant,
                number_value.into_value(),
            );
            current_numeric_value += 1.0;
        }

        ctx.add_instruction(Instruction::ObjectDefineProperty);
    }

    // Add reverse mappings for numeric enums
    if is_numeric_enum {
        current_numeric_value = 0f64;

        for member in enum_decl.body.members.iter() {
            let member_name = match &member.id {
                ast::TSEnumMemberName::Identifier(ident) => ident.name.as_str(),
                _ => "unknown",
            };

            let reverse_key_value = if let Some(ref initializer) = member.initializer {
                if let ast::Expression::NumericLiteral(num_lit) = initializer {
                    current_numeric_value = num_lit.value + 1.0;
                    num_lit.value
                } else {
                    current_numeric_value += 1.0;
                    continue;
                }
            } else {
                let value = current_numeric_value;
                current_numeric_value += 1.0;
                value
            };

            let key_number = ctx.create_number(reverse_key_value);
            ctx.add_instruction_with_constant(Instruction::LoadConstant, key_number.into_value());

            let name_string = ctx.create_string(member_name);
            ctx.add_instruction_with_constant(Instruction::StoreConstant, name_string.into_value());

            ctx.add_instruction(Instruction::ObjectDefineProperty);
        }
    }

    // Move the enum object from stack to result register
    ctx.add_instruction(Instruction::Store);
    // Now initialize the binding (reference is on stack, value is in result)
    ctx.add_instruction(Instruction::InitializeReferencedBinding);
}
