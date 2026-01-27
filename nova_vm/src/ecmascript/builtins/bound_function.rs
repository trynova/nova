// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, BoundFunctionHeapData, Function, FunctionInternalProperties, InternalMethods,
        JsResult, Object, OrdinaryObject, String, TryResult, Value, call_function, construct,
        function_handle, is_constructor, unwrap_try,
    },
    engine::{
        Bindable, GcScope,
        Scopable,
    },
    heap::{
        ArenaAccess, ArenaAccessMut, BaseIndex, CompactionLists, CreateHeapData, Heap,
        HeapMarkAndSweep, HeapSweepWeakReference, WorkQueues, arena_vec_access,
        ElementsVector,
    },
};

use super::ArgumentsList;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct BoundFunction<'a>(BaseIndex<'a, BoundFunctionHeapData<'static>>);
function_handle!(BoundFunction);
arena_vec_access!(
    BoundFunction,
    'a,
    BoundFunctionHeapData,
    bound_functions
);

impl<'f> BoundFunction<'f> {
    pub fn is_constructor(self, agent: &Agent) -> bool {
        // A bound function has the [[Construct]] method if the target function
        // does.
        self.bound_target_function(agent).is_constructor(agent)
    }

    /// ### \[\[BoundTargetFunction]]
    pub(crate) fn bound_target_function(self, agent: &Agent) -> Function<'f> {
        self.get(agent).bound_target_function
    }

    /// ### \[\[BoundThis]]
    pub(crate) fn bound_this(self, agent: &Agent) -> Value<'f> {
        self.get(agent).bound_this
    }

    /// ### \[\[BoundArguments]]
    pub(crate) fn bound_arguments(self, agent: &Agent) -> ElementsVector<'f> {
        self.get(agent).bound_arguments
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
) -> JsResult<'a, BoundFunction<'a>> {
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
            .internal_get_prototype_of(agent, gc.reborrow())
            .unbind()?
            .bind(gc.nogc());
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
    let mut elements = match agent
        .heap
        .elements
        .allocate_elements_with_length(bound_args.len())
    {
        Ok(e) => e,
        Err(err) => {
            return Err(agent.throw_allocation_exception(err, gc.into_nogc()));
        }
    };
    elements.len = u32::try_from(bound_args.len()).unwrap();
    // SAFETY: Option<Value> is an extra variant of the Value enum.
    // The transmute effectively turns Value into Some(Value).
    elements
        .get_mut(agent)
        .copy_from_slice(unsafe { core::mem::transmute::<&[Value], &[Option<Value>]>(bound_args) });
    let data = BoundFunctionHeapData {
        object_index: None,
        length: 0,
        bound_target_function: target_function.unbind(),
        bound_this: bound_this.unbind(),
        bound_arguments: elements,
        name: None,
    };
    // 7. Set obj.[[BoundTargetFunction]] to targetFunction.
    // 8. Set obj.[[BoundThis]] to boundThis.
    // 9. Set obj.[[BoundArguments]] to boundArgs.
    let obj = agent.heap.create(data);
    unwrap_try(obj.try_set_prototype_of(agent, proto, gc.nogc()));
    // 10. Return obj.
    Ok(obj)
}

impl<'a> FunctionInternalProperties<'a> for BoundFunction<'a> {
    fn get_name(self, agent: &Agent) -> &String<'a> {
        self.get(agent)
            .name
            .as_ref()
            .unwrap_or(&String::EMPTY_STRING)
    }

    fn get_length(self, agent: &Agent) -> u8 {
        self.get(agent).length
    }

    #[inline(always)]
    fn get_function_backing_object(self, agent: &Agent) -> Option<OrdinaryObject<'static>> {
        self.get(agent).object_index.unbind()
    }

    fn set_function_backing_object(
        self,
        agent: &mut Agent,
        backing_object: OrdinaryObject<'static>,
    ) {
        assert!(
            self.get_mut(agent)
                .object_index
                .replace(backing_object.unbind())
                .is_none()
        );
    }

    /// ### [10.4.1.1 \[\[Call\]\] ( thisArgument, argumentsList )](https://tc39.es/ecma262/#sec-bound-function-exotic-objects-call-thisargument-argumentslist)
    ///
    /// The \[\[Call]] internal method of a bound function exotic object F
    /// takes arguments thisArgument (an ECMAScript language value) and
    /// argumentsList (a List of ECMAScript language values) and returns either
    /// a normal completion containing an ECMAScript language value or a throw
    /// completion.
    fn function_call<'gc>(
        self,
        agent: &mut Agent,
        _: Value,
        arguments_list: ArgumentsList,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Value<'gc>> {
        agent.check_call_depth(gc.nogc()).unbind()?;
        let f = self.bind(gc.nogc());
        let arguments_list = arguments_list.bind(gc.nogc());
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = f.bound_target_function(agent);
        // 2. Let boundThis be F.[[BoundThis]].
        let bound_this = f.bound_this(agent);
        // 3. Let boundArgs be F.[[BoundArguments]].
        let bound_args = f.bound_arguments(agent);
        // 4. Let args be the list-concatenation of boundArgs and argumentsList.
        if bound_args.is_empty() {
            // Optimisation: If only `this` is bound, then we can pass the
            // arguments list without changes to the bound function.
            call_function(
                agent,
                target.unbind(),
                bound_this.unbind(),
                Some(arguments_list.unbind()),
                gc,
            )
        } else {
            // Note: We cannot optimise against an empty arguments list, as we
            // must create a Vec from the bound_args ElementsVector in any case
            // to use it as arguments. A slice pointing to it would be unsound
            // as calling to JS may invalidate the slice pointer. Arguments
            // must also be given as exclusive slice, which we couldn't provide
            // if we were basing it on the ElementsVector's data in the heap.
            let mut args: Vec<Value<'static>> =
                Vec::with_capacity(bound_args.len() as usize + arguments_list.len());
            bound_args
                .get(agent)
                .iter()
                .for_each(|item| args.push(item.unwrap().unbind()));
            args.extend_from_slice(&arguments_list.unbind());
            // 5. Return ? Call(target, boundThis, args).
            call_function(
                agent,
                target.unbind(),
                bound_this.unbind(),
                Some(ArgumentsList::from_mut_slice(&mut args)),
                gc,
            )
        }
    }

    /// ### [10.4.1.2 \[\[Construct\]\] ( argumentsList, newTarget )](https://tc39.es/ecma262/#sec-bound-function-exotic-objects-construct-argumentslist-newtarget)
    ///
    /// The \[\[Construct]] internal method of a bound function exotic object F
    /// takes arguments argumentsList (a List of ECMAScript language values)
    /// and newTarget (a constructor) and returns either a normal completion
    /// containing an Object or a throw completion.
    fn function_construct<'gc>(
        self,
        agent: &mut Agent,
        arguments_list: ArgumentsList,
        new_target: Function,
        gc: GcScope<'gc, '_>,
    ) -> JsResult<'gc, Object<'gc>> {
        agent.check_call_depth(gc.nogc()).unbind()?;
        let arguments_list = arguments_list.bind(gc.nogc());
        let new_target = new_target.bind(gc.nogc());
        // 1. Let target be F.[[BoundTargetFunction]].
        let target = self.bound_target_function(agent).bind(gc.nogc());
        // 2. Assert: IsConstructor(target) is true.
        assert!(is_constructor(agent, target).is_some());
        // 3. Let boundArgs be F.[[BoundArguments]].
        let bound_args = &self.get(agent).bound_arguments;
        // 5. If SameValue(F, newTarget) is true, set newTarget to target.
        let f: Function = self.into();
        let new_target = if f == new_target { target } else { new_target };
        // 4. Let args be the list-concatenation of boundArgs and argumentsList.
        // Note: We currently cannot optimise against an empty arguments
        // list, as we must create a Vec from the bound_args ElementsVector
        // in any case to use it as arguments. A slice pointing to it would
        // be unsound as calling to JS may invalidate the slice pointer.
        let mut args = Vec::with_capacity(bound_args.len() as usize + arguments_list.len());
        bound_args
            .get(agent)
            .iter()
            .for_each(|item| args.push(item.unwrap().unbind()));
        args.extend_from_slice(&arguments_list.unbind());
        // 6. Return ? Construct(target, args, newTarget).
        construct(
            agent,
            target.unbind(),
            Some(ArgumentsList::from_mut_slice(&mut args)),
            Some(new_target.unbind()),
            gc,
        )
    }
}

impl<'a> CreateHeapData<BoundFunctionHeapData<'a>, BoundFunction<'a>> for Heap {
    fn create(&mut self, data: BoundFunctionHeapData<'a>) -> BoundFunction<'a> {
        self.bound_functions.push(data.unbind());
        self.alloc_counter += core::mem::size_of::<BoundFunctionHeapData<'static>>();
        BoundFunction(BaseIndex::last(&self.bound_functions))
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

impl HeapSweepWeakReference for BoundFunction<'static> {
    fn sweep_weak_reference(self, compactions: &CompactionLists) -> Option<Self> {
        compactions
            .bound_functions
            .shift_weak_index(self.0)
            .map(Self)
    }
}
