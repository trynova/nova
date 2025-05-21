// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::Cell, collections::hash_map::Entry};

use crate::{
    ecmascript::{
        execution::agent::ExceptionType,
        syntax_directed_operations::{
            function_definitions::CompileFunctionBodyData,
            scope_analysis::{
                LexicallyScopedDeclaration, VarScopedDeclaration,
                class_static_block_lexically_scoped_declarations,
                class_static_block_var_declared_names, class_static_block_var_scoped_declarations,
            },
        },
        types::{BUILTIN_STRING_MEMORY, String, Value},
    },
    engine::{
        CompileContext, CompileEvaluation, FunctionExpression, Instruction,
        NamedEvaluationParameter, SendableRef, is_reference,
    },
};
use ahash::{AHashMap, AHashSet};
use oxc_ast::ast::{self, MethodDefinitionKind};
use oxc_ecmascript::{BoundNames, PrivateBoundIdentifiers, PropName};

use super::{IndexType, is_anonymous_function_definition};

impl<'s> CompileEvaluation<'s> for ast::Class<'s> {
    /// ClassTail : ClassHeritage_opt { ClassBody_opt }
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        ctx.add_instruction(Instruction::Debug);
        let anonymous_class_name = ctx.name_identifier.take();

        // 1. Let env be the LexicalEnvironment of the running execution context.
        // 2. Let classEnv be NewDeclarativeEnvironment(env).
        // Note: The specification doesn't enter the declaration here, but
        // no user code is run between here and first enter.
        ctx.enter_lexical_scope();

        // 3. If classBinding is not undefined, then
        let mut has_class_name_on_stack = false;
        let mut class_identifier = None;
        if let Some(class_binding) = &self.id {
            // a. Perform ! classEnv.CreateImmutableBinding(classBinding, true).
            let identifier = String::from_str(ctx.agent, class_binding.name.as_str(), ctx.gc);
            class_identifier = Some(identifier);
            ctx.add_instruction_with_identifier(Instruction::CreateImmutableBinding, identifier);
        } else if let Some(anonymous_class_name) = anonymous_class_name {
            has_class_name_on_stack = true;
            match anonymous_class_name {
                NamedEvaluationParameter::Result => {
                    ctx.add_instruction(Instruction::Load);
                }
                NamedEvaluationParameter::Stack => {
                    ctx.add_instruction(Instruction::StoreCopy);
                    ctx.add_instruction(Instruction::Load);
                }
            }
        }

        #[derive(Clone, Copy, PartialEq, Eq)]
        enum PrivateFieldKind {
            Field,
            Method,
            Get,
            Set,
            GetSet,
        }

        impl From<Option<MethodDefinitionKind>> for PrivateFieldKind {
            fn from(value: Option<MethodDefinitionKind>) -> Self {
                match value {
                    Some(MethodDefinitionKind::Constructor) => unreachable!(),
                    Some(MethodDefinitionKind::Get) => Self::Get,
                    Some(MethodDefinitionKind::Set) => Self::Set,
                    Some(MethodDefinitionKind::Method) => Self::Method,
                    None => Self::Field,
                }
            }
        }

        let mut has_constructor_parent = false;

        // 7. If ClassHeritage is present, then
        if let Some(super_class) = &self.super_class {
            if super_class.is_null() {
                // Note: If the super class is null, we can skip evaluating it
                // on the stack and just set the prototype to null.
                // Hence we do not need to set has_constructor_parent true.
                // But we do need to remember that this is still a derived
                // class.
                ctx.add_instruction(Instruction::ObjectCreate);
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Null);
                ctx.add_instruction(Instruction::ObjectSetPrototype);
            } else {
                // Constructor parent is known only at runtime, so we must
                // consider it.
                has_constructor_parent = true;
                // a. Set the running execution context's LexicalEnvironment to classEnv.
                // b. NOTE: The running execution context's PrivateEnvironment
                //    is outerPrivateEnvironment when evaluating ClassHeritage.
                // c. Let superclassRef be Completion(Evaluation of ClassHeritage).
                super_class.compile(ctx);
                // d. Set the running execution context's LexicalEnvironment to env.
                // Note: We are not following specification properly here:
                // The GetValue here and EvaluatePropertyAccessWithIdentifierKey
                // below should be performed in the parent environment. We do
                // them in classEnv. Whether there's a difference I don't know.
                if is_reference(super_class) {
                    // e. Let superclass be ? GetValue(? superclassRef).
                    ctx.add_instruction(Instruction::GetValue);
                }
                // f. If superclass is null, then
                ctx.add_instruction(Instruction::LoadCopy);
                ctx.add_instruction(Instruction::IsNull);
                let jump_to_else = ctx.add_instruction_with_jump_slot(Instruction::JumpIfNot);
                // i. Let protoParent be null.
                // Note: We already have null on the stack.
                // 9. Let proto be OrdinaryObjectCreate(protoParent).
                ctx.add_instruction(Instruction::ObjectCreate);
                // Now we have proto on the stack followed be null (protoParent).
                ctx.add_instruction(Instruction::Swap);
                // Now we have null (protoParent) followed by proto.
                ctx.add_instruction(Instruction::Load);
                // Now null is in the result register and proto is at the top of
                // the stack.
                ctx.add_instruction(Instruction::ObjectSetPrototype);
                // ii. Let constructorParent be %Function.prototype%.
                ctx.add_instruction_with_constant(
                    Instruction::LoadConstant,
                    ctx.agent
                        .current_realm_record()
                        .intrinsics()
                        .function_prototype(),
                );

                // Note: constructorParent is now at the top of the stack, and
                // proto is after it. We can jump to the end.
                let jump_over_else = ctx.add_instruction_with_jump_slot(Instruction::Jump);

                ctx.set_jump_target_here(jump_to_else);
                // g. Else if IsConstructor(superclass) is false, then
                ctx.add_instruction(Instruction::StoreCopy);
                ctx.add_instruction(Instruction::IsConstructor);
                let jump_over_throw = ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);
                // Pop the superclass from the stack.
                ctx.add_instruction(Instruction::Store);
                // i. Throw a TypeError exception.
                let error_message = String::from_static_str(
                    ctx.agent,
                    "class heritage is not a constructor",
                    ctx.gc,
                );
                ctx.add_instruction_with_constant(Instruction::StoreConstant, error_message);
                ctx.add_instruction_with_immediate(
                    Instruction::ThrowError,
                    ExceptionType::TypeError as usize,
                );

                // h. Else,
                ctx.set_jump_target_here(jump_over_throw);
                // i. Let protoParent be ? Get(superclass, "prototype").
                ctx.add_instruction(Instruction::StoreCopy);
                ctx.add_instruction_with_identifier(
                    Instruction::EvaluatePropertyAccessWithIdentifierKey,
                    BUILTIN_STRING_MEMORY.prototype,
                );
                ctx.add_instruction(Instruction::GetValue);

                // Note: superclass is now at the top of the stack, and protoParent
                // in the result register.

                // ii. If protoParent is not an Object and protoParent is not null,
                ctx.add_instruction(Instruction::LoadCopy);
                ctx.add_instruction(Instruction::IsObject);
                let jump_over_null_check_and_throw =
                    ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

                ctx.add_instruction(Instruction::StoreCopy);
                ctx.add_instruction(Instruction::IsNull);
                let jump_over_throw = ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

                // ... throw a TypeError exception.
                let error_message = String::from_static_str(
                    ctx.agent,
                    "class heritage is not an object or null",
                    ctx.gc,
                );
                ctx.add_instruction_with_constant(Instruction::StoreConstant, error_message);
                ctx.add_instruction_with_immediate(
                    Instruction::ThrowError,
                    ExceptionType::TypeError as usize,
                );
                ctx.set_jump_target_here(jump_over_throw);
                ctx.set_jump_target_here(jump_over_null_check_and_throw);

                // Note: protoParent is now at the top of the stack, and superclass
                // is after it.

                // 9. Let proto be OrdinaryObjectCreate(protoParent)
                ctx.add_instruction(Instruction::ObjectCreate);
                ctx.add_instruction(Instruction::Swap);
                // Now protoParent is at the top of the stack, proto is second, and
                // superclass is third.
                ctx.add_instruction(Instruction::Store);
                ctx.add_instruction(Instruction::ObjectSetPrototype);

                // Now proto is first and superclass second.
                ctx.add_instruction(Instruction::Swap);
                // Now superclass is first and proto is second.

                // iii. Let constructorParent be superclass.
                ctx.set_jump_target_here(jump_over_else);
                // Now constructorParent is at the top of the stack, and
                // proto is after it.
            }
        } else {
            // a. Let protoParent be %Object.prototype%.
            // 9. Let proto be OrdinaryObjectCreate(protoParent).
            ctx.add_instruction(Instruction::ObjectCreate);
            // b. Let constructorParent be %Function.prototype%.
            // We omit constructor parent as we statically know it is
            // uninteresting.
        }

        // 10. If ClassBody is not present, let constructor be empty.
        // 11. Else, let constructor be the ConstructorMethod of ClassBody.
        let constructor = self.body.body.iter().find_map(|class_element| {
            if let ast::ClassElement::MethodDefinition(c) = class_element {
                if c.kind.is_constructor() {
                    Some(c)
                } else {
                    None
                }
            } else {
                None
            }
        });

        // Note: We have returned to classEnv if we ever left it.
        // 12. Set the running execution context's LexicalEnvironment to classEnv.
        // 13. Set the running execution context's PrivateEnvironment to classPrivateEnvironment.
        let private_bound_identifiers = self
            .body
            .body
            .iter()
            .filter_map(|class_element| {
                class_element.private_bound_identifiers().map(|p| {
                    (
                        p.name.as_str(),
                        class_element,
                        PrivateFieldKind::from(class_element.method_definition_kind()),
                    )
                })
            })
            .collect::<Box<[_]>>();
        let mut private_name_lookup_map = AHashMap::with_capacity(private_bound_identifiers.len());

        let mut instance_private_fields = vec![];
        let mut instance_private_methods = vec![];
        let mut static_private_fields = vec![];
        let mut static_private_methods = vec![];
        let mut instance_private_field_count = 0;
        let mut instance_private_method_count = 0;
        let mut static_private_field_count = 0;
        let mut static_private_method_count = 0;
        // OPTIMISATION: do not create a private environment if it is going to be empty.
        let enter_private_environment = !private_bound_identifiers.is_empty();
        // 6. If ClassBody is present, then
        if enter_private_environment {
            assert!(u32::try_from(private_bound_identifiers.len()).is_ok());
            // 4. Let outerPrivateEnvironment be the running execution context's PrivateEnvironment.
            // 5. Let classPrivateEnvironment be NewPrivateEnvironment(outerPrivateEnvironment).
            // a. For each String dn of the PrivateBoundIdentifiers of ClassBody, do
            for (dn, class_element, kind) in private_bound_identifiers.into_iter() {
                let i: u32;
                if let ast::ClassElement::PropertyDefinition(prop) = class_element {
                    if class_element.r#static() {
                        i = static_private_field_count;
                        static_private_field_count += 1;
                        static_private_fields.push((dn, prop.value.as_ref()));
                    } else {
                        i = instance_private_field_count;
                        instance_private_field_count += 1;
                        instance_private_fields.push((dn, prop.value.as_ref()));
                    }
                } else if let ast::ClassElement::MethodDefinition(method) = class_element {
                    if class_element.r#static() {
                        i = static_private_method_count;
                        static_private_method_count += 1;
                        static_private_methods.push((dn, &**method));
                    } else {
                        i = instance_private_method_count;
                        instance_private_method_count += 1;
                        instance_private_methods.push((dn, &**method));
                    }
                } else {
                    unreachable!()
                }
                // i. If classPrivateEnvironment.[[Names]] contains a Private
                //    Name pn such that pn.[[Description]] is dn, then
                match private_name_lookup_map.entry(dn) {
                    Entry::Occupied(mut pn) => {
                        // 1. Assert: This is only possible for getter/setter pairs.
                        let (dup_kind, i) = *pn.get();
                        assert!(
                            dup_kind == PrivateFieldKind::Get && kind == PrivateFieldKind::Set
                                || dup_kind == PrivateFieldKind::Set
                                    && kind == PrivateFieldKind::Get
                        );
                        // Note: this change of kind from Get/Set to GetSet
                        // makes the pair checking exclusive.
                        pn.insert((PrivateFieldKind::GetSet, i));
                    }
                    // ii. Else,
                    Entry::Vacant(slot) => {
                        // 1. Let name be a new Private Name whose [[Description]] is dn.
                        // 2. Append name to classPrivateEnvironment.[[Names]].
                        slot.insert((kind, i));
                    }
                }
            }
            ctx.enter_private_scope(private_name_lookup_map.len());
        }

        // Before calling CreateDefaultConstructor we need to smuggle the
        // className to the top of the stack.
        // The current stack is either:
        // - [proto, class_name]
        // - [proto]
        // - [constructor_parent, proto, class_name]
        // - [constructor_parent, proto]
        if has_class_name_on_stack {
            ctx.add_instruction(Instruction::Debug);
            if has_constructor_parent {
                // stack: [constructor_parent, proto, class_name]
                ctx.add_instruction(Instruction::Store);
                // stack: [proto, class_name]
                ctx.add_instruction(Instruction::Swap);
                // stack: [class_name, proto]
                ctx.add_instruction(Instruction::Load);
                // stack: [constructor_parent, class_name, proto]
                ctx.add_instruction(Instruction::Swap);
                // stack: [class_name, constructor_parent, proto]
            } else {
                // stack: [proto, class_name]
                ctx.add_instruction(Instruction::Swap);
                // stack: [class_name, proto]
            }
        } else {
            // We don't have the class name on the stack, so we can just
            // push it there.
            ctx.add_instruction_with_constant(
                Instruction::LoadConstant,
                class_identifier.unwrap_or(String::EMPTY_STRING),
            );
            // stack: [class_name, constructor_parent?, proto]
        }

        // 14. If constructor is not empty, then
        let constructor_index = if let Some(constructor) = constructor {
            // a. Let constructorInfo be ! DefineMethod of constructor with arguments proto and constructorParent.
            define_constructor_method(ctx, constructor, has_constructor_parent)
            // b. Let F be constructorInfo.[[Closure]].
            // c. Perform MakeClassConstructor(F).
            // d. Perform SetFunctionName(F, className).
        } else {
            // 15. Else,
            // a. Let defaultConstructor be a new Abstract Closure with no
            //    parameters that captures nothing and performs the following
            //    steps when called:
            // ...
            // b. Let F be CreateBuiltinFunction(defaultConstructor, 0,
            //    className, « [[ConstructorKind]], [[SourceText]] », the
            //    current Realm Record, constructorParent).
            let index = ctx.get_next_class_initializer_index();
            ctx.add_instruction_with_immediate(
                Instruction::ClassDefineDefaultConstructor,
                index.into(),
            );
            index
        };

        // result: F
        // stack: [proto]
        ctx.add_instruction(Instruction::Load);
        // stack: [constructor, proto]

        let has_instance_private_fields_or_methods =
            !instance_private_fields.is_empty() || !instance_private_methods.is_empty();

        // Note: These steps have been performed by ClassDefineConstructor or
        // ClassDefineDefaultConstructor.
        // 16. Perform MakeConstructor(F, false, proto).
        // 17. If ClassHeritage is present, set F.[[ConstructorKind]] to derived.
        // 18. Perform ! ObjectDefineMethod(proto, "constructor", F, false).
        for (key, _) in instance_private_fields {
            let key = ctx.create_identifier(key);
            ctx.add_instruction_with_identifier_and_immediate(
                Instruction::ClassDefinePrivateProperty,
                key,
                // instance
                false.into(),
            );
        }
        for (key, method) in instance_private_methods {
            define_private_method(key, method, false, ctx);
        }
        for (key, _) in static_private_fields {
            let key = ctx.create_identifier(key);
            ctx.add_instruction_with_identifier_and_immediate(
                Instruction::ClassDefinePrivateProperty,
                key,
                // static
                true.into(),
            );
        }
        for (key, method) in static_private_methods {
            define_private_method(key, method, true, ctx);
        }

        // During binding of methods, we need to swap between the proto and
        // the constructor being on top of the stack. This is because the
        // top of the stack is the object that the method is being bound to.
        let proto_is_on_top = Cell::new(false);
        let swap_to_proto = |ctx: &mut CompileContext| {
            if !proto_is_on_top.get() {
                ctx.add_instruction(Instruction::Swap);
                proto_is_on_top.set(true);
            }
        };
        let swap_to_constructor = |ctx: &mut CompileContext| {
            if proto_is_on_top.get() {
                ctx.add_instruction(Instruction::Swap);
                proto_is_on_top.set(false);
            }
        };

        // 19. If ClassBody is not present, let elements be a new empty List.
        // 20. Else, let elements be the NonConstructorElements of ClassBody.
        // 21. Let instancePrivateMethods be a new empty List.
        // 22. Let staticPrivateMethods be a new empty List.
        // 23. Let instanceFields be a new empty List.
        let mut instance_fields = vec![];
        // 24. Let staticElements be a new empty List.
        let mut static_elements = vec![];
        // 25. For each ClassElement e of elements, do
        let mut computed_field_initialiser_count: u32 = 0;
        for e in self.body.body.iter() {
            let is_static: bool;
            let element = match e {
                ast::ClassElement::StaticBlock(static_block) => {
                    // Note: Evaluating a ClassStaticBlockDefinition just
                    // creates a function that will be immediately invoked
                    // later. The function is never visible to JavaScript code
                    // and thus doesn't _actually_ need to get created here.
                    is_static = true;
                    PropertyInitializerField::StaticBlock(static_block)
                }
                // a. If IsStatic of e is false, then
                // i. Let element be Completion(ClassElementEvaluation of e with argument proto).
                // b. Else,
                // i. Let element be Completion(ClassElementEvaluation of e with argument F).
                ast::ClassElement::MethodDefinition(method_definition) => {
                    if method_definition.kind.is_constructor()
                        || method_definition.private_bound_identifiers().is_some()
                    {
                        // We have already separated and created these earlier.
                        continue;
                    }
                    let is_static = method_definition.r#static;
                    if is_static {
                        swap_to_constructor(ctx);
                    } else {
                        swap_to_proto(ctx);
                    }
                    define_method(method_definition, ctx);
                    continue;
                }
                ast::ClassElement::PropertyDefinition(prop) => {
                    is_static = prop.r#static;
                    if let ast::PropertyKey::StaticIdentifier(key) = &prop.key {
                        // Fields with static initialisers cannot cause errors
                        // at this stage: we simply store the key and the value
                        // expression for later compilation into the
                        // constructor init code.
                        PropertyInitializerField::Field((key.name.as_str(), prop.value.as_ref()))
                    } else if let ast::PropertyKey::PrivateIdentifier(key) = &prop.key {
                        // Private fields likewise cannot cause errors at this
                        // stage. Interestingly, we don't need to know the
                        // [[Description]] string of the  of the private field
                        // when initialising it, so we get rid of that here.
                        PropertyInitializerField::Private((
                            key.name.as_str(),
                            private_name_lookup_map.get(key.name.as_str()).unwrap().1,
                            prop.value.as_ref(),
                        ))
                    } else {
                        // Computed fields must compute their name immediately
                        // but the value must be computed later.
                        let computed_field_id = computed_field_initialiser_count;
                        computed_field_initialiser_count += 1;
                        compile_computed_field_name(
                            ctx,
                            computed_field_id,
                            prop.key.as_expression().unwrap(),
                            prop.value.as_ref(),
                        )
                    }
                }
                #[cfg(feature = "typescript")]
                ast::ClassElement::AccessorProperty(_) => todo!(),
                #[cfg(not(feature = "typescript"))]
                ast::ClassElement::AccessorProperty(_) => unreachable!(),
                #[cfg(feature = "typescript")]
                ast::ClassElement::TSIndexSignature(_) => todo!(),
                #[cfg(not(feature = "typescript"))]
                ast::ClassElement::TSIndexSignature(_) => unreachable!(),
            };
            // c. If element is an abrupt completion, then
            //     i. Set the running execution context's LexicalEnvironment to env.
            //     ii. Set the running execution context's PrivateEnvironment to outerPrivateEnvironment.
            //     iii. Return ? element.
            // d. Set element to ! element.
            // e. If element is a PrivateElement, then
            //     i. Assert: element.[[Kind]] is either method or accessor.
            //     ii. If IsStatic of e is false, let container be instancePrivateMethods.
            //     iii. Else, let container be staticPrivateMethods.
            //     iv. If container contains a PrivateElement pe such that pe.[[Key]] is element.[[Key]], then
            //         1. Assert: element.[[Kind]] and pe.[[Kind]] are both accessor.
            //         2. If element.[[Get]] is undefined, then
            //             a. Let combined be PrivateElement { [[Key]]: element.[[Key]], [[Kind]]: accessor, [[Get]]: pe.[[Get]], [[Set]]: element.[[Set]] }.
            //         3. Else,
            //             a. Let combined be PrivateElement { [[Key]]: element.[[Key]], [[Kind]]: accessor, [[Get]]: element.[[Get]], [[Set]]: pe.[[Set]] }.
            //         4. Replace pe in container with combined.
            //     v. Else,
            //         1. Append element to container.
            // f. Else if element is a ClassFieldDefinition Record, then
            //     i. If IsStatic of e is false, append element to instanceFields.
            //     ii. Else, append element to staticElements.
            // g. Else if element is a ClassStaticBlockDefinition Record, then
            //     i. Append element to staticElements.
            if is_static {
                static_elements.push(element);
            } else {
                instance_fields.push(element);
            }
        }
        // Drop proto from stack: It is no longer needed.
        swap_to_proto(ctx);
        ctx.add_instruction(Instruction::Store);

        // stack: [constructor]

        // 26. Set the running execution context's LexicalEnvironment to env.
        // Note: We do not exit classEnv here. First, classBinding is
        // initialized in classEnv. Second, the static elements are "functions"
        // that were "created" in the classEnv, and they are "evaluated" below.
        // The evaluation is done inline so we need the classEnv to be active,
        // and the "function environments" to be created in it.

        // 27. If classBinding is not undefined, then
        // Note: The classBinding needs to be initialized in classEnv, as any
        // class method calls access the classBinding through the classEnv.
        if let Some(class_binding) = class_identifier {
            // a. Perform ! classEnv.InitializeBinding(classBinding, F).
            ctx.add_instruction(Instruction::StoreCopy);
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, class_binding);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }

        // 28. Set F.[[PrivateMethods]] to instancePrivateMethods.
        // 29. Set F.[[Fields]] to instanceFields.
        if has_instance_private_fields_or_methods || !instance_fields.is_empty() {
            let mut constructor_ctx = CompileContext::new(ctx.agent, ctx.gc);
            // Resolve 'this' into the stack.
            constructor_ctx.add_instruction(Instruction::ResolveThisBinding);
            constructor_ctx.add_instruction(Instruction::Load);
            if has_instance_private_fields_or_methods {
                constructor_ctx.add_instruction(Instruction::ClassInitializePrivateElements);
            }
            for ele in instance_fields {
                match ele {
                    PropertyInitializerField::Field((property_key, value)) => {
                        compile_class_static_id_field(property_key, value, &mut constructor_ctx);
                    }
                    PropertyInitializerField::Computed((key_id, value)) => {
                        compile_class_computed_field(key_id, value, &mut constructor_ctx);
                    }
                    PropertyInitializerField::Private((description, private_identifier, value)) => {
                        compile_class_private_field(
                            description,
                            private_identifier,
                            value,
                            &mut constructor_ctx,
                        );
                    }
                    PropertyInitializerField::StaticBlock(_) => unreachable!(),
                }
            }
            // Pop the `this` value off the stack.
            constructor_ctx.add_instruction(Instruction::Store);
            if let Some(constructor) = constructor {
                let constructor_data = CompileFunctionBodyData {
                    body: constructor.value.body.as_ref().unwrap(),
                    params: &constructor.value.params,
                    is_concise_body: false,
                    is_lexical: false,
                    // Class code is always strict.
                    is_strict: true,
                };
                constructor_ctx.compile_function_body(constructor_data);
                let executable = constructor_ctx.finish();
                ctx.set_function_expression_bytecode(constructor_index, executable);
            } else {
                let executable = constructor_ctx.finish();
                ctx.add_class_initializer_bytecode(executable, has_constructor_parent);
            }
        } else if constructor.is_none() {
            ctx.add_class_initializer(has_constructor_parent);
        }
        // 30. For each PrivateElement method of staticPrivateMethods, do
        //     a. Perform ! PrivateMethodOrAccessorAdd(F, method).
        // Note: this has already been performed by the
        // ClassInitializePrivateElements instruction earlier.
        // 31. For each element elementRecord of staticElements, do
        for element_record in static_elements {
            match element_record {
                // a. If elementRecord is a ClassFieldDefinition Record, then
                PropertyInitializerField::StaticBlock(static_block) => {
                    // i. Let result be Completion(DefineField(F, elementRecord)).
                    static_block.compile(ctx);
                }
                // b. Else,
                // i. Assert: elementRecord is a ClassStaticBlockDefinition Record.
                // ii. Let result be Completion(Call(elementRecord.[[BodyFunction]], F)).
                PropertyInitializerField::Field((property_key, value)) => {
                    compile_class_static_id_field(property_key, value, ctx);
                }
                PropertyInitializerField::Computed((key_id, value)) => {
                    compile_class_computed_field(key_id, value, ctx);
                }
                PropertyInitializerField::Private((description, private_identifier, value)) => {
                    // Note: Static private fields follow third after private
                    // fields and methods, so their identifiers are offset.
                    let private_identifier = instance_private_field_count
                        + instance_private_method_count
                        + private_identifier;
                    compile_class_private_field(description, private_identifier, value, ctx);
                }
            }
            // c. If result is an abrupt completion, then
            //     i. Set the running execution context's PrivateEnvironment to outerPrivateEnvironment.
            //     ii. Return ? result.
        }
        // Note: We finally leave classEnv here. See step 26.
        ctx.exit_lexical_scope();
        // 32. Set the running execution context's PrivateEnvironment to outerPrivateEnvironment.
        // 33. Return F.
        if enter_private_environment {
            ctx.exit_private_scope();
        }

        // 15.7.15 Runtime Semantics: BindingClassDeclarationEvaluation
        // ClassDeclaration: class BindingIdentifier ClassTail
        if self.is_declaration() {
            let class_identifier = class_identifier.unwrap();
            // 4. Let env be the running execution context's LexicalEnvironment.
            // 5. Perform ? InitializeBoundName(className, value, env).
            // => a. Perform ! environment.InitializeBinding(name, value).
            ctx.add_instruction(Instruction::StoreCopy);
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, class_identifier);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }

        ctx.add_instruction(Instruction::Store);
        // result: constructor
    }
}

#[derive(Debug)]
enum PropertyInitializerField<'a, 'gc> {
    Field((&'a str, Option<&'a ast::Expression<'a>>)),
    Private((&'a str, u32, Option<&'a ast::Expression<'a>>)),
    Computed((String<'gc>, Option<&'a ast::Expression<'a>>)),
    StaticBlock(&'a ast::StaticBlock<'a>),
}

/// Compiles a computed field name and stores the result in a local variable
/// with an invalid name: as the name is invalid in normal JavaScript, it
/// cannot be observed by the user.
fn compile_computed_field_name<'s, 'gc>(
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
    next_computed_key_id: u32,
    key: &'s ast::Expression<'s>,
    value: Option<&'s ast::Expression<'s>>,
) -> PropertyInitializerField<'s, 'gc> {
    let computed_key_id =
        String::from_string(ctx.agent, format!("^{next_computed_key_id}"), ctx.gc);
    ctx.add_instruction_with_identifier(Instruction::CreateImmutableBinding, computed_key_id);
    // 1. Let name be ? Evaluation of ClassElementName.
    // ### ComputedPropertyName : [ AssignmentExpression ]
    // 1. Let exprValue be ? Evaluation of AssignmentExpression.
    key.compile(ctx);
    if is_reference(key) {
        // 2. Let propName be ? GetValue(exprValue).
        ctx.add_instruction(Instruction::GetValue);
    }
    // TODO: To be fully compliant, we need to perform ToPropertyKey here as
    // otherwise we change the order of errors thrown.
    // 3. Return ? ToPropertyKey(propName).
    ctx.add_instruction_with_identifier(Instruction::ResolveBinding, computed_key_id);
    ctx.add_instruction(Instruction::InitializeReferencedBinding);
    PropertyInitializerField::Computed((computed_key_id, value))
}

/// Creates an ECMAScript constructor for a class.
///
/// The class name should be at the top of the stack, followed by the
/// constructor parent if `has_constructor_parent` is true, and finally the
/// prototype.
///
/// After this call, the constructor will be in the result slot and the class
/// prototype will be at the top of the stack.
///
/// Returns the index of the constructor FunctionExpression
fn define_constructor_method(
    ctx: &mut CompileContext,
    class_element: &ast::MethodDefinition,
    has_constructor_parent: bool,
) -> IndexType {
    // stack: [class_name, proto] or [class_name, constructor_parent, proto]

    // 1. Let propKey be ? Evaluation of ClassElementName.
    assert!(class_element.kind.is_constructor());

    // 2. Let env be the running execution context's LexicalEnvironment.
    // 3. Let privateEnv be the running execution context's PrivateEnvironment.
    // 4. If functionPrototype is present, then
    //     a. Let prototype be functionPrototype.
    // 5. Else,
    //     a. Let prototype be %Function.prototype%.
    // 6. Let sourceText be the source text matched by MethodDefinition.
    // 7. Let closure be OrdinaryFunctionCreate(prototype, sourceText, UniqueFormalParameters, FunctionBody, non-lexical-this, env, privateEnv).

    // result: method
    // stack: [proto]

    // 8. Perform MakeMethod(closure, proto).
    // Note: MakeMethod is performed as part of ClassDefineConstructor.
    // 9. Return the Record { [[Key]]: propKey, [[Closure]]: closure }.
    ctx.add_instruction_with_function_expression_and_immediate(
        Instruction::ClassDefineConstructor,
        FunctionExpression {
            expression: SendableRef::new(unsafe {
                core::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                    &class_element.value,
                )
            }),
            // CompileContext holds a name identifier for us if this is NamedEvaluation.
            identifier: None,
            compiled_bytecode: None,
        },
        has_constructor_parent.into(),
    )
}

/// Creates a method for an object.
///
/// The object should be at the top of the stack.
///
/// After this call, the method will be in the result slot and its key will be
/// at the top of the stack. The object is second on the stack.
fn define_method<'s>(
    class_element: &'s ast::MethodDefinition<'s>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // 1. Let propKey be ? Evaluation of ClassElementName.
    if let Some(prop_name) = class_element.prop_name() {
        let prop_name = ctx.create_identifier(prop_name.0);
        ctx.add_instruction_with_constant(Instruction::LoadConstant, prop_name);
    } else {
        // Computed method name.
        let key = class_element.key.as_expression().unwrap();
        key.compile(ctx);
        if is_reference(key) {
            ctx.add_instruction(Instruction::GetValue);
        }
        ctx.add_instruction(Instruction::Load);
    };
    // stack: [key, object]

    // 2. Let env be the running execution context's LexicalEnvironment.
    // 3. Let privateEnv be the running execution context's PrivateEnvironment.
    // 4. If functionPrototype is present, then
    //     a. Let prototype be functionPrototype.
    // 5. Else,
    //     a. Let prototype be %Function.prototype%.
    // 6. Let sourceText be the source text matched by MethodDefinition.
    // 7. Let closure be OrdinaryFunctionCreate(prototype, sourceText, UniqueFormalParameters, FunctionBody, non-lexical-this, env, privateEnv).
    let instruction = match &class_element.kind {
        MethodDefinitionKind::Constructor => unreachable!(),
        MethodDefinitionKind::Method => Instruction::ObjectDefineMethod,
        MethodDefinitionKind::Get => Instruction::ObjectDefineGetter,
        MethodDefinitionKind::Set => Instruction::ObjectDefineSetter,
    };

    // 8. Perform MakeMethod(closure, object).
    // Note: MakeMethod is performed as part of ObjectDefineMethod.

    // result: None
    // stack: [object]

    // 9. Return the Record { [[Key]]: propKey, [[Closure]]: closure }.
    ctx.add_instruction_with_function_expression_and_immediate(
        instruction,
        FunctionExpression {
            expression: SendableRef::new(unsafe {
                core::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                    &class_element.value,
                )
            }),
            // Note: method name is always found in the result register.
            identifier: Some(NamedEvaluationParameter::Result),
            compiled_bytecode: None,
        },
        // enumerable: false,
        false.into(),
    );
}

fn define_private_method<'s>(
    key: &'s str,
    method: &'s ast::MethodDefinition<'s>,
    is_static: bool,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // stack: [constructor, proto]

    // 1. Let propKey be ? Evaluation of ClassElementName.
    // ###  ClassElementName : PrivateIdentifier
    // 1. Let privateIdentifier be the StringValue of PrivateIdentifier.
    // 2. Let privateEnvRec be the running execution context's PrivateEnvironment.
    // 3. Let names be privateEnvRec.[[Names]].
    // 4. Assert: Exactly one element of names is a Private Name whose [[Description]] is privateIdentifier.
    // 5. Let privateName be the Private Name in names whose [[Description]] is privateIdentifier.
    // 6. Return privateName.
    let prop_name = ctx.create_identifier(key);
    ctx.add_instruction_with_constant(Instruction::StoreConstant, prop_name);
    // result: privateName
    // stack: [constructor, proto]

    // 2. Let env be the running execution context's LexicalEnvironment.
    // 3. Let privateEnv be the running execution context's PrivateEnvironment.
    // 4. If functionPrototype is present, then
    //     a. Let prototype be functionPrototype.
    // 5. Else,
    //     a. Let prototype be %Function.prototype%.
    // 6. Let sourceText be the source text matched by MethodDefinition.
    // 7. Let closure be OrdinaryFunctionCreate(
    //        prototype,
    //        sourceText,
    //        UniqueFormalParameters,
    //        FunctionBody,
    //        non-lexical-this,
    //        env,
    //        privateEnv
    //     ).
    let immediate: u8 = match method.kind {
        MethodDefinitionKind::Constructor => unreachable!(),
        MethodDefinitionKind::Method => {
            if is_static {
                0b100
            } else {
                0b000
            }
        }
        MethodDefinitionKind::Get => {
            if is_static {
                0b101
            } else {
                0b001
            }
        }
        MethodDefinitionKind::Set => {
            if is_static {
                0b110
            } else {
                0b010
            }
        }
    };

    // 8. Perform MakeMethod(closure, object).
    // Note: MakeMethod is performed as part of ClassDefinePrivateMethod.

    // 9. Return the Record { [[Key]]: propKey, [[Closure]]: closure }.
    ctx.add_instruction_with_function_expression_and_immediate(
        Instruction::ClassDefinePrivateMethod,
        FunctionExpression {
            expression: SendableRef::new(unsafe {
                core::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                    &method.value,
                )
            }),
            identifier: Some(NamedEvaluationParameter::Result),
            compiled_bytecode: None,
        },
        immediate.into(),
    );
}

impl<'s> CompileEvaluation<'s> for ast::StaticBlock<'s> {
    fn compile(&'s self, ctx: &mut CompileContext<'_, 's, '_, '_>) {
        // 12. Let functionNames be a new empty List.
        // 13. Let functionsToInitialize be a new empty List.
        // NOTE: the keys of `functions` will be `functionNames`, its values will be
        // `functionsToInitialize`.
        let mut functions = AHashMap::new();
        for d in class_static_block_var_scoped_declarations(self) {
            // a. If d is neither a VariableDeclaration nor a ForBinding nor a BindingIdentifier, then
            if let VarScopedDeclaration::Function(d) = d {
                // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
                // ii. Let fn be the sole element of the BoundNames of d.
                let f_name = d.id.as_ref().unwrap().name;
                // iii. If functionNames does not contain fn, then
                //   1. Insert fn as the first element of functionNames.
                //   2. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
                //   3. Insert d as the first element of functionsToInitialize.
                functions.insert(f_name, d);
            }
        }

        // 27. If hasParameterExpressions is false, then
        // a. NOTE: Only a single Environment Record is needed for the parameters and top-level vars.
        // b. Let instantiatedVarNames be a copy of the List parameterBindings.
        let mut instantiated_var_names = AHashSet::new();
        let var_names = class_static_block_var_declared_names(self);
        let lex_declarations = class_static_block_lexically_scoped_declarations(self);
        // c. For each element n of varNames, do
        ctx.enter_class_static_block();
        for n in var_names {
            // i. If instantiatedVarNames does not contain n, then
            if instantiated_var_names.contains(&n) {
                continue;
            }
            // 1. Append n to instantiatedVarNames.
            let n_string = String::from_str(ctx.agent, &n, ctx.gc);
            instantiated_var_names.insert(n);
            // 2. Perform ! env.CreateMutableBinding(n, false).
            ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, n_string);
            // 3. Perform ! env.InitializeBinding(n, undefined).
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, n_string);
            ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }

        // 34. For each element d of lexDeclarations, do
        for d in lex_declarations {
            // a. NOTE: A lexically declared name cannot be the same as a function/generator declaration, formal parameter, or a var name. Lexically declared names are only instantiated here but not initialized.
            // b. For each element dn of the BoundNames of d, do
            match d {
                // i. If IsConstantDeclaration of d is true, then
                LexicallyScopedDeclaration::Variable(decl) if decl.kind.is_const() => {
                    {
                        decl.id.bound_names(&mut |identifier| {
                            let dn = String::from_str(ctx.agent, &identifier.name, ctx.gc);
                            // 1. Perform ! lexEnv.CreateImmutableBinding(dn, true).
                            ctx.add_instruction_with_identifier(
                                Instruction::CreateImmutableBinding,
                                dn,
                            );
                        })
                    }
                }
                // ii. Else,
                //   1. Perform ! lexEnv.CreateMutableBinding(dn, false).
                LexicallyScopedDeclaration::Variable(decl) => {
                    decl.id.bound_names(&mut |identifier| {
                        let dn = String::from_str(ctx.agent, &identifier.name, ctx.gc);
                        ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
                    })
                }
                LexicallyScopedDeclaration::Function(decl) => {
                    let dn = String::from_str(ctx.agent, &decl.id.as_ref().unwrap().name, ctx.gc);
                    ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
                }
                LexicallyScopedDeclaration::Class(decl) => {
                    let dn = String::from_str(ctx.agent, &decl.id.as_ref().unwrap().name, ctx.gc);
                    ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
                }
                LexicallyScopedDeclaration::DefaultExport => {
                    let dn = BUILTIN_STRING_MEMORY._default_;
                    ctx.add_instruction_with_identifier(Instruction::CreateMutableBinding, dn);
                }
            }
        }

        // 36. For each Parse Node f of functionsToInitialize, do
        for f in functions.values() {
            // b. Let fo be InstantiateFunctionObject of f with arguments lexEnv and privateEnv.
            f.compile(ctx);
            // a. Let fn be the sole element of the BoundNames of f.
            let f_name = String::from_str(ctx.agent, &f.id.as_ref().unwrap().name, ctx.gc);
            // c. Perform ! varEnv.SetMutableBinding(fn, fo, false).
            // TODO: This compilation is incorrect if !strict, when varEnv != lexEnv.
            ctx.add_instruction_with_identifier(Instruction::ResolveBinding, f_name);
            ctx.add_instruction(Instruction::PutValue);
        }

        for statement in self.body.iter() {
            statement.compile(ctx);
        }
        ctx.exit_class_static_block();
    }
}

/// Compile a class static identifier field with an optional initializer.
fn compile_class_static_id_field<'s>(
    identifier_name: &'s str,
    value: Option<&'s ast::Expression<'s>>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // stack: [constructor]
    // Load the key constant onto the stack.
    let identifier = String::from_str(ctx.agent, identifier_name, ctx.gc);
    ctx.add_instruction_with_constant(Instruction::LoadConstant, identifier);
    if let Some(value) = value {
        if is_anonymous_function_definition(value) {
            ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
        }
        value.compile(ctx);
        if is_reference(value) {
            ctx.add_instruction(Instruction::GetValue);
        }
    } else {
        // Same optimisation is unconditionally valid here.
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    }
    // stack: [key, constructor]
    // result: value
    ctx.add_instruction(Instruction::ObjectDefineProperty);
    // stack: [constructor]
}

/// Compile a class computed field with an optional initializer.
fn compile_class_computed_field<'s, 'gc>(
    property_key_id: String<'gc>,
    value: Option<&'s ast::Expression<'s>>,
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
) {
    // stack: [constructor]
    // Resolve the static computed key ID to the actual computed key value.
    ctx.add_instruction_with_identifier(Instruction::ResolveBinding, property_key_id);
    // Load the computed key value into the stack.
    ctx.add_instruction(Instruction::GetValue);
    ctx.add_instruction(Instruction::Load);
    if let Some(value) = value {
        // If we have a value, compile it and put it into the result register.
        if is_anonymous_function_definition(value) {
            ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
        }
        value.compile(ctx);
        if is_reference(value) {
            ctx.add_instruction(Instruction::GetValue);
        }
    } else {
        // Otherwise, put `undefined` into the result register.
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    }
    // stack: [key, constructor]
    // result: value
    ctx.add_instruction(Instruction::ObjectDefineProperty);
    // stack: [constructor]
}

/// Compile a class private field with an optional initializer.
fn compile_class_private_field<'s>(
    description: &'s str,
    private_name_identifier: u32,
    value: Option<&'s ast::Expression<'s>>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) {
    // stack: [target]
    if let Some(value) = value {
        if is_anonymous_function_definition(value) {
            let name = String::from_string(ctx.agent, format!("#{description}"), ctx.gc);
            ctx.add_instruction_with_constant(Instruction::StoreConstant, name);
            ctx.name_identifier = Some(NamedEvaluationParameter::Result);
            // stack: [target]
            // result: `#{description}`
        }
        value.compile(ctx);
        if is_reference(value) {
            ctx.add_instruction(Instruction::GetValue);
        }
    } else {
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    }
    // stack: [target]
    // result: value
    ctx.add_instruction_with_immediate(
        Instruction::ClassInitializePrivateValue,
        private_name_identifier as usize,
    );
}
