// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::ecmascript::{
    builders::ordinary_object_builder::OrdinaryObjectBuilder,
    execution::{Agent, RealmIdentifier},
    types::{BUILTIN_STRING_MEMORY, IntoValue, String},
};

pub(crate) struct NativeErrorPrototypes;
impl NativeErrorPrototypes {
    pub(crate) fn create_intrinsic(agent: &mut Agent, realm: RealmIdentifier) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let error_prototype = intrinsics.error_prototype();
        let eval_constructor = intrinsics.eval_error();
        let eval_this = intrinsics.eval_error_prototype();
        let range_constructor = intrinsics.range_error();
        let range_this = intrinsics.range_error_prototype();
        let reference_constructor = intrinsics.reference_error();
        let reference_this = intrinsics.reference_error_prototype();
        let syntax_constructor = intrinsics.syntax_error();
        let syntax_this = intrinsics.syntax_error_prototype();
        let type_constructor = intrinsics.type_error();
        let type_this = intrinsics.type_error_prototype();
        let uri_constructor = intrinsics.uri_error();
        let uri_this = intrinsics.uri_error_prototype();

        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, eval_this)
            .with_property_capacity(3)
            .with_prototype(error_prototype)
            .with_constructor_property(eval_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.EvalError.into_value())
                    .build()
            })
            .build();
        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, range_this)
            .with_property_capacity(3)
            .with_prototype(error_prototype)
            .with_constructor_property(range_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.RangeError.into_value())
                    .build()
            })
            .build();
        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, reference_this)
            .with_property_capacity(3)
            .with_prototype(error_prototype)
            .with_constructor_property(reference_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.ReferenceError.into_value())
                    .build()
            })
            .build();
        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, syntax_this)
            .with_property_capacity(3)
            .with_prototype(error_prototype)
            .with_constructor_property(syntax_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.SyntaxError.into_value())
                    .build()
            })
            .build();
        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, type_this)
            .with_property_capacity(3)
            .with_prototype(error_prototype)
            .with_constructor_property(type_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.TypeError.into_value())
                    .build()
            })
            .build();
        OrdinaryObjectBuilder::new_intrinsic_object(agent, realm, uri_this)
            .with_property_capacity(3)
            .with_prototype(error_prototype)
            .with_constructor_property(uri_constructor)
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.message.into())
                    .with_value(String::EMPTY_STRING.into_value())
                    .build()
            })
            .with_property(|builder| {
                builder
                    .with_enumerable(false)
                    .with_key(BUILTIN_STRING_MEMORY.name.into())
                    .with_value(BUILTIN_STRING_MEMORY.URIError.into_value())
                    .build()
            })
            .build();
    }
}
