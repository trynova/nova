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
    mut gc: GcScope<'_, '_>,
    vm: &mut Vm,
    executable: Executable,
    mut iterator: VmIterator,
    environment: Option<EnvironmentIndex>,
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
                    array_create(agent, gc.nogc(), 0, 0, None)
                        .unwrap()
                        .into_value()
                } else {
                    let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                    let rest = array_create(agent, gc.nogc(), 0, capacity, None).unwrap();
                    let mut idx = 0u32;
                    while let Some(result) = iterator.step_value(agent, gc.reborrow())? {
                        try_create_data_property_or_throw(
                            agent,
                            gc.nogc(),
                            rest,
                            PropertyKey::from(idx),
                            result,
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
                    let binding_id = binding_id.unbind();
                    resolve_binding(agent, gc.reborrow(), binding_id, environment)?
                        .unbind()
                        .bind(gc.nogc())
                };
                if environment.is_none() {
                    let lhs = lhs.unbind();
                    put_value(agent, gc.reborrow(), &lhs, value)?;
                } else {
                    let lhs = lhs.unbind();
                    initialize_referenced_binding(agent, gc.reborrow(), lhs, value)?;
                }
            }
            Instruction::BindingPatternGetValue | Instruction::BindingPatternGetRestValue => {
                execute_nested_simple_binding(
                    agent,
                    gc.reborrow(),
                    vm,
                    executable,
                    value,
                    environment,
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
            iterator_close(agent, gc.reborrow(), &iterator_record, Ok(Value::Undefined))?;
        }
    }

    Ok(())
}

pub(super) fn execute_simple_object_binding(
    agent: &mut Agent,
    mut gc: GcScope<'_, '_>,
    vm: &mut Vm,
    executable: Executable,
    object: Object,
    environment: Option<EnvironmentIndex>,
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
                let property_key = property_key.unbind();
                excluded_names.insert(property_key);

                let lhs = {
                    let binding_id = binding_id.unbind();
                    resolve_binding(agent, gc.reborrow(), binding_id, environment)?.unbind()
                };
                let v = get(agent, gc.reborrow(), object, property_key)?;
                if environment.is_none() {
                    put_value(agent, gc.reborrow(), &lhs, v)?;
                } else {
                    initialize_referenced_binding(agent, gc.reborrow(), lhs, v)?;
                }
            }
            Instruction::BindingPatternGetValueNamed => {
                let property_key = PropertyKey::from_value(
                    agent,
                    gc.nogc(),
                    executable.fetch_constant(agent, instr.args[0].unwrap() as usize),
                )
                .unwrap();
                let property_key = property_key.unbind();
                excluded_names.insert(property_key);
                let v = get(agent, gc.reborrow(), object, property_key)?;
                execute_nested_simple_binding(
                    agent,
                    gc.reborrow(),
                    vm,
                    executable,
                    v,
                    environment,
                )?;
            }
            Instruction::BindingPatternBindRest => {
                // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc.nogc());
                let lhs = {
                    let binding_id = binding_id.unbind();
                    resolve_binding(agent, gc.reborrow(), binding_id, environment)?.unbind()
                };
                // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                let rest_obj = copy_data_properties_into_object(
                    agent,
                    gc.reborrow(),
                    object,
                    &excluded_names,
                )?
                .into_value();
                // 4. If environment is undefined, return ? PutValue(lhs, restObj).
                // 5. Return ? InitializeReferencedBinding(lhs, restObj).
                if environment.is_none() {
                    put_value(agent, gc.reborrow(), &lhs, rest_obj)?;
                } else {
                    initialize_referenced_binding(agent, gc.reborrow(), lhs, rest_obj)?;
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
    mut gc: GcScope<'_, '_>,
    vm: &mut Vm,
    executable: Executable,
    value: Value,
    environment: Option<EnvironmentIndex>,
) -> JsResult<()> {
    let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
    match instr.kind {
        Instruction::BeginSimpleArrayBindingPattern => {
            let new_iterator = VmIterator::from_value(agent, gc.reborrow(), value)?;
            execute_simple_array_binding(
                agent,
                gc.reborrow(),
                vm,
                executable,
                new_iterator,
                environment,
            )
        }
        Instruction::BeginSimpleObjectBindingPattern => {
            let object = to_object(agent, gc.nogc(), value)?;
            execute_simple_object_binding(agent, gc.reborrow(), vm, executable, object, environment)
        }
        _ => unreachable!(),
    }
}
