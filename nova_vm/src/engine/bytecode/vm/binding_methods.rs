// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use ahash::AHashSet;

use crate::{
    ecmascript::{
        abstract_operations::operations_on_iterator_objects::try_iterator_close,
        execution::{Agent, JsResult},
        types::{
            try_initialize_referenced_binding, try_put_value, IntoValue, Object, PropertyKey, Value,
        },
    },
    engine::{
        bytecode::vm::{
            array_create, to_object, try_create_data_property_or_throw, try_get,
            try_resolve_binding, EnvironmentIndex, Executable, Instruction, Vm, VmIterator,
        },
        context::NoGcScope,
    },
};

use super::try_copy_data_properties_into_object;

pub(super) fn execute_simple_array_binding<'a>(
    agent: &mut Agent,
    gc: NoGcScope<'a, '_>,
    vm: &mut Vm,
    executable: Executable,
    mut iterator: VmIterator<'a>,
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
                let result = iterator.step_value(agent, gc)?;
                iterator_is_done = result.is_none();

                if instr.kind == Instruction::BindingPatternSkip {
                    continue;
                }
                result.unwrap_or(Value::Undefined)
            }
            Instruction::BindingPatternBindRest | Instruction::BindingPatternGetRestValue => {
                break_after_bind = true;
                if iterator_is_done {
                    array_create(agent, gc, 0, 0, None).unwrap().into_value()
                } else {
                    let capacity = iterator.remaining_length_estimate(agent).unwrap_or(0);
                    let rest = array_create(agent, gc, 0, capacity, None).unwrap();
                    let mut idx = 0u32;
                    while let Some(result) = iterator.step_value(agent, gc)? {
                        try_create_data_property_or_throw(
                            agent,
                            gc,
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
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc);
                let lhs = {
                    let binding_id = binding_id.unbind();
                    try_resolve_binding(agent, gc, binding_id, environment)
                        .expect("TODO: Interleaved GC in simple array binding")
                        .unbind()
                        .bind(gc)
                };
                if environment.is_none() {
                    let lhs = lhs.unbind();
                    try_put_value(agent, gc, &lhs, value)
                        .expect("TODO: Interleaved GC in simple array binding")?;
                } else {
                    let lhs = lhs.unbind();
                    try_initialize_referenced_binding(agent, gc, lhs, value)
                        .expect("TODO: Interleaved GC in simple array binding")?;
                }
            }
            Instruction::BindingPatternGetValue | Instruction::BindingPatternGetRestValue => {
                execute_nested_simple_binding(agent, gc, vm, executable, value, environment)?;
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
            try_iterator_close(agent, gc, &iterator_record, Ok(Value::Undefined))
                .expect("TODO: Interleaved GC in simple array binding")?;
        }
    }

    Ok(())
}

pub(super) fn execute_simple_object_binding(
    agent: &mut Agent,
    gc: NoGcScope<'_, '_>,
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
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc);
                let property_key = if instr.kind == Instruction::BindingPatternBind {
                    binding_id.bind(gc).into()
                } else {
                    let key_value =
                        executable.fetch_constant(agent, instr.args[1].unwrap() as usize);
                    PropertyKey::try_from(key_value).unwrap()
                };
                excluded_names.insert(property_key);

                let lhs = if let Some(lhs) = try_resolve_binding(agent, gc, binding_id, environment)
                {
                    lhs
                } else {
                    // TODO: Root object and property_key
                    let binding_id = binding_id.unbind();
                    try_resolve_binding(agent, gc, binding_id, environment)
                        .expect("TODO: Interleaved GC in simple object binding")
                };
                let v = if let Some(v) = try_get(agent, gc, object, property_key) {
                    v
                } else {
                    // TODO: Root lhs
                    try_get(agent, gc, object, property_key)
                        .expect("TODO: Interleaved GC in simple object binding")
                };
                excluded_names.insert(property_key);
                if environment.is_none() {
                    try_put_value(agent, gc, &lhs, v)
                        .expect("TODO: Interleaved GC in simple object binding")?;
                } else {
                    try_initialize_referenced_binding(agent, gc, lhs, v)
                        .expect("TODO: Interleaved GC in simple object binding")?;
                }
            }
            Instruction::BindingPatternGetValueNamed => {
                let property_key = PropertyKey::from_value(
                    agent,
                    gc,
                    executable.fetch_constant(agent, instr.args[0].unwrap() as usize),
                )
                .unwrap();
                excluded_names.insert(property_key);
                let v = try_get(agent, gc, object, property_key)
                    .expect("TODO: Interleaved GC in simple object binding");
                execute_nested_simple_binding(agent, gc, vm, executable, v, environment)?;
            }
            Instruction::BindingPatternBindRest => {
                // 1. Let lhs be ? ResolveBinding(StringValue of BindingIdentifier, environment).
                let binding_id =
                    executable.fetch_identifier(agent, instr.args[0].unwrap() as usize, gc);
                let lhs = {
                    let binding_id = binding_id.unbind();
                    try_resolve_binding(agent, gc, binding_id, environment)
                        .expect("TODO: Interleaved GC in simple object binding")
                };
                // 2. Let restObj be OrdinaryObjectCreate(%Object.prototype%).
                // 3. Perform ? CopyDataProperties(restObj, value, excludedNames).
                let rest_obj =
                    try_copy_data_properties_into_object(agent, gc, object, &excluded_names)
                        .expect("TODO: Interleaved GC in simple object binding")
                        .into_value();
                // 4. If environment is undefined, return ? PutValue(lhs, restObj).
                // 5. Return ? InitializeReferencedBinding(lhs, restObj).
                if environment.is_none() {
                    try_put_value(agent, gc, &lhs, rest_obj)
                        .expect("TODO: Interleaved GC in simple object binding")?;
                } else {
                    try_initialize_referenced_binding(agent, gc, lhs, rest_obj)
                        .expect("TODO: Interleaved GC in simple object binding")?;
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
    gc: NoGcScope<'_, '_>,
    vm: &mut Vm,
    executable: Executable,
    value: Value,
    environment: Option<EnvironmentIndex>,
) -> JsResult<()> {
    let instr = executable.get_instruction(agent, &mut vm.ip).unwrap();
    match instr.kind {
        Instruction::BeginSimpleArrayBindingPattern => {
            let new_iterator = VmIterator::from_value(agent, gc, value)?;
            execute_simple_array_binding(agent, gc, vm, executable, new_iterator, environment)
        }
        Instruction::BeginSimpleObjectBindingPattern => {
            let object = to_object(agent, gc, value)?;
            execute_simple_object_binding(agent, gc, vm, executable, object, environment)
        }
        _ => unreachable!(),
    }
}
