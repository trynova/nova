// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::ControlFlow;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{
                private_get, private_set, set, throw_no_private_name_error, try_private_get,
                try_private_set, try_set,
            },
            type_conversion::{to_object, to_property_key, to_property_key_simple},
        },
        builtins::{ordinary::caches::PropertyLookupCache, proxy::Proxy},
        execution::{
            Environment,
            agent::{
                self, ExceptionType, JsError, TryError, TryResult, js_result_into_try,
                option_into_try,
            },
            get_global_object,
        },
        types::{
            Function, InternalMethods, Object, PropertyKey, SetResult, String, TryGetResult, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::Scopable,
    },
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};
use agent::{Agent, JsResult};

use super::PrivateName;

#[derive(Debug, Clone)]
pub(crate) struct VariableReference<'a> {
    /// ### \[\[Base]]
    ///
    /// The Environment Record which holds the binding.
    base: Environment<'a>,
    /// ### \[\[ReferencedName]]
    ///
    /// The name of the binding.
    referenced_name: String<'a>,
    /// Property lookup cache for the variable reference.
    cache: Option<PropertyLookupCache<'a>>,
}

#[derive(Debug, Clone)]
pub(crate) struct PropertyExpressionReference<'a> {
    /// ### \[\[Base]]
    ///
    /// The value which holds the binding.
    base: Value<'a>,
    /// ### \[\[ReferencedName]]
    ///
    /// The name of the binding.
    referenced_name: Value<'a>,
}

#[derive(Debug, Clone)]
pub(crate) struct PropertyReference<'a> {
    /// ### \[\[Base]]
    ///
    /// The value which holds the binding.
    base: Value<'a>,
    /// ### \[\[ReferencedName]]
    ///
    /// The name of the binding.
    referenced_name: PropertyKey<'a>,
}

#[derive(Debug, Clone)]
pub(crate) struct SuperExpressionReference<'a> {
    /// ### \[\[Base]]
    ///
    /// The value which holds the binding.
    base: Value<'a>,
    /// ### \[\[ReferencedName]]
    ///
    /// The name of the binding.
    referenced_name: Value<'a>,
    /// ### \[\[ThisValue]]
    ///
    /// The \[\[ThisValue]] field holds the this value at the time the
    /// Reference Record was created.
    this_value: Value<'a>,
}

#[derive(Debug, Clone)]
pub(crate) struct SuperReference<'a> {
    /// ### \[\[Base]]
    ///
    /// The value which holds the binding.
    base: Value<'a>,
    /// ### \[\[ReferencedName]]
    ///
    /// The name of the binding.
    referenced_name: PropertyKey<'a>,
    /// ### \[\[ThisValue]]
    ///
    /// The \[\[ThisValue]] field holds the this value at the time the
    /// Reference Record was created.
    this_value: Value<'a>,
}

/// ### [6.2.5 The Reference Record Specification Type](https://tc39.es/ecma262/#sec-reference-record-specification-type)
///
/// The Reference Record type is used to explain the behaviour of such
/// operators as delete, typeof, the assignment operators, the super keyword
/// and other language features. For example, the left-hand operand of an
/// assignment is expected to produce a Reference Record.
#[derive(Debug, Clone)]
#[repr(u8)]
pub(crate) enum Reference<'a> {
    /// Unresolvable Reference.
    ///
    /// Contains the referenced name.
    Unresolvable(String<'a>) = 0b0000,
    /// Unresolvable strict Reference.
    ///
    /// Contains the referenced name.
    UnresolvableStrict(String<'a>) = 0b0001,
    /// Variable Reference.
    Variable(VariableReference<'a>) = 0b0010,
    /// Variable strict Reference.
    VariableStrict(VariableReference<'a>) = 0b0011,
    /// Unchecked Property Reference.
    ///
    /// `ToPropertyKey` must be called on the referenced name before using the
    /// reference.
    PropertyExpression(PropertyExpressionReference<'a>) = 0b1100,
    /// Unchecked strict Property Reference.
    ///
    /// `ToPropertyKey` must be called on the referenced name before using the
    /// reference.
    PropertyExpressionStrict(PropertyExpressionReference<'a>) = 0b1101,
    /// Checked Property Reference.
    Property(PropertyReference<'a>) = 0b0100,
    /// Checked strict Property Reference.
    PropertyStrict(PropertyReference<'a>) = 0b0101,
    /// Unchecked Super Reference.
    ///
    /// `ToPropertyKey` must be called on the referenced name before using the
    /// reference.
    SuperExpression(SuperExpressionReference<'a>) = 0b11100,
    /// Unchecked strict Super Reference.
    ///
    /// `ToPropertyKey` must be called on the referenced name before using the
    /// reference.
    SuperExpressionStrict(SuperExpressionReference<'a>) = 0b11101,
    /// Checked Super Reference.
    Super(SuperReference<'a>) = 0b10100,
    /// Checked strict Super Reference.
    SuperStrict(SuperReference<'a>) = 0b10101,
}

impl<'a> Reference<'a> {
    pub(crate) fn new_unresolvable_reference(referenced_name: String<'a>, strict: bool) -> Self {
        if strict {
            Self::UnresolvableStrict(referenced_name)
        } else {
            Self::Unresolvable(referenced_name)
        }
    }

    pub(crate) fn new_variable_reference(
        base: Environment<'a>,
        referenced_name: String<'a>,
        cache: Option<PropertyLookupCache<'a>>,
        strict: bool,
    ) -> Self {
        let reference = VariableReference {
            base,
            referenced_name,
            cache,
        };
        if strict {
            Self::VariableStrict(reference)
        } else {
            Self::Variable(reference)
        }
    }

    pub(crate) fn new_property_expression_reference(
        base: Value<'a>,
        referenced_name: Value<'a>,
        strict: bool,
    ) -> Self {
        let reference = PropertyExpressionReference {
            base,
            referenced_name,
        };
        if strict {
            Self::PropertyExpressionStrict(reference)
        } else {
            Self::PropertyExpression(reference)
        }
    }

    pub(crate) fn new_property_reference(
        base: Value<'a>,
        referenced_name: PropertyKey<'a>,
        strict: bool,
    ) -> Self {
        let reference = PropertyReference {
            base,
            referenced_name,
        };
        if strict {
            Self::PropertyStrict(reference)
        } else {
            Self::Property(reference)
        }
    }

    pub(crate) fn new_super_expression_reference(
        base: Value<'a>,
        referenced_name: Value<'a>,
        this_value: Value<'a>,
        strict: bool,
    ) -> Self {
        let reference = SuperExpressionReference {
            base,
            referenced_name,
            this_value,
        };
        if strict {
            Self::SuperExpressionStrict(reference)
        } else {
            Self::SuperExpression(reference)
        }
    }

    pub(crate) fn new_super_reference(
        base: Value<'a>,
        referenced_name: PropertyKey<'a>,
        this_value: Value<'a>,
        strict: bool,
    ) -> Self {
        let reference = SuperReference {
            base,
            referenced_name,
            this_value,
        };
        if strict {
            Self::SuperStrict(reference)
        } else {
            Self::Super(reference)
        }
    }

    pub(crate) fn new_private_reference(base: Value<'a>, referenced_name: PrivateName) -> Self {
        let reference = PropertyReference {
            base,
            referenced_name: referenced_name.into(),
        };
        Self::PropertyStrict(reference)
    }

    /// Get data associated with a Private reference.
    ///
    /// ## Panics
    ///
    /// Panics if the reference is not a private reference.
    pub(crate) fn into_private_reference_data(self) -> (Value<'a>, PrivateName) {
        let Self::PropertyStrict(PropertyReference {
            base,
            referenced_name: PropertyKey::PrivateName(name),
        }) = self
        else {
            unreachable!()
        };
        (base, name)
    }

    /// Get \[\[ThisValue]] or \[\[Base]] as a Value.
    ///
    /// ## Panics
    ///
    /// Panics if the reference is a variable reference or unresolvable.
    pub(crate) fn this_value(&self, agent: &Agent) -> Value<'a> {
        match self {
            Reference::PropertyExpression(v) | Reference::PropertyExpressionStrict(v) => v.base,
            Reference::Property(v) | Reference::PropertyStrict(v) => v.base,
            Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => v.this_value,
            Reference::Super(v) | Reference::SuperStrict(v) => v.this_value,
            Reference::Variable(v) | Reference::VariableStrict(v) => match v.base {
                Environment::Global(e) => e.get_binding_object(agent).into(),
                Environment::Object(e) => e.get_binding_object(agent).into(),
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }

    /// Get \[\[Base]] as a Value.
    ///
    /// ## Panics
    ///
    /// Panics if the reference is a variable reference or unresolvable.
    pub(crate) fn base_value(&self) -> Value<'a> {
        match self {
            Reference::PropertyExpression(v) | Reference::PropertyExpressionStrict(v) => v.base,
            Reference::Property(v) | Reference::PropertyStrict(v) => v.base,
            Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => v.base,
            Reference::Super(v) | Reference::SuperStrict(v) => v.base,
            _ => unreachable!("{:?}", self),
        }
    }

    /// Get \[\[Base]] as an Environment.
    ///
    /// ## Panics
    ///
    /// Panics if the reference is a property reference or unresolvable.
    pub(crate) fn base_env(&self) -> Environment<'a> {
        match self {
            Reference::Variable(v) | Reference::VariableStrict(v) => v.base,
            _ => unreachable!(),
        }
    }

    /// Get \[\[ReferencedName]] as a Value.
    ///
    /// ## Panics
    ///
    /// Panics if the referenced name is not a Value.
    pub(crate) fn referenced_name_value(&self) -> Value<'a> {
        match self {
            Reference::PropertyExpression(v) | Reference::PropertyExpressionStrict(v) => {
                v.referenced_name
            }
            Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => {
                v.referenced_name
            }
            _ => unreachable!(),
        }
    }

    /// Get \[\[ReferencedName]] as a PropertyKey.
    ///
    /// ## Panics
    ///
    /// Panics if the referenced name is not a PropertyKey.
    pub(crate) fn referenced_name_property_key(&self) -> PropertyKey<'a> {
        match self {
            Reference::Property(v) | Reference::PropertyStrict(v) => v.referenced_name,
            Reference::Super(v) | Reference::SuperStrict(v) => v.referenced_name,
            Reference::Unresolvable(name) | Reference::UnresolvableStrict(name) => {
                unreachable!("{:?}", name.to_property_key())
            }
            _ => unreachable!("{self:?}"),
        }
    }

    /// Get \[\[ReferencedName]] as a String.
    ///
    /// ## Panics
    ///
    /// Panics if the referenced name is not a String.
    pub(crate) fn referenced_name_string(&self) -> String<'a> {
        match self {
            Reference::Variable(v) | Reference::VariableStrict(v) => v.referenced_name,
            _ => unreachable!(),
        }
    }

    /// Replace a \[\[ReferencedName]] Value with its ToPropertyKey result
    pub(crate) fn set_referenced_name_to_property_key(&mut self, referenced_name: PropertyKey) {
        match self {
            Reference::PropertyExpression(v) => {
                *self = Self::new_property_reference(v.base, referenced_name.unbind(), false)
            }
            Reference::PropertyExpressionStrict(v) => {
                *self = Self::new_property_reference(v.base, referenced_name.unbind(), true)
            }
            Reference::SuperExpression(v) => {
                *self =
                    Self::new_super_reference(v.base, referenced_name.unbind(), v.this_value, false)
            }
            Reference::SuperExpressionStrict(v) => {
                *self =
                    Self::new_super_reference(v.base, referenced_name.unbind(), v.this_value, true)
            }
            _ => unreachable!(),
        }
    }

    pub(crate) fn is_static_property_reference(&self) -> bool {
        matches!(
            self,
            Reference::Property(_)
                | Reference::PropertyStrict(_)
                | Reference::Super(_)
                | Reference::SuperStrict(_)
        )
    }

    /// ### \[\[Strict]]
    pub(crate) fn strict(&self) -> bool {
        matches!(
            self,
            Self::UnresolvableStrict(_)
                | Self::VariableStrict(_)
                | Self::PropertyExpressionStrict(_)
                | Self::PropertyStrict(_)
                | Self::SuperExpressionStrict(_)
                | Self::SuperStrict(_)
        )
    }
}

bindable_handle!(Reference);

/// ### [6.2.5.1 IsPropertyReference ( V )](https://tc39.es/ecma262/#sec-ispropertyreference)
///
/// The abstract operation IsPropertyReference takes argument V (a Reference
/// Record) and returns a Boolean.
pub(crate) fn is_property_reference(reference: &Reference) -> bool {
    match reference {
        // 1. if V.[[Base]] is unresolvable, return false.
        Reference::Unresolvable(_) |
        Reference::UnresolvableStrict(_) |
        // 2. If V.[[Base]] is an Environment Record, return false;
        Reference::Variable(_) |
        Reference::VariableStrict(_) => false,
        // otherwise return true.
        _ => true,
    }
}

/// ### [6.2.5.2 IsUnresolvableReference ( V )](https://tc39.es/ecma262/#sec-isunresolvablereference)
///
/// The abstract operation IsUnresolvableReference takes argument V (a
/// Reference Record) and returns a Boolean.
pub(crate) fn is_unresolvable_reference(reference: &Reference) -> bool {
    // 1. If V.[[Base]] is unresolvable, return true; otherwise return false.
    matches!(
        reference,
        Reference::Unresolvable(_) | Reference::UnresolvableStrict(_)
    )
}

/// ### [6.2.5.3 IsSuperReference ( V )](https://tc39.es/ecma262/#sec-issuperreference)
///
/// The abstract operation IsSuperReference takes argument V (a Reference
/// Record) and returns a Boolean.
pub(crate) fn is_super_reference(reference: &Reference) -> bool {
    // 1. If V.[[ThisValue]] is not empty, return true; otherwise return false.
    matches!(
        reference,
        Reference::SuperExpression(_)
            | Reference::SuperExpressionStrict(_)
            | Reference::Super(_)
            | Reference::SuperStrict(_)
    )
}

/// ### [6.2.5.4 IsPrivateReference ( V )](https://tc39.es/ecma262/#sec-isprivatereference)
///
/// The abstract operation IsPrivateReference takes argument V (a Reference
/// Record) and returns a Boolean.
pub(crate) fn is_private_reference(reference: &Reference) -> bool {
    // 1. If V.[[ReferencedName]] is a Private Name, return true; otherwise return false.
    matches!(
        reference,
        Reference::PropertyStrict(PropertyReference {
            referenced_name: PropertyKey::PrivateName(_),
            ..
        })
    )
}

/// ### [6.2.5.5 GetValue ( V )](https://tc39.es/ecma262/#sec-getvalue)
/// The abstract operation GetValue takes argument V (a Reference Record or an
/// ECMAScript language value) and returns either a normal completion
/// containing an ECMAScript language value or an abrupt completion.
pub(crate) fn get_value<'gc>(
    agent: &mut Agent,
    reference: &Reference,
    mut gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    // 1. If V is not a Reference Record, return V.
    // Note: we never perform GetValue on Reference Records, as we know
    // statically if it's needed or not.
    match reference {
        // 2. If IsUnresolvableReference(V) is true, throw a ReferenceError exception.
        Reference::Unresolvable(referenced_name)
        | Reference::UnresolvableStrict(referenced_name) => {
            let gc = gc.into_nogc();
            let error_message = format!(
                "Cannot access undeclared variable '{}'.",
                referenced_name.to_string_lossy_(agent)
            );
            Err(agent.throw_exception(ExceptionType::ReferenceError, error_message, gc))
        }
        // 3. If IsPropertyReference(V) is true, then
        Reference::PropertyExpression(_)
        | Reference::PropertyExpressionStrict(_)
        | Reference::SuperExpression(_)
        | Reference::SuperExpressionStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base = reference.base_value().bind(gc.nogc());
            let referenced_name = reference.referenced_name_value().bind(gc.nogc());
            if base.is_undefined() || base.is_null() {
                return Err(throw_read_undefined_or_null_error(
                    agent,
                    referenced_name.unbind(),
                    base.unbind(),
                    gc.into_nogc(),
                ));
            }
            let this_value = get_maybe_this_value(reference).map(|v| v.scope(agent, gc.nogc()));
            let base = base.scope(agent, gc.nogc());
            // c. If V.[[ReferencedName]] is not a property key, then
            // i. Set V.[[ReferencedName]] to ? ToPropertyKey(V.[[ReferencedName]]).
            let referenced_name = to_property_key(agent, referenced_name.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            // SAFETY: not shared.
            let base = unsafe { base.take(agent) }.bind(gc.nogc());
            let this_value = this_value.map_or(base, |v| unsafe { v.take(agent) }.bind(gc.nogc()));
            if let Ok(base_obj) = Object::try_from(base) {
                // d. Return ? baseObj.[[Get]](V.[[ReferencedName]], GetThisValue(V)).
                base_obj.unbind().internal_get(
                    agent,
                    referenced_name.unbind(),
                    this_value.unbind(),
                    gc,
                )
            } else {
                // base is not an object; we handle primitives separately.
                handle_primitive_get_value(agent, referenced_name.unbind(), base.unbind(), gc)
            }
        }
        Reference::Property(_)
        | Reference::PropertyStrict(_)
        | Reference::Super(_)
        | Reference::SuperStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base = reference.base_value().bind(gc.nogc());
            let base_obj = Object::try_from(base);
            let referenced_name = reference.referenced_name_property_key().bind(gc.nogc());
            // b. If IsPrivateReference(V) is true, then
            if let PropertyKey::PrivateName(referenced_name) = referenced_name {
                // i. Return ? PrivateGet(baseObj, V.[[ReferencedName]]).
                if let Ok(base_obj) = base_obj {
                    private_get(agent, base_obj.unbind(), referenced_name.unbind(), gc)
                } else {
                    Err(throw_no_private_name_error(agent, gc.into_nogc()))
                }
            } else if let Ok(base_obj) = base_obj {
                // d. Return ? baseObj.[[Get]](V.[[ReferencedName]], GetThisValue(V)).
                base_obj.unbind().internal_get(
                    agent,
                    referenced_name.unbind(),
                    get_this_value(reference),
                    gc,
                )
            } else {
                // base is not an object; we handle primitives separately.
                handle_primitive_get_value(agent, referenced_name.unbind(), base.unbind(), gc)
            }
        }
        Reference::Variable(v) | Reference::VariableStrict(v) => {
            // 4. Else,
            // a. Let base be V.[[Base]].
            let base = v.base;
            // b. Assert: base is an Environment Record.
            // c. Return ? base.GetBindingValue(V.[[ReferencedName]], V.[[Strict]]) (see 9.1).
            base.get_binding_value(agent, v.referenced_name, reference.strict(), gc)
        }
    }
}

fn handle_primitive_get_value<'a>(
    agent: &mut Agent,
    referenced_name: PropertyKey,
    value: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Value<'a>> {
    // Primitive value. annoying stuff.
    if referenced_name.is_private_name() {
        // i. Return ? PrivateGet(baseObj, V.[[ReferencedName]]).
        return Err(throw_no_private_name_error(agent, gc.into_nogc()));
    }
    match value {
        Value::Undefined | Value::Null => {
            Err(throw_read_undefined_or_null_error(
                agent,
                // SAFETY: We do not care about the conversion validity in
                // error message logging.
                unsafe { referenced_name.into_value_unchecked() },
                value,
                gc.into_nogc(),
            ))
        }
        Value::Boolean(_) => agent
            .current_realm_record()
            .intrinsics()
            .boolean_prototype()
            .internal_get(agent, referenced_name.unbind(), value, gc),
        Value::String(_) | Value::SmallString(_) => {
            let string = String::try_from(value).unwrap();
            if let Some(prop_desc) = string.get_property_descriptor(agent, referenced_name) {
                Ok(prop_desc.value.unwrap())
            } else {
                agent
                    .current_realm_record()
                    .intrinsics()
                    .string_prototype()
                    .internal_get(agent, referenced_name.unbind(), value, gc)
            }
        }
        Value::Symbol(_) => agent
            .current_realm_record()
            .intrinsics()
            .symbol_prototype()
            .internal_get(agent, referenced_name.unbind(), value, gc),
        Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => agent
            .current_realm_record()
            .intrinsics()
            .number_prototype()
            .internal_get(agent, referenced_name.unbind(), value, gc),
        Value::BigInt(_) | Value::SmallBigInt(_) => agent
            .current_realm_record()
            .intrinsics()
            .big_int_prototype()
            .internal_get(agent, referenced_name.unbind(), value, gc),
        _ => unreachable!(),
    }
}

pub(crate) fn throw_read_undefined_or_null_error<'a>(
    agent: &mut Agent,
    referenced_value: Value,
    value: Value,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    let error_message = format!(
        "Cannot read property '{}' of {}.",
        referenced_value
            .try_string_repr(agent, gc)
            .to_string_lossy_(agent),
        if value.is_undefined() {
            "undefined"
        } else {
            "null"
        }
    );
    agent.throw_exception(ExceptionType::TypeError, error_message, gc)
}

fn try_handle_primitive_get_value<'a>(
    agent: &mut Agent,
    referenced_name: PropertyKey<'a>,
    receiver: Value<'a>,
    cache: Option<PropertyLookupCache<'a>>,
    gc: NoGcScope<'a, '_>,
) -> ControlFlow<TryError<'a>, TryGetValueContinue<'a>> {
    // b. If IsPrivateReference(V) is true, then
    if referenced_name.is_private_name() {
        // i. Return ? PrivateGet(baseObj, V.[[ReferencedName]]).
        return throw_no_private_name_error(agent, gc).into();
    }
    // Primitive value. annoying stuff.
    let prototype: Object = match receiver {
        Value::Undefined | Value::Null => {
            return throw_read_undefined_or_null_error(
                agent,
                // SAFETY: We do not care about the conversion validity in
                // error message logging.
                unsafe { referenced_name.into_value_unchecked() },
                receiver,
                gc,
            )
            .into();
        }
        Value::Boolean(_) => agent
            .current_realm_record()
            .intrinsics()
            .boolean_prototype()
            .into(),
        Value::String(_) | Value::SmallString(_) => {
            let string = String::try_from(receiver).unwrap();
            if let Some(prop_desc) = string.get_property_descriptor(agent, referenced_name) {
                return TryGetValueContinue::Value(prop_desc.value.unwrap()).into();
            }
            agent
                .current_realm_record()
                .intrinsics()
                .string_prototype()
                .into()
        }
        Value::Symbol(_) => agent
            .current_realm_record()
            .intrinsics()
            .symbol_prototype()
            .into(),
        Value::Number(_) | Value::Integer(_) | Value::SmallF64(_) => agent
            .current_realm_record()
            .intrinsics()
            .number_prototype()
            .into(),
        Value::BigInt(_) | Value::SmallBigInt(_) => agent
            .current_realm_record()
            .intrinsics()
            .big_int_prototype()
            .into(),
        _ => unreachable!(),
    };
    prototype
        .try_get(agent, referenced_name, receiver, cache, gc)
        .map_continue(|c| TryGetValueContinue::from_get_continue(c, receiver, referenced_name))
}

pub(crate) enum TryGetValueContinue<'a> {
    /// No property exists in the object or its prototype chain.
    Unset,
    /// A data property was found.
    Value(Value<'a>),
    /// A getter call is needed.
    ///
    /// This means that the method ran to completion but could not call the
    /// getter itself.
    Get {
        getter: Function<'a>,
        receiver: Value<'a>,
    },
    /// A Proxy trap call is needed.
    ///
    /// This means that the method ran to completion but could not call the
    /// Proxy trap itself.
    Proxy {
        proxy: Proxy<'a>,
        receiver: Value<'a>,
        property_key: PropertyKey<'a>,
    },
}
bindable_handle!(TryGetValueContinue);

impl<'a> TryGetValueContinue<'a> {
    fn from_get_continue(
        c: TryGetResult<'a>,
        receiver: Value<'a>,
        property_key: PropertyKey<'a>,
    ) -> Self {
        match c {
            TryGetResult::Unset => Self::Unset,
            TryGetResult::Value(value) => Self::Value(value),
            TryGetResult::Get(getter) => Self::Get { getter, receiver },
            TryGetResult::Proxy(proxy) => Self::Proxy {
                proxy,
                receiver,
                property_key,
            },
        }
    }
}

impl<'a> From<TryGetValueContinue<'a>> for ControlFlow<TryError<'a>, TryGetValueContinue<'a>> {
    fn from(b: TryGetValueContinue<'a>) -> Self {
        Self::Continue(b)
    }
}

/// ### [6.2.5.5 GetValue ( V )](https://tc39.es/ecma262/#sec-getvalue)
/// The abstract operation GetValue takes argument V (a Reference Record or an
/// ECMAScript language value) and returns either a normal completion
/// containing an ECMAScript language value or an abrupt completion.
pub(crate) fn try_get_value<'gc>(
    agent: &mut Agent,
    reference: &Reference,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> ControlFlow<TryError<'gc>, TryGetValueContinue<'gc>> {
    let cache = cache.bind(gc);
    // 1. If V is not a Reference Record, return V.
    // Note: we never perform GetValue on Reference Records, as we know
    // statically if it's needed or not.
    match reference {
        // 2. If IsUnresolvableReference(V) is true, throw a ReferenceError exception.
        Reference::Unresolvable(referenced_name)
        | Reference::UnresolvableStrict(referenced_name) => {
            let error_message = format!(
                "Cannot access undeclared variable '{}'.",
                referenced_name.to_string_lossy_(agent)
            );
            agent
                .throw_exception(ExceptionType::ReferenceError, error_message, gc)
                .into()
        }
        // 3. If IsPropertyReference(V) is true, then
        Reference::PropertyExpression(_)
        | Reference::PropertyExpressionStrict(_)
        | Reference::SuperExpression(_)
        | Reference::SuperExpressionStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let this_value = get_this_value(reference).bind(gc);
            let referenced_name = reference.referenced_name_value().bind(gc);
            let base = reference.base_value().bind(gc);
            if base.is_undefined() || base.is_null() {
                return throw_read_undefined_or_null_error(agent, referenced_name, base, gc).into();
            }
            // c. If V.[[ReferencedName]] is not a property key, then
            // i. Set V.[[ReferencedName]] to ? ToPropertyKey(V.[[ReferencedName]]).
            let referenced_name = match to_property_key_simple(agent, referenced_name, gc) {
                Some(r) => r,
                None => return TryError::GcError.into(),
            };
            if let Ok(base_obj) = Object::try_from(base) {
                // d. Return ? baseObj.[[Get]](V.[[ReferencedName]], GetThisValue(V)).
                base_obj
                    .try_get(agent, referenced_name, this_value, cache, gc)
                    .map_continue(|c| {
                        TryGetValueContinue::from_get_continue(c, this_value, referenced_name)
                    })
            } else {
                // base is not an object; we handle primitives separately.
                try_handle_primitive_get_value(agent, referenced_name, base, cache, gc)
            }
        }
        Reference::Property(_)
        | Reference::PropertyStrict(_)
        | Reference::Super(_)
        | Reference::SuperStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base = reference.base_value().bind(gc);
            let base_obj = Object::try_from(base).bind(gc);
            let referenced_name = reference.referenced_name_property_key().bind(gc);
            // b. If IsPrivateReference(V) is true, then
            if let PropertyKey::PrivateName(referenced_name) = referenced_name {
                // i. Return ? PrivateGet(baseObj, V.[[ReferencedName]]).
                if let Ok(base_obj) = base_obj {
                    try_private_get(agent, base_obj, referenced_name, gc).map_continue(|c| {
                        TryGetValueContinue::from_get_continue(
                            c,
                            base_obj.into(),
                            referenced_name.into(),
                        )
                    })
                } else {
                    throw_no_private_name_error(agent, gc).into()
                }
            } else if let Ok(base_obj) = base_obj {
                // d. Return ? baseObj.[[Get]](V.[[ReferencedName]], GetThisValue(V)).
                let this_value = reference.this_value(agent).bind(gc);
                base_obj
                    .try_get(agent, referenced_name, this_value, cache, gc)
                    .map_continue(|c| {
                        TryGetValueContinue::from_get_continue(c, this_value, referenced_name)
                    })
            } else {
                // base is not an object; we handle primitives separately.
                try_handle_primitive_get_value(agent, referenced_name, base, cache, gc)
            }
        }
        Reference::Variable(v) | Reference::VariableStrict(v) => {
            // 4. Else,
            // a. Let base be V.[[Base]].
            let base = v.base.bind(gc);
            // b. Assert: base is an Environment Record.
            // c. Return ? base.GetBindingValue(V.[[ReferencedName]], V.[[Strict]]) (see 9.1).
            TryGetValueContinue::Value(base.try_get_binding_value(
                agent,
                v.referenced_name.bind(gc),
                v.cache.bind(gc),
                reference.strict(),
                gc,
            )?)
            .into()
        }
    }
}

/// ### [6.2.5.6 PutValue ( V, W )](https://tc39.es/ecma262/#sec-putvalue)
///
/// The abstract operation PutValue takes arguments V (a Reference Record or an
/// ECMAScript language value) and W (an ECMAScript language value) and returns
/// either a normal completion containing UNUSED or an abrupt completion.
pub(crate) fn put_value<'a>(
    agent: &mut Agent,
    reference: &Reference,
    w: Value,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let w = w.bind(gc.nogc());
    // 1. If V is not a Reference Record, throw a ReferenceError exception.
    match reference {
        // 2. If IsUnresolvableReference(V) is true, then
        Reference::Unresolvable(referenced_name) => {
            // b. Let globalObj be GetGlobalObject().
            let global_obj = get_global_object(agent, gc.nogc());
            // c. Perform ? Set(globalObj, V.[[ReferencedName]], W, false).
            set(
                agent,
                global_obj.unbind(),
                // Note: variable names cannot be numeric.
                referenced_name.to_property_key(),
                w.unbind(),
                false,
                gc,
            )?;
            // d. Return UNUSED.
            Ok(())
        }
        Reference::UnresolvableStrict(referenced_name) => {
            // a. If V.[[Strict]] is true, throw a ReferenceError exception.
            let error_message = format!(
                "Cannot assign to undeclared variable '{}'.",
                referenced_name.to_string_lossy_(agent)
            );
            Err(agent.throw_exception(ExceptionType::ReferenceError, error_message, gc.into_nogc()))
        }
        // 3. If IsPropertyReference(V) is true, then
        Reference::PropertyExpression(_)
        | Reference::PropertyExpressionStrict(_)
        | Reference::SuperExpression(_)
        | Reference::SuperExpressionStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base_obj = to_object(agent, reference.base_value(), gc.nogc())
                .unbind()?
                .scope(agent, gc.nogc());
            let this_value = get_this_value(reference).scope(agent, gc.nogc());
            let w = w.scope(agent, gc.nogc());
            let referenced_name = reference.referenced_name_value().bind(gc.nogc());
            // c. If V.[[ReferencedName]] is not a property key, then
            // i. Set V.[[ReferencedName]] to ? ToPropertyKey(V.[[ReferencedName]]).
            let referenced_name = to_property_key(agent, referenced_name.unbind(), gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            let scoped_referenced_name = if reference.strict() {
                Some(referenced_name.scope(agent, gc.nogc()))
            } else {
                None
            };
            // c. Let succeeded be ? baseObj.[[Set]](V.[[ReferencedName]], W, GetThisValue(V)).
            let succeeded = base_obj
                .get(agent)
                .internal_set(
                    agent,
                    referenced_name.unbind(),
                    // SAFETY: not shared.
                    unsafe { w.take(agent) },
                    // SAFETY: not shared.
                    unsafe { this_value.take(agent) },
                    gc.reborrow(),
                )
                .unbind()?;
            // d. If succeeded is false and V.[[Strict]] is true,
            if !succeeded && let Some(scoped_referenced_name) = scoped_referenced_name {
                // throw a TypeError exception.
                // SAFETY: not shared.
                let base_obj_repr = unsafe {
                    Value::from(base_obj.take(agent))
                        .string_repr(agent, gc.reborrow())
                        .unbind()
                        .bind(gc.nogc())
                };
                // SAFETY: not shared.
                let referenced_name = unsafe { scoped_referenced_name.take(agent) }.bind(gc.nogc());
                return Err(throw_cannot_set_property(
                    agent,
                    base_obj_repr.unbind().into(),
                    referenced_name.unbind(),
                    gc.into_nogc(),
                ));
            }
            if let Some(scoped_referenced_name) = scoped_referenced_name {
                // SAFETY: not shared.
                let _ = unsafe { scoped_referenced_name.take(agent) }.bind(gc.nogc());
            };
            // e. Return UNUSED.
            Ok(())
        }
        Reference::Property(_)
        | Reference::PropertyStrict(_)
        | Reference::Super(_)
        | Reference::SuperStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base_obj = to_object(agent, reference.base_value(), gc.nogc())
                .unbind()?
                .bind(gc.nogc());
            let referenced_name = reference.referenced_name_property_key().bind(gc.nogc());
            let this_value = get_this_value(reference).bind(gc.nogc());
            // b. If IsPrivateReference(V) is true, then
            if let PropertyKey::PrivateName(referenced_name) = referenced_name {
                // i. Return ? PrivateSet(baseObj, V.[[ReferencedName]], W).
                return private_set(
                    agent,
                    base_obj.unbind(),
                    referenced_name.unbind(),
                    w.unbind(),
                    gc,
                );
            }
            let scoped_strict_error_data = if reference.strict() {
                Some((
                    referenced_name.scope(agent, gc.nogc()),
                    base_obj.scope(agent, gc.nogc()),
                ))
            } else {
                None
            };
            // c. Let succeeded be ? baseObj.[[Set]](V.[[ReferencedName]], W, GetThisValue(V)).
            let succeeded = base_obj
                .unbind()
                .internal_set(
                    agent,
                    referenced_name.unbind(),
                    w.unbind(),
                    this_value.unbind(),
                    gc.reborrow(),
                )
                .unbind()?;
            if !succeeded
                && let Some((scoped_referenced_name, scoped_base_obj)) = scoped_strict_error_data
            {
                // d. If succeeded is false and V.[[Strict]] is true, throw a TypeError exception.
                // SAFETY: not shared.
                let base_obj_repr = unsafe {
                    Value::from(scoped_base_obj.take(agent))
                        .string_repr(agent, gc.reborrow())
                        .unbind()
                        .bind(gc.nogc())
                };
                // SAFETY: not shared.
                let referenced_name = unsafe { scoped_referenced_name.take(agent) }.bind(gc.nogc());
                return Err(throw_cannot_set_property(
                    agent,
                    base_obj_repr.unbind().into(),
                    referenced_name.unbind(),
                    gc.into_nogc(),
                ));
            }
            if let Some((scoped_referenced_name, scoped_base_obj)) = scoped_strict_error_data {
                // SAFETY: not shared.
                let _ = unsafe { scoped_base_obj.take(agent) }.bind(gc.nogc());
                // SAFETY: not shared.
                let _ = unsafe { scoped_referenced_name.take(agent) }.bind(gc.nogc());
            };
            // e. Return UNUSED.
            Ok(())
        }
        Reference::Variable(v) | Reference::VariableStrict(v) => {
            // 4. Else,
            // a. Let base be V.[[Base]].
            let base = v.base;
            // b. Assert: base is an Environment Record.
            // c. Return ? base.SetMutableBinding(V.[[ReferencedName]], W, V.[[Strict]]) (see 9.1).
            base.set_mutable_binding(
                agent,
                v.referenced_name,
                v.cache,
                w.unbind(),
                reference.strict(),
                gc,
            )
        }
    }
    // NOTE
    // The object that may be created in step 3.a is not accessible outside of
    // the above abstract operation and the ordinary object [[Set]] internal
    // method. An implementation might choose to avoid the actual creation of
    // that object.
}

/// ### [6.2.5.6 PutValue ( V, W )](https://tc39.es/ecma262/#sec-putvalue)
///
/// The abstract operation PutValue takes arguments V (a Reference Record or an
/// ECMAScript language value) and W (an ECMAScript language value) and returns
/// either a normal completion containing UNUSED or an abrupt completion.
pub(crate) fn try_put_value<'gc>(
    agent: &mut Agent,
    reference: &mut Reference,
    w: Value,
    cache: Option<PropertyLookupCache>,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    // 1. If V is not a Reference Record, throw a ReferenceError exception.
    match reference {
        // 2. If IsUnresolvableReference(V) is true, then
        Reference::Unresolvable(referenced_name) => {
            // b. Let globalObj be GetGlobalObject().
            let global_obj = get_global_object(agent, gc);
            // c. Perform ? Set(globalObj, V.[[ReferencedName]], W, false).
            try_set(
                agent,
                global_obj.unbind(),
                // Note: variable names cannot be numeric.
                referenced_name.to_property_key(),
                w.unbind(),
                false,
                cache,
                gc,
            )
            // d. Return UNUSED.
        }
        Reference::UnresolvableStrict(referenced_name) => {
            // a. If V.[[Strict]] is true, throw a ReferenceError exception.
            let error_message = format!(
                "Cannot assign to undeclared variable '{}'.",
                referenced_name.to_string_lossy_(agent)
            );
            agent
                .throw_exception(ExceptionType::ReferenceError, error_message, gc)
                .into()
        }
        // 3. If IsPropertyReference(V) is true, then
        Reference::PropertyExpression(_)
        | Reference::PropertyExpressionStrict(_)
        | Reference::SuperExpression(_)
        | Reference::SuperExpressionStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base_obj = js_result_into_try(to_object(agent, reference.base_value(), gc))?;
            let this_value = get_this_value(reference);
            let referenced_name = reference.referenced_name_value();
            // c. If V.[[ReferencedName]] is not a property key, then
            // i. Set V.[[ReferencedName]] to ? ToPropertyKey(V.[[ReferencedName]]).
            let referenced_name =
                option_into_try(to_property_key_simple(agent, referenced_name, gc))?;
            reference.set_referenced_name_to_property_key(referenced_name);
            // c. Let succeeded be ? baseObj.[[Set]](V.[[ReferencedName]], W, GetThisValue(V)).
            let result = base_obj.try_set(agent, referenced_name, w, this_value, cache, gc)?;
            // d. If succeeded is false and V.[[Strict]] is true,
            if result.failed() && reference.strict() {
                // throw a TypeError exception.
                return throw_cannot_set_property(agent, base_obj.into(), referenced_name, gc)
                    .into();
            }
            // e. Return UNUSED.
            result.into()
        }
        Reference::Property(_)
        | Reference::PropertyStrict(_)
        | Reference::Super(_)
        | Reference::SuperStrict(_) => {
            // a. Let baseObj be ? ToObject(V.[[Base]]).
            let base_obj = js_result_into_try(to_object(agent, reference.base_value(), gc))?;
            let referenced_name = reference.referenced_name_property_key();
            // b. If IsPrivateReference(V) is true, then
            if let PropertyKey::PrivateName(referenced_name) = referenced_name {
                // i. Return ? PrivateSet(baseObj, V.[[ReferencedName]], W).
                return try_private_set(agent, base_obj, referenced_name, w, gc);
            }
            // c. Let succeeded be ? baseObj.[[Set]](V.[[ReferencedName]], W, GetThisValue(V)).
            let result = base_obj.try_set(
                agent,
                referenced_name,
                w,
                get_this_value(reference),
                None,
                gc,
            )?;
            if result.failed() && reference.strict() {
                // d. If succeeded is false and V.[[Strict]] is true, throw a TypeError exception.
                return throw_cannot_set_property(agent, base_obj.into(), referenced_name, gc)
                    .into();
            }
            // e. Return UNUSED.
            result.into()
        }
        Reference::Variable(v) | Reference::VariableStrict(v) => {
            // 4. Else,
            // a. Let base be V.[[Base]].
            let base = v.base;
            // b. Assert: base is an Environment Record.
            // c. Return ? base.SetMutableBinding(V.[[ReferencedName]], W, V.[[Strict]]) (see 9.1).
            base.try_set_mutable_binding(
                agent,
                v.referenced_name,
                v.cache,
                w,
                reference.strict(),
                gc,
            )
        }
    }
}

pub(crate) fn throw_cannot_set_property<'a>(
    agent: &mut Agent,
    base: Value,
    property_key: PropertyKey,
    gc: NoGcScope<'a, '_>,
) -> JsError<'a> {
    let base = base.try_string_repr(agent, gc);
    let error_message = format!(
        "Could not set property '{}' of {}.",
        property_key.as_display(agent),
        base.to_string_lossy_(agent)
    );
    agent.throw_exception(ExceptionType::TypeError, error_message, gc)
}

/// ### [6.2.5.7 GetThisValue ( V )](https://tc39.es/ecma262/#sec-getthisvalue)
///
/// The abstract operation GetThisValue takes argument V (a Reference Record)
/// and returns an ECMAScript language value.
pub(crate) fn get_this_value<'a>(v: &Reference<'a>) -> Value<'a> {
    // 1. Assert: IsPropertyReference(V) is true.
    debug_assert!(is_property_reference(v));
    // 2. If IsSuperReference(V) is true, return V.[[ThisValue]]; otherwise return V.[[Base]].
    match v {
        Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => v.this_value,
        Reference::Super(v) | Reference::SuperStrict(v) => v.this_value,
        Reference::PropertyExpression(v) | Reference::PropertyExpressionStrict(v) => v.base,
        Reference::Property(v) | Reference::PropertyStrict(v) => v.base,
        _ => unreachable!(),
    }
}

fn get_maybe_this_value<'a>(v: &Reference<'a>) -> Option<Value<'a>> {
    // 1. Assert: IsPropertyReference(V) is true.
    debug_assert!(is_property_reference(v));
    // 2. If IsSuperReference(V) is true, return V.[[ThisValue]]; otherwise return V.[[Base]].
    match v {
        Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => Some(v.this_value),
        Reference::Super(v) | Reference::SuperStrict(v) => Some(v.this_value),
        Reference::PropertyExpression(_) | Reference::PropertyExpressionStrict(_) => None,
        Reference::Property(_) | Reference::PropertyStrict(_) => None,
        _ => unreachable!(),
    }
}

/// ### {6.2.5.8 InitializeReferencedBinding ( V, W )}(https://tc39.es/ecma262/#sec-initializereferencedbinding)
/// The abstract operation InitializeReferencedBinding takes arguments V (a Reference Record) and W
/// (an ECMAScript language value) and returns either a normal completion containing unused or an
/// abrupt completion.
pub(crate) fn initialize_referenced_binding<'a>(
    agent: &mut Agent,
    v: Reference,
    w: Value,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    // 1. Assert: IsUnresolvableReference(V) is false.
    // 2. Let base be V.[[Base]].
    // 3. Assert: base is an Environment Record.
    match v {
        Reference::Variable(v) | Reference::VariableStrict(v) => {
            // 4. Return ? base.InitializeBinding(V.[[ReferencedName]], W).
            v.base
                .initialize_binding(agent, v.referenced_name, v.cache, w, gc)
        }
        _ => unreachable!(),
    }
}

/// ### [6.2.5.8 InitializeReferencedBinding ( V, W )](https://tc39.es/ecma262/#sec-initializereferencedbinding)
///
/// The abstract operation InitializeReferencedBinding takes arguments V (a
/// Reference Record) and W (an ECMAScript language value) and returns either a
/// normal completion containing unused or an abrupt completion.
pub(crate) fn try_initialize_referenced_binding<'gc>(
    agent: &mut Agent,
    v: Reference,
    w: Value,
    gc: NoGcScope<'gc, '_>,
) -> TryResult<'gc, SetResult<'gc>> {
    // 1. Assert: IsUnresolvableReference(V) is false.
    // 2. Let base be V.[[Base]].
    // 3. Assert: base is an Environment Record.
    match v {
        Reference::Variable(v) | Reference::VariableStrict(v) => {
            // 4. Return ? base.InitializeBinding(V.[[ReferencedName]], W).
            v.base
                .try_initialize_binding(agent, v.referenced_name, v.cache, w, gc)
        }
        _ => unreachable!(),
    }
}

impl HeapMarkAndSweep for Reference<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Reference::Unresolvable(v) | Reference::UnresolvableStrict(v) => v.mark_values(queues),
            Reference::Variable(v) | Reference::VariableStrict(v) => v.mark_values(queues),
            Reference::PropertyExpression(v) | Reference::PropertyExpressionStrict(v) => {
                v.mark_values(queues)
            }
            Reference::Property(v) | Reference::PropertyStrict(v) => v.mark_values(queues),
            Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => {
                v.mark_values(queues)
            }
            Reference::Super(v) | Reference::SuperStrict(v) => v.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Reference::Unresolvable(v) | Reference::UnresolvableStrict(v) => {
                v.sweep_values(compactions)
            }
            Reference::Variable(v) | Reference::VariableStrict(v) => v.sweep_values(compactions),
            Reference::PropertyExpression(v) | Reference::PropertyExpressionStrict(v) => {
                v.sweep_values(compactions)
            }
            Reference::Property(v) | Reference::PropertyStrict(v) => v.sweep_values(compactions),
            Reference::SuperExpression(v) | Reference::SuperExpressionStrict(v) => {
                v.sweep_values(compactions)
            }
            Reference::Super(v) | Reference::SuperStrict(v) => v.sweep_values(compactions),
        }
    }
}

impl HeapMarkAndSweep for VariableReference<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            base,
            referenced_name,
            cache,
        } = self;
        base.mark_values(queues);
        referenced_name.mark_values(queues);
        cache.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            base,
            referenced_name,
            cache,
        } = self;
        base.sweep_values(compactions);
        referenced_name.sweep_values(compactions);
        cache.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for PropertyReference<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            base,
            referenced_name,
        } = self;
        base.mark_values(queues);
        referenced_name.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            base,
            referenced_name,
        } = self;
        base.sweep_values(compactions);
        referenced_name.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for PropertyExpressionReference<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            base,
            referenced_name,
        } = self;
        base.mark_values(queues);
        referenced_name.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            base,
            referenced_name,
        } = self;
        base.sweep_values(compactions);
        referenced_name.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for SuperReference<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            base,
            referenced_name,
            this_value,
        } = self;
        base.mark_values(queues);
        referenced_name.mark_values(queues);
        this_value.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            base,
            referenced_name,
            this_value,
        } = self;
        base.sweep_values(compactions);
        referenced_name.sweep_values(compactions);
        this_value.sweep_values(compactions);
    }
}

impl HeapMarkAndSweep for SuperExpressionReference<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            base,
            referenced_name,
            this_value,
        } = self;
        base.mark_values(queues);
        referenced_name.mark_values(queues);
        this_value.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            base,
            referenced_name,
            this_value,
        } = self;
        base.sweep_values(compactions);
        referenced_name.sweep_values(compactions);
        this_value.sweep_values(compactions);
    }
}
