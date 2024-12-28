// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashSet;

use crate::{
    ecmascript::{
        abstract_operations::operations_on_objects::get,
        execution::{Agent, JsResult},
        types::{IntoValue, Object, PropertyKey, Value},
    },
    engine::{
        bytecode::vm::{
            array_create, copy_data_properties_into_object, initialize_referenced_binding,
            iterator_close, put_value, resolve_binding, to_object,
            try_create_data_property_or_throw, EnvironmentIndex, Executable, Instruction, Vm,
            VmIterator,
        },
        context::GcScope,
    },
};

pub(super) fn execute_simple_array_binding(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Executable,
    mut iterator: VmIterator,
    environment: Option<EnvironmentIndex>,
    mut gc: GcScope<'_, '_>,
) -> JsResult<()> {
    let mut iterator_is_done = false;

    loop {
        let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
        let mut break_after_bind = false;

        let value = match instr.kind {
            Instruction::BindingPatternBind
            | Instruction::BindingPatternGetValue
            | Instruction::BindingPatternSkip => {
                let result = iterator.step_value(agent, gc.reborrow())?;
                iterator_is_done = result.is_none();

                if instr.kind == Instruction::BindingPatternSkip {
                    continue;
                }
                result.unwrap_or(Value::Undefined)
            }
            Instruction::BindingPatternBindRest | Instruction::BindingPatternGetRestValue => {
                break_after_bind = true;
                if iterator_is_done {
                    array_create(agent, 0, 0, None, gc.nogc())
                        .unwrap()
                        .into_value()
                } else {
                    let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                    let rest = array_create(agent, 0, capacity, None, gc.nogc()).unwrap();
                    let mut idx = 0u32;
                    while let Some(result) = iterator.step_value(agent, gc.reborrow())? {
                        try_create_data_property_or_throw(
                            agent,
                            rest,
                            PropertyKey::from(idx),
                            result,
                            gc.nogc(),
                        )
                        .unwrap()
                        .unwrap();
                        idx += 1;
                    }

                    iterator_is_done = true;
                    rest.into_value()
                }
            }
            Instruction::FinishBindingPattern => break,
            _ => unreachable!(),
        };

        match instr.kind {
            Instruction::BindingPatternBind | Instruction::BindingPatternBindRest => {
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
                let lhs = {
                    resolve_binding(agent, binding_id.unbind(), environment, gc.reborrow())?
                        .unbind()
                        .bind(gc.nogc())
                };
                if environment.is_none() {
                    put_value(agent, &lhs.unbind(), value, gc.reborrow())?;
                } else {
                    initialize_referenced_binding(agent, lhs.unbind(), value, gc.reborrow())?;
                }
            }
            Instruction::BindingPatternGetValue | Instruction::BindingPatternGetRestValue => {
                execute_nested_simple_binding(
                    agent,
                    vm,
                    executable,
                    value,
                    environment,
                    gc.reborrow(),
                )?;
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
            iterator_close(agent, &iterator_record, Ok(Value::Undefined), gc.reborrow())?;
        }
    }

    Ok(())
}

pub(super) fn execute_simple_object_binding(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Executable,
    object: Object,
    environment: Option<EnvironmentIndex>,
    mut gc: GcScope<'_, '_>,
) -> JsResult<()> {
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
                    let key_value =
                        executable.fetch_constant(agent, instr.args[1].unwrap() as usize);
                    PropertyKey::try_from(key_value).unwrap()
                };

                excluded_names.insert(property_key.unbind());

                // TODO: Properly handle potential GC.
                let property_key = property_key.unbind();
                let lhs = resolve_binding(agent, binding_id.unbind(), environment, gc.reborrow())?
                    .unbind();
                let v = get(agent, object, property_key.unbind(), gc.reborrow())?;
                if environment.is_none() {
                    put_value(agent, &lhs, v, gc.reborrow())?;
                } else {
                    initialize_referenced_binding(agent, lhs, v, gc.reborrow())?;
                }
            }
            Instruction::BindingPatternGetValueNamed => {
                let property_key = PropertyKey::from_value(
                    agent,
                    executable.fetch_constant(agent, instr.args[0].unwrap() as usize),
                    gc.nogc(),
                )
                .unwrap();

                excluded_names.insert(property_key.unbind());
                let v = get(agent, object, property_key.unbind(), gc.reborrow())?;
                execute_nested_simple_binding(
                    agent,
                    vm,
                    executable,
                    v,
                    environment,
                    gc.reborrow(),
                )?;
            }
            Instruction::BindingPatternBindRest => {
                // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
                let lhs = {
                    resolve_binding(agent, binding_id.unbind(), environment, gc.reborrow())?
                        .unbind()
                };
                // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                let rest_obj = copy_data_properties_into_object(
                    agent,
                    object,
                    &excluded_names,
                    gc.reborrow(),
                )?
                .into_value();
                // 4. If environment is undefined, return ? PutValue(lhs, restObj).
                // 5. Return ? InitializeReferencedBinding(lhs, restObj).
                if environment.is_none() {
                    put_value(agent, &lhs, rest_obj, gc.reborrow())?;
                } else {
                    initialize_referenced_binding(agent, lhs, rest_obj, gc.reborrow())?;
                }
                break;
            }
            Instruction::FinishBindingPattern => break,
            _ => unreachable!(),
        }
    }
    Ok(())
}

pub(super) fn execute_nested_simple_binding(
    agent: &mut Agent,
    vm: &mut Vm,
    executable: Executable,
    value: Value,
    environment: Option<EnvironmentIndex>,
    mut gc: GcScope<'_, '_>,
) -> JsResult<()> {
    let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
    match instr.kind {
        Instruction::BeginSimpleArrayBindingPattern => {
            let new_iterator = VmIterator::from_value(agent, value, gc.reborrow())?;
            execute_simple_array_binding(
                agent,
                vm,
                executable,
                new_iterator,
                environment,
                gc.reborrow(),
            )
        }
        Instruction::BeginSimpleObjectBindingPattern => {
            let object = to_object(agent, value, gc.nogc())?;
            execute_simple_object_binding(agent, vm, executable, object, environment, gc.reborrow())
        }
        _ => unreachable!(),
    }
}
