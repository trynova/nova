// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::ops::{Index, IndexMut};

use crate::ecmascript::types::{function_try_get, function_try_has_property, function_try_set};
use crate::engine::context::{GcScope, NoGcScope};
use crate::engine::rootable::{HeapRootData, HeapRootRef, Rootable};
use crate::engine::{Scoped, TryResult};
use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_objects::{call_function, construct},
            testing_and_comparison::is_constructor,
        },
        execution::{Agent, JsResult, ProtoIntrinsics},
        types::{
            function_create_backing_object, function_internal_define_own_property,
            function_internal_delete, function_internal_get, function_internal_get_own_property,
            function_internal_has_property, function_internal_own_property_keys,
            function_internal_set, BoundFunctionHeapData, Function, FunctionInternalProperties,
            InternalMethods, InternalSlots, IntoFunction, IntoObject, IntoValue, Object,
            OrdinaryObject, PropertyDescriptor, PropertyKey, String, Value,
        },
    },
    heap::{
        indexes::BoundFunctionIndex, CompactionLists, CreateHeapData, Heap, HeapMarkAndSweep,
        WorkQueues,
    },
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BoundFunction<'a>(BoundFunctionIndex<'a>);

impl BoundFunction<'_> {
    /// Unbind this BoundFunction from its current lifetime. This is necessary to use
    /// the BoundFunction as a parameter in a call that can perform garbage
    /// collection.
    pub fn unbind(self) -> BoundFunction<'static> {
        unsafe { std::mem::transmute::<BoundFunction, BoundFunction<'static>>(self) }
    }

    // Bind this BoundFunction to the garbage collection lifetime. This enables Rust's
    // borrow checker to verify that your BoundFunctions cannot not be invalidated by
    // garbage collection being performed.
    //
    // This function is best called with the form
    // ```rs
    // let number = number.bind(&gc);
    // ```
    // to make sure that the unbound BoundFunction cannot be used after binding.
    pub const fn bind<'gc>(self, _: NoGcScope<'gc, '_>) -> BoundFunction<'gc> {
        unsafe { std::mem::transmute::<BoundFunction, BoundFunction<'gc>>(self) }
    }

    pub fn scope<'scope>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'_, 'scope>,
    ) -> Scoped<'scope, BoundFunction<'static>> {
        Scoped::new(agent, self.unbind(), gc)
    }

    pub(crate) const fn _def() -> Self {
        BoundFunction(BoundFunctionIndex::from_u32_index(0))
    }

    pub(crate) const fn get_index(self) -> usize {
        self.0.into_index()
    }

    pub fn is_constructor(self, agent: &Agent) -> bool {
        // A bound function has the [[Construct]] method if the target function
        // does.
        agent[self].bound_target_function.is_constructor(agent)
    }
}

impl IntoValue for BoundFunction<'_> {
    fn into_value(self) -> Value {
        Value::BoundFunction(self.unbind())
    }
}

impl IntoObject for BoundFunction<'_> {
    fn into_object(self) -> Object {
        Object::BoundFunction(self.unbind())
    }
}

impl<'a> IntoFunction<'a> for BoundFunction<'a> {
    fn into_function(self) -> Function<'a> {
        Function::BoundFunction(self.unbind())
    }
}

/// ### [10.4.1.3 BoundFunctionCreate ( targetFunction, boundThis, boundArgs )](https://tc39.es/ecma262/#sec-boundfunctioncreate)
///
/// The abstract operation BoundFunctionCreate takes arguments targetFunction
/// (a function object), boundThis (an ECMAScript language value), and
/// boundArgs (a List of ECMAScript language values) and returns either a
/// normal completion containing a function object or a throw completion. It is
/// used to specify the creation of new bound function exotic objects.
pub(crate) fn bound_function_create<'a>(
    agent: &mut Agent,
    target_function: Function,
    bound_this: Value,
    bound_args: &[Value],
    mut gc: GcScope<'a, '_>,
) -> JsResult<BoundFunction<'a>> {
    let mut target_function = target_function.bind(gc.nogc());
    // 1. Let proto be ? targetFunction.[[GetPrototypeOf]]().
    let proto = if let TryResult::Continue(proto) =
        target_function.try_get_prototype_of(agent, gc.nogc())
    {
        proto
    } else {
        let scoped_target_function = target_function.scope(agent, gc.nogc());
        let proto = target_function
            .unbind()
            .internal_get_prototype_of(agent, gc.reborrow())?;
        target_function = scoped_target_function.get(agent).bind(gc.nogc());
        proto
    };
    // 2. Let internalSlotsList be the list-concatenation of « [[Prototype]],
    //     [[Extensible]] » and the internal slots listed in Table 31.
    // 3. Let obj be MakeBasicObject(internalSlotsList).
    // 4. Set obj.[[Prototype]] to proto.
    // 5. Set obj.[[Call]] as described in 10.4.1.1.
    // 6. If IsConstructor(targetFunction) is true, then
    // a. Set obj.[[Construct]] as described in 10.4.1.2.
    let mut elements = agent
        .heap
        .elements
        .allocate_elements_with_capacity(bound_args.len());
    elements.len = u32::try_from(bound_args.len()).unwrap();
    // SAFETY: Option<Value> is an extra variant of the Value enum.
    // The transmute effectively turns Value into Some(Value).
    agent[elements]
        .copy_from_slice(unsafe { std::mem::transmute::<&[Value], &[Option<Value>]>(bound_args) });
    let data = BoundFunctionHeapData {
        object_index: None,
        length: 0,
        bound_target_function: target_function.unbind(),
        bound_this,
        bound_arguments: elements,
        name: None,
    };
    // 7. Set obj.[[BoundTargetFunction]] to targetFunction.
    // 8. Set obj.[[BoundThis]] to boundThis.
    // 9. Set obj.[[BoundArguments]] to boundArgs.
    let obj = agent.heap.create(data);
    obj.internal_set_prototype_of(agent, proto, gc.reborrow())
        .unwrap();
    // 10. Return obj.
    Ok(obj)
}

impl<'a> FunctionInternalProperties<'a> for BoundFunction<'a> {
    fn get_name(self, agent: &Agent) -> String<'static> {
        agent[self].name.unwrap_or(String::EMPTY_STRING)
    }

    fn get_length(self, agent: &Agent) -> u8 {
        agent[self].length
    }
}

impl InternalSlots for BoundFunction<'_> {
    const DEFAULT_PROTOTYPE: ProtoIntrinsics = ProtoIntrinsics::Function;

    #[inline(always)]
    fn get_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        agent[self].object_index
    }

    fn set_backing_object(self, agent: &mut Agent, backing_object: OrdinaryObject<'static>) {
        assert!(agent[self]
            .object_index
            .replace(backing_object.unbind())
            .is_none());
    }

    fn create_backing_object(self, agent: &mut Agent) -> OrdinaryObject<'static> {
        function_create_backing_object(self, agent)
    }
}

impl InternalMethods for BoundFunction<'_> {
    fn try_get_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        _gc: NoGcScope<'_, '_>,
    ) -> TryResult<Option<PropertyDescriptor>> {
        TryResult::Continue(function_internal_get_own_property(
            self,
            agent,
            property_key,
        ))
    }

    fn try_define_own_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        property_descriptor: PropertyDescriptor,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_define_own_property(
            self,
            agent,
            property_key,
            property_descriptor,
            gc,
        ))
    }

    fn try_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        function_try_has_property(self, agent, property_key, gc)
    }

    fn internal_has_property(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        function_internal_has_property(self, agent, property_key, gc)
    }

    fn try_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<Value> {
        function_try_get(self, agent, property_key, receiver, gc)
    }

    fn internal_get(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        function_internal_get(self, agent, property_key, receiver, gc)
    }

    fn try_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        function_try_set(self, agent, property_key, value, receiver, gc)
    }

    fn internal_set(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        value: Value,
        receiver: Value,
        gc: GcScope<'_, '_>,
    ) -> JsResult<bool> {
        function_internal_set(self, agent, property_key, value, receiver, gc)
    }

    fn try_delete(
        self,
        agent: &mut Agent,
        property_key: PropertyKey,
        gc: NoGcScope<'_, '_>,
    ) -> TryResult<bool> {
        TryResult::Continue(function_internal_delete(self, agent, property_key, gc))
    }

    fn try_own_property_keys<'a>(
        self,
        agent: &mut Agent,
        gc: NoGcScope<'a, '_>,
    ) -> TryResult<Vec<PropertyKey<'a>>> {
        TryResult::Continue(function_internal_own_property_keys(self, agent, gc))
    }

    /// ### [10.4.1.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-bound-function-exotic-objects-call-thisargument-argumentslist)
    ///
    /// The \[\[Call]] internal method of a bound function exotic object F
    /// takes arguments thisArgument (an ECMAScript language value) and
    /// argumentsList (a List of ECMAScript language values) and returns either
    /// a normal completion containing an ECMAScript language value or a throw
    /// completion.
    fn internal_call(
        self,
        agent: &mut Agent,
        _: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'_, '_>,
    ) -> JsResult<Value> {
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = agent[self].bound_target_function;
        // 2. Let boundThis be F.[[BoundThis]].
        let bound_this = agent[self].bound_this;
        // 3. Let boundArgs be F.[[BoundArguments]].
        let bound_args = agent[self].bound_arguments;
        // 4. Let args be the list-concatenation of boundArgs and argumentsList.
        if bound_args.is_empty() {
            // Optimisation: If only `this` is bound, then we can pass the
            // arguments list without changes to the bound function.
            call_function(agent, target, bound_this, Some(arguments_list), gc)
        } else {
            // Note: We currently cannot optimise against an empty arguments
            // list, as we must create a Vec from the bound_args ElementsVector
            // in any case to use it as arguments. A slice pointing to it would
            // be unsound as calling to JS may invalidate the slice pointer.
            let mut args = Vec::with_capacity(bound_args.len() as usize + arguments_list.len());
            agent[bound_args]
                .iter()
                .for_each(|item| args.push(item.unwrap()));
            args.extend_from_slice(&arguments_list);
            // 5. Return ? Call(target, boundThis, args).
            call_function(agent, target, bound_this, Some(ArgumentsList(&args)), gc)
        }
    }

    /// ### [10.4.1.2 \[\[Construct\]\] ( argumentsList, newTarget )](https://tc39.es/ecma262/#sec-bound-function-exotic-objects-construct-argumentslist-newtarget)
    ///
    /// The \[\[Construct]] internal method of a bound function exotic object F
    /// takes arguments argumentsList (a List of ECMAScript language values)
    /// and newTarget (a constructor) and returns either a normal completion
    /// containing an Object or a throw completion.
    fn internal_construct(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        mut gc: GcScope<'_, '_>,
    ) -> JsResult<Object> {
        let new_target = new_target.bind(gc.nogc());
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = agent[self].bound_target_function;
        // 2. Assert: IsConstructor(target) is true.
        assert!(is_constructor(agent, target).is_some());
        // 3. Let boundArgs be F.[[BoundArguments]].
        let bound_args = agent[self].bound_arguments;
        // 5. If SameValue(F, newTarget) is true, set newTarget to target.
        let new_target = if self.into_function() == new_target {
            target
        } else {
            new_target
        };
        // 4. Let args be the list-concatenation of boundArgs and argumentsList.
        // Note: We currently cannot optimise against an empty arguments
        // list, as we must create a Vec from the bound_args ElementsVector
        // in any case to use it as arguments. A slice pointing to it would
        // be unsound as calling to JS may invalidate the slice pointer.
        let mut args = Vec::with_capacity(bound_args.len() as usize + arguments_list.len());
        agent[bound_args]
            .iter()
            .for_each(|item| args.push(item.unwrap()));
        args.extend_from_slice(&arguments_list);
        // 6. Return ? Construct(target, args, newTarget).
        construct(
            agent,
            target,
            Some(ArgumentsList(&args)),
            Some(new_target.unbind()),
            gc.reborrow(),
        )
    }
}

impl<'a> Index<BoundFunction<'a>> for Agent {
    type Output = BoundFunctionHeapData;

    fn index(&self, index: BoundFunction<'a>) -> &Self::Output {
        &self.heap.bound_functions[index]
    }
}

impl<'a> IndexMut<BoundFunction<'a>> for Agent {
    fn index_mut(&mut self, index: BoundFunction<'a>) -> &mut Self::Output {
        &mut self.heap.bound_functions[index]
    }
}

impl<'a> Index<BoundFunction<'a>> for Vec<Option<BoundFunctionHeapData>> {
    type Output = BoundFunctionHeapData;

    fn index(&self, index: BoundFunction<'a>) -> &Self::Output {
        self.get(index.get_index())
            .expect("BoundFunction out of bounds")
            .as_ref()
            .expect("BoundFunction slot empty")
    }
}

impl<'a> IndexMut<BoundFunction<'a>> for Vec<Option<BoundFunctionHeapData>> {
    fn index_mut(&mut self, index: BoundFunction<'a>) -> &mut Self::Output {
        self.get_mut(index.get_index())
            .expect("BoundFunction out of bounds")
            .as_mut()
            .expect("BoundFunction slot empty")
    }
}

impl CreateHeapData<BoundFunctionHeapData, BoundFunction<'static>> for Heap {
    fn create(&mut self, data: BoundFunctionHeapData) -> BoundFunction<'static> {
        self.bound_functions.push(Some(data));
        BoundFunction(BoundFunctionIndex::last(&self.bound_functions))
    }
}

impl Rootable for BoundFunction<'_> {
    type RootRepr = HeapRootRef;

    fn to_root_repr(value: Self) -> Result<Self::RootRepr, HeapRootData> {
        Err(HeapRootData::BoundFunction(value.unbind()))
    }

    fn from_root_repr(value: &Self::RootRepr) -> Result<Self, HeapRootRef> {
        Err(*value)
    }

    fn from_heap_ref(heap_ref: HeapRootRef) -> Self::RootRepr {
        heap_ref
    }

    fn from_heap_data(heap_data: HeapRootData) -> Option<Self> {
        match heap_data {
            HeapRootData::BoundFunction(d) => Some(d),
            _ => None,
        }
    }
}

impl HeapMarkAndSweep for BoundFunction<'static> {
    fn mark_values(&self, queues: &mut WorkQueues) {
        queues.bound_functions.push(*self);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        compactions.bound_functions.shift_index(&mut self.0);
    }
}

impl HeapMarkAndSweep for BoundFunctionHeapData {
    fn mark_values(&self, queues: &mut WorkQueues) {
        let Self {
            object_index,
            length: _,
            bound_target_function,
            bound_this,
            bound_arguments,
            name,
        } = self;
        name.mark_values(queues);
        bound_target_function.mark_values(queues);
        object_index.mark_values(queues);
        bound_this.mark_values(queues);
        bound_arguments.mark_values(queues);
    }

    fn sweep_values(&mut self, compactions: &CompactionLists) {
        let Self {
            object_index,
            length: _,
            bound_target_function,
            bound_this,
            bound_arguments,
            name,
        } = self;
        name.sweep_values(compactions);
        bound_target_function.sweep_values(compactions);
        object_index.sweep_values(compactions);
        bound_this.sweep_values(compactions);
        bound_arguments.sweep_values(compactions);
    }
}
