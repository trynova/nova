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

pub(crate) use declarative_environment::{DeclarativeEnvironment, new_declarative_environment};
pub(crate) use function_environment::{
    FunctionEnvironment, ThisBindingStatus, new_class_field_initializer_environment,
    new_class_static_element_environment, new_function_environment,
};
pub(crate) use global_environment::GlobalEnvironment;
pub(crate) use object_environment::ObjectEnvironment;
pub(crate) use private_environment::PrivateEnvironment;

use crate::engine::TryResult;
use crate::engine::context::{Bindable, GcScope, GcToken, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable, Scopable};
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
pub(super) type OuterEnv<'a> = Option<EnvironmentIndex<'a>>;

macro_rules! create_environment_index {
    ($name: ident, $index: ident, $entry: ident) => {
        /// An index used to access an environment from [`Environments`].
        /// Internally, we store the index in a [`NonZeroU32`] with the index
        /// plus one. This allows us to not use an empty value in storage for
        /// the zero index while still saving room for a [`None`] value when
        /// stored in an [`Option`].
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        pub struct $index<'a>(NonZeroU32, PhantomData<$name>, PhantomData<&'a GcToken>);

        impl core::fmt::Debug for $index<'_> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "$index({:?})", self.0)
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

            pub(crate) fn last(vec: &[Option<$name>]) -> Self {
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
                Err(HeapRootData::$name(value.unbind()))
            }

            fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
                Err(*value)
            }

            fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
                heap_ref
            }

            fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
                match heap_data {
                    HeapRootData::$name(object) => Some(object),
                    _ => None,
                }
            }
        }

        impl core::ops::Index<$index<'_>> for Agent {
            type Output = $name;

            fn index(&self, index: $index) -> &Self::Output {
                &self.heap.environments.$entry[index]
            }
        }

        impl core::ops::IndexMut<$index<'_>> for Agent {
            fn index_mut(&mut self, index: $index) -> &mut Self::Output {
                &mut self.heap.environments.$entry[index]
            }
        }

        impl core::ops::Index<$index<'_>> for Vec<Option<$name>> {
            type Output = $name;

            fn index(&self, index: $index) -> &Self::Output {
                self.get(index.into_index())
                    .expect("Environment out of bounds")
                    .as_ref()
                    .expect("Environment slot empty")
            }
        }

        impl core::ops::IndexMut<$index<'_>> for Vec<Option<$name>> {
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
    DeclarativeEnvironment,
    DeclarativeEnvironmentIndex,
    declarative
);
create_environment_index!(FunctionEnvironment, FunctionEnvironmentIndex, function);
create_environment_index!(GlobalEnvironment, GlobalEnvironmentIndex, global);
create_environment_index!(ObjectEnvironment, ObjectEnvironmentIndex, object);
create_environment_index!(PrivateEnvironment, PrivateEnvironmentIndex, private);

impl<'a> From<DeclarativeEnvironmentIndex<'a>> for EnvironmentIndex<'a> {
    fn from(value: DeclarativeEnvironmentIndex<'a>) -> Self {
        EnvironmentIndex::Declarative(value)
    }
}

impl<'a> From<GlobalEnvironmentIndex<'a>> for EnvironmentIndex<'a> {
    fn from(value: GlobalEnvironmentIndex<'a>) -> Self {
        EnvironmentIndex::Global(value)
    }
}

impl<'a> From<ObjectEnvironmentIndex<'a>> for EnvironmentIndex<'a> {
    fn from(value: ObjectEnvironmentIndex<'a>) -> Self {
        EnvironmentIndex::Object(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ModuleEnvironmentIndex<'a>(
    NonZeroU32,
    PhantomData<DeclarativeEnvironment>,
    PhantomData<&'a GcToken>,
);
impl ModuleEnvironmentIndex<'_> {
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

    pub(crate) const fn into_index(self) -> usize {
        self.0.get() as usize - 1
    }

    pub(crate) const fn into_u32(self) -> u32 {
        self.0.get()
    }

    pub(crate) fn last(vec: &[Option<DeclarativeEnvironment>]) -> Self {
        Self::from_u32(vec.len() as u32)
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for ModuleEnvironmentIndex<'_> {
    type Of<'a> = ModuleEnvironmentIndex<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl Rootable for ModuleEnvironmentIndex<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::ModuleEnvironment(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::ModuleEnvironment(object) => Some(object),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for ModuleEnvironmentIndex<'_> {
    fn mark_values(&self, _queues: &mut WorkQueues) {
        todo!()
    }

    fn sweep_values(&mut self, _compactions: &CompactionLists) {
        todo!()
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
pub(crate) enum EnvironmentIndex<'a> {
    // Leave 0 for None option
    Declarative(DeclarativeEnvironmentIndex<'a>) = 1,
    Function(FunctionEnvironmentIndex<'a>),
    Global(GlobalEnvironmentIndex<'a>),
    // Module(ModuleEnvironmentIndex<'a>),
    Object(ObjectEnvironmentIndex<'a>),
}

impl EnvironmentIndex<'_> {
    pub(crate) fn get_outer_env<'a>(self, agent: &Agent, _: NoGcScope<'a, '_>) -> OuterEnv<'a> {
        match self {
            EnvironmentIndex::Declarative(index) => agent[index].outer_env,
            EnvironmentIndex::Function(index) => {
                agent[agent[index].declarative_environment].outer_env
            }
            EnvironmentIndex::Global(_) => None,
            EnvironmentIndex::Object(index) => agent[index].outer_env,
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
            EnvironmentIndex::Declarative(idx) => TryResult::Continue(idx.has_binding(agent, name)),
            EnvironmentIndex::Function(idx) => TryResult::Continue(idx.has_binding(agent, name)),
            EnvironmentIndex::Global(idx) => idx.try_has_binding(agent, name, gc),
            EnvironmentIndex::Object(idx) => idx.try_has_binding(agent, name, gc),
        }
    }

    /// ### [HasBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record has a binding for the String value
    /// N. Return true if it does and false if it does not.
    pub(crate) fn has_binding(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            EnvironmentIndex::Declarative(idx) => Ok(idx.has_binding(agent, name)),
            EnvironmentIndex::Function(idx) => Ok(idx.has_binding(agent, name)),
            EnvironmentIndex::Global(idx) => idx.has_binding(agent, name, gc),
            EnvironmentIndex::Object(idx) => idx.has_binding(agent, name, gc),
        }
    }

    /// ### Try [CreateMutableBinding(N, D)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If the
    /// Boolean argument D is true the binding may be subsequently deleted.
    pub(crate) fn try_create_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
        gc: NoGcScope,
    ) -> TryResult<JsResult<()>> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.create_mutable_binding(agent, name, is_deletable);
                TryResult::Continue(Ok(()))
            }
            EnvironmentIndex::Function(idx) => {
                idx.create_mutable_binding(agent, name, is_deletable);
                TryResult::Continue(Ok(()))
            }
            EnvironmentIndex::Global(idx) => {
                TryResult::Continue(idx.create_mutable_binding(agent, name, is_deletable, gc))
            }
            EnvironmentIndex::Object(idx) => {
                idx.try_create_mutable_binding(agent, name, is_deletable, gc)
            }
        }
    }

    /// ### [CreateMutableBinding(N, D)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If the
    /// Boolean argument D is true the binding may be subsequently deleted.
    pub(crate) fn create_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_deletable: bool,
        gc: GcScope,
    ) -> JsResult<()> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.create_mutable_binding(agent, name, is_deletable);
                Ok(())
            }
            EnvironmentIndex::Function(idx) => {
                idx.create_mutable_binding(agent, name, is_deletable);
                Ok(())
            }
            EnvironmentIndex::Global(idx) => {
                idx.create_mutable_binding(agent, name, is_deletable, gc.nogc())
            }
            EnvironmentIndex::Object(idx) => {
                idx.create_mutable_binding(agent, name, is_deletable, gc)
            }
        }
    }

    /// ### [CreateImmutableBinding(N, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Create a new but uninitialized immutable binding in an Environment
    /// Record. The String value N is the text of the bound name. If S is true
    /// then attempts to set it after it has been initialized will always throw
    /// an exception, regardless of the strict mode setting of operations that
    /// reference that binding.
    pub(crate) fn create_immutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope,
    ) -> JsResult<()> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.create_immutable_binding(agent, name, is_strict);
                Ok(())
            }
            EnvironmentIndex::Function(idx) => {
                idx.create_immutable_binding(agent, name, is_strict);
                Ok(())
            }
            EnvironmentIndex::Global(idx) => {
                idx.create_immutable_binding(agent, name, is_strict, gc)
            }
            EnvironmentIndex::Object(idx) => {
                idx.create_immutable_binding(agent, name, is_strict);
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
    pub(crate) fn try_initialize_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        gc: NoGcScope,
    ) -> TryResult<JsResult<()>> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.initialize_binding(agent, name, value);
                TryResult::Continue(Ok(()))
            }
            EnvironmentIndex::Function(idx) => {
                idx.initialize_binding(agent, name, value);
                TryResult::Continue(Ok(()))
            }
            EnvironmentIndex::Global(idx) => idx.try_initialize_binding(agent, name, value, gc),
            EnvironmentIndex::Object(idx) => idx.try_initialize_binding(agent, name, value, gc),
        }
    }

    /// ### [InitializeBinding(N, V)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing but uninitialized binding in an
    /// Environment Record. The String value N is the text of the bound name.
    /// V is the value for the binding and is a value of any ECMAScript
    /// language type.
    pub(crate) fn initialize_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        gc: GcScope,
    ) -> JsResult<()> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.initialize_binding(agent, name, value);
                Ok(())
            }
            EnvironmentIndex::Function(idx) => {
                idx.initialize_binding(agent, name, value);
                Ok(())
            }
            EnvironmentIndex::Global(idx) => idx.initialize_binding(agent, name, value, gc),
            EnvironmentIndex::Object(idx) => idx.initialize_binding(agent, name, value, gc),
        }
    }

    /// ### Try [SetMutableBinding(N, V, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. V is the
    /// value for the binding and may be a value of any ECMAScript language
    /// type. S is a Boolean flag. If S is true and the binding cannot be set
    /// throw a TypeError exception.
    pub(crate) fn try_set_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        is_strict: bool,
        gc: NoGcScope,
    ) -> TryResult<JsResult<()>> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                TryResult::Continue(idx.set_mutable_binding(agent, name, value, is_strict, gc))
            }
            EnvironmentIndex::Function(idx) => {
                TryResult::Continue(idx.set_mutable_binding(agent, name, value, is_strict, gc))
            }
            EnvironmentIndex::Global(idx) => {
                idx.try_set_mutable_binding(agent, name, value, is_strict, gc)
            }
            EnvironmentIndex::Object(idx) => {
                idx.try_set_mutable_binding(agent, name, value, is_strict, gc)
            }
        }
    }

    /// ### [SetMutableBinding(N, V, S)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Set the value of an already existing mutable binding in an Environment
    /// Record. The String value N is the text of the bound name. V is the
    /// value for the binding and may be a value of any ECMAScript language
    /// type. S is a Boolean flag. If S is true and the binding cannot be set
    /// throw a TypeError exception.
    pub(crate) fn set_mutable_binding(
        self,
        agent: &mut Agent,
        name: String,
        value: Value,
        is_strict: bool,
        gc: GcScope,
    ) -> JsResult<()> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.set_mutable_binding(agent, name, value, is_strict, gc.nogc())
            }
            EnvironmentIndex::Function(idx) => {
                idx.set_mutable_binding(agent, name, value, is_strict, gc.nogc())
            }
            EnvironmentIndex::Global(idx) => {
                idx.set_mutable_binding(agent, name, value, is_strict, gc)
            }
            EnvironmentIndex::Object(idx) => {
                idx.set_mutable_binding(agent, name, value, is_strict, gc)
            }
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
    pub(crate) fn try_get_binding_value<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: NoGcScope<'gc, '_>,
    ) -> TryResult<JsResult<Value<'gc>>> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                TryResult::Continue(idx.get_binding_value(agent, name, is_strict, gc))
            }
            EnvironmentIndex::Function(idx) => {
                TryResult::Continue(idx.get_binding_value(agent, name, is_strict, gc))
            }
            EnvironmentIndex::Global(idx) => idx.try_get_binding_value(agent, name, is_strict, gc),
            EnvironmentIndex::Object(idx) => idx.try_get_binding_value(agent, name, is_strict, gc),
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
    pub(crate) fn get_binding_value<'gc>(
        self,
        agent: &mut Agent,
        name: String,
        is_strict: bool,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<Value<'gc>> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                idx.get_binding_value(agent, name, is_strict, gc.into_nogc())
            }
            EnvironmentIndex::Function(idx) => {
                idx.get_binding_value(agent, name, is_strict, gc.into_nogc())
            }
            EnvironmentIndex::Global(idx) => idx.get_binding_value(agent, name, is_strict, gc),
            EnvironmentIndex::Object(idx) => idx.get_binding_value(agent, name, is_strict, gc),
        }
    }

    /// ### Try [DeleteBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Delete a binding from an Environment Record. The String value N is the
    /// text of the bound name. If a binding for N exists, remove the binding
    /// and return true. If the binding exists but cannot be removed return
    /// false.
    pub(crate) fn try_delete_binding(
        self,
        agent: &mut Agent,
        name: String,
        gc: NoGcScope,
    ) -> TryResult<JsResult<bool>> {
        match self {
            EnvironmentIndex::Declarative(idx) => {
                TryResult::Continue(Ok(idx.delete_binding(agent, name)))
            }
            EnvironmentIndex::Function(idx) => {
                TryResult::Continue(Ok(idx.delete_binding(agent, name)))
            }
            EnvironmentIndex::Global(idx) => idx.try_delete_binding(agent, name, gc),
            EnvironmentIndex::Object(idx) => {
                TryResult::Continue(Ok(idx.try_delete_binding(agent, name, gc)?))
            }
        }
    }

    /// ### [DeleteBinding(N)](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Delete a binding from an Environment Record. The String value N is the
    /// text of the bound name. If a binding for N exists, remove the binding
    /// and return true. If the binding exists but cannot be removed return
    /// false.
    pub(crate) fn delete_binding(
        self,
        agent: &mut Agent,
        name: String,
        gc: GcScope,
    ) -> JsResult<bool> {
        match self {
            EnvironmentIndex::Declarative(idx) => Ok(idx.delete_binding(agent, name)),
            EnvironmentIndex::Function(idx) => Ok(idx.delete_binding(agent, name)),
            EnvironmentIndex::Global(idx) => idx.delete_binding(agent, name, gc),
            EnvironmentIndex::Object(idx) => idx.delete_binding(agent, name, gc),
        }
    }

    /// ### [HasThisBinding()](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record establishes a this binding. Return
    /// true if it does and false if it does not.
    pub(crate) fn has_this_binding(self, agent: &mut Agent) -> bool {
        match self {
            EnvironmentIndex::Declarative(_) => false,
            EnvironmentIndex::Function(idx) => idx.has_this_binding(agent),
            EnvironmentIndex::Global(_) => true,
            EnvironmentIndex::Object(_) => false,
        }
    }

    /// ### [HasSuperBinding()](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// Determine if an Environment Record establishes a super method binding.
    /// Return true if it does and false if it does not.
    pub(crate) fn has_super_binding(self, agent: &mut Agent) -> bool {
        match self {
            EnvironmentIndex::Declarative(idx) => idx.has_super_binding(),
            EnvironmentIndex::Function(idx) => idx.has_super_binding(agent),
            EnvironmentIndex::Global(idx) => idx.has_super_binding(),
            EnvironmentIndex::Object(idx) => idx.has_super_binding(),
        }
    }

    /// ### [WithBaseObject()](https://tc39.es/ecma262/#table-abstract-methods-of-environment-records)
    ///
    /// If this Environment Record is associated with a with statement, return
    /// the with object. Otherwise, return undefined.
    pub(crate) fn with_base_object(self, agent: &mut Agent) -> Option<Object> {
        match self {
            EnvironmentIndex::Declarative(idx) => idx.with_base_object(),
            EnvironmentIndex::Function(idx) => idx.with_base_object(),
            EnvironmentIndex::Global(idx) => idx.with_base_object(),
            EnvironmentIndex::Object(idx) => idx.with_base_object(agent),
        }
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for EnvironmentIndex<'_> {
    type Of<'a> = EnvironmentIndex<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl core::fmt::Debug for EnvironmentIndex<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            EnvironmentIndex::Declarative(d) => write!(f, "DeclarativeEnvironment({:?})", d.0),
            EnvironmentIndex::Function(d) => write!(f, "FunctionEnvironment({:?})", d.0),
            EnvironmentIndex::Global(d) => write!(f, "GlobalEnvironment({:?})", d.0),
            EnvironmentIndex::Object(d) => write!(f, "ObjectEnvironment({:?})", d.0),
            // EnvironmentIndex::Module(d) => {}
        }
    }
}

impl Rootable for EnvironmentIndex<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        match value {
            EnvironmentIndex::Declarative(declarative_environment_index) => Err(
                HeapRootData::DeclarativeEnvironment(declarative_environment_index.unbind()),
            ),
            EnvironmentIndex::Function(function_environment_index) => Err(
                HeapRootData::FunctionEnvironment(function_environment_index.unbind()),
            ),
            EnvironmentIndex::Global(global_environment_index) => Err(
                HeapRootData::GlobalEnvironment(global_environment_index.unbind()),
            ),
            EnvironmentIndex::Object(object_environment_index) => Err(
                HeapRootData::ObjectEnvironment(object_environment_index.unbind()),
            ),
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
            HeapRootData::DeclarativeEnvironment(declarative_environment_index) => {
                Some(EnvironmentIndex::Declarative(declarative_environment_index))
            }
            HeapRootData::FunctionEnvironment(function_environment_index) => {
                Some(EnvironmentIndex::Function(function_environment_index))
            }
            HeapRootData::GlobalEnvironment(global_environment_index) => {
                Some(EnvironmentIndex::Global(global_environment_index))
            }
            HeapRootData::ObjectEnvironment(object_environment_index) => {
                Some(EnvironmentIndex::Object(object_environment_index))
            }
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for EnvironmentIndex<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        match self {
            EnvironmentIndex::Declarative(idx) => idx.mark_values(queues),
            EnvironmentIndex::Function(idx) => idx.mark_values(queues),
            EnvironmentIndex::Global(idx) => idx.mark_values(queues),
            EnvironmentIndex::Object(idx) => idx.mark_values(queues),
        }
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        match self {
            EnvironmentIndex::Declarative(idx) => idx.sweep_values(compactions),
            EnvironmentIndex::Function(idx) => idx.sweep_values(compactions),
            EnvironmentIndex::Global(idx) => idx.sweep_values(compactions),
            EnvironmentIndex::Object(idx) => idx.sweep_values(compactions),
        }
    }
}

#[derive(Debug)]
pub struct Environments {
    pub(crate) declarative: Vec<Option<DeclarativeEnvironment>>,
    pub(crate) function: Vec<Option<FunctionEnvironment>>,
    pub(crate) global: Vec<Option<GlobalEnvironment>>,
    pub(crate) object: Vec<Option<ObjectEnvironment>>,
    pub(crate) private: Vec<Option<PrivateEnvironment>>,
}

impl Default for Environments {
    fn default() -> Self {
        Self {
            declarative: Vec::with_capacity(256),
            function: Vec::with_capacity(1024),
            global: Vec::with_capacity(1),
            object: Vec::with_capacity(1024),
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
    env: Option<EnvironmentIndex>,
    name: String<'a>,
    strict: bool,
    gc: NoGcScope<'a, '_>,
) -> TryResult<Reference<'a>> {
    // 1. If env is null, then
    let Some(env) = env else {
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
    env: Option<EnvironmentIndex>,
    name: String<'b>,
    strict: bool,
    mut gc: GcScope<'a, 'b>,
) -> JsResult<Reference<'a>> {
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
            .has_binding(agent, name.unbind(), gc.reborrow())?;
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
        env: DeclarativeEnvironment,
        _: NoGcScope<'a, '_>,
    ) -> DeclarativeEnvironmentIndex<'a> {
        self.declarative.push(Some(env));
        DeclarativeEnvironmentIndex::from_u32(self.declarative.len() as u32)
    }

    pub(crate) fn push_function_environment<'a>(
        &mut self,
        env: FunctionEnvironment,
        _: NoGcScope<'a, '_>,
    ) -> FunctionEnvironmentIndex<'a> {
        self.function.push(Some(env));
        FunctionEnvironmentIndex::from_u32(self.function.len() as u32)
    }

    pub(crate) fn push_global_environment<'a>(
        &mut self,
        env: GlobalEnvironment,
        _: NoGcScope<'a, '_>,
    ) -> GlobalEnvironmentIndex<'a> {
        self.global.push(Some(env));
        GlobalEnvironmentIndex::from_u32(self.global.len() as u32)
    }

    pub(crate) fn push_object_environment<'a>(
        &mut self,
        env: ObjectEnvironment,
        _: NoGcScope<'a, '_>,
    ) -> ObjectEnvironmentIndex<'a> {
        self.object.push(Some(env));
        ObjectEnvironmentIndex::from_u32(self.object.len() as u32)
    }

    pub(crate) fn get_declarative_environment(
        &self,
        index: DeclarativeEnvironmentIndex,
    ) -> &DeclarativeEnvironment {
        self.declarative
            .get(index.into_index())
            .expect("DeclarativeEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("DeclarativeEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_declarative_environment_mut(
        &mut self,
        index: DeclarativeEnvironmentIndex,
    ) -> &mut DeclarativeEnvironment {
        self.declarative
            .get_mut(index.into_index())
            .expect("DeclarativeEnvironmentIndex did not match to any vector index")
            .as_mut()
            .expect("DeclarativeEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_function_environment(
        &self,
        index: FunctionEnvironmentIndex,
    ) -> &FunctionEnvironment {
        self.function
            .get(index.into_index())
            .expect("FunctionEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("FunctionEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_function_environment_mut(
        &mut self,
        index: FunctionEnvironmentIndex,
    ) -> &mut FunctionEnvironment {
        self.function
            .get_mut(index.into_index())
            .expect("FunctionEnvironmentIndex did not match to any vector index")
            .as_mut()
            .expect("FunctionEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_global_environment(
        &self,
        index: GlobalEnvironmentIndex,
    ) -> &GlobalEnvironment {
        self.global
            .get(index.into_index())
            .expect("GlobalEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("GlobalEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_global_environment_mut(
        &mut self,
        index: GlobalEnvironmentIndex,
    ) -> &mut GlobalEnvironment {
        self.global
            .get_mut(index.into_index())
            .expect("GlobalEnvironmentIndex did not match to any vector index")
            .as_mut()
            .expect("GlobalEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_object_environment(
        &self,
        index: ObjectEnvironmentIndex,
    ) -> &ObjectEnvironment {
        self.object
            .get(index.into_index())
            .expect("ObjectEnvironmentIndex did not match to any vector index")
            .as_ref()
            .expect("ObjectEnvironmentIndex pointed to a None")
    }

    pub(crate) fn get_object_environment_mut(
        &mut self,
        index: ObjectEnvironmentIndex,
    ) -> &mut ObjectEnvironment {
        self.object
            .get_mut(index.into_index())
            .expect("ObjectEnvironmentIndex did not match to any vector index")
            .as_mut()
            .expect("ObjectEnvironmentIndex pointed to a None")
    }
}

/// ### [9.4.3 GetThisEnvironment ( )](https://tc39.es/ecma262/#sec-getthisenvironment)
/// The abstract operation GetThisEnvironment takes no arguments and returns an
/// Environment Record. It finds the Environment Record that currently supplies
/// the binding of the keyword this.
pub(crate) fn get_this_environment<'a>(
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
) -> EnvironmentIndex<'a> {
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
