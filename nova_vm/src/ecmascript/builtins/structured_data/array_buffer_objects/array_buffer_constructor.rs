// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::{
        builders::builtin_function_builder::BuiltinFunctionBuilder,
        builtins::{ArgumentsList, Behaviour, Builtin, BuiltinGetter, BuiltinIntrinsicConstructor},
        execution::{Agent, JsResult, RealmIdentifier},
        types::{IntoObject, Object, PropertyKey, String, Value, BUILTIN_STRING_MEMORY},
    },
    heap::{IntrinsicConstructorIndexes, WellKnownSymbolIndexes},
};

pub(crate) struct ArrayBufferConstructor;
impl Builtin for ArrayBufferConstructor {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.ArrayBuffer;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Constructor(ArrayBufferConstructor::behaviour);
}
impl BuiltinIntrinsicConstructor for ArrayBufferConstructor {
    const INDEX: IntrinsicConstructorIndexes = IntrinsicConstructorIndexes::ArrayBuffer;
}

struct ArrayBufferIsView;
impl Builtin for ArrayBufferIsView {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.isView;

    const LENGTH: u8 = 1;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferConstructor::is_view);
}

struct ArrayBufferGetSpecies;
impl Builtin for ArrayBufferGetSpecies {
    const NAME: String<'static> = BUILTIN_STRING_MEMORY.get__Symbol_species_;

    const KEY: Option<PropertyKey> = Some(WellKnownSymbolIndexes::Species.to_property_key());

    const LENGTH: u8 = 0;

    const BEHAVIOUR: Behaviour = Behaviour::Regular(ArrayBufferConstructor::species);
}
impl BuiltinGetter for ArrayBufferGetSpecies {}

impl ArrayBufferConstructor {
    fn behaviour<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
        _new_target: Option<Object<'gen>>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn is_view<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    fn species<'gen>(
        _agent: &mut Agent<'gen>,
        _this_value: Value<'gen>,
        _arguments: ArgumentsList<'_, 'gen>,
    ) -> JsResult<'gen, Value<'gen>> {
        todo!()
    }

    pub(crate) fn create_intrinsic<'gen>(agent: &mut Agent<'gen>, realm: RealmIdentifier<'gen>) {
        let intrinsics = agent.get_realm(realm).intrinsics();
        let array_buffer_prototype = intrinsics.array_buffer_prototype();

        BuiltinFunctionBuilder::new_intrinsic_constructor::<ArrayBufferConstructor>(agent, realm)
            .with_property_capacity(3)
            .with_builtin_function_property::<ArrayBufferIsView>()
            .with_prototype_property(array_buffer_prototype.into_object())
            .with_builtin_function_getter_property::<ArrayBufferGetSpecies>()
            .build();
    }
}
