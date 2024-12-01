// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::{
    marker::PhantomData,
    ops::{Deref, Index, IndexMut},
};
use std::{hint::unreachable_unchecked, ptr::NonNull};

use crate::{
    ecmascript::{
        execution::{Agent, ExecutionContext, JsResult, Realm, agent::ExceptionType},
        types::{
            BUILTIN_STRING_MEMORY, BuiltinFunctionHeapData, Function, FunctionInternalProperties,
            InternalSlots, IntoFunction, IntoObject, IntoValue, Object, OrdinaryObject,
            PropertyKey, ScopedValuesIterator, String, Value,
        },
    },
    engine::{
        context::{Bindable, GcScope, NoGcScope},
        rootable::{HeapRootCollectionData, HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        IntrinsicConstructorIndexes, IntrinsicFunctionIndexes, ObjectEntry,
        ObjectEntryPropertyDescriptor, WorkQueues, indexes::BuiltinFunctionIndex,
    },
};

#[derive(Default)]
#[repr(transparent)]
pub struct ArgumentsList<'slice, 'value> {
    slice: &'slice mut [Value<'static>],
    value: PhantomData<Value<'value>>,
}

impl core::fmt::Debug for ArgumentsList<'_, '_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.slice.fmt(f)
    }
}

impl<'slice, 'value> ArgumentsList<'slice, 'value> {
    /// Create an ArgumentsList from a single Value.
    pub fn from_mut_value(value: &'slice mut Value<'value>) -> Self {
        Self {
            // SAFETY: The Value lifetime is moved over to the PhantomData.
            slice: core::slice::from_mut(unsafe {
                core::mem::transmute::<&'slice mut Value<'value>, &'slice mut Value<'static>>(value)
            }),
            value: PhantomData,
        }
    }

    /// Create an ArgumentsList from a Value slice.
    pub fn from_mut_slice(slice: &'slice mut [Value<'value>]) -> Self {
        Self {
            // SAFETY: The Value lifetime is moved over to the PhantomData.
            slice: unsafe {
                core::mem::transmute::<&'slice mut [Value<'value>], &'slice mut [Value<'static>]>(
                    slice,
                )
            },
            value: PhantomData,
        }
    }

    pub fn with_scoped<'a, R>(
        &mut self,
        agent: &mut Agent,
        work: impl FnOnce(&mut Agent, ScopedArgumentsList<'_>, GcScope<'a, '_>) -> R,
        mut gc: GcScope<'a, '_>,
    ) -> R
    where
        R: 'a,
    {
        // First take the arguments from ArgumentsList. Note: This is
        // strictly extra work from a computational standpoint, but makes
        // the code safer from a memory stand point. Any errors will also
        // become more obvious.
        let slice = core::mem::take(&mut self.slice);
        // SAFETY: We push the slice to the heap temporarily, for which we
        // need to transmute its lifetime to static. This is strictly not
        // correct: The slice points either to the stack or to a Vec owned
        // by an above call frame. But! We give our best possible guarantee
        // that the slice will be taken out of the heap before the end of
        // this call. As long as that holds, then this is all perfectly
        // legal. Note that unwinding of panics will break that guarantee
        // currently.
        let slice = unsafe {
            std::mem::transmute::<&mut [Value<'static>], &'static mut [Value<'static>]>(slice)
        };
        // Store the slice's end pointer for validity checking later.
        let slice_end_ptr = slice.as_ptr_range().end;
        let slice_len = slice.len();
        // Now we push the slice onto the heap.
        let (stack_refs_len, len) = {
            let stack_refs_len = u32::try_from(agent.stack_refs.borrow().len())
                .expect("Stack references overflowed");
            let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
            let len = stack_ref_collections.len();
            stack_ref_collections.push(HeapRootCollectionData::ArgumentsList(slice.into()));
            // Elsewhere we make assumptions about the size of the stack.
            // Thus check it here as well.
            (
                stack_refs_len,
                u32::try_from(len).expect("Stack reference collections overflowed"),
            )
        };
        let result = {
            let sub_gc = gc.subscope();
            let scoped_args = ScopedArgumentsList::new(len, sub_gc.nogc());
            // Once the slice is on the heap, we can perform the user's work.
            work(agent, scoped_args, sub_gc)
        };
        // After the user's work is done, we can get to work returning the
        // slice from the heap.
        let mut slice = {
            agent
                .stack_refs
                .borrow_mut()
                .truncate(stack_refs_len as usize);

            // We look at the slot where we put the slice in and check that
            // it contains an arguments list.
            let mut stack_ref_collections = agent.stack_ref_collections.borrow_mut();
            debug_assert!(stack_ref_collections.len() >= len as usize);
            let stack_slot = &mut stack_ref_collections[len as usize];
            if !matches!(stack_slot, HeapRootCollectionData::ArgumentsList(_)) {
                unreachable!()
            }
            // We take the slice back from the heap by replacing the data
            // with an empty collection, and then truncate the heap stack
            // to its previous size.
            let HeapRootCollectionData::ArgumentsList(slice) =
                core::mem::replace(stack_slot, HeapRootCollectionData::Empty)
            else {
                // SAFETY: Checked above against the stack_slot.
                unsafe { unreachable_unchecked() }
            };
            stack_ref_collections.truncate(len as usize);
            slice
        };
        // Confirm that we have the right slice and that only valid subslicing
        // has occurred.
        // SAFETY: We had exclusive access to the slice and we've checked that it's
        let post_work_slice_len = slice.len();
        // SAFETY: This computes the end pointer of the slice meaning that we
        // necessarily do not wrap around or go out of bounds (by more than 1).
        let post_work_slice_end_ptr = unsafe {
            slice
                .as_ptr()
                .cast::<Value<'static>>()
                .add(post_work_slice_len)
        };
        assert!(slice_len >= post_work_slice_len);
        assert_eq!(slice_end_ptr, post_work_slice_end_ptr);
        // Now that we have our slice back, we can place it back where it
        // came from. The end result is that the slice has temporarily been
        // held by the heap and has been given back to the caller. While
        // the heap held it, the caller couldn't have done so, and thus the
        // exclusive reference requirement cannot have been broken by this
        // method.
        // The only thing we can break here is the lifetime requirement,
        // and that is only possible if the user method panicked and that
        // panic was caught and recovered from above us. ie. We're not
        // panic safe currently.
        // SAFETY: We've confirmed that this slice is the same slice that we
        // got originally in self.slice; it may have become a subslice of the
        // original slice but otherwise it is the same. Thus converting it
        // back to a mutable slice is safe.
        let slice = core::mem::replace(&mut self.slice, unsafe { slice.as_mut() });
        debug_assert!(slice.is_empty());
        result
    }

    pub(crate) fn slice_from(self, start: usize) -> ArgumentsList<'slice, 'value> {
        ArgumentsList {
            slice: self
                .slice
                .split_at_mut_checked(start)
                .map_or(&mut [], |(_, tail)| tail),
            value: PhantomData,
        }
    }

    /// Access the Values in an ArgumentsList as a slice.
    pub fn as_slice(&self) -> &[Value<'value>] {
        self.slice
    }

    /// Access the Values in an ArgumentsList as a mut slice.
    pub(crate) fn as_mut_slice(&mut self) -> &mut [Value<'static>] {
        self.slice
    }

    pub(crate) fn into_raw_parts_mut(self) -> (*mut Value<'static>, usize) {
        (self.slice.as_mut_ptr(), self.slice.len())
    }

    /// Get a Value by index from an ArgumentsList.
    ///
    /// If a Value with that index isn't present, `undefined` is returned.
    #[inline]
    pub fn get(&self, index: usize) -> Value<'value> {
        *self.slice.get(index).unwrap_or(&Value::Undefined)
    }

    /// Get a Value by index from an ArgumentsList if present.
    ///
    /// If a Value with that index isn't present, None.
    #[inline]
    pub fn get_if_present(&self, index: usize) -> Option<Value<'value>> {
        self.slice.get(index).copied()
    }
}

// SAFETY: Properly implemented as a lifetime transmute.
unsafe impl<'slice> Bindable for ArgumentsList<'slice, '_> {
    type Of<'a> = ArgumentsList<'slice, 'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe {
            core::mem::transmute::<ArgumentsList<'slice, '_>, ArgumentsList<'slice, 'static>>(self)
        }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe {
            core::mem::transmute::<ArgumentsList<'slice, '_>, ArgumentsList<'slice, 'a>>(self)
        }
    }
}

impl<'value> Deref for ArgumentsList<'_, 'value> {
    type Target = [Value<'value>];

    fn deref(&self) -> &Self::Target {
        self.slice
    }
}

/// Arguments list stored temporarily on the Agent's heap.
///
/// The arguments can be accessed through the Agent for the duration of the
/// function call.
pub struct ScopedArgumentsList<'scope> {
    index: u32,
    value: PhantomData<Value<'scope>>,
}

impl<'scope> ScopedArgumentsList<'scope> {
    pub(crate) fn new(index: u32, _: NoGcScope<'_, 'scope>) -> Self {
        Self {
            index,
            value: PhantomData,
        }
    }

    pub fn get<'gc>(&self, agent: &Agent, index: u32, gc: NoGcScope<'gc, '_>) -> Value<'gc> {
        if let HeapRootCollectionData::ArgumentsList(args) = agent
            .stack_ref_collections
            .borrow()
            .get(self.index as usize)
            .unwrap()
        {
            // SAFETY: The arguments list slice is guaranteed to be held
            // exclusively by an above call frame, and placed onto the heap as
            // a NonNull slice. Creating a temporary slice of it is safe.
            // Exception: if a panic has occurred then we could theoretically
            // be looking at an invalid slice, but in that case we shouldn't
            // have access to the ScopedArgumentsList anymore either.
            unsafe {
                args.as_ref()
                    .get(index as usize)
                    .unwrap_or(&Value::Undefined)
                    .bind(gc)
            }
        } else {
            unreachable!()
        }
    }

    pub fn len(&self, agent: &Agent) -> usize {
        if let HeapRootCollectionData::ArgumentsList(args) = agent
            .stack_ref_collections
            .borrow()
            .get(self.index as usize)
            .unwrap()
        {
            args.len()
        } else {
            unreachable!()
        }
    }

    pub(crate) fn unshift<'gc>(&self, agent: &Agent, gc: NoGcScope<'gc, '_>) -> Option<Value<'gc>> {
        let mut collections = agent.stack_ref_collections.borrow_mut();
        let collection_data: &mut HeapRootCollectionData =
            collections.get_mut(self.index as usize).unwrap();
        if let HeapRootCollectionData::ArgumentsList(args_ref) = collection_data {
            // SAFETY: args_ref points to a valid exclusively held slice in an
            // above call frame.
            let args: &mut [Value<'static>] = unsafe { args_ref.as_mut() };
            if let Some((first, rest)) = args.split_first_mut() {
                let result = first.unbind().bind(gc);
                let rest = NonNull::from(rest);
                *args_ref = rest;
                Some(result)
            } else {
                None
            }
        } else {
            unreachable!()
        }
    }

    pub(crate) fn iter(&self, agent: &mut Agent) -> ScopedValuesIterator<'_> {
        if let HeapRootCollectionData::ArgumentsList(args) = agent
            .stack_ref_collections
            .borrow()
            .get(self.index as usize)
            .unwrap()
        {
            // SAFETY: args points to a uniquely owned slice in an above
            // call frame, we can safely dereference it.
            ScopedValuesIterator::from_slice(unsafe { args.as_ref() })
        } else {
            unreachable!()
        }
    }

    /// Get access to the backing reference slice as a pointer slice.
    ///
    /// ## Safety
    ///
    /// Garbage collection must not be called while this slice is exposed.
    ///
    /// Stack values must not be accessed while this slice is exposed.
    pub(crate) unsafe fn as_non_null_slice(&self, agent: &Agent) -> NonNull<[Value<'static>]> {
        if let HeapRootCollectionData::ArgumentsList(args) = agent
            .stack_ref_collections
            .borrow()
            .get(self.index as usize)
            .unwrap()
        {
            *args
        } else {
            unreachable!()
        }
    }
}

impl core::fmt::Debug for ScopedArgumentsList<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "ScopedArgumentsList({})", self.index)
    }
}

pub type RegularFn =
    for<'gc> fn(&mut Agent, Value, ArgumentsList, GcScope<'gc, '_>) -> JsResult<'gc, Value<'gc>>;
pub type ConstructorFn = for<'gc> fn(
    &mut Agent,
    Value,
    ArgumentsList,
    Option<Object>,
    GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>>;

#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Behaviour {
    Regular(RegularFn),
    Constructor(ConstructorFn),
}

impl Behaviour {
    pub(crate) fn is_constructor(&self) -> bool {
        matches!(self, Behaviour::Constructor(_))
    }
}

pub trait Builtin {
    const NAME: String<'static>;
    const LENGTH: u8;
    const BEHAVIOUR: Behaviour;

    /// Set to Some if this builtin's property key is different from `NAME`.
    const KEY: Option<PropertyKey<'static>> = None;

    /// If the builtin function is created as a property then this controls the
    /// property's `[[Writable]]` value.
    const WRITABLE: bool = true;
    /// If the builtin function is created as a property then this controls the
    /// property's `[[Enumerable]]` value.
    const ENUMERABLE: bool = false;
    /// If the builtin function is created as a property then this controls the
    /// property's `[[Configurable]]` value.
    const CONFIGURABLE: bool = true;
}

pub(crate) trait BuiltinIntrinsicConstructor: Builtin {
    const INDEX: IntrinsicConstructorIndexes;
}

pub(crate) trait BuiltinIntrinsic: Builtin {
    const INDEX: IntrinsicFunctionIndexes;
}
pub trait BuiltinGetter: Builtin {
    const GETTER_NAME: String<'static> = Self::NAME;
    const GETTER_BEHAVIOUR: Behaviour = Self::BEHAVIOUR;
}
pub trait BuiltinSetter: Builtin {
    const SETTER_NAME: String<'static> = Self::NAME;
    const SETTER_BEHAVIOUR: Behaviour = Self::BEHAVIOUR;
}

#[derive(Debug, Default)]
pub struct BuiltinFunctionArgs<'a> {
    pub length: u32,
    pub name: &'static str,
    pub realm: Option<Realm<'a>>,
    pub prototype: Option<Object<'a>>,
    pub prefix: Option<&'static str>,
}

impl<'a> BuiltinFunctionArgs<'a> {
    pub fn new(length: u32, name: &'static str) -> Self {
        Self {
            length,
            name,
            ..Default::default()
        }
    }

    pub fn new_with_realm(length: u32, name: &'static str, realm: Realm<'a>) -> Self {
        Self {
            length,
            name,
            realm: Some(realm),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuiltinFunction<'a>(pub(crate) BuiltinFunctionIndex<'a>);

impl BuiltinFunction<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BuiltinFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_constructor(self, agent: &Agent) -> bool {
        // A builtin function has the [[Construct]] method if its behaviour is
        // a constructor behaviour.
        agent[self].behaviour.is_constructor()
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BuiltinFunction<'_> {
    type Of<'a> = BuiltinFunction<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl<'a> From<BuiltinFunctionIndex<'a>> for BuiltinFunction<'a> {
    fn from(value: BuiltinFunctionIndex<'a>) -> Self {
        Self(value)
    }
}

impl<'a> From<BuiltinFunction<'a>> for Value<'a> {
    fn from(value: BuiltinFunction<'a>) -> Self {
        Value::BuiltinFunction(value)
    }
}

impl<'a> From<BuiltinFunction<'a>> for Object<'a> {
    fn from(value: BuiltinFunction) -> Self {
        Object::BuiltinFunction(value.unbind())
    }
}

impl<'a> From<BuiltinFunction<'a>> for Function<'a> {
    fn from(value: BuiltinFunction<'a>) -> Self {
        Function::BuiltinFunction(value)
    }
}

impl Index<BuiltinFunction<'_>> for Agent {
    type Output = BuiltinFunctionHeapData<'static>;

    fn index(&self, index: BuiltinFunction) -> &Self::Output {
        &self.heap.builtin_functions[index]
    }
}

impl IndexMut<BuiltinFunction<'_>> for Agent {
    fn index_mut(&mut self, index: BuiltinFunction) -> &mut Self::Output {
        &mut self.heap.builtin_functions[index]
    }
}

impl Index<BuiltinFunction<'_>> for Vec<Option<BuiltinFunctionHeapData<'static>>> {
    type Output = BuiltinFunctionHeapData<'static>;

    fn index(&self, index: BuiltinFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinFunction out of bounds")
            .as_ref()
            .expect("BuiltinFunction slot empty")
    }
}

impl IndexMut<BuiltinFunction<'_>> for Vec<Option<BuiltinFunctionHeapData<'static>>> {
    fn index_mut(&mut self, index: BuiltinFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinFunction out of bounds")
            .as_mut()
            .expect("BuiltinFunction slot empty")
    }
}

impl<'a> FunctionInternalProperties<'a> for BuiltinFunction<'a> {
    fn get_name(self, agent: &Agent) -> String<'static> {
        agent[self].initial_name.unwrap_or(String::EMPTY_STRING)
    }

    fn get_length(self, agent: &Agent) -> u8 {
        agent[self].length
    }

    #[inline(always)]
    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    ) {
        assert!(
            agent[self]
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    /// ### [10.3.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-built-in-function-objects-call-thisargument-argumentslist)
    ///
    /// The [[Call]] internal method of a built-in function object F takes
    /// arguments thisArgument (an ECMAScript language value) and argumentsList
    /// (a List of ECMAScript language values) and returns either a normal
    /// completion containing an ECMAScript language value or a throw
    /// completion.
    fn function_call<'gc>(
        self,
        agent: &mut Agent,
        this_argument: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        #[usdt::provider]
        mod nova {
            fn start_builtin_call(name: &str) {}
            fn stop_builtin_call(name: &str) {}
        }
        nova::start_builtin_call!(|| { self.get_name(agent).to_string_lossy(agent).to_string() });
        let result =
            // 1. Return ? BuiltinCallOrConstruct(F, thisArgument, argumentsList, undefined).
            builtin_call_or_construct(agent, self, Some(this_argument), arguments_list, None, gc);
        nova::stop_builtin_call!(|| { self.get_name(agent).to_string_lossy(agent).to_string() });
        result
    }

    /// ### [10.3.2 \[\[Construct\]\] ( argumentsList, newTarget )](https://tc39.es/ecma262/#sec-built-in-function-objects-construct-argumentslist-newtarget)
    ///
    /// The [[Construct]] internal method of a built-in function object F (when
    /// the method is present) takes arguments argumentsList (a List of
    /// ECMAScript language values) and newTarget (a constructor) and returns
    /// either a normal completion containing an Object or a throw completion.
    fn function_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        #[usdt::provider]
        mod nova {
            fn start_builtin_constructor(name: &str) {}
            fn stop_builtin_constructor(name: &str) {}
        }
        nova::start_builtin_constructor!(|| {
            self.get_name(agent).to_string_lossy(agent).to_string()
        });
        // 1. Return ? BuiltinCallOrConstruct(F, uninitialized, argumentsList, newTarget).
        let result =
            builtin_call_or_construct(agent, self, None, arguments_list, Some(new_target), gc)
                .map(|result| result.try_into().unwrap());
        nova::stop_builtin_constructor!(|| {
            self.get_name(agent).to_string_lossy(agent).to_string()
        });
        result
    }
}

/// ### [10.3.3 BuiltinCallOrConstruct ( F, thisArgument, argumentsList, newTarget )](https://tc39.es/ecma262/#sec-builtincallorconstruct)
///
/// The abstract operation BuiltinCallOrConstruct takes arguments F (a built-in
/// function object), thisArgument (an ECMAScript language value or
/// uninitialized), argumentsList (a List of ECMAScript language values), and
/// newTarget (a constructor or undefined) and returns either a normal
/// completion containing an ECMAScript language value or a throw completion.
pub(crate) fn builtin_call_or_construct<'gc>(
    agent: &mut Agent,
    f: BuiltinFunction,
    this_argument: Option<Value>,
    arguments_list: ArgumentsList,
    new_target: Option<Function>,
    gc: GcScope<'gc, '_>,
) -> JsResult<'gc, Value<'gc>> {
    let f = f.bind(gc.nogc());
    let this_argument = this_argument.bind(gc.nogc());
    let arguments_list = arguments_list.bind(gc.nogc());
    let new_target = new_target.bind(gc.nogc());
    // 1. Let callerContext be the running execution context.
    let caller_context = agent.running_execution_context();
    // 2. If callerContext is not already suspended, suspend callerContext.
    caller_context.suspend();
    // 5. Let calleeRealm be F.[[Realm]].
    let heap_data = &agent[f];
    let callee_realm = heap_data.realm;
    let func = heap_data.behaviour;
    // 3. Let calleeContext be a new execution context.
    let callee_context = ExecutionContext {
        // 8. Perform any necessary implementation-defined initialization of calleeContext.
        ecmascript_code: None,
        // 4. Set the Function of calleeContext to F.
        function: Some(f.into_function().unbind()),
        // 6. Set the Realm of calleeContext to calleeRealm.
        realm: callee_realm,
        // 7. Set the ScriptOrModule of calleeContext to null.
        script_or_module: None,
    };
    // 9. Push calleeContext onto the execution context stack; calleeContext is now the running execution context.
    agent.push_execution_context(callee_context);
    // 10. Let result be the Completion Record that is the result of evaluating F in a manner that conforms to
    // the specification of F. If thisArgument is uninitialized, the this value is uninitialized; otherwise,
    // thisArgument provides the this value. argumentsList provides the named parameters. newTarget provides the NewTarget value.
    let result = match func {
        Behaviour::Regular(func) => {
            if new_target.is_some() {
                Err(agent.throw_exception_with_static_message(
                    ExceptionType::TypeError,
                    "Not a constructor",
                    gc.into_nogc(),
                ))
            } else {
                func(
                    agent,
                    this_argument.unwrap_or(Value::Undefined).unbind(),
                    arguments_list.unbind(),
                    gc,
                )
            }
        }
        Behaviour::Constructor(func) => func(
            agent,
            this_argument.unwrap_or(Value::Undefined).unbind(),
            arguments_list.unbind(),
            new_target.map(|target| target.into_object().unbind()),
            gc,
        ),
    };
    // 11. NOTE: If F is defined in this document, “the specification of F” is the behaviour specified for it via
    // algorithm steps or other means.
    // 12. Remove calleeContext from the execution context stack and restore callerContext as the running
    // execution context.
    // Note
    // When calleeContext is removed from the execution context stack it must not be destroyed if it has been
    // suspended and retained by an accessible Generator for later resumption.
    let _callee_context = agent.pop_execution_context();
    // 13. Return ? result.
    result
}

/// ### [10.3.4 CreateBuiltinFunction ( behaviour, length, name, additionalInternalSlotsList \[ , realm \[ , prototype \[ , prefix \] \] \] )](https://tc39.es/ecma262/#sec-createbuiltinfunction)
///
/// The abstract operation CreateBuiltinFunction takes arguments behaviour (an
/// Abstract Closure, a set of algorithm steps, or some other definition of a
/// function's behaviour provided in this specification), length (a
/// non-negative integer or +∞), name (a property key or a Private Name), and
/// additionalInternalSlotsList (a List of names of internal slots) and
/// optional arguments realm (a Realm Record), prototype (an Object or null),
/// and prefix (a String) and returns a function object.
/// additionalInternalSlotsList contains the names of additional internal slots
/// that must be defined as part of the object. This operation creates a
/// built-in function object.
pub fn create_builtin_function<'a>(
    agent: &mut Agent,
    behaviour: Behaviour,
    args: BuiltinFunctionArgs,
    gc: NoGcScope<'a, '_>,
) -> BuiltinFunction<'a> {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = args.realm.unwrap_or(agent.current_realm(gc));

    // 9. Set func.[[InitialName]] to null.
    // Note: SetFunctionName inlined here: We know name is a string
    let initial_name = if let Some(prefix) = args.prefix {
        // 12. Else,
        // a. Perform SetFunctionName(func, name, prefix).
        String::from_string(agent, format!("{} {}", args.name, prefix), gc)
    } else {
        // 11. If prefix is not present, then
        // a. Perform SetFunctionName(func, name).
        String::from_str(agent, args.name, gc)
    };

    // 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].

    // 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3
    //    requires for the built-in function object that is about to be created.
    // 4. Append to internalSlotsList the elements of additionalInternalSlotsList.
    // Note: The BuiltinFunctionHeapData implements all internal slots that 10.3 requires.
    // The currently appearing spec-defined additional slots are:
    // * [[ConstructorKind]] and [[SourceText]] for class constructors.
    // * [[Promise]] and [[AlreadyResolved]] for Promise resolver functions
    // * [[AlreadyCalled]], [[Index]], [[Values]], [[Capability]], and [[RemainingElements]] for
    //   Promise.all's onFulfilled function.
    // We do not yet support these, and how these end up supported is not yet fully clear.

    // 5. Let func be a new built-in function object that, when called, performs the action
    //    described by behaviour using the provided arguments as the values of the corresponding
    //    parameters specified by behaviour. The new function object has internal slots whose names
    //    are the elements of internalSlotsList, and an [[InitialName]] internal slot.
    let object_index = if let Some(prototype) = args.prototype {
        // If a prototype is set, then check that it is not the %Function.prototype%
        let realm_function_prototype = agent
            .get_realm_record_by_id(realm)
            .intrinsics()
            .get_intrinsic_default_proto(BuiltinFunction::DEFAULT_PROTOTYPE);
        if prototype == realm_function_prototype {
            // If the prototype matched the realm function prototype, then ignore it
            // as the BuiltinFunctionHeapData indirectly implies this prototype.
            None
        } else {
            // If some other prototype is defined then we need to create a backing object.
            // 6. Set func.[[Prototype]] to prototype.
            // 7. Set func.[[Extensible]] to true.
            let length_entry = ObjectEntry {
                key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: args.length.into(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            let name_entry = ObjectEntry {
                key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
                value: ObjectEntryPropertyDescriptor::Data {
                    value: initial_name.into_value(),
                    writable: false,
                    enumerable: false,
                    configurable: true,
                },
            };
            Some(OrdinaryObject::create_object(
                agent,
                Some(prototype),
                &[length_entry, name_entry],
            ))
        }
    } else {
        None
    };

    // 13. Return func.
    agent
        .heap
        .create(BuiltinFunctionHeapData {
            behaviour,
            initial_name: Some(initial_name),
            // 10. Perform SetFunctionLength(func, length).
            length: args.length as u8,
            // 8. Set func.[[Realm]] to realm.
            realm,
            object_index,
        })
        .bind(gc)
}

impl<'a> CreateHeapData<BuiltinFunctionHeapData<'a>, BuiltinFunction<'a>> for Heap {
    fn create(&mut self, data: BuiltinFunctionHeapData<'a>) -> BuiltinFunction<'a> {
        self.builtin_functions.push(Some(data.unbind()));
        self.alloc_counter += core::mem::size_of::<Option<BuiltinFunctionHeapData<'static>>>();
        BuiltinFunctionIndex::last(&self.builtin_functions).into()
    }
}

impl Rootable for BuiltinFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::BuiltinFunction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BuiltinFunction(d) => Some(d),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for BuiltinFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.builtin_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.builtin_functions.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for BuiltinFunction<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .builtin_functions
            .shift_weak_index(self.0)
            .map(Self)
    }
}

// SAFETY: Property implemented as a lifetime transmute.
unsafe impl Bindable for BuiltinFunctionHeapData<'_> {
    type Of<'a> = BuiltinFunctionHeapData<'a>;

    #[inline(always)]
    fn unbind(self) -> Self::Of<'static> {
        unsafe { core::mem::transmute::<Self, Self::Of<'static>>(self) }
    }

    #[inline(always)]
    fn bind<'a>(self, _gc: NoGcScope<'a, '_>) -> Self::Of<'a> {
        unsafe { core::mem::transmute::<Self, Self::Of<'a>>(self) }
    }
}

impl HeapMarkAndSweep for BuiltinFunctionHeapData<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            length: _,
            realm,
            initial_name,
            behaviour: _,
        } = self;
        realm.mark_values(queues);
        initial_name.mark_values(queues);
        object_index.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            length: _,
            realm,
            initial_name,
            behaviour: _,
        } = self;
        realm.sweep_values(compactions);
        initial_name.sweep_values(compactions);
        object_index.sweep_values(compactions);
    }
}
