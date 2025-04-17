// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashSet;

use crate::{
    ecmascript::{
        abstract_operations::{
            operations_on_iterator_objects::iterator_close_with_value, operations_on_objects::get,
        },
        execution::{Agent, JsResult},
        types::{IntoValue, Object, PropertyKey, Value},
    },
    engine::{
        Scoped,
        bytecode::vm::{
            Environment, Executable, Instruction, Vm, VmIterator, array_create,
            copy_data_properties_into_object, initialize_referenced_binding, put_value,
            resolve_binding, to_object, try_create_data_property_or_throw,
        },
        context::{Bindable, GcScope},
        rootable::Scopable,
        unwrap_try,
    },
};

pub(super) fn execute_simple_array_binding<'a>(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Scoped<Executable>,
    mut iterator: VmIterator,
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
                let result = iterator
                    .step_value(agent, gc.reborrow())
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
                    rest.get(agent).into_value()
                }
            }
            Instruction::FinishBindingPattern => break,
            _ => unreachable!(),
        };

        match instr.kind {
            Instruction::BindingPatternBind | Instruction::BindingPatternBindRest => {
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
                let value = value.scope(agent, gc.nogc());
                let lhs = resolve_binding(
                    agent,
                    binding_id.unbind(),
                    environment.as_ref().map(|v| v.get(agent)),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                if environment.is_none() {
                    put_value(agent, &lhs.unbind(), value.get(agent), gc.reborrow())
                        .unbind()?
                        .bind(gc.nogc());
                } else {
                    initialize_referenced_binding(
                        agent,
                        lhs.unbind(),
                        value.get(agent),
                        gc.reborrow(),
                    )
                    .unbind()?
                    .bind(gc.nogc());
                }
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
        if let VmIterator::GenericIterator(iterator_record) = iterator {
            iterator_close_with_value(agent, iterator_record.iterator, Value::Undefined, gc)?;
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
    let mut excluded_names = AHashSet::new();

    loop {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        match instr.kind {
            Instruction::BindingPatternBind | Instruction::BindingPatternBindNamed => {
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
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

                excluded_names.insert(property_key.unbind());

                // TODO: Properly handle potential GC.
                let property_key = property_key.unbind();
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
                    property_key.unbind(),
                    gc.reborrow(),
                )
                .unbind()?
                .bind(gc.nogc());
                if environment.is_none() {
                    put_value(agent, &lhs, v.unbind(), gc.reborrow()).unbind()?;
                } else {
                    initialize_referenced_binding(agent, lhs, v.unbind(), gc.reborrow()).unbind()?
                }
            }
            Instruction::BindingPatternGetValueNamed => {
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

                excluded_names.insert(property_key.unbind());
                let v = get(
                    agent,
                    object.get(agent),
                    property_key.unbind(),
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
                // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
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
                    &excluded_names,
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
                    initialize_referenced_binding(agent, lhs, rest_obj.unbind(), gc.reborrow())
                        .unbind()?;
                }
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
            let new_iterator = VmIterator::from_value(agent, value, gc.reborrow())
                .unbind()?
                .bind(gc.nogc());
            execute_simple_array_binding(
                agent,
                vm,
                executable,
                new_iterator,
                environment.clone(),
                gc,
            )
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
