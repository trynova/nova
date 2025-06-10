// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ### [9.1 Environment Records](https://tc39.es/ecma262/#sec-environment-records)
//!
//! Environment Record is a specification type used to define the association of
//! Identifiers to specific variables and functions, based upon the lexical
//! nesting structure of ECMAScript code. Usually an Environment Record is
//! associated with some specific syntactic structure of ECMAScript code such as
//! a FunctionDeclaration, a BlockStatement, or a Catch clause of a
//! TryStatement. Each time such code is evaluated, a new Environment Record is
//! created to record the identifier bindings that are created by that code.
//!
//! Every Environment Record has an \[\[OuterEnv\]\] field, which is either null or
//! a reference to an outer Environment Record. This is used to model the
//! logical nesting of Environment Record values. The outer reference of an
//! (inner) Environment Record is a reference to the Environment Record that
//! logically surrounds the inner Environment Record. An outer Environment
//! Record may, of course, have its own outer Environment Record. An Environment
//! Record may serve as the outer environment for multiple inner Environment
//! Records. For example, if a FunctionDeclaration contains two nested
//! FunctionDeclarations then the Environment Records of each of the nested
//! functions will have as their outer Environment Record the Environment Record
//! of the current evaluation of the surrounding function.

use core::{marker::PhantomData, num::NonZeroU32};

mod declarative_environment;
mod function_environment;
mod global_environment;
mod module_environment;
mod object_environment;
mod private_environment;

pub(crate) use declarative_environment::{
    DeclarativeEnvironmentRecord, new_declarative_environment,
};
pub(crate) use function_environment::{
    FunctionEnvironmentRecord, ThisBindingStatus, new_class_field_initializer_environment,
    new_class_static_element_environment, new_function_environment,
};
pub(crate) use global_environment::{GlobalEnvironmentRecord, new_global_environment};
use module_environment::ModuleEnvironmentRecord;
pub(crate) use module_environment::{
    create_import_binding, initialize_import_binding, new_module_environment,
    throw_uninitialized_binding,
};
pub(crate) use object_environment::ObjectEnvironmentRecord;
pub(crate) use private_environment::{
    PrivateEnvironmentRecord, PrivateField, PrivateMethod, new_private_environment,
    resolve_private_identifier,
};

use crate::ecmascript::types::IntoValue;
use crate::engine::TryResult;
use crate::engine::context::{Bindable, GcScope, GcToken, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable, Scopable};
use crate::heap::Heap;
use crate::{
    ecmascript::types::{Base, Object, Reference, String, Value},
    heap::{CompactionLists, HeapMarkAndSweep, WorkQueues},
};

use super::{Agent, JsResult};

/// ### [\[\[OuterEnv\]\]](https://tc39.es/ecma262/#sec-environment-records)
///
/// Every Environment Record has an \[\[OuterEnv\]\] field, which is either
/// null or a reference to an outer Environment Record. This is used to model
/// the logical nesting of Environment Record values. The outer reference of an
/// (inner) Environment Record is a reference to the Environment Record that
/// logically surrounds the inner Environment Record. An outer Environment
/// Record may, of course, have its own outer Environment Record. An
/// Environment Record may serve as the outer environment for multiple inner
/// Environment Records. For example, if a FunctionDeclaration contains two
/// nested FunctionDeclarations then the Environment Records of each of the
/// nested functions will have as their outer Environment Record the
/// Environment Record of the current evaluation of the surrounding function.
pub(super) type OuterEnv<'a> = Option<Environment<'a>>;

macro_rules! create_environment_index {
    ($record: ident, $index: ident, $entry: ident) => {
        /// An index used to access an environment from [`Environments`].
        /// Internally, we store the index in a [`NonZeroU32`] with the index
        /// plus one. This allows us to not use an empty value in storage for
        /// the zero index while still saving room for a [`None`] value when
        /// stored in an [`Option`].
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $index<'a>(NonZeroU32, PhantomData<$record>, PhantomData<&'a GcToken>);

        impl core::fmt::Debug for $index<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "$index({:?})", self.into_u32_index())
            }
        }

        impl $index<'_> {
            /// Creates a new index from a u32.
            ///
            /// ## Panics
            /// - If the value is equal to 0.
            pub(crate) const fn from_u32(value: u32) -> Self {
                assert!(value != 0);
                // SAFETY: Number is not 0 and will not overflow to zero.
                // This check is done manually to allow const context.
                Self(
                    unsafe { NonZeroU32::new_unchecked(value) },
                    PhantomData,
                    PhantomData,
                )
            }

            pub(crate) const fn from_u32_index(value: u32) -> Self {
                // SAFETY: Number is not 0 and will not overflow to zero.
                // This check is done manually to allow const context.
                Self(
                    unsafe { NonZeroU32::new_unchecked(value + 1) },
                    PhantomData,
                    PhantomData,
                )
            }

            pub(crate) const fn into_index(self) -> usize {
                self.0.get() as usize - 1
            }

            pub(crate) const fn into_u32(self) -> u32 {
                self.0.get()
            }

            pub(crate) const fn into_u32_index(self) -> u32 {
                self.0.get() - 1
            }

            pub(crate) fn last(vec: &[Option<$record>]) -> Self {
                Self::from_u32(vec.len() as u32)
            }
        }

        // SAFETY: Property implemented as a lifetime transmute.
        unsafe impl Bindable for $index<'_> {
            type Of<'a> = $index<'a>;

            #[inline(always)]
            fn unbind(self) -> Self::Of<'static> {
                unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
            }

            #[inline(always)]
            fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
                unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
            }
        }

        impl Rootable for $index<'_> {
            type RootRepr = HeapRootRef;

            fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
                Err(HeapRootData::$index(value.unbind()))
            }

            fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
                Err(*value)
            }

            fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
                heap_ref
            }

            fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
                match heap_data {
                    HeapRootData::$index(object) => Some(object),
                    _ => None,
                }
            }
        }

        impl core::ops::Index<$index<'_>> for Agent {
            type Output = $record;

            fn index(&self, index: $index) -> &Self::Output {
                &self.heap.environments.$entry[index]
            }
        }

        impl core::ops::IndexMut<$index<'_>> for Agent {
            fn index_mut(&mut self, index: $index) -> &mut Self::Output {
                &mut self.heap.environments.$entry[index]
            }
        }

        impl core::ops::Index<$index<'_>> for Vec<Option<$record>> {
            type Output = $record;

            fn index(&self, index: $index) -> &Self::Output {
                self.get(index.into_index())
                    .expect("Environment out of bounds")
                    .as_ref()
                    .expect("Environment slot empty")
            }
        }

        impl core::ops::IndexMut<$index<'_>> for Vec<Option<$record>> {
            fn index_mut(&mut self, index: $index) -> &mut Self::Output {
                self.get_mut(index.into_index())
                    .expect("Environment out of bounds")
                    .as_mut()
                    .expect("Environment slot empty")
            }
        }
    };
}

create_environment_index!(
    DeclarativeEnvironmentRecord,
    DeclarativeEnvironment,
    declarative
);
create_environment_index!(FunctionEnvironmentRecord, FunctionEnvironment, function);
create_environment_index!(GlobalEnvironmentRecord, GlobalEnvironment, global);
create_environment_index!(ObjectEnvironmentRecord, ObjectEnvironment, object);
create_environment_index!(ModuleEnvironmentRecord, ModuleEnvironment, module);
create_environment_index!(PrivateEnvironmentRecord, PrivateEnvironment, private);

impl<'a> From<DeclarativeEnvironment<'a>> for Environment<'a> {
    fn from(value: DeclarativeEnvironment<'a>) -> Self {
        Environment::Declarative(value)
    }
}

impl<'a> From<GlobalEnvironment<'a>> for Environment<'a> {
    fn from(value: GlobalEnvironment<'a>) -> Self {
        Environment::Global(value)
    }
}

impl<'a> From<ModuleEnvironment<'a>> for Environment<'a> {
    fn from(value: ModuleEnvironment<'a>) -> Self {
        Environment::Module(value)
    }
}

impl<'a> From<ObjectEnvironment<'a>> for Environment<'a> {
    fn from(value: ObjectEnvironment<'a>) -> Self {
        Environment::Object(value)
    }
}

/// ### [9.1.1 The Environment Record Type Hierarchy](https://tc39.es/ecma262/#sec-the-environment-record-type-hierarchy)
///
/// Environment Records can be thought of as existing in a simple
/// object-oriented hierarchy where Environment Record is an abstract class
/// with three concrete subclasses: Declarative Environment Record, Object
/// Environment Record, and Global Environment Record. Function Environment
/// Records and Module Environment Records are subclasses of Declarative
/// Environment Record.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Environment<'a> {
    // Leave 0 for None option
    Declarative(DeclarativeEnvironment<'a>) = 1,
    Function(FunctionEnvironment<'a>),
    Global(GlobalEnvironment<'a>),
    Module(ModuleEnvironment<'a>),
    Object(ObjectEnvironment<'a>),
}

impl Environment<'_> {
    pub(crate) fn get_outer_env<'a>(self, agent: &Agent, gc: NoGcScope<'a, '_>) -> OuterEnv<'a> {
        match self {
            Environment::Declarative(e) => e.get_outer_env(agent, gc),
            Environment::Function(e) => e.get_outer_env(agent, gc),
            Environment::Global(_) => None,
            Environment::Module(e) => e.get_outer_env(agent, gc),
            Environment::Object(e) => e.get_outer_env(agent, gc),
        }
    }

    /// ### Try [HasBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record has a binding for the String value
    /// N. Return true if it does and false if it does not.
    pub(crate) fn try_has_binding(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope,
    ) -> TryResult<bool> {
        match self {
            Environment::Declarative(e) => TryResult::Continue(e.has_binding(agent, name)),
            Environment::Function(e) => TryResult::Continue(e.has_binding(agent, name)),
            Environment::Global(e) => e.try_has_binding(agent, name, gc),
            Environment::Module(e) => TryResult::Continue(e.has_binding(agent, name)),
            Environment::Object(e) => e.try_has_binding(agent, name, gc),
        }
    }

    /// ### [HasBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record has a binding for the String value
    /// N. Return true if it does and false if it does not.
    pub(crate) fn has_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        match self {
            Environment::Declarative(e) => Ok(e.has_binding(agent, name)),
            Environment::Function(e) => Ok(e.has_binding(agent, name)),
            Environment::Global(e) => e.has_binding(agent, name, gc),
            Environment::Module(e) => Ok(e.has_binding(agent, name)),
            Environment::Object(e) => e.has_binding(agent, name, gc),
        }
    }

    /// ### Try [CreateMutableBinding(N, D)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If the
    /// Boolean argument D is true the binding may be subsequently deleted.
    pub(crate) fn try_create_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, ()>> {
        match self {
            Environment::Declarative(e) => {
                e.create_mutable_binding(agent, name, is_deletable);
                TryResult::Continue(Ok(()))
            }
            Environment::Function(e) => {
                e.create_mutable_binding(agent, name, is_deletable);
                TryResult::Continue(Ok(()))
            }
            Environment::Global(e) => {
                TryResult::Continue(e.create_mutable_binding(agent, name, is_deletable, gc))
            }
            Environment::Module(e) => {
                e.create_mutable_binding(agent, name, is_deletable);
                TryResult::Continue(Ok(()))
            }
            Environment::Object(e) => e.try_create_mutable_binding(agent, name, is_deletable, gc),
        }
    }

    /// ### [CreateMutableBinding(N, D)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If the
    /// Boolean argument D is true the binding may be subsequently deleted.
    pub(crate) fn create_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        match self {
            Environment::Declarative(e) => {
                e.create_mutable_binding(agent, name, is_deletable);
                Ok(())
            }
            Environment::Function(e) => {
                e.create_mutable_binding(agent, name, is_deletable);
                Ok(())
            }
            Environment::Global(e) => {
                e.create_mutable_binding(agent, name, is_deletable, gc.into_nogc())
            }
            Environment::Module(e) => {
                e.create_mutable_binding(agent, name, is_deletable);
                Ok(())
            }
            Environment::Object(e) => e.create_mutable_binding(agent, name, is_deletable, gc),
        }
    }

    /// ### [CreateImmutableBinding(N, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized immutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If S is true
    /// then attempts to set it after it has been initialized will always throw
    /// an exception, regardless of the strict mode setting of operations that
    /// reference that binding.
    pub(crate) fn create_immutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        match self {
            Environment::Declarative(e) => {
                e.create_immutable_binding(agent, name, is_strict);
                Ok(())
            }
            Environment::Function(e) => {
                e.create_immutable_binding(agent, name, is_strict);
                Ok(())
            }
            Environment::Global(e) => e.create_immutable_binding(agent, name, is_strict, gc),
            Environment::Module(e) => {
                debug_assert!(is_strict);
                e.create_immutable_binding(agent.as_mut(), name);
                Ok(())
            }
            Environment::Object(e) => {
                e.create_immutable_binding(agent, name, is_strict);
                Ok(())
            }
        }
    }

    /// ### Try [InitializeBinding(N, V)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing but uninitialized binding in an
    /// Environment Record. The String value N is the text of the bound name.
    /// V is the value for the binding and is a value of any ECMAScript
    /// language type.
    pub(crate) fn try_initialize_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, ()>> {
        match self {
            Environment::Declarative(e) => {
                e.initialize_binding(agent, name, value);
                TryResult::Continue(Ok(()))
            }
            Environment::Function(e) => {
                e.initialize_binding(agent, name, value);
                TryResult::Continue(Ok(()))
            }
            Environment::Global(e) => e.try_initialize_binding(agent, name, value, gc),
            Environment::Module(e) => {
                e.initialize_binding(agent.as_mut(), name, value);
                TryResult::Continue(Ok(()))
            }
            Environment::Object(e) => e.try_initialize_binding(agent, name, value, gc),
        }
    }

    /// ### [InitializeBinding(N, V)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing but uninitialized binding in an
    /// Environment Record. The String value N is the text of the bound name.
    /// V is the value for the binding and is a value of any ECMAScript
    /// language type.
    pub(crate) fn initialize_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        match self {
            Environment::Declarative(e) => {
                e.initialize_binding(agent, name, value);
                Ok(())
            }
            Environment::Function(e) => {
                e.initialize_binding(agent, name, value);
                Ok(())
            }
            Environment::Global(e) => e.initialize_binding(agent, name, value, gc),
            Environment::Module(e) => {
                e.initialize_binding(agent.as_mut(), name, value);
                Ok(())
            }
            Environment::Object(e) => e.initialize_binding(agent, name, value, gc),
        }
    }

    /// ### Try [SetMutableBinding(N, V, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. V is the
    /// value for the binding and may be a value of any ECMAScript language
    /// type. S is a Boolean flag. If S is true and the binding cannot be set
    /// throw a TypeError exception.
    pub(crate) fn try_set_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        is_strict: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, ()>> {
        match self {
            Environment::Declarative(e) => {
                TryResult::Continue(e.set_mutable_binding(agent, name, value, is_strict, gc))
            }
            Environment::Function(e) => {
                TryResult::Continue(e.set_mutable_binding(agent, name, value, is_strict, gc))
            }
            Environment::Global(e) => e.try_set_mutable_binding(agent, name, value, is_strict, gc),
            Environment::Module(e) => {
                debug_assert!(is_strict);
                TryResult::Continue(e.set_mutable_binding(agent, name, value, gc))
            }
            Environment::Object(e) => e.try_set_mutable_binding(agent, name, value, is_strict, gc),
        }
    }

    /// ### [SetMutableBinding(N, V, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. V is the
    /// value for the binding and may be a value of any ECMAScript language
    /// type. S is a Boolean flag. If S is true and the binding cannot be set
    /// throw a TypeError exception.
    pub(crate) fn set_mutable_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        is_strict: bool,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, ()> {
        match self {
            Environment::Declarative(e) => {
                e.set_mutable_binding(agent, name, value, is_strict, gc.into_nogc())
            }
            Environment::Function(e) => {
                e.set_mutable_binding(agent, name, value, is_strict, gc.into_nogc())
            }
            Environment::Global(e) => e.set_mutable_binding(agent, name, value, is_strict, gc),
            Environment::Module(e) => e.set_mutable_binding(agent, name, value, gc.into_nogc()),
            Environment::Object(e) => e.set_mutable_binding(agent, name, value, is_strict, gc),
        }
    }

    /// ### Try [GetBindingValue(N, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Returns the value of an already existing binding from an Environment
    /// Record. The String value N is the text of the bound name. S is used to
    /// identify references originating in strict mode code or that otherwise
    /// require strict mode reference semantics. If S is true and the binding
    /// does not exist throw a ReferenceError exception. If the binding exists
    /// but is uninitialized a ReferenceError is thrown, regardless of the
    /// value of S.
    pub(crate) fn try_get_binding_value<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, Value<'a>>> {
        match self {
            Environment::Declarative(e) => {
                TryResult::Continue(e.get_binding_value(agent, name, is_strict, gc))
            }
            Environment::Function(e) => {
                TryResult::Continue(e.get_binding_value(agent, name, is_strict, gc))
            }
            Environment::Global(e) => e.try_get_binding_value(agent, name, is_strict, gc),
            Environment::Module(e) => {
                let Heap {
                    environments,
                    source_text_module_records,
                    ..
                } = &agent.heap;
                let Some(value) = e.get_binding_value(
                    environments,
                    source_text_module_records,
                    name,
                    is_strict,
                    gc,
                ) else {
                    return TryResult::Continue(Err(throw_uninitialized_binding(agent, name, gc)));
                };
                TryResult::Continue(Ok(value))
            }
            Environment::Object(e) => e.try_get_binding_value(agent, name, is_strict, gc),
        }
    }

    /// ### [GetBindingValue(N, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Returns the value of an already existing binding from an Environment
    /// Record. The String value N is the text of the bound name. S is used to
    /// identify references originating in strict mode code or that otherwise
    /// require strict mode reference semantics. If S is true and the binding
    /// does not exist throw a ReferenceError exception. If the binding exists
    /// but is uninitialized a ReferenceError is thrown, regardless of the
    /// value of S.
    pub(crate) fn get_binding_value<'a>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        match self {
            Environment::Declarative(e) => {
                e.get_binding_value(agent, name, is_strict, gc.into_nogc())
            }
            Environment::Function(e) => e.get_binding_value(agent, name, is_strict, gc.into_nogc()),
            Environment::Global(e) => e.get_binding_value(agent, name, is_strict, gc),
            Environment::Module(e) => {
                let gc = gc.into_nogc();
                let Heap {
                    environments,
                    source_text_module_records,
                    ..
                } = &agent.heap;
                let Some(value) = e.get_binding_value(
                    environments,
                    source_text_module_records,
                    name,
                    is_strict,
                    gc,
                ) else {
                    return Err(throw_uninitialized_binding(agent, name, gc));
                };
                Ok(value)
            }
            Environment::Object(e) => e.get_binding_value(agent, name, is_strict, gc),
        }
    }

    /// ### Try [DeleteBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Delete a binding from an Environment Record. The String value N is the
    /// text of the bound name. If a binding for N exists, remove the binding
    /// and return true. If the binding exists but cannot be removed return
    /// false.
    pub(crate) fn try_delete_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<JsResult<'a, bool>> {
        match self {
            Environment::Declarative(e) => TryResult::Continue(Ok(e.delete_binding(agent, name))),
            Environment::Function(e) => TryResult::Continue(Ok(e.delete_binding(agent, name))),
            Environment::Global(e) => e.try_delete_binding(agent, name, gc),
            // NOTE: Module Environment Records are only used within strict
            // code and an early error rule prevents the delete operator, in
            // strict code, from being applied to a Reference Record that would
            // resolve to a Module Environment Record binding. See 13.5.1.1.
            Environment::Module(_) => unreachable!(),
            Environment::Object(e) => {
                TryResult::Continue(Ok(e.try_delete_binding(agent, name, gc)?))
            }
        }
    }

    /// ### [DeleteBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Delete a binding from an Environment Record. The String value N is the
    /// text of the bound name. If a binding for N exists, remove the binding
    /// and return true. If the binding exists but cannot be removed return
    /// false.
    pub(crate) fn delete_binding<'a>(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope<'a, '_>,
    ) -> JsResult<'a, bool> {
        match self {
            Environment::Declarative(e) => Ok(e.delete_binding(agent, name)),
            Environment::Function(e) => Ok(e.delete_binding(agent, name)),
            Environment::Global(e) => e.delete_binding(agent, name, gc),
            // NOTE: Module Environment Records are only used within strict
            // code and an early error rule prevents the delete operator, in
            // strict code, from being applied to a Reference Record that would
            // resolve to a Module Environment Record binding. See 13.5.1.1.
            Environment::Module(_) => unreachable!(),
            Environment::Object(e) => e.delete_binding(agent, name, gc),
        }
    }

    /// ### [HasThisBinding()](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record establishes a this binding. Return
    /// true if it does and false if it does not.
    pub(crate) fn has_this_binding(self, agent: &Agent) -> bool {
        match self {
            Environment::Declarative(_) => false,
            Environment::Function(e) => e.has_this_binding(agent),
            Environment::Global(_) => true,
            Environment::Module(_) => true,
            Environment::Object(_) => false,
        }
    }

    /// Get the `this` binding value of this environment.
    ///
    /// ## Panics
    ///
    /// Panics if the environment does not have a `this` binding.
    pub(crate) fn get_this_binding<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> JsResult<'a, Value<'a>> {
        match self {
            Environment::Function(e) => e.get_this_binding(agent, gc),
            Environment::Global(e) => Ok(e.get_this_binding(agent, gc).into_value()),
            Environment::Module(_) => Ok(Value::Undefined),
            _ => unreachable!(),
        }
    }

    /// ### [HasSuperBinding()](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record establishes a super method binding.
    /// Return true if it does and false if it does not.
    pub(crate) fn has_super_binding(self, agent: &mut Agent) -> bool {
        match self {
            Environment::Function(e) => e.has_super_binding(agent),
            _ => false,
        }
    }

    /// ### [WithBaseObject()](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// If this Environment Record is associated with a with statement, return
    /// the with object. Otherwise, return undefined.
    pub(crate) fn with_base_object(self, agent: &mut Agent) -> Option<Object> {
        match self {
            Environment::Object(e) => e.with_base_object(agent),
            _ => None,
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for Environment<'_> {
    type Of<'a> = Environment<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl core::fmt::Debug for Environment<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Environment::Declarative(d) => {
                write!(f, "DeclarativeEnvironment({:?})", d.into_u32_index())
            }
            Environment::Function(d) => write!(f, "FunctionEnvironment({:?})", d.into_u32_index()),
            Environment::Global(d) => write!(f, "GlobalEnvironment({:?})", d.into_u32_index()),
            Environment::Module(d) => write!(f, "ModuleEnvironment({:?})", d.into_u32_index()),
            Environment::Object(d) => write!(f, "ObjectEnvironment({:?})", d.into_u32_index()),
            // EnvironmentIndex::Module(d) => {}
        }
    }
}

impl Rootable for Environment<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            Environment::Declarative(e) => Err(HeapRootData::DeclarativeEnvironment(e.unbind())),
            Environment::Function(e) => Err(HeapRootData::FunctionEnvironment(e.unbind())),
            Environment::Global(e) => Err(HeapRootData::GlobalEnvironment(e.unbind())),
            Environment::Module(e) => Err(HeapRootData::ModuleEnvironment(e.unbind())),
            Environment::Object(e) => Err(HeapRootData::ObjectEnvironment(e.unbind())),
        }
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::DeclarativeEnvironment(e) => Some(Environment::Declarative(e)),
            HeapRootData::FunctionEnvironment(e) => Some(Environment::Function(e)),
            HeapRootData::GlobalEnvironment(e) => Some(Environment::Global(e)),
            HeapRootData::ModuleEnvironment(e) => Some(Environment::Module(e)),
            HeapRootData::ObjectEnvironment(e) => Some(Environment::Object(e)),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for Environment<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            Environment::Declarative(e) => e.mark_values(queues),
            Environment::Function(e) => e.mark_values(queues),
            Environment::Global(e) => e.mark_values(queues),
            Environment::Module(e) => e.mark_values(queues),
            Environment::Object(e) => e.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            Environment::Declarative(e) => e.sweep_values(compactions),
            Environment::Function(e) => e.sweep_values(compactions),
            Environment::Global(e) => e.sweep_values(compactions),
            Environment::Module(e) => e.sweep_values(compactions),
            Environment::Object(e) => e.sweep_values(compactions),
        }
    }
}

#[derive(Debug)]
pub struct Environments {
    pub(crate) declarative: Vec<Option<DeclarativeEnvironmentRecord>>,
    pub(crate) function: Vec<Option<FunctionEnvironmentRecord>>,
    pub(crate) global: Vec<Option<GlobalEnvironmentRecord>>,
    pub(crate) object: Vec<Option<ObjectEnvironmentRecord>>,
    pub(crate) module: Vec<Option<ModuleEnvironmentRecord>>,
    pub(crate) private: Vec<Option<PrivateEnvironmentRecord>>,
}

impl Default for Environments {
    fn default() -> Self {
        Self {
            declarative: Vec::with_capacity(256),
            function: Vec::with_capacity(1024),
            global: Vec::with_capacity(1),
            object: Vec::with_capacity(1024),
            module: Vec::with_capacity(8),
            private: Vec::with_capacity(0),
        }
    }
}

/// ### Try [9.1.2.1 GetIdentifierReference ( env, name, strict )](https://tc39.es/ecma262/#sec-getidentifierreference)
///
/// The abstract operation GetIdentifierReference takes arguments env (an
/// Environment Record or null), name (a String), and strict (a Boolean) and
/// returns either a normal completion containing a Reference Record or a throw
/// completion.
pub(crate) fn try_get_identifier_reference<'a>(
    agent: &mut Agent,
    env: Environment,
    name: String,
    strict: bool,
    gc: NoGcScope<'a, '_>,
) -> TryResult<Reference<'a>> {
    let env = env.bind(gc);
    let name = name.bind(gc);
    // 1. If env is null, then
    // 2. Let exists be ? env.HasBinding(name).
    let exists = env.try_has_binding(agent, name, gc)?;

    // 3. If exists is true, then
    if exists {
        // a. Return the Reference Record {
        TryResult::Continue(Reference {
            // [[Base]]: env,
            base: Base::Environment(env.unbind()),
            // [[ReferencedName]]: name,
            referenced_name: name.into(),
            // [[Strict]]: strict,
            strict,
            // [[ThisValue]]: EMPTY
            this_value: None,
        })
        // }.
    }
    // 4. Else,
    else {
        // a. Let outer be env.[[OuterEnv]].
        let outer = env.get_outer_env(agent, gc);

        let Some(outer) = outer else {
            // a. Return the Reference Record {
            return TryResult::Continue(Reference {
                // [[Base]]: UNRESOLVABLE,
                base: Base::Unresolvable,
                // [[ReferencedName]]: name,
                referenced_name: name.into(),
                // [[Strict]]: strict,
                strict,
                // [[ThisValue]]: EMPTY
                this_value: None,
            });
            // }.
        };

        // b. Return ? GetIdentifierReference(outer, name, strict).
        try_get_identifier_reference(agent, outer, name, strict, gc)
    }
}

/// ### [9.1.2.1 GetIdentifierReference ( env, name, strict )](https://tc39.es/ecma262/#sec-getidentifierreference)
///
/// The abstract operation GetIdentifierReference takes arguments env (an
/// Environment Record or null), name (a String), and strict (a Boolean) and
/// returns either a normal completion containing a Reference Record or a throw
/// completion.
pub(crate) fn get_identifier_reference<'a, 'b>(
    agent: &mut Agent,
    env: Option<Environment>,
    name: String<'b>,
    strict: bool,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<'a, Reference<'a>> {
    let env = env.bind(gc.nogc());
    let mut name = name.bind(gc.nogc());

    // 1. If env is null, then
    let Some(mut env) = env else {
        let name = name.unbind().bind(gc.into_nogc());
        // a. Return the Reference Record {
        return Ok(Reference {
            // [[Base]]: UNRESOLVABLE,
            base: Base::Unresolvable,
            // [[ReferencedName]]: name,
            referenced_name: name.into(),
            // [[Strict]]: strict,
            strict,
            // [[ThisValue]]: EMPTY
            this_value: None,
        });
        // }.
    };

    // 2. Let exists be ? env.HasBinding(name).
    let exists = if let TryResult::Continue(result) = env.try_has_binding(agent, name, gc.nogc()) {
        result
    } else {
        let env_scoped = env.scope(agent, gc.nogc());
        let name_scoped = name.scope(agent, gc.nogc());
        let result = env
            .unbind()
            .has_binding(agent, name.unbind(), gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
        env = env_scoped.get(agent);
        name = name_scoped.get(agent);
        result
    };

    // 3. If exists is true, then
    if exists {
        let env = env.unbind();
        let name = name.unbind();
        let gc = gc.into_nogc();
        // a. Return the Reference Record {
        Ok(Reference {
            // [[Base]]: env,
            base: Base::Environment(env.bind(gc)),
            // [[ReferencedName]]: name,
            referenced_name: name.bind(gc).into(),
            // [[Strict]]: strict,
            strict,
            // [[ThisValue]]: EMPTY
            this_value: None,
        })
        // }.
    }
    // 4. Else,
    else {
        // a. Let outer be env.[[OuterEnv]].
        let outer = env.unbind().get_outer_env(agent, gc.nogc());

        // b. Return ? GetIdentifierReference(outer, name, strict).
        get_identifier_reference(agent, outer.unbind(), name.unbind(), strict, gc)
    }
}

impl Environments {
    pub(crate) fn push_declarative_environment<'a>(
        &mut self,
        env: DeclarativeEnvironmentRecord,
        _: NoGcScope<'a, '_>,
    ) -> DeclarativeEnvironment<'a> {
        self.declarative.push(Some(env));
        DeclarativeEnvironment::from_u32(self.declarative.len() as u32)
    }

    pub(crate) fn push_function_environment<'a>(
        &mut self,
        env: FunctionEnvironmentRecord,
        _: NoGcScope<'a, '_>,
    ) -> FunctionEnvironment<'a> {
        self.function.push(Some(env));
        FunctionEnvironment::from_u32(self.function.len() as u32)
    }

    pub(crate) fn push_global_environment<'a>(
        &mut self,
        env: GlobalEnvironmentRecord,
        _: NoGcScope<'a, '_>,
    ) -> GlobalEnvironment<'a> {
        self.global.push(Some(env));
        GlobalEnvironment::from_u32(self.global.len() as u32)
    }

    pub(crate) fn push_module_environment<'a>(
        &mut self,
        env: ModuleEnvironmentRecord,
        _: NoGcScope<'a, '_>,
    ) -> ModuleEnvironment<'a> {
        self.module.push(Some(env));
        ModuleEnvironment::from_u32(self.module.len() as u32)
    }

    pub(crate) fn push_object_environment<'a>(
        &mut self,
        env: ObjectEnvironmentRecord,
        decl_env: DeclarativeEnvironmentRecord,
        _: NoGcScope<'a, '_>,
    ) -> (ObjectEnvironment<'a>, DeclarativeEnvironment<'a>) {
        self.object.push(Some(env));
        self.declarative.push(Some(decl_env));
        (
            ObjectEnvironment::from_u32(self.object.len() as u32),
            DeclarativeEnvironment::from_u32(self.declarative.len() as u32),
        )
    }

    pub(crate) fn push_private_environment<'a>(
        &mut self,
        env: PrivateEnvironmentRecord,
        _: NoGcScope<'a, '_>,
    ) -> PrivateEnvironment<'a> {
        self.private.push(Some(env));
        PrivateEnvironment::from_u32(self.private.len() as u32)
    }

    pub(crate) fn get_declarative_environment(
        &self,
        index: DeclarativeEnvironment,
    ) -> &DeclarativeEnvironmentRecord {
        self.declarative
            .get(index.into_index())
            .expect("DeclarativeEnvironment did not match to any vector index")
            .as_ref()
            .expect("DeclarativeEnvironment pointed to a None")
    }

    pub(crate) fn get_declarative_environment_mut(
        &mut self,
        index: DeclarativeEnvironment,
    ) -> &mut DeclarativeEnvironmentRecord {
        self.declarative
            .get_mut(index.into_index())
            .expect("DeclarativeEnvironment did not match to any vector index")
            .as_mut()
            .expect("DeclarativeEnvironment pointed to a None")
    }

    pub(crate) fn get_function_environment(
        &self,
        index: FunctionEnvironment,
    ) -> &FunctionEnvironmentRecord {
        self.function
            .get(index.into_index())
            .expect("FunctionEnvironment did not match to any vector index")
            .as_ref()
            .expect("FunctionEnvironment pointed to a None")
    }

    pub(crate) fn get_function_environment_mut(
        &mut self,
        index: FunctionEnvironment,
    ) -> &mut FunctionEnvironmentRecord {
        self.function
            .get_mut(index.into_index())
            .expect("FunctionEnvironment did not match to any vector index")
            .as_mut()
            .expect("FunctionEnvironment pointed to a None")
    }

    pub(crate) fn get_module_environment(
        &self,
        index: ModuleEnvironment,
    ) -> &ModuleEnvironmentRecord {
        self.module
            .get(index.into_index())
            .expect("ModuleEnvironment did not match to any vector index")
            .as_ref()
            .expect("ModuleEnvironment pointed to a None")
    }

    pub(crate) fn get_module_environment_mut(
        &mut self,
        index: ModuleEnvironment,
    ) -> &mut ModuleEnvironmentRecord {
        self.module
            .get_mut(index.into_index())
            .expect("ModuleEnvironment did not match to any vector index")
            .as_mut()
            .expect("ModuleEnvironment pointed to a None")
    }

    pub(crate) fn get_global_environment(
        &self,
        index: GlobalEnvironment,
    ) -> &GlobalEnvironmentRecord {
        self.global
            .get(index.into_index())
            .expect("GlobalEnvironment did not match to any vector index")
            .as_ref()
            .expect("GlobalEnvironment pointed to a None")
    }

    pub(crate) fn get_global_environment_mut(
        &mut self,
        index: GlobalEnvironment,
    ) -> &mut GlobalEnvironmentRecord {
        self.global
            .get_mut(index.into_index())
            .expect("GlobalEnvironment did not match to any vector index")
            .as_mut()
            .expect("GlobalEnvironment pointed to a None")
    }

    pub(crate) fn get_object_environment(
        &self,
        index: ObjectEnvironment,
    ) -> &ObjectEnvironmentRecord {
        self.object
            .get(index.into_index())
            .expect("ObjectEnvironment did not match to any vector index")
            .as_ref()
            .expect("ObjectEnvironment pointed to a None")
    }

    pub(crate) fn get_object_environment_mut(
        &mut self,
        index: ObjectEnvironment,
    ) -> &mut ObjectEnvironmentRecord {
        self.object
            .get_mut(index.into_index())
            .expect("ObjectEnvironment did not match to any vector index")
            .as_mut()
            .expect("ObjectEnvironment pointed to a None")
    }

    pub(crate) fn get_private_environment(
        &self,
        index: PrivateEnvironment,
    ) -> &PrivateEnvironmentRecord {
        self.private
            .get(index.into_index())
            .expect("PrivateEnvironment did not match to any vector index")
            .as_ref()
            .expect("PrivateEnvironment pointed to a None")
    }

    pub(crate) fn get_private_environment_mut(
        &mut self,
        index: PrivateEnvironment,
    ) -> &mut PrivateEnvironmentRecord {
        self.private
            .get_mut(index.into_index())
            .expect("PrivateEnvironment did not match to any vector index")
            .as_mut()
            .expect("PrivateEnvironment pointed to a None")
    }
}

/// ### [9.4.3 GetThisEnvironment ( )](https://tc39.es/ecma262/#sec-getthisenvironment)
/// The abstract operation GetThisEnvironment takes no arguments and returns an
/// Environment Record. It finds the Environment Record that currently supplies
/// the binding of the keyword this.
pub(crate) fn get_this_environment<'a>(agent: &Agent, gc: NoGcScope<'a, '_>) -> Environment<'a> {
    // 1. Let env be the running execution context's LexicalEnvironment.
    let mut env = agent.current_lexical_environment(gc);
    // 2. Repeat,
    loop {
        // a. Let exists be env.HasThisBinding().
        // b. If exists is true, return env.
        if env.has_this_binding(agent) {
            return env;
        }
        // c. Let outer be env.[[OuterEnv]].
        // d. Assert: outer is not null.
        // e. Set env to outer.
        env = env.get_outer_env(agent, gc).unwrap();
    }
}

impl AsRef<Environments> for Environments {
    fn as_ref(&self) -> &Environments {
        self
    }
}

impl AsMut<Environments> for Environments {
    fn as_mut(&mut self) -> &mut Environments {
        self
    }
}

impl AsRef<Environments> for Agent {
    fn as_ref(&self) -> &Environments {
        &self.heap.environments
    }
}

impl AsMut<Environments> for Agent {
    fn as_mut(&mut self) -> &mut Environments {
        &mut self.heap.environments
    }
}
