// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::{
    ecmascript::types::{String, Value, BUILTIN_STRING_MEMORY},
    engine::{
        is_reference, CompileContext, CompileEvaluation, FunctionExpression, Instruction,
        NamedEvaluationParameter, SendableRef,
    },
};
use oxc_ast::{
    ast::{self, MethodDefinitionKind},
    syntax_directed_operations::{PrivateBoundIdentifiers, PropName},
};

impl CompileEvaluation for ast::Class<'_> {
    /// ClassTail : ClassHeritage_opt { ClassBody_opt }
    fn compile(&self, ctx: &mut CompileContext) {
        let anonymous_class_name = ctx.name_identifier.take();

        // 1. Let env be the LexicalEnvironment of the running execution context.
        // 2. Let classEnv be NewDeclarativeEnvironment(env).
        // Note: The specification doesn't enter the declaration here, but
        // no user code is run between here and first enter.
        ctx.exe
            .add_instruction(Instruction::EnterDeclarativeEnvironment);

        // 3. If classBinding is not undefined, then
        let mut has_class_name_on_stack = false;
        let mut class_identifier = None;
        if let Some(class_binding) = &self.id {
            // a. Perform ! classEnv.CreateImmutableBinding(classBinding, true).
            let identifier = String::from_str(ctx.agent, class_binding.name.as_str());
            class_identifier = Some(identifier);
            ctx.exe
                .add_instruction_with_identifier(Instruction::CreateImmutableBinding, identifier);
        } else if let Some(anonymous_class_name) = anonymous_class_name {
            has_class_name_on_stack = true;
            match anonymous_class_name {
                NamedEvaluationParameter::Result => {
                    ctx.exe.add_instruction(Instruction::Load);
                }
                NamedEvaluationParameter::Stack => {}
                NamedEvaluationParameter::Reference => {
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe.add_instruction(Instruction::Load);
                }
                NamedEvaluationParameter::ReferenceStack => {
                    ctx.exe.add_instruction(Instruction::PopReference);
                    ctx.exe.add_instruction(Instruction::GetValue);
                    ctx.exe.add_instruction(Instruction::Load);
                }
            }
        }
        // 4. Let outerPrivateEnvironment be the running execution context's PrivateEnvironment.
        // 5. Let classPrivateEnvironment be NewPrivateEnvironment(outerPrivateEnvironment).
        // 6. If ClassBody is present, then
        for dn in self
            .body
            .body
            .iter()
            .filter(|class_element| class_element.private_bound_identifiers().is_some())
        {
            let dn = dn.private_bound_identifiers().unwrap();
            let _dn = String::from_str(ctx.agent, dn.name.as_str());
            // TODO: Private elements.
            // a. For each String dn of the PrivateBoundIdentifiers of ClassBody, do
            //     i. If classPrivateEnvironment.[[Names]] contains a Private Name pn such that pn.[[Description]] is dn, then
            //         1. Assert: This is only possible for getter/setter pairs.
            //     ii. Else,
            //         1. Let name be a new Private Name whose [[Description]] is dn.
            //         2. Append name to classPrivateEnvironment.[[Names]].
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
                ctx.exe.add_instruction(Instruction::ObjectCreate);
                ctx.exe
                    .add_instruction_with_constant(Instruction::StoreConstant, Value::Null);
                ctx.exe.add_instruction(Instruction::ObjectSetPrototype);
            } else {
                // Constructor parent is known only at runtime, so we must
                // consider it.
                has_constructor_parent = true;
                // a. Set the running execution context's LexicalEnvironment to classEnv.
                // b. NOTE: The running execution context's PrivateEnvironment is outerPrivateEnvironment when evaluating ClassHeritage.
                // c. Let superclassRef be Completion(Evaluation of ClassHeritage).
                super_class.compile(ctx);
                // d. Set the running execution context's LexicalEnvironment to env.
                // Note: We are not following specification properly here:
                // The GetValue here and EvaluatePropertyAccessWithIdentifierKey
                // below should be performed in the parent environment. We do
                // them in classEnv. Whether there's a difference I don't know.
                if is_reference(super_class) {
                    // e. Let superclass be ? GetValue(? superclassRef).
                    ctx.exe.add_instruction(Instruction::GetValue);
                }
                // f. If superclass is null, then
                ctx.exe.add_instruction(Instruction::LoadCopy);
                ctx.exe.add_instruction(Instruction::IsNull);
                let jump_to_else = ctx
                    .exe
                    .add_instruction_with_jump_slot(Instruction::JumpIfNot);
                // i. Let protoParent be null.
                // Note: We already have null on the stack.
                // 9. Let proto be OrdinaryObjectCreate(protoParent).
                ctx.exe.add_instruction(Instruction::ObjectCreate);
                // Now we have proto on the stack followed be null (protoParent).
                ctx.exe.add_instruction(Instruction::Swap);
                // Now we have null (protoParent) followed by proto.
                ctx.exe.add_instruction(Instruction::Load);
                // Now null is in the result register and proto is at the top of
                // the stack.
                ctx.exe.add_instruction(Instruction::ObjectSetPrototype);
                // ii. Let constructorParent be %Function.prototype%.
                ctx.exe.add_instruction_with_constant(
                    Instruction::LoadConstant,
                    ctx.agent.current_realm().intrinsics().function_prototype(),
                );

                // Note: constructorParent is now at the top of the stack, and
                // proto is after it. We can jump to the end.
                let jump_over_else = ctx.exe.add_instruction_with_jump_slot(Instruction::Jump);

                ctx.exe.set_jump_target_here(jump_to_else);
                // g. Else if IsConstructor(superclass) is false, then
                ctx.exe.add_instruction(Instruction::StoreCopy);
                ctx.exe.add_instruction(Instruction::IsConstructor);
                let jump_over_throw = ctx
                    .exe
                    .add_instruction_with_jump_slot(Instruction::JumpIfTrue);
                // Pop the superclass from the stack.
                ctx.exe.add_instruction(Instruction::Store);
                // i. Throw a TypeError exception.
                ctx.exe.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    String::from_static_str(ctx.agent, "class heritage is not a constructor"),
                );
                ctx.exe.add_instruction(Instruction::Throw);

                // h. Else,
                ctx.exe.set_jump_target_here(jump_over_throw);
                // i. Let protoParent be ? Get(superclass, "prototype").
                ctx.exe.add_instruction(Instruction::StoreCopy);
                ctx.exe.add_instruction_with_identifier(
                    Instruction::EvaluatePropertyAccessWithIdentifierKey,
                    BUILTIN_STRING_MEMORY.prototype,
                );
                ctx.exe.add_instruction(Instruction::GetValue);

                // Note: superclass is now at the top of the stack, and protoParent
                // in the result register.

                // ii. If protoParent is not an Object and protoParent is not null,
                ctx.exe.add_instruction(Instruction::LoadCopy);
                ctx.exe.add_instruction(Instruction::IsObject);
                let jump_over_null_check_and_throw = ctx
                    .exe
                    .add_instruction_with_jump_slot(Instruction::JumpIfTrue);

                ctx.exe.add_instruction(Instruction::StoreCopy);
                ctx.exe.add_instruction(Instruction::IsNull);
                let jump_over_throw = ctx
                    .exe
                    .add_instruction_with_jump_slot(Instruction::JumpIfTrue);

                // ... throw a TypeError exception.
                ctx.exe.add_instruction_with_constant(
                    Instruction::StoreConstant,
                    String::from_static_str(ctx.agent, "class heritage is not an object or null"),
                );
                ctx.exe.add_instruction(Instruction::Throw);
                ctx.exe.set_jump_target_here(jump_over_throw);
                ctx.exe.set_jump_target_here(jump_over_null_check_and_throw);

                // Note: protoParent is now at the top of the stack, and superclass
                // is after it.

                // 9. Let proto be OrdinaryObjectCreate(protoParent)
                ctx.exe.add_instruction(Instruction::ObjectCreate);
                ctx.exe.add_instruction(Instruction::Swap);
                // Now protoParent is at the top of the stack, proto is second, and
                // superclass is third.
                ctx.exe.add_instruction(Instruction::Store);
                ctx.exe.add_instruction(Instruction::ObjectSetPrototype);

                // Now proto is first and superclass second.
                ctx.exe.add_instruction(Instruction::Swap);
                // Now superclass is first and proto is second.

                // iii. Let constructorParent be superclass.
                ctx.exe.set_jump_target_here(jump_over_else);
                // Now constructorParent is at the top of the stack, and
                // proto is after it.
            }
        } else {
            // a. Let protoParent be %Object.prototype%.
            // 9. Let proto be OrdinaryObjectCreate(protoParent).
            ctx.exe.add_instruction(Instruction::ObjectCreate);
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
                ctx.exe.add_instruction(Instruction::Store);
                // stack: [proto, class_name]
                ctx.exe.add_instruction(Instruction::Swap);
                // stack: [class_name, proto]
                ctx.exe.add_instruction(Instruction::Load);
                // stack: [constructor_parent, class_name, proto]
                ctx.exe.add_instruction(Instruction::Swap);
                // stack: [class_name, constructor_parent, proto]
            } else {
                // stack: [proto, class_name]
                ctx.exe.add_instruction(Instruction::Swap);
                // stack: [class_name, proto]
            }
        } else {
            // We don't have the class name on the stack, so we can just
            // push it there.
            ctx.exe.add_instruction_with_constant(
                Instruction::LoadConstant,
                class_identifier.unwrap_or(String::EMPTY_STRING),
            );
            // stack: [class_name, constructor_parent?, proto]
        }

        // 14. If constructor is not empty, then
        if let Some(constructor) = constructor {
            // a. Let constructorInfo be ! DefineMethod of constructor with arguments proto and constructorParent.
            define_constructor_method(ctx, constructor, has_constructor_parent);
            // b. Let F be constructorInfo.[[Closure]].
            // c. Perform MakeClassConstructor(F).
            // d. Perform SetFunctionName(F, className).
        } else {
            // 15. Else,
            // a. Let defaultConstructor be a new Abstract Closure with no parameters that captures nothing and performs the following steps when called:
            // ...
            // b. Let F be CreateBuiltinFunction(defaultConstructor, 0, className, « [[ConstructorKind]], [[SourceText]] », the current Realm Record, constructorParent).

            ctx.exe.add_instruction_with_immediate(
                Instruction::ClassDefineDefaultConstructor,
                has_constructor_parent.into(),
            );
        }

        // result: F
        // stack: [proto]
        ctx.exe.add_instruction(Instruction::Load);
        // stack: [constructor, proto]

        // Note: These steps have been performed by ClassDefineConstructor or
        // ClassDefineDefaultConstructor.
        // 16. Perform MakeConstructor(F, false, proto).
        // 17. If ClassHeritage is present, set F.[[ConstructorKind]] to derived.
        // 18. Perform ! ObjectDefineMethod(proto, "constructor", F, false).

        // During binding of methods, we need to swap between the proto and
        // the constructor being on top of the stack. This is because the
        // top of the stack is the object that the method is being bound to.
        let mut proto_is_on_top = false;
        let swap_to_proto = |ctx: &mut CompileContext, proto_is_on_top: &mut bool| {
            if !*proto_is_on_top {
                ctx.exe.add_instruction(Instruction::Swap);
                *proto_is_on_top = true;
            }
        };
        let swap_to_constructor = |ctx: &mut CompileContext, proto_is_on_top: &mut bool| {
            if *proto_is_on_top {
                ctx.exe.add_instruction(Instruction::Swap);
                *proto_is_on_top = false;
            }
        };

        // 19. If ClassBody is not present, let elements be a new empty List.
        // 20. Else, let elements be the NonConstructorElements of ClassBody.
        // 21. Let instancePrivateMethods be a new empty List.
        // let mut instance_private_methods = vec![];
        // 22. Let staticPrivateMethods be a new empty List.
        // let mut static_private_methods = vec![];
        // 23. Let instanceFields be a new empty List.
        // let mut instance_fields = vec![];
        // 24. Let staticElements be a new empty List.
        let mut static_elements = vec![];
        // 25. For each ClassElement e of elements, do
        for e in self.body.body.iter() {
            match e {
                ast::ClassElement::StaticBlock(static_block) => {
                    // Note: Evaluating a ClassStaticBlockDefinition just
                    // creates a function that will be immediately invoked
                    // later. The function is never visible to JavaScript code
                    // and thus doesn't _actually_ need to get created here.
                    static_elements.push(static_block.as_ref());
                }
                // a. If IsStatic of e is false, then
                // i. Let element be Completion(ClassElementEvaluation of e with argument proto).
                // b. Else,
                // i. Let element be Completion(ClassElementEvaluation of e with argument F).
                ast::ClassElement::MethodDefinition(method_definition) => {
                    if method_definition.kind.is_constructor() {
                        // We already handled this.
                        continue;
                    }
                    let is_static = method_definition.r#static;
                    if is_static {
                        swap_to_constructor(ctx, &mut proto_is_on_top);
                    } else {
                        swap_to_proto(ctx, &mut proto_is_on_top);
                    }
                    define_method(method_definition, ctx);
                }
                ast::ClassElement::PropertyDefinition(_) => todo!(),
                ast::ClassElement::AccessorProperty(_) => todo!(),
                #[cfg(feature = "typescript")]
                ast::ClassElement::TSIndexSignature(_) => {}
                #[cfg(not(feature = "typescript"))]
                ast::ClassElement::TSIndexSignature(_) => unreachable!(),
            }
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
        }
        // Drop proto from stack: It is no longer needed.
        swap_to_proto(ctx, &mut proto_is_on_top);
        ctx.exe.add_instruction(Instruction::Store);

        // stack: [constructor]

        // 26. Set the running execution context's LexicalEnvironment to env.
        // 27. If classBinding is not undefined, then

        // Note: The classBinding needs to be initialized in classEnv, as any
        // class method calls access the classBinding through the classEnv.
        if let Some(class_binding) = class_identifier {
            // a. Perform ! classEnv.InitializeBinding(classBinding, F).
            ctx.exe.add_instruction(Instruction::StoreCopy);
            ctx.exe
                .add_instruction_with_identifier(Instruction::ResolveBinding, class_binding);
            ctx.exe
                .add_instruction(Instruction::InitializeReferencedBinding);
        }

        ctx.exe
            .add_instruction(Instruction::ExitDeclarativeEnvironment);
        // 28. Set F.[[PrivateMethods]] to instancePrivateMethods.
        // 29. Set F.[[Fields]] to instanceFields.
        // 30. For each PrivateElement method of staticPrivateMethods, do
        //     a. Perform ! PrivateMethodOrAccessorAdd(F, method).
        // 31. For each element elementRecord of staticElements, do
        for _element_record in static_elements.iter() {
            // a. If elementRecord is a ClassFieldDefinition Record, then
            //     i. Let result be Completion(DefineField(F, elementRecord)).
            // b. Else,
            //     i. Assert: elementRecord is a ClassStaticBlockDefinition Record.
            //     ii. Let result be Completion(Call(elementRecord.[[BodyFunction]], F)).
            // c. If result is an abrupt completion, then
            //     i. Set the running execution context's PrivateEnvironment to outerPrivateEnvironment.
            //     ii. Return ? result.
            todo!();
        }
        // 32. Set the running execution context's PrivateEnvironment to outerPrivateEnvironment.
        // 33. Return F.

        // 15.7.15 Runtime Semantics: BindingClassDeclarationEvaluation
        // ClassDeclaration: class BindingIdentifier ClassTail
        if let Some(class_identifier) = class_identifier {
            // 4. Let env be the running execution context's LexicalEnvironment.
            // 5. Perform ? InitializeBoundName(className, value, env).
            // => a. Perform ! environment.InitializeBinding(name, value).
            ctx.exe.add_instruction(Instruction::StoreCopy);
            ctx.exe
                .add_instruction_with_identifier(Instruction::ResolveBinding, class_identifier);
            ctx.exe
                .add_instruction(Instruction::InitializeReferencedBinding);
        }

        ctx.exe.add_instruction(Instruction::Store);
        // result: constructor
    }
}

/// Creates an ECMAScript constructor for a class.
///
/// The class name should be at the top of the stack, followed by the
/// constructor parent if `has_constructor_parent` is true, and finally the
/// prototype.
///
/// After this call, the constructor will be in the result slot and the class
/// prototype will be at the top of the stack.
fn define_constructor_method(
    ctx: &mut CompileContext,
    class_element: &ast::MethodDefinition,
    has_constructor_parent: bool,
) {
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
    ctx.exe
        .add_instruction_with_function_expression_and_immediate(
            Instruction::ClassDefineConstructor,
            FunctionExpression {
                expression: SendableRef::new(unsafe {
                    std::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                        &class_element.value,
                    )
                }),
                // CompileContext holds a name identifier for us if this is NamedEvaluation.
                identifier: None,
            },
            has_constructor_parent.into(),
        );

    // result: method
    // stack: [proto]

    // 8. Perform MakeMethod(closure, proto).
    // Note: MakeMethod is performed as part of ClassDefineConstructor.
    // 9. Return the Record { [[Key]]: propKey, [[Closure]]: closure }.
}

/// Creates a method for an object.
///
/// The object should be at the top of the stack.
///
/// After this call, the method will be in the result slot and its key will be
/// at the top of the stack. The object is second on the stack.
fn define_method(class_element: &ast::MethodDefinition, ctx: &mut CompileContext) {
    // 1. Let propKey be ? Evaluation of ClassElementName.
    if let Some(prop_name) = class_element.prop_name() {
        ctx.exe.add_instruction_with_constant(
            Instruction::LoadConstant,
            String::from_str(ctx.agent, prop_name.0),
        );
    } else {
        // Computed method name.
        let key = class_element.key.as_expression().unwrap();
        key.compile(ctx);
        if is_reference(key) {
            ctx.exe.add_instruction(Instruction::GetValue);
        }
        ctx.exe.add_instruction(Instruction::Load);
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
    ctx.exe
        .add_instruction_with_function_expression_and_immediate(
            instruction,
            FunctionExpression {
                expression: SendableRef::new(unsafe {
                    std::mem::transmute::<&ast::Function<'_>, &'static ast::Function<'static>>(
                        &class_element.value,
                    )
                }),
                // CompileContext holds a name identifier for us if this is NamedEvaluation.
                identifier: ctx.name_identifier.take(),
            },
            // enumerable: false,
            false.into(),
        );

    // 8. Perform MakeMethod(closure, object).
    // Note: MakeMethod is performed as part of ObjectDefineMethod.

    // result: None
    // stack: [object]

    // 9. Return the Record { [[Key]]: propKey, [[Closure]]: closure }.
}
