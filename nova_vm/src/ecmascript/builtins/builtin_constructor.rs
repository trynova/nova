// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::ops::{Index, IndexMut};

use oxc_span::Span;

use crate::{
    ecmascript::{
        execution::{
            Agent, Environment, ExecutionContext, JsResult, PrivateEnvironment,
            agent::ExceptionType,
        },
        scripts_and_modules::source_code::SourceCode,
        syntax_directed_operations::class_definitions::{
            base_class_default_constructor, derived_class_default_constructor,
        },
        types::{
            BUILTIN_STRING_MEMORY, BuiltinConstructorRecord, Function, FunctionInternalProperties,
            IntoFunction, IntoObject, IntoValue, Object, OrdinaryObject, PropertyKey, String,
            Value,
        },
    },
    engine::{
        Executable,
        context::{Bindable, GcScope, NoGcScope, bindable_handle},
        rootable::{HeapRootData, HeapRootRef, Rootable},
    },
    heap::{
        CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep, HeapSweepWeakReference,
        ObjectEntry, ObjectEntryPropertyDescriptor, WorkQueues, indexes::BaseIndex,
    },
    ndt,
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BuiltinConstructorFunction<'a>(BaseIndex<'a, BuiltinConstructorRecord<'static>>);

impl BuiltinConstructorFunction<'_> {
    pub(crate) const fn _def() -> Self {
        Self(BaseIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub const fn is_constructor(self) -> bool {
        true
    }
}

bindable_handle!(BuiltinConstructorFunction);

impl<'a> From<BuiltinConstructorFunction<'a>> for Value<'a> {
    fn from(value: BuiltinConstructorFunction<'a>) -> Self {
        Value::BuiltinConstructorFunction(value)
    }
}

impl<'a> From<BuiltinConstructorFunction<'a>> for Object<'a> {
    fn from(value: BuiltinConstructorFunction) -> Self {
        Self::BuiltinConstructorFunction(value.unbind())
    }
}

impl<'a> From<BuiltinConstructorFunction<'a>> for Function<'a> {
    fn from(value: BuiltinConstructorFunction<'a>) -> Self {
        Self::BuiltinConstructorFunction(value)
    }
}

impl<'a> TryFrom<Value<'a>> for BuiltinConstructorFunction<'a> {
    type Error = ();

    fn try_from(value: Value<'a>) -> Result<Self, Self::Error> {
        match value {
            Value::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Object<'a>> for BuiltinConstructorFunction<'a> {
    type Error = ();

    fn try_from(value: Object<'a>) -> Result<Self, Self::Error> {
        match value {
            Object::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl<'a> TryFrom<Function<'a>> for BuiltinConstructorFunction<'a> {
    type Error = ();

    fn try_from(value: Function<'a>) -> Result<Self, Self::Error> {
        match value {
            Function::BuiltinConstructorFunction(data) => Ok(data),
            _ => Err(()),
        }
    }
}

impl Index<BuiltinConstructorFunction<'_>> for Agent {
    type Output = BuiltinConstructorRecord<'static>;

    fn index(&self, index: BuiltinConstructorFunction) -> &Self::Output {
        &self.heap.builtin_constructors[index]
    }
}

impl IndexMut<BuiltinConstructorFunction<'_>> for Agent {
    fn index_mut(&mut self, index: BuiltinConstructorFunction) -> &mut Self::Output {
        &mut self.heap.builtin_constructors[index]
    }
}

impl Index<BuiltinConstructorFunction<'_>> for Vec<BuiltinConstructorRecord<'static>> {
    type Output = BuiltinConstructorRecord<'static>;

    fn index(&self, index: BuiltinConstructorFunction) -> &Self::Output {
        self.get(index.get_index())
            .expect("BuiltinConstructorFunction out of bounds")
    }
}

impl IndexMut<BuiltinConstructorFunction<'_>> for Vec<BuiltinConstructorRecord<'static>> {
    fn index_mut(&mut self, index: BuiltinConstructorFunction) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BuiltinConstructorFunction out of bounds")
    }
}

impl<'a> FunctionInternalProperties<'a> for BuiltinConstructorFunction<'a> {
    fn get_name(self, agent: &Agent) -> &String<'a> {
        &agent[self].class_name
    }

    fn get_length(self, _: &Agent) -> u8 {
        unreachable!();
    }

    #[inline(always)]
    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].backing_object
    }

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    ) {
        assert!(agent[self].backing_object.replace(backing_object).is_none());
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
        _: Value,
        _: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        // 1. Return ? BuiltinCallOrConstruct(F, thisArgument, argumentsList, undefined).
        // ii. If NewTarget is undefined, throw a TypeError exception.
        Err(agent.throw_exception_with_static_message(
            ExceptionType::TypeError,
            "class constructors must be invoked with 'new'",
            gc.into_nogc(),
        ))
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
        let mut id = 0;
        ndt::builtin_constructor_start!(|| {
            id = create_id(agent, self);
            let name = self.get_name(agent).to_string_lossy(agent);
            (name, id)
        });
        // 1. Return ? BuiltinCallOrConstruct(F, uninitialized, argumentsList, newTarget).
        let result = builtin_call_or_construct(agent, self, arguments_list, new_target, gc);
        ndt::builtin_constructor_done!(|| id);
        result
    }
}

#[inline(never)]
fn create_id(agent: &Agent, f: BuiltinConstructorFunction) -> u64 {
    ((f.0.into_u32() as u64) << 32) | agent[f].source_text.start as u64
}

/// ### [10.3.3 BuiltinCallOrConstruct ( F, thisArgument, argumentsList, newTarget )](https://tc39.es/ecma262/#sec-builtincallorconstruct)
///
/// The abstract operation BuiltinCallOrConstruct takes arguments F (a built-in
/// function object), thisArgument (an ECMAScript language value or
/// uninitialized), argumentsList (a List of ECMAScript language values), and
/// newTarget (a constructor or undefined) and returns either a normal
/// completion containing an ECMAScript language value or a throw completion.
fn builtin_call_or_construct<'a>(
    agent: &mut Agent,
    f: BuiltinConstructorFunction,
    arguments_list: ArgumentsList,
    new_target: Function,
    gc: GcScope<'a, '_>,
) -> JsResult<'a, Object<'a>> {
    let f = f.bind(gc.nogc());
    let arguments_list = arguments_list.bind(gc.nogc());
    let new_target = new_target.bind(gc.nogc());
    // 1. Let callerContext be the running execution context.
    let caller_context = agent.running_execution_context();
    // 2. If callerContext is not already suspended, suspend callerContext.
    caller_context.suspend();
    // 5. Let calleeRealm be F.[[Realm]].
    let heap_data = &agent[f];
    let callee_realm = heap_data.realm;
    let is_derived = heap_data.is_derived;
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
    let result = if is_derived {
        derived_class_default_constructor(
            agent,
            arguments_list.unbind(),
            new_target.into_object().unbind(),
            gc,
        )
    } else {
        base_class_default_constructor(agent, new_target.into_object().unbind(), gc)
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

pub(crate) struct BuiltinConstructorArgs<'a> {
    pub(crate) is_derived: bool,
    pub(crate) class_name: String<'a>,
    pub(crate) prototype: Option<Object<'a>>,
    pub(crate) prototype_property: Object<'a>,
    pub(crate) compiled_initializer_bytecode: Option<Executable<'a>>,
    pub(crate) env: Environment<'a>,
    pub(crate) private_env: Option<PrivateEnvironment<'a>>,
    pub(crate) source_code: SourceCode<'a>,
    pub(crate) source_text: Span,
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
pub(crate) fn create_builtin_constructor<'a>(
    agent: &mut Agent,
    args: BuiltinConstructorArgs,
    gc: NoGcScope<'a, '_>,
) -> BuiltinConstructorFunction<'a> {
    // 1. If realm is not present, set realm to the current Realm Record.
    let realm = agent.current_realm(gc);

    // 9. Set func.[[InitialName]] to null.

    // 2. If prototype is not present, set prototype to realm.[[Intrinsics]].[[%Function.prototype%]].

    // 3. Let internalSlotsList be a List containing the names of all the internal slots that 10.3
    //    requires for the built-in function object that is about to be created.
    // 4. Append to internalSlotsList the elements of additionalInternalSlotsList.
    // * [[ConstructorKind]] and [[SourceText]] for class constructors.

    // 5. Let func be a new built-in function object that, when called, performs the action
    //    described by behaviour using the provided arguments as the values of the corresponding
    //    parameters specified by behaviour. The new function object has internal slots whose names
    //    are the elements of internalSlotsList, and an [[InitialName]] internal slot.

    // 7. Set func.[[Extensible]] to true.
    let length_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.length),
        value: ObjectEntryPropertyDescriptor::Data {
            value: 0.into(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let name_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.name),
        value: ObjectEntryPropertyDescriptor::Data {
            value: args.class_name.into_value(),
            writable: false,
            enumerable: false,
            configurable: true,
        },
    };
    let prototype_entry = ObjectEntry {
        key: PropertyKey::from(BUILTIN_STRING_MEMORY.prototype),
        value: ObjectEntryPropertyDescriptor::Data {
            value: args.prototype_property.into_value(),
            writable: false,
            enumerable: false,
            configurable: false,
        },
    };
    let entries = [length_entry, name_entry, prototype_entry];
    let backing_object = OrdinaryObject::create_intrinsic_object(agent, args.prototype, &entries);

    // 13. Return func.
    agent
        .heap
        .create(BuiltinConstructorRecord {
            // 10. Perform SetFunctionLength(func, length).
            // Skipped as length of builtin constructors is always 0.
            // 8. Set func.[[Realm]] to realm.
            realm,
            compiled_initializer_bytecode: args.compiled_initializer_bytecode,
            is_derived: args.is_derived,
            backing_object: Some(backing_object),
            environment: args.env,
            private_environment: args.private_env,
            source_text: args.source_text,
            source_code: args.source_code,
            class_name: args.class_name,
        })
        .bind(gc)
}

impl Rootable for BuiltinConstructorFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::BuiltinConstructorFunction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BuiltinConstructorFunction(d) => Some(d),
            _ => None,
        }
    }
}

impl<'a> CreateHeapData<BuiltinConstructorRecord<'a>, BuiltinConstructorFunction<'a>> for Heap {
    fn create(&mut self, data: BuiltinConstructorRecord) -> BuiltinConstructorFunction<'a> {
        self.builtin_constructors.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<BuiltinConstructorRecord<'static>>();

        BuiltinConstructorFunction(BaseIndex::last(&self.builtin_constructors))
    }
}

impl HeapMarkAndSweep for BuiltinConstructorFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.builtin_constructors.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.builtin_constructors.shift_index(&mut self.0);
    }
}

impl HeapSweepWeakReference for BuiltinConstructorFunction<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .builtin_constructors
            .shift_weak_index(self.0)
            .map(Self)
    }
}

bindable_handle!(BuiltinConstructorRecord);

impl HeapMarkAndSweep for BuiltinConstructorRecord<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            backing_object: object_index,
            realm,
            is_derived: _,
            compiled_initializer_bytecode,
            environment,
            private_environment,
            source_text: _,
            source_code,
            class_name,
        } = self;
        realm.mark_values(queues);
        object_index.mark_values(queues);
        environment.mark_values(queues);
        private_environment.mark_values(queues);
        source_code.mark_values(queues);
        compiled_initializer_bytecode.mark_values(queues);
        class_name.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            backing_object: object_index,
            realm,
            is_derived: _,
            compiled_initializer_bytecode,
            environment,
            private_environment,
            source_text: _,
            source_code,
            class_name,
        } = self;
        realm.sweep_values(compactions);
        object_index.sweep_values(compactions);
        environment.sweep_values(compactions);
        private_environment.sweep_values(compactions);
        source_code.sweep_values(compactions);
        compiled_initializer_bytecode.sweep_values(compactions);
        class_name.sweep_values(compactions);
    }
}
