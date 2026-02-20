// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        Agent, Environment, JsResult, Object, PropertyKey, PropertyKeySet, Value, array_create,
        copy_data_properties_into_object, get, initialize_referenced_binding, put_value,
        resolve_binding, to_object, try_create_data_property_or_throw, unwrap_try,
    },
    engine::{
        ActiveIterator, Bindable, Executable, GcScope, Instruction, Scopable, ScopableCollection,
        Scoped, Vm, VmIteratorRecord,
    },
};

use super::with_vm_gc;

pub(super) fn execute_simple_array_binding<'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Scoped<Executable>,
    environment: Option<Scoped<Environment>>,
    mut gc: GcScope,
) -> JsResult<'a, ()> {
    let mut iterator_is_done = false;

    loop {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        if agent.options.print_internals {
            eprintln!("Executing: {:?}", instr.kind);
        }
        let mut break_after_bind = false;

        let value = match instr.kind {
            Instruction::Debug => {
                if agent.options.print_internals {
                    eprintln!("Debug: {vm:#?}");
                }
                continue;
            }
            Instruction::BindingPatternBind
            | Instruction::BindingPatternBindToIndex
            | Instruction::BindingPatternGetValue
            | Instruction::BindingPatternSkip => {
                let result = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| ActiveIterator::new(agent, gc.nogc()).step_value(agent, gc),
                    gc.reborrow(),
                )?;

                result.map(|r| {
                    iterator_is_done = r.is_none();
                    r.unwrap_or(Value::Undefined)
                })
            }
            Instruction::BindingPatternBindRest
            | Instruction::BindingPatternGetRestValue
            | Instruction::BindingPatternBindRestToIndex => {
                break_after_bind = true;
                if iterator_is_done {
                    Ok(array_create(agent, 0, 0, None, gc.nogc()).unwrap().into())
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
                            while let Some(result) = iterator.step_value(agent, gc.reborrow())? {
                                unwrap_try(try_create_data_property_or_throw(
                                    agent,
                                    rest.get(agent).local(),
                                    PropertyKey::from(idx),
                                    result,
                                    None,
                                    gc.nogc(),
                                ));
                                idx += 1;
                            }
                            iterator_is_done = true;
                            // SAFETY: rest is not shared
                            let rest: Value = unsafe { rest.take(agent).local().into() };
                            JsResult::Ok(rest)
                        },
                        gc.reborrow(),
                    )?
                }
            }
            Instruction::FinishBindingPattern => break,
            _ => unreachable!(),
        };

        let value = match value {
            Ok(value) => value,
            Err(err) => {
                // IteratorStep threw an error: this means that the iterator is
                // immediately marked as closed and IteratorClose should not be
                // observably called by our error handler. To ensure that, we
                // replace the iterator with the empty slice iterator that will
                // simply ignore a return call.
                *vm.get_active_iterator_mut() = VmIteratorRecord::EmptySliceIterator;
                // Now we're ready to rethrow the error.
                return Err(err);
            }
        };

        match instr.kind {
            Instruction::BindingPatternSkip => {
                if break_after_bind {
                    break;
                } else {
                    continue;
                }
            }
            Instruction::BindingPatternBind | Instruction::BindingPatternBindRest => {
                let value = value;
                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let value = value.scope(agent, gc.nogc());
                        let binding_id =
                            executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                        let lhs = resolve_binding(
                            agent,
                            binding_id,
                            None,
                            environment.as_ref().map(|v| v.get(agent).local()),
                            gc.reborrow(),
                        )?;
                        if environment.is_none() {
                            put_value(
                                agent,
                                &lhs,
                                // SAFETY: value is not shared.
                                unsafe { value.take(agent).local() },
                                gc.reborrow(),
                            )?;
                        } else {
                            initialize_referenced_binding(
                                agent,
                                lhs,
                                // SAFETY: value is not shared.
                                unsafe { value.take(agent).local() },
                                gc.reborrow(),
                            )?;
                        }
                        JsResult::Ok(())
                    },
                    gc.reborrow(),
                )?;
            }
            Instruction::BindingPatternBindToIndex | Instruction::BindingPatternBindRestToIndex => {
                let stack_slot = instr.get_first_index();
                vm.stack[stack_slot] = value;
            }
            Instruction::BindingPatternGetValue | Instruction::BindingPatternGetRestValue => {
                execute_nested_simple_binding(
                    agent,
                    vm,
                    executable.clone(),
                    value,
                    environment.clone(),
                    gc.reborrow(),
                )?;
            }
            _ => unreachable!(),
        };

        if break_after_bind {
            break;
        }
    }

    // 8.6.2 Runtime Semantics: BindingInitialization
    // BindingPattern : ArrayBindingPattern
    // 3. If iteratorRecord.[[Done]] is false, return
    //    ? IteratorClose(iteratorRecord, result).
    // NOTE: `result` here is always UNUSED. We use `undefined` as a stand-in
    // since that way we don't need to implement a separate iterator_close.
    if !iterator_is_done {
        let iter = vm.get_active_iterator_mut();
        if iter.requires_return_call(agent, gc.nogc()) {
            let result = with_vm_gc(
                agent,
                vm,
                |agent, gc| {
                    ActiveIterator::new(agent, gc.nogc()).r#return(
                        agent,
                        Some(Value::Undefined),
                        gc,
                    )
                },
                gc,
            )?;
            vm.result = result;
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
    mut gc: GcScope,
) -> JsResult<'a, ()> {
    let object = object.scope(agent, gc.nogc());
    let mut excluded_names = PropertyKeySet::new(gc.nogc()).scope(agent, gc.nogc());

    loop {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        if agent.options.print_internals {
            eprintln!("Executing: {:?}", instr.kind);
        }
        match instr.kind {
            Instruction::Debug => {
                if agent.options.print_internals {
                    eprintln!("Debug: {vm:#?}");
                }
            }
            Instruction::BindingPatternBind | Instruction::BindingPatternBindNamed => {
                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        let binding_id =
                            executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                        let property_key = if instr.kind == Instruction::BindingPatternBind {
                            binding_id.into()
                        } else {
                            let key_value = executable.fetch_constant(
                                agent,
                                instr.get_second_index(),
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
                            binding_id,
                            None,
                            environment.as_ref().map(|v| v.get(agent).local()),
                            gc.reborrow(),
                        )?;
                        let v = get(
                            agent,
                            object.get(agent).local(),
                            // SAFETY: property_key is not shared.
                            unsafe { property_key.take(agent).local() },
                            gc.reborrow(),
                        )?;
                        if environment.is_none() {
                            put_value(agent, &lhs, v, gc.reborrow())?;
                        } else {
                            initialize_referenced_binding(agent, lhs, v, gc.reborrow())?
                        }
                        JsResult::Ok(())
                    },
                    gc.reborrow(),
                )?;
            }
            Instruction::BindingPatternBindToIndex => {
                let value = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        let key_value =
                            executable.fetch_constant(agent, instr.get_second_index(), gc.nogc());
                        // SAFETY: It should be impossible for binding pattern
                        // names to be integer strings.
                        let key_value = unsafe { PropertyKey::from_value_unchecked(key_value) };

                        excluded_names.insert(agent, key_value);

                        get(agent, object.get(agent).local(), key_value, gc)
                    },
                    gc.reborrow(),
                )?;
                let stack_slot = instr.get_first_index();
                vm.stack[stack_slot] = value;
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
                                instr.get_first_index(),
                                gc.nogc(),
                            ))
                        };

                        excluded_names.insert(agent, property_key);
                        get(agent, object.get(agent).local(), property_key, gc)
                    },
                    gc.reborrow(),
                )?;
                execute_nested_simple_binding(
                    agent,
                    vm,
                    executable.clone(),
                    v,
                    environment.clone(),
                    gc.reborrow(),
                )?;
            }
            Instruction::BindingPatternBindRest => {
                with_vm_gc(
                    agent,
                    vm,
                    |agent, mut gc| {
                        // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                        let binding_id =
                            executable.fetch_identifier(agent, instr.get_first_index(), gc.nogc());
                        let lhs = resolve_binding(
                            agent,
                            binding_id,
                            None,
                            environment.as_ref().map(|v| v.get(agent).local()),
                            gc.reborrow(),
                        )?;
                        // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                        // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                        let rest_obj = copy_data_properties_into_object(
                            agent,
                            object.get(agent).local(),
                            excluded_names,
                            gc.reborrow(),
                        )?;
                        // 4. If environment is undefined, return ? PutValue(lhs, restObj).
                        // 5. Return ? InitializeReferencedBinding(lhs, restObj).
                        if environment.is_none() {
                            put_value(agent, &lhs, rest_obj.into(), gc.reborrow())?;
                        } else {
                            initialize_referenced_binding(
                                agent,
                                lhs,
                                rest_obj.into(),
                                gc.reborrow(),
                            )?;
                        }
                        JsResult::Ok(())
                    },
                    gc.reborrow(),
                )?;
                break;
            }
            Instruction::BindingPatternBindRestToIndex => {
                let rest_obj = with_vm_gc(
                    agent,
                    vm,
                    |agent, gc| {
                        // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                        // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                        // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                        copy_data_properties_into_object(
                            agent,
                            object.get(agent).local(),
                            excluded_names,
                            gc,
                        )
                    },
                    gc.reborrow(),
                )?;
                let stack_slot = instr.get_first_index();
                vm.stack[stack_slot] = rest_obj.into();
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
    mut gc: GcScope,
) -> JsResult<'a, ()> {
    let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
    if agent.options.print_internals {
        eprintln!("Executing: {:?}", instr.kind);
    }
    match instr.kind {
        Instruction::BeginSimpleArrayBindingPattern => {
            let result = with_vm_gc(
                agent,
                vm,
                |agent, gc| VmIteratorRecord::from_value(agent, value, gc),
                gc.reborrow(),
            )?;
            vm.iterator_stack.push(result);
            let result = execute_simple_array_binding(
                agent,
                vm,
                executable,
                environment.clone(),
                gc.reborrow(),
            );
            let gc = gc.into_nogc();
            crate::engine::bind!(let result = result, gc);
            vm.pop_iterator(gc);
            result
        }
        Instruction::BeginSimpleObjectBindingPattern => {
            let object = to_object(agent, value, gc.nogc())?;
            execute_simple_object_binding(agent, vm, executable, object, environment.clone(), gc)
        }
        _ => unreachable!(),
    }
}
