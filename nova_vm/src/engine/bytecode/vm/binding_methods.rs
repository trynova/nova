// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::iterator_close_with_value, operations_on_objects::get,
        },
        execution::{Agent, JsResult},
        types::{IntoValue, Object, PropertyKey, PropertyKeySet, Value},
    },
    engine::{
        ScopableCollection, Scoped,
        bytecode::vm::{
            Environment, Executable, Instruction, Vm, VmIteratorRecord, array_create,
            copy_data_properties_into_object, initialize_referenced_binding, put_value,
            resolve_binding, to_object, try_create_data_property_or_throw,
        },
        context::{Bindable, GcScope},
        iterator::ActiveIterator,
        rootable::Scopable,
        unwrap_try,
    },
};

use super::with_vm_gc;

pub(super) fn execute_simple_array_binding<'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Scoped<Executable>,
    environment: Option<Scoped<Environment>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let mut iterator_is_done = false;

    loop {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        let mut break_after_bind = false;

        let value = match instr.kind {
            Instruction::BindingPatternBind
            | Instruction::BindingPatternGetValue
            | Instruction::BindingPatternSkip => {
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).step_value(agent, gc),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                iterator_is_done = result.is_none();

                if instr.kind == Instruction::BindingPatternSkip {
                    continue;
                }
                result.unwrap_or(Value::Undefined).unbind().bind(gc.nogc())
            }
            Instruction::BindingPatternBindRest | Instruction::BindingPatternGetRestValue => {
                break_after_bind = true;
                if iterator_is_done {
                    array_create(agent, 0, 0, None, gc.nogc())
                        .unwrap()
                        .into_value()
                } else {
                    with_vm_gc(
                        agent,
                        vm,
                        |agent, mut gc| {
                            let mut iterator = ActiveIterator::new(agent, gc.nogc());
                            let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                            let rest = array_create(agent, 0, capacity, None, gc.nogc())
                                .unwrap()
                                .scope(agent, gc.nogc());
                            let mut idx = 0u32;
                            while let Some(result) = iterator
                                .step_value(agent, gc.reborrow())
                                .unbind()?
                                .bind(gc.nogc())
                            {
                                unwrap_try(try_create_data_property_or_throw(
                                    agent,
                                    rest.get(agent),
                                    PropertyKey::from(idx),
                                    result.unbind(),
                                    gc.nogc(),
                                ))
                                .unwrap();
                                idx += 1;
                            }
                            iterator_is_done = true;
                            // SAFETY: rest is not shared
                            JsResult::Ok(unsafe { rest.take(agent).into_value() })
                        },
                        gc.reborrow(),
                    )
                    .unbind()?
                }
            }
            Instruction::FinishBindingPattern => break,
            _ => unreachable!(),
        };

        match instr.kind {
            Instruction::BindingPatternBind | Instruction::BindingPatternBindRest => {
                let value = value.unbind();
                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let value = value.scope(agent, gc.nogc());
                        let binding_id = executable.fetch_identifier(
                            agent,
                            instr.args[0].unwrap() as usize,
                            gc.nogc(),
                        );
                        let lhs = resolve_binding(
                            agent,
                            binding_id.unbind(),
                            environment.as_ref().map(|v| v.get(agent)),
                            gc.reborrow(),
                        )
                        .unbind()?
                        .bind(gc.nogc());
                        if environment.is_none() {
                            put_value(
                                agent,
                                &lhs.unbind(),
                                // SAFETY: value is not shared.
                                unsafe { value.take(agent) },
                                gc.reborrow(),
                            )
                            .unbind()?
                            .bind(gc.nogc());
                        } else {
                            initialize_referenced_binding(
                                agent,
                                lhs.unbind(),
                                // SAFETY: value is not shared.
                                unsafe { value.take(agent) },
                                gc.reborrow(),
                            )
                            .unbind()?
                            .bind(gc.nogc());
                        }
                        JsResult::Ok(())
                    },
                    gc.reborrow(),
                )
                .unbind()?;
            }
            Instruction::BindingPatternGetValue | Instruction::BindingPatternGetRestValue => {
                execute_nested_simple_binding(
                    agent,
                    vm,
                    executable.clone(),
                    value.unbind(),
                    environment.clone(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
            }
            _ => unreachable!(),
        }

        if break_after_bind {
            break;
        }
    }

    // 8.6.2 Runtime Semantics: BindingInitialization
    // BindingPattern : ArrayBindingPattern
    // 3. If iteratorRecord.[[Done]] is false, return ? IteratorClose(iteratorRecord, result).
    // NOTE: `result` here seems to be UNUSED, which isn't a Value. This seems to be a spec bug.
    if !iterator_is_done {
        if let VmIteratorRecord::GenericIterator(iterator_record) = vm.get_active_iterator() {
            let iterator = iterator_record.iterator.unbind();
            with_vm_gc(
                agent,
                vm,
                |agent, gc| {
                    iterator_close_with_value(agent, iterator, Value::Undefined, gc)?;
                    JsResult::Ok(())
                },
                gc,
            )
            .unbind()?;
        }
    }

    Ok(())
}

pub(super) fn execute_simple_object_binding<'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Scoped<Executable>,
    object: Object,
    environment: Option<Scoped<Environment>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let object = object.scope(agent, gc.nogc());
    let mut excluded_names = PropertyKeySet::new(gc.nogc()).scope(agent, gc.nogc());

    loop {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        match instr.kind {
            Instruction::BindingPatternBind | Instruction::BindingPatternBindNamed => {
                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let binding_id = executable.fetch_identifier(
                            agent,
                            instr.args[0].unwrap() as usize,
                            gc.nogc(),
                        );
                        let property_key = if instr.kind == Instruction::BindingPatternBind {
                            binding_id.into()
                        } else {
                            let key_value = executable.fetch_constant(
                                agent,
                                instr.args[1].unwrap() as usize,
                                gc.nogc(),
                            );
                            // SAFETY: It should be impossible for binding pattern
                            // names to be integer strings.
                            unsafe { PropertyKey::from_value_unchecked(key_value) }
                        };

                        excluded_names.insert(agent, property_key);

                        let property_key = property_key.scope(agent, gc.nogc());
                        let lhs = resolve_binding(
                            agent,
                            binding_id.unbind(),
                            environment.as_ref().map(|v| v.get(agent)),
                            gc.reborrow(),
                        )
                        .unbind()?;
                        let v = get(
                            agent,
                            object.get(agent),
                            // SAFETY: property_key is not shared.
                            unsafe { property_key.take(agent) },
                            gc.reborrow(),
                        )
                        .unbind()?
                        .bind(gc.nogc());
                        if environment.is_none() {
                            put_value(agent, &lhs, v.unbind(), gc.reborrow()).unbind()?;
                        } else {
                            initialize_referenced_binding(agent, lhs, v.unbind(), gc.reborrow())
                                .unbind()?
                        }
                        JsResult::Ok(())
                    },
                    gc.reborrow(),
                )
                .unbind()?;
            }
            Instruction::BindingPatternGetValueNamed => {
                let v = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        // SAFETY: The constant was created using PropertyKey::from_str
                        // which checks for integer-ness, and then converted to Value
                        // without conversion, or is a floating point number string.
                        let property_key = unsafe {
                            PropertyKey::from_value_unchecked(executable.fetch_constant(
                                agent,
                                instr.args[0].unwrap() as usize,
                                gc.nogc(),
                            ))
                        };

                        excluded_names.insert(agent, property_key);
                        get(agent, object.get(agent), property_key.unbind(), gc)
                    },
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                execute_nested_simple_binding(
                    agent,
                    vm,
                    executable.clone(),
                    v.unbind(),
                    environment.clone(),
                    gc.reborrow(),
                )
                .unbind()?;
            }
            Instruction::BindingPatternBindRest => {
                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                        let binding_id = executable.fetch_identifier(
                            agent,
                            instr.args[0].unwrap() as usize,
                            gc.nogc(),
                        );
                        // TODO: Properly handle potential GC.
                        let lhs = resolve_binding(
                            agent,
                            binding_id.unbind(),
                            environment.as_ref().map(|v| v.get(agent)),
                            gc.reborrow(),
                        )
                        .unbind()?;
                        // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                        // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                        let rest_obj = copy_data_properties_into_object(
                            agent,
                            object.get(agent),
                            excluded_names,
                            gc.reborrow(),
                        )
                        .unbind()?
                        .bind(gc.nogc())
                        .into_value();
                        // 4. If environment is undefined, return ? PutValue(lhs, restObj).
                        // 5. Return ? InitializeReferencedBinding(lhs, restObj).
                        if environment.is_none() {
                            put_value(agent, &lhs, rest_obj.unbind(), gc.reborrow()).unbind()?;
                        } else {
                            initialize_referenced_binding(
                                agent,
                                lhs,
                                rest_obj.unbind(),
                                gc.reborrow(),
                            )
                            .unbind()?;
                        }
                        JsResult::Ok(())
                    },
                    gc.reborrow(),
                )
                .unbind()?;
                break;
            }
            Instruction::FinishBindingPattern => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}

pub(super) fn execute_nested_simple_binding<'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Scoped<Executable>,
    value: Value,
    environment: Option<Scoped<Environment>>,
    mut gc: GcScope<'a, '_>,
) -> JsResult<'a, ()> {
    let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
    match instr.kind {
        Instruction::BeginSimpleArrayBindingPattern => {
            let result = with_vm_gc(
                agent,
                vm,
                |agent, gc| VmIteratorRecord::from_value(agent, value, gc),
                gc.reborrow(),
            )
            .unbind()?
            .bind(gc.nogc());
            vm.iterator_stack.push(result.unbind());
            let result =
                execute_simple_array_binding(agent, vm, executable, environment.clone(), gc);
            vm.iterator_stack.pop().unwrap();
            result
        }
        Instruction::BeginSimpleObjectBindingPattern => {
            let object = to_object(agent, value, gc.nogc()).unbind()?.bind(gc.nogc());
            execute_simple_object_binding(
                agent,
                vm,
                executable,
                object.unbind(),
                environment.clone(),
                gc,
            )
        }
        _ => unreachable!(),
    }
}
