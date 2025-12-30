// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{cell::Cell, collections::hash_map::Entry};

use crate::{
    ecmascript::{
        builtins::FunctionAstRef,
        execution::agent::ExceptionType,
        syntax_directed_operations::{
            function_definitions::CompileFunctionBodyData,
            scope_analysis::{
                LexicallyScopedDeclaration, LexicallyScopedDeclarations, VarDeclaredNames,
                VarScopedDeclaration, VarScopedDeclarations,
            },
        },
        types::{BUILTIN_STRING_MEMORY, String, Value},
    },
    engine::{
        CompileContext, CompileEvaluation, FunctionExpression, Instruction,
        NamedEvaluationParameter, SendableRef,
        bytecode::bytecode_compiler::{ExpressionError, ValueOutput, variable_escapes_scope},
    },
};
use ahash::{AHashMap, AHashSet};
use oxc_ast::ast::{self, MethodDefinitionKind};
use oxc_ecmascript::{BoundNames, PrivateBoundIdentifiers, PropName};

use super::{IndexType, is_anonymous_function_definition};

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::Class<'s> {
    type Output = Result<(), ExpressionError>;
    /// ClassTail : ClassHeritage_opt { ClassBody_opt }
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) -> Self::Output {
        let anonymous_class_name = ctx.name_identifier.take();

        // 1. Let env be the LexicalEnvironment of the running execution context.
        // 2. Let classEnv be NewDeclarativeEnvironment(env).
        // Note: The specification doesn't enter the declaration here, but
        // no user code is run between here and first enter.
        let class_env = ctx.enter_lexical_scope();

        let needs_binding = class_has_self_references(self, ctx);

        // 3. If classBinding is not undefined, then
        let mut has_class_name_on_stack = false;
        let mut class_identifier = None;
        if let Some(class_binding) = &self.id {
            // if let Some(stack_index) = ctx.get_variable_stack_index(class_binding.symbol_id()) {}
            // a. Perform ! classEnv.CreateImmutableBinding(classBinding, true).
            let identifier = ctx.create_string(class_binding.name.as_str());
            class_identifier = Some(identifier);
            if needs_binding {
                ctx.add_instruction_with_identifier(
                    Instruction::CreateImmutableBinding,
                    identifier.to_property_key(),
                );
            }
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
                // d. Set the running execution context's LexicalEnvironment to env.
                // Note: We are not following specification properly here:
                // The GetValue here and EvaluatePropertyAccessWithIdentifierKey
                // below should be performed in the parent environment. We do
                // them in classEnv. Whether there's a difference I don't know.
                // e. Let superclass be ? GetValue(? superclassRef).
                let superclass = super_class.compile(ctx).and_then(|sc| sc.get_value(ctx));
                if let Err(err) = superclass {
                    class_env.exit(ctx);
                    return Err(err);
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
                ctx.add_instruction(Instruction::Store);
                // Now null is in the result register and proto is at the top of
                // the stack.
                ctx.add_instruction(Instruction::ObjectSetPrototype);
                // ii. Let constructorParent be %Function.prototype%.
                ctx.add_instruction_with_constant(
                    Instruction::LoadConstant,
                    ctx.get_agent()
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
                let error_message = ctx.create_string("class heritage is not a constructor");
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
                    BUILTIN_STRING_MEMORY.prototype.to_property_key(),
                );
                let cache = ctx.create_property_lookup_cache(
                    BUILTIN_STRING_MEMORY.prototype.to_property_key(),
                );
                ctx.add_instruction_with_cache(Instruction::GetValueWithCache, cache);

                // Note: superclass is now at the top of the stack, and protoParent
                // in the result register.

                // ii. If protoParent is not an Object and protoParent is not null,
                ctx.add_instruction(Instruction::LoadCopy);
                ctx.add_instruction(Instruction::IsNull);
                let jump_over_verify_is_object =
                    ctx.add_instruction_with_jump_slot(Instruction::JumpIfTrue);

                ctx.add_instruction(Instruction::Store);
                // ... throw a TypeError exception.
                let error_message = ctx.create_string("class heritage is not an object or null");
                ctx.add_instruction_with_identifier(
                    Instruction::VerifyIsObject,
                    error_message.to_property_key(),
                );
                ctx.add_instruction(Instruction::Load);
                ctx.set_jump_target_here(jump_over_verify_is_object);

                // Note: protoParent is now at the top of the stack, and
                // superclass is second in the stack.

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
        // 6. If ClassBody is present, then
        let private_env = if !private_bound_identifiers.is_empty() {
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
            Some(ctx.enter_private_scope(private_name_lookup_map.len()))
        } else {
            None
        };

        // Before calling CreateDefaultConstructor we need to smuggle the
        // className to the top of the stack.
        // The current stack is either:
        // - [proto, class_name]
        // - [proto]
        // - [constructor_parent, proto, class_name]
        // - [constructor_parent, proto]
        if has_class_name_on_stack {
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
        let stack_proto = ctx.mark_stack_value();
        // stack: [constructor, proto]
        let stack_constructor = ctx.load_to_stack();

        let has_instance_private_fields_or_methods =
            !instance_private_fields.is_empty() || !instance_private_methods.is_empty();

        // Note: These steps have been performed by ClassDefineConstructor or
        // ClassDefineDefaultConstructor.
        // 16. Perform MakeConstructor(F, false, proto).
        // 17. If ClassHeritage is present, set F.[[ConstructorKind]] to derived.
        // 18. Perform ! ObjectDefineMethod(proto, "constructor", F, false).
        for (key, _) in instance_private_fields {
            let key = ctx.create_string(key);
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
            let key = ctx.create_string(key);
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
                    if let Err(err) = define_method(method_definition, ctx) {
                        stack_constructor.exit(ctx);
                        stack_proto.exit(ctx);
                        if let Some(private_env) = private_env {
                            private_env.exit(ctx);
                        }
                        class_env.exit(ctx);
                        return Err(err);
                    }
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
                        match compile_computed_field_name(
                            ctx,
                            computed_field_id,
                            prop.key.as_expression().unwrap(),
                            prop.value.as_ref(),
                        ) {
                            Ok(field) => field,
                            Err(err) => {
                                stack_constructor.exit(ctx);
                                stack_proto.exit(ctx);
                                if let Some(private_env) = private_env {
                                    private_env.exit(ctx);
                                }
                                class_env.exit(ctx);
                                return Err(err);
                            }
                        }
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
        stack_proto.pop(ctx);

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
        if needs_binding && let Some(class_binding) = class_identifier {
            // a. Perform ! classEnv.InitializeBinding(classBinding, F).
            ctx.add_instruction(Instruction::StoreCopy);
            ctx.add_instruction_with_identifier(
                Instruction::ResolveBinding,
                class_binding.to_property_key(),
            );
            ctx.add_instruction(Instruction::InitializeReferencedBinding);
        }

        // 28. Set F.[[PrivateMethods]] to instancePrivateMethods.
        // 29. Set F.[[Fields]] to instanceFields.
        if has_instance_private_fields_or_methods || !instance_fields.is_empty() {
            let source_code = ctx.get_source_code();
            let (agent, gc) = ctx.get_agent_and_gc();
            let mut constructor_ctx = CompileContext::new(agent, source_code, gc);
            // Resolve 'this' into the stack.
            constructor_ctx.add_instruction(Instruction::ResolveThisBinding);
            constructor_ctx.add_instruction(Instruction::Load);
            if has_instance_private_fields_or_methods {
                constructor_ctx.add_instruction(Instruction::ClassInitializePrivateElements);
            }
            for ele in instance_fields {
                match ele {
                    PropertyInitializerField::Field((property_key, value)) => {
                        if compile_class_static_id_field(property_key, value, &mut constructor_ctx)
                            .is_err()
                        {
                            break;
                        }
                    }
                    PropertyInitializerField::Computed((key_id, value)) => {
                        if compile_class_computed_field(key_id, value, &mut constructor_ctx)
                            .is_err()
                        {
                            break;
                        }
                    }
                    PropertyInitializerField::Private((description, private_identifier, value)) => {
                        if compile_class_private_field(
                            description,
                            private_identifier,
                            value,
                            &mut constructor_ctx,
                        )
                        .is_err()
                        {
                            break;
                        }
                    }
                    PropertyInitializerField::StaticBlock(_) => unreachable!(),
                }
            }
            // Pop the `this` value off the stack.
            constructor_ctx.add_instruction(Instruction::Store);
            let source_code = constructor_ctx.get_source_code();
            if let Some(constructor) = constructor {
                let constructor_data = CompileFunctionBodyData {
                    source_code,
                    is_lexical: false,
                    // Class code is always strict.
                    is_strict: true,
                    ast: FunctionAstRef::ClassConstructor(&constructor.value),
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
        let static_env = if !static_elements.is_empty() {
            Some(ctx.enter_class_static_block())
        } else {
            None
        };
        for element_record in static_elements {
            let result = match element_record {
                // a. If elementRecord is a ClassFieldDefinition Record, then
                PropertyInitializerField::StaticBlock(static_block) => {
                    // i. Let result be Completion(DefineField(F, elementRecord)).
                    static_block.compile(ctx);
                    Ok(())
                }
                // b. Else,
                // i. Assert: elementRecord is a ClassStaticBlockDefinition Record.
                // ii. Let result be Completion(Call(elementRecord.[[BodyFunction]], F)).
                PropertyInitializerField::Field((property_key, value)) => {
                    compile_class_static_id_field(property_key, value, ctx)
                }
                PropertyInitializerField::Computed((key_id, value)) => {
                    compile_class_computed_field(key_id, value, ctx)
                }
                PropertyInitializerField::Private((description, private_identifier, value)) => {
                    // Note: Static private fields follow third after private
                    // fields and methods, so their identifiers are offset.
                    let private_identifier = instance_private_field_count
                        + instance_private_method_count
                        + private_identifier;
                    compile_class_private_field(description, private_identifier, value, ctx)
                }
            };
            // c. If result is an abrupt completion, then
            if let Err(result) = result {
                // i. Set the running execution context's PrivateEnvironment to
                //    outerPrivateEnvironment.
                if let Some(static_env) = static_env {
                    static_env.exit(ctx);
                }
                stack_constructor.pop(ctx);
                if let Some(private_env) = private_env {
                    private_env.exit(ctx);
                }
                class_env.exit(ctx);
                // ii. Return ? result.
                return Err(result);
            }
        }
        if let Some(static_env) = static_env {
            static_env.exit(ctx);
        }
        // result: constructor
        stack_constructor.store(ctx);

        // 32. Set the running execution context's PrivateEnvironment to outerPrivateEnvironment.
        if let Some(private_env) = private_env {
            private_env.exit(ctx);
        }

        // Note: We finally leave classEnv here. See step 26.
        class_env.exit(ctx);
        // 33. Return F.

        // 15.7.15 Runtime Semantics: BindingClassDeclarationEvaluation
        // ClassDeclaration: class BindingIdentifier ClassTail
        if self.is_declaration() && class_identifier.is_some() {
            // 4. Let env be the running execution context's LexicalEnvironment.
            // 5. Perform ? InitializeBoundName(className, value, env).
            // => a. Perform ! environment.InitializeBinding(name, value).
            ctx.add_instruction(Instruction::LoadCopy);
            let name = self.id.as_ref().unwrap().compile(ctx);
            name.initialise_referenced_binding(ctx, ValueOutput::Value);
            ctx.add_instruction(Instruction::Store);
        }
        Ok(())
    }
}

fn class_has_self_references(class: &ast::Class, ctx: &CompileContext) -> bool {
    let Some(class_binding) = &class.id else {
        // An unnamed class cannot be self-referential.
        return false;
    };
    let agent = ctx.get_agent();
    let sc = ctx.get_source_code();
    let scoping = sc.get_scoping(agent);
    let nodes = sc.get_nodes(agent);
    let s = class_binding.symbol_id();
    let class_scope = class.scope_id();
    if scoping.scope_flags(class_scope).contains_direct_eval() {
        return true;
    }
    for reference in scoping.get_resolved_references(s) {
        let mut scope = nodes.get_node(reference.node_id()).scope_id();
        if scope == class_scope {
            // Reference to class from within the scope itself.
            return true;
        }
        while let Some(s) = scoping.scope_parent_id(scope) {
            if s == class_scope {
                // Reference to class from within the scope itself.
                return true;
            }
            scope = s;
        }
    }
    // No references, or no references within the class scope itself. The class
    // reference itself may escape the scope it was created in, but it is not
    // self-referential.
    false
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
) -> Result<PropertyInitializerField<'s, 'gc>, ExpressionError> {
    let computed_key_id = ctx.create_string_from_owned(format!("^{next_computed_key_id}"));
    ctx.add_instruction_with_identifier(
        Instruction::CreateImmutableBinding,
        computed_key_id.to_property_key(),
    );
    // 1. Let name be ? Evaluation of ClassElementName.
    // ### ComputedPropertyName : [ AssignmentExpression ]
    // 1. Let exprValue be ? Evaluation of AssignmentExpression.
    // 2. Let propName be ? GetValue(exprValue).
    key.compile(ctx)?.get_value(ctx)?;

    // TODO: To be fully compliant, we need to perform ToPropertyKey here as
    // otherwise we change the order of errors thrown.
    // 3. Return ? ToPropertyKey(propName).
    ctx.add_instruction_with_identifier(
        Instruction::ResolveBinding,
        computed_key_id.to_property_key(),
    );
    ctx.add_instruction(Instruction::InitializeReferencedBinding);
    Ok(PropertyInitializerField::Computed((computed_key_id, value)))
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
) -> Result<(), ExpressionError> {
    // 1. Let propKey be ? Evaluation of ClassElementName.
    if let Some(prop_name) = class_element.prop_name() {
        let prop_name = ctx.create_string(prop_name.0);
        ctx.add_instruction_with_constant(Instruction::LoadConstant, prop_name);
    } else {
        // Computed method name.
        let key = class_element.key.as_expression().unwrap();
        key.compile(ctx)?.get_value(ctx)?;

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
    Ok(())
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
    let prop_name = ctx.create_string(key);
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

impl<'a, 's, 'gc, 'scope> CompileEvaluation<'a, 's, 'gc, 'scope> for ast::StaticBlock<'s> {
    type Output = ();
    fn compile(&'s self, ctx: &mut CompileContext<'a, 's, 'gc, 'scope>) {
        // 12. Let functionNames be a new empty List.
        // 13. Let functionsToInitialize be a new empty List.
        // NOTE: the keys of `functions` will be `functionNames`, its values will be
        // `functionsToInitialize`.
        let mut functions = AHashMap::new();
        self.var_scoped_declarations(&mut |d| {
            // a. If d is neither a VariableDeclaration nor a ForBinding nor a BindingIdentifier, then
            let VarScopedDeclaration::Function(d) = d else {
                return;
            };
            // i. Assert: d is either a FunctionDeclaration, a GeneratorDeclaration, an AsyncFunctionDeclaration, or an AsyncGeneratorDeclaration.
            // ii. Let fn be the sole element of the BoundNames of d.
            let f_name = d.id.as_ref().unwrap().name;
            // iii. If functionNames does not contain fn, then
            // 1. Insert fn as the first element of functionNames.
            // 2. NOTE: If there are multiple function declarations for the same name, the last declaration is used.
            // 3. Insert d as the first element of functionsToInitialize.
            functions.insert(f_name, d);
        });

        // 27. If hasParameterExpressions is false, then
        // a. NOTE: Only a single Environment Record is needed for the parameters and top-level vars.
        // b. Let instantiatedVarNames be a copy of the List parameterBindings.
        let mut instantiated_var_names = AHashSet::new();
        let static_env = ctx.enter_lexical_scope();
        let mut stack_variables = vec![];

        // c. For each element n of varNames, do
        self.var_declared_names(&mut |identifier| {
            let n = identifier.name;
            // i. If instantiatedVarNames does not contain n, then
            // 1. Append n to instantiatedVarNames.
            if !instantiated_var_names.insert(n) {
                return;
            }
            let n_string = ctx.create_string(&n);
            if variable_escapes_scope(ctx, identifier) {
                // 2. Perform ! env.CreateMutableBinding(n, false).
                ctx.add_instruction_with_identifier(
                    Instruction::CreateMutableBinding,
                    n_string.to_property_key(),
                );
                // 3. Perform ! env.InitializeBinding(n, undefined).
                ctx.add_instruction_with_identifier(
                    Instruction::ResolveBinding,
                    n_string.to_property_key(),
                );
                ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
                ctx.add_instruction(Instruction::InitializeReferencedBinding);
            } else {
                stack_variables.push(ctx.push_stack_variable(identifier.symbol_id(), false));
            }
        });

        // 34. For each element d of lexDeclarations, do
        {
            // a. NOTE: A lexically declared name cannot be the same as a function/generator declaration, formal parameter, or a var name. Lexically declared names are only instantiated here but not initialized.
            // b. For each element dn of the BoundNames of d, do
            // i. If IsConstantDeclaration of d is true, then
            // 1. Perform ! lexEnv.CreateImmutableBinding(dn, true).
            // ii. Else,
            // 1. Perform ! lexEnv.CreateMutableBinding(dn, false).
            let is_constant_declaration = Cell::new(false);
            let cb = &mut |identifier: &ast::BindingIdentifier<'s>| {
                if variable_escapes_scope(ctx, identifier) {
                    let dn = ctx.create_string(&identifier.name);
                    ctx.add_instruction_with_identifier(
                        // i. If IsConstantDeclaration of d is true, then
                        if is_constant_declaration.get() {
                            // 1. Perform ! lexEnv.CreateImmutableBinding(dn, true).
                            Instruction::CreateImmutableBinding
                        } else {
                            // ii. Else,
                            // 1. Perform ! lexEnv.CreateMutableBinding(dn, false).
                            Instruction::CreateMutableBinding
                        },
                        dn.to_property_key(),
                    );
                } else {
                    stack_variables.push(ctx.push_stack_variable(identifier.symbol_id(), false));
                }
            };
            let mut create_default_export = false;
            self.lexically_scoped_declarations(&mut |d| match d {
                LexicallyScopedDeclaration::Variable(decl) => {
                    is_constant_declaration.set(decl.kind.is_const());
                    decl.id.bound_names(cb);
                    is_constant_declaration.set(false);
                }
                LexicallyScopedDeclaration::Function(decl) => {
                    // Skip function declarations with declare modifier - they are TypeScript ambient declarations
                    #[cfg(feature = "typescript")]
                    if decl.declare {
                        return;
                    }

                    decl.bound_names(cb);
                }
                LexicallyScopedDeclaration::Class(decl) => {
                    decl.bound_names(cb);
                }
                LexicallyScopedDeclaration::DefaultExport => {
                    create_default_export = true;
                }
                #[cfg(feature = "typescript")]
                LexicallyScopedDeclaration::TSEnum(decl) => {
                    decl.id.bound_names(cb);
                }
            });
            if create_default_export {
                let dn = BUILTIN_STRING_MEMORY._default_;
                ctx.add_instruction_with_identifier(
                    Instruction::CreateMutableBinding,
                    dn.to_property_key(),
                );
            }
        }

        // 36. For each Parse Node f of functionsToInitialize, do
        for f in functions.values() {
            // b. Let fo be InstantiateFunctionObject of f with arguments lexEnv and privateEnv.
            f.compile(ctx);
            // a. Let fn be the sole element of the BoundNames of f.
            let f = f.id.as_ref().unwrap().compile(ctx);
            // c. Perform ! varEnv.SetMutableBinding(fn, fo, false).
            // TODO: This compilation is incorrect if !strict, when varEnv != lexEnv.
            f.put_value(ctx, ValueOutput::Value).unwrap();
        }

        for statement in self.body.iter() {
            let result = statement.compile(ctx);
            if result.is_break() {
                break;
            }
        }
        for stack_variable in stack_variables {
            stack_variable.exit(ctx);
        }
        static_env.exit(ctx);
    }
}

/// Compile a class static identifier field with an optional initializer.
fn compile_class_static_id_field<'s>(
    identifier_name: &'s str,
    value: Option<&'s ast::Expression<'s>>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) -> Result<(), ExpressionError> {
    // stack: [constructor]
    // Load the key constant onto the stack.
    let identifier = ctx.create_string(identifier_name);
    ctx.add_instruction_with_constant(Instruction::LoadConstant, identifier);
    if let Some(value) = value {
        if is_anonymous_function_definition(value) {
            ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
        }
        value.compile(ctx)?.get_value(ctx)?;
    } else {
        // Same optimisation is unconditionally valid here.
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    }
    // stack: [key, constructor]
    // result: value
    ctx.add_instruction(Instruction::ObjectDefineProperty);
    // stack: [constructor]
    Ok(())
}

/// Compile a class computed field with an optional initializer.
fn compile_class_computed_field<'s, 'gc>(
    property_key_id: String<'gc>,
    value: Option<&'s ast::Expression<'s>>,
    ctx: &mut CompileContext<'_, 's, 'gc, '_>,
) -> Result<(), ExpressionError> {
    // stack: [constructor]
    // Resolve the static computed key ID to the actual computed key value.
    ctx.add_instruction_with_identifier(
        Instruction::ResolveBinding,
        property_key_id.to_property_key(),
    );
    // Load the computed key value into the stack.
    ctx.add_instruction(Instruction::GetValue);
    ctx.add_instruction(Instruction::Load);
    if let Some(value) = value {
        // If we have a value, compile it and put it into the result register.
        if is_anonymous_function_definition(value) {
            ctx.name_identifier = Some(NamedEvaluationParameter::Stack);
        }
        value.compile(ctx)?.get_value(ctx)?;
    } else {
        // Otherwise, put `undefined` into the result register.
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    }
    // stack: [key, constructor]
    // result: value
    ctx.add_instruction(Instruction::ObjectDefineProperty);
    // stack: [constructor]
    Ok(())
}

/// Compile a class private field with an optional initializer.
fn compile_class_private_field<'s>(
    description: &'s str,
    private_name_identifier: u32,
    value: Option<&'s ast::Expression<'s>>,
    ctx: &mut CompileContext<'_, 's, '_, '_>,
) -> Result<(), ExpressionError> {
    // stack: [target]
    if let Some(value) = value {
        if is_anonymous_function_definition(value) {
            let name = ctx.create_string_from_owned(format!("#{description}"));
            ctx.add_instruction_with_constant(Instruction::StoreConstant, name);
            ctx.name_identifier = Some(NamedEvaluationParameter::Result);
            // stack: [target]
            // result: `#{description}`
        }
        value.compile(ctx)?.get_value(ctx)?;
    } else {
        ctx.add_instruction_with_constant(Instruction::StoreConstant, Value::Undefined);
    }
    // stack: [target]
    // result: value
    ctx.add_instruction_with_immediate(
        Instruction::ClassInitializePrivateValue,
        private_name_identifier as usize,
    );
    Ok(())
}
