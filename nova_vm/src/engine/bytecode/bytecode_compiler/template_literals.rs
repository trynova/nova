// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! [13.2.8 Template Literals](https://tc39.es/ecma262/#sec-template-literals)

use std::ptr::NonNull;

use ahash::AHashMap;
use oxc_ast::ast;

use crate::{
    ecmascript::{
        builtins::{Array, array_create},
        execution::Agent,
        types::{BUILTIN_STRING_MEMORY, InternalMethods, InternalSlots, IntoValue, String, Value},
    },
    engine::{
        context::{Bindable, NoGcScope},
        unwrap_try,
    },
    heap::element_array::ElementDescriptor,
};

/// ### [13.2.8.4 GetTemplateObject ( templateLiteral )](https://tc39.es/ecma262/#sec-gettemplateobject)
///
/// The abstract operation GetTemplateObject takes argument templateLiteral (a
/// Parse Node) and returns an Array.
pub(super) fn get_template_object<'a>(
    agent: &mut Agent,
    template_literal: &ast::TemplateLiteral,
    gc: NoGcScope<'a, '_>,
) -> Array<'a> {
    // 1. Let realm be the current Realm Record.
    // 2. Let templateRegistry be realm.[[TemplateMap]].
    // 3. For each element e of templateRegistry, do
    // a. If e.[[Site]] is the same Parse Node as templateLiteral, then
    // i. Return e.[[Array]].
    // 4. Let rawStrings be the TemplateStrings of templateLiteral with argument true.
    // 5. Assert: rawStrings is a List of Strings.
    // 6. Let cookedStrings be the TemplateStrings of templateLiteral with argument false.
    // 7. Let count be the number of elements in the List cookedStrings.
    // 8. Assert: count ≤ 2**32 - 1.
    let len = template_literal.quasis.len();
    debug_assert!(len < 2usize.pow(32));
    // 9. Let template be ! ArrayCreate(count).
    let template = array_create(agent, len, len, None, gc).unwrap();
    // 10. Let rawObj be ! ArrayCreate(count).
    let raw_obj = array_create(agent, len, len, None, gc).unwrap();
    // 11. Let index be 0.
    // 12. Repeat, while index < count,

    // First, ensure that template Array descriptors exist.
    let template_storage = agent.heap.arrays[template]
        .elements
        .get_storage_mut(&mut agent.heap.elements);
    template_storage
        .descriptors
        .insert_entry(AHashMap::with_capacity(len));

    // Second, ensure that raw_obj Array descriptors exist and grab the
    // pointers to the values and descriptors.
    let raw_obj_storage = agent.heap.arrays[raw_obj]
        .elements
        .get_storage_mut(&mut agent.heap.elements);
    let mut raw_obj_values = NonNull::from(raw_obj_storage.values);
    let mut raw_obj_descriptors = NonNull::from(
        raw_obj_storage
            .descriptors
            .insert_entry(AHashMap::with_capacity(len))
            .into_mut(),
    );

    // Third, get the template values and descriptors; since they already
    // exist, this cannot move the raw_obj descriptors.
    let template_storage = agent.heap.arrays[template]
        .elements
        .get_storage_mut(&mut agent.heap.elements);
    let template_values = template_storage.values;
    let template_descriptors = template_storage
        .descriptors
        .insert_entry(AHashMap::with_capacity(len))
        .into_mut();

    // SAFETY: Finally, get the raw_obj values and descriptors; they cannot
    // have moved as per the above comment, and they're different from the
    // template values and descriptors so this is not mutable aliasing.
    let raw_obj_values = unsafe { raw_obj_values.as_mut() };
    let raw_obj_descriptors = unsafe { raw_obj_descriptors.as_mut() };

    let strings = &mut agent.heap.strings;
    let string_lookup_table = &mut agent.heap.string_lookup_table;
    let string_hasher = &mut agent.heap.string_hasher;
    let alloc_counter = &mut agent.heap.alloc_counter;

    for (prop, quasi) in template_literal.quasis.iter().enumerate() {
        // a. Let prop be ! ToString(𝔽(index)).
        // b. Let cookedValue be cookedStrings[index].
        let cooked_value = quasi.value.cooked.map_or(Value::Undefined, |cooked_value| {
            String::from_str_direct(
                strings,
                string_lookup_table,
                string_hasher,
                alloc_counter,
                cooked_value.as_str(),
                gc,
            )
            .into_value()
        });
        // d. Let rawValue be the String value rawStrings[index].
        let raw_value = String::from_str_direct(
            strings,
            string_lookup_table,
            string_hasher,
            alloc_counter,
            quasi.value.raw.as_str(),
            gc,
        )
        .into_value();
        // c. Perform ! DefinePropertyOrThrow(template, prop,
        //    PropertyDescriptor {
        //        [[Value]]: cookedValue,
        template_values[prop] = Some(cooked_value.unbind());
        //        [[Writable]]: false,
        //        [[Enumerable]]: true,
        //        [[Configurable]]: false
        template_descriptors.insert(
            prop as u32,
            ElementDescriptor::ReadOnlyEnumerableUnconfigurableData,
        );
        //    }).
        // e. Perform ! DefinePropertyOrThrow(rawObj, prop,
        //    PropertyDescriptor {
        //        [[Value]]: rawValue,
        raw_obj_values[prop] = Some(raw_value.unbind());
        //        [[Writable]]: false,
        //        [[Enumerable]]: true,
        //        [[Configurable]]: false
        raw_obj_descriptors.insert(
            prop as u32,
            ElementDescriptor::ReadOnlyEnumerableUnconfigurableData,
        );
        //    }).
        // f. Set index to index + 1.
    }
    // 13. Perform ! SetIntegrityLevel(rawObj, frozen).
    unwrap_try(raw_obj.try_prevent_extensions(agent, gc));
    let template_backing_object = template.get_or_create_backing_object(agent);
    let template_backing_storage =
        &mut agent.heap.objects[template_backing_object].property_storage;
    template_backing_storage.reserve(&mut agent.heap.elements, 1);
    let template_backing_storage_data =
        template_backing_storage.get_storage_uninit(&mut agent.heap.elements);
    // 14. Perform ! DefinePropertyOrThrow(template, "raw",
    template_backing_storage_data.keys[0] = Some(BUILTIN_STRING_MEMORY.raw.to_property_key());
    //     PropertyDescriptor {
    //         [[Value]]: rawObj,
    template_backing_storage_data.values[0] = Some(raw_obj.into_value().unbind());
    //         [[Writable]]: false,
    //         [[Enumerable]]: false,
    //         [[Configurable]]: false
    template_backing_storage_data
        .descriptors
        .or_insert_with(|| AHashMap::with_capacity(1))
        .insert(0, ElementDescriptor::ReadOnlyUnenumerableUnconfigurableData);
    //     }).
    template_backing_storage.len += 1;
    // 15. Perform ! SetIntegrityLevel(template, frozen).
    unwrap_try(template.try_prevent_extensions(agent, gc));
    // 16. Append the Record { [[Site]]: templateLiteral, [[Array]]: template }
    //     to realm.[[TemplateMap]].
    // 17. Return template.
    template
}
