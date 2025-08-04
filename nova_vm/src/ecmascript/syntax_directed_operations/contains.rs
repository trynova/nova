// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! ## [8.5 Contains](https://tc39.es/ecma262/#sec-static-semantics-contains)

use oxc_ast::ast;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContainsSymbol {
    Await,
    NewTarget,
    Super,
    This,
    Yield,
}

/// ### [8.5 Contains](https://tc39.es/ecma262/#sec-static-semantics-contains)
///
/// The syntax-directed operation ComputedPropertyContains takes argument
/// symbol (a grammar symbol) and returns a Boolean.
pub(crate) trait Contains {
    /// ### [8.5 Contains](https://tc39.es/ecma262/#sec-static-semantics-contains)
    fn contains(&self, symbol: ContainsSymbol) -> bool;
}

impl Contains for [ast::Statement<'_>] {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.iter().any(|st| st.contains(symbol))
    }
}

impl Contains for ast::Program<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.body.iter().any(|st| st.contains(symbol))
    }
}

impl Contains for ast::Function<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        false
    }
}

impl Contains for ast::Class<'_> {
    /// ClassTail : ClassHeritageopt { ClassBody }
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        // 1. If symbol is ClassBody, return true.
        // 2. If symbol is ClassHeritage, then
        if let Some(heritage) = &self.super_class {
            // a. If ClassHeritage is present, return true; otherwise return
            //    false.
            // 3. If ClassHeritage is present, then
            // a. If ClassHeritage Contains symbol is true, return true.
            if heritage.contains(symbol) {
                return true;
            }
        }
        // 4. Return the result of ComputedPropertyContains of ClassBody with
        //    argument symbol.
        self.body.computed_property_contains(symbol)

        // Note 2

        // Static semantic rules that depend upon substructure generally do not
        // look into class bodies except for PropertyNames.
    }
}

impl Contains for ast::ArrowFunctionExpression<'_> {
    /// ArrowFunction : ArrowParameters => ConciseBody
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        // 1. If symbol is not one of NewTarget, SuperProperty, SuperCall, super, or this,
        if !matches!(
            symbol,
            ContainsSymbol::NewTarget | ContainsSymbol::Super | ContainsSymbol::This
        ) {
            // return false.
            return false;
        }
        // 2. If ArrowParameters Contains symbol is true, return true.
        if self.params.items.iter().any(|p| p.contains(symbol)) {
            return true;
        }
        // 3. Return ConciseBody Contains symbol.
        self.body.statements.iter().any(|st| st.contains(symbol))
    }
}

impl Contains for ast::FormalParameter<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.pattern.contains(symbol)
    }
}

impl Contains for ast::BindingPattern<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match &self.kind {
            ast::BindingPatternKind::BindingIdentifier(_) => false,
            ast::BindingPatternKind::ObjectPattern(e) => {
                e.properties
                    .iter()
                    .any(|p| p.key.contains(symbol) || p.value.contains(symbol))
                    || e.rest.as_ref().is_some_and(|e| e.argument.contains(symbol))
            }
            ast::BindingPatternKind::ArrayPattern(e) => e
                .elements
                .iter()
                .any(|e| e.as_ref().is_some_and(|e| e.contains(symbol))),
            ast::BindingPatternKind::AssignmentPattern(e) => {
                e.left.contains(symbol) || e.right.contains(symbol)
            }
        }
    }
}

impl Contains for ast::ParenthesizedExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.expression.contains(symbol)
    }
}

impl Contains for ast::SequenceExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.expressions.iter().any(|e| e.contains(symbol))
    }
}

impl Contains for ast::MethodDefinition<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.computed_property_contains(symbol)
    }
}

impl Contains for ast::StaticMemberExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.object.contains(symbol)
    }
}

impl Contains for ast::Super {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        symbol == ContainsSymbol::Super
    }
}

impl Contains for ast::CallExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.callee.contains(symbol) || self.arguments.iter().any(|arg| arg.contains(symbol))
    }
}

impl Contains for ast::ChainExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.expression.contains(symbol)
    }
}

impl Contains for ast::ChainElement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::ChainElement::CallExpression(e) => e.contains(symbol),
            ast::ChainElement::TSNonNullExpression(e) => e.expression.contains(symbol),
            _ => self.as_member_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::TemplateLiteral<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.expressions.iter().any(|e| e.contains(symbol))
    }
}

impl Contains for ast::MetaProperty<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        symbol == ContainsSymbol::NewTarget
            && self.meta.name == "new"
            && self.property.name == "target"
    }
}

impl Contains for ast::ArrayExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.elements.iter().any(|e| e.contains(symbol))
    }
}

impl Contains for ast::ArrayExpressionElement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::ArrayExpressionElement::SpreadElement(e) => e.argument.contains(symbol),
            ast::ArrayExpressionElement::Elision(_) => false,
            _ => self.as_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::AssignmentExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.left.contains(symbol) || self.right.contains(symbol)
    }
}

impl Contains for ast::AssignmentTarget<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        if let Some(e) = self.as_simple_assignment_target() {
            e.contains(symbol)
        } else {
            self.as_assignment_target_pattern()
                .unwrap()
                .contains(symbol)
        }
    }
}

impl Contains for ast::SimpleAssignmentTarget<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::SimpleAssignmentTarget::AssignmentTargetIdentifier(_) => false,
            ast::SimpleAssignmentTarget::TSAsExpression(e) => e.expression.contains(symbol),
            ast::SimpleAssignmentTarget::TSSatisfiesExpression(e) => e.expression.contains(symbol),
            ast::SimpleAssignmentTarget::TSNonNullExpression(e) => e.expression.contains(symbol),
            ast::SimpleAssignmentTarget::TSTypeAssertion(e) => e.expression.contains(symbol),
            _ => self.as_member_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::MemberExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::MemberExpression::ComputedMemberExpression(e) => e.contains(symbol),
            ast::MemberExpression::StaticMemberExpression(e) => e.contains(symbol),
            ast::MemberExpression::PrivateFieldExpression(e) => e.contains(symbol),
        }
    }
}

impl Contains for ast::AssignmentTargetPattern<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::AssignmentTargetPattern::ArrayAssignmentTarget(e) => e.contains(symbol),
            ast::AssignmentTargetPattern::ObjectAssignmentTarget(e) => e.contains(symbol),
        }
    }
}

impl Contains for ast::ArrayAssignmentTarget<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.elements.iter().any(|e| {
            let Some(e) = e else {
                return false;
            };
            e.contains(symbol)
        }) || self
            .rest
            .as_ref()
            .is_some_and(|e| e.target.contains(symbol))
    }
}

impl Contains for ast::AssignmentTargetMaybeDefault<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(e) => e.contains(symbol),
            _ => self.as_assignment_target().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::AssignmentTargetWithDefault<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.binding.contains(symbol) || self.init.contains(symbol)
    }
}

impl Contains for ast::ObjectAssignmentTarget<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.properties.iter().any(|e| match e {
            ast::AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(e) => {
                e.init.as_ref().is_some_and(|e| e.contains(symbol))
            }
            ast::AssignmentTargetProperty::AssignmentTargetPropertyProperty(e) => {
                e.contains(symbol)
            }
        })
    }
}

impl Contains for ast::AssignmentTargetPropertyProperty<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.binding.contains(symbol)
    }
}

impl Contains for ast::AwaitExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        if symbol == ContainsSymbol::Await {
            return true;
        }
        self.argument.contains(symbol)
    }
}

impl Contains for ast::BinaryExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.left.contains(symbol) || self.right.contains(symbol)
    }
}

impl Contains for ast::ConditionalExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.alternate.contains(symbol)
            || self.consequent.contains(symbol)
            || self.test.contains(symbol)
    }
}

impl Contains for ast::ImportExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.options.as_ref().is_some_and(|e| e.contains(symbol)) || self.source.contains(symbol)
    }
}

impl Contains for ast::LogicalExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.left.contains(symbol) || self.right.contains(symbol)
    }
}

impl Contains for ast::NewExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.callee.contains(symbol) || self.arguments.iter().any(|arg| arg.contains(symbol))
    }
}

impl Contains for ast::Argument<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::Argument::SpreadElement(e) => e.argument.contains(symbol),
            _ => self.as_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::ObjectExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.properties.iter().any(|p| match p {
            ast::ObjectPropertyKind::ObjectProperty(p) => {
                p.key.contains(symbol) || p.value.contains(symbol)
            }
            ast::ObjectPropertyKind::SpreadProperty(p) => p.argument.contains(symbol),
        })
    }
}

impl Contains for ast::PropertyKey<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::PropertyKey::StaticIdentifier(_) | ast::PropertyKey::PrivateIdentifier(_) => false,
            _ => self.as_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::TaggedTemplateExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.tag.contains(symbol) || self.quasi.contains(symbol)
    }
}

impl Contains for ast::ThisExpression {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        symbol == ContainsSymbol::This
    }
}

impl Contains for ast::UnaryExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.argument.contains(symbol)
    }
}

impl Contains for ast::UpdateExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.argument.contains(symbol)
    }
}

impl Contains for ast::YieldExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        if symbol == ContainsSymbol::Yield {
            return true;
        }
        self.argument.as_ref().is_some_and(|e| e.contains(symbol))
    }
}

impl Contains for ast::PrivateInExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.right.contains(symbol)
    }
}

impl Contains for ast::V8IntrinsicExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.arguments.iter().any(|e| e.contains(symbol))
    }
}

impl Contains for ast::ComputedMemberExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.object.contains(symbol) || self.expression.contains(symbol)
    }
}

impl Contains for ast::PrivateFieldExpression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.object.contains(symbol)
    }
}

impl Contains for ast::Expression<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::Expression::BooleanLiteral(_)
            | ast::Expression::NullLiteral(_)
            | ast::Expression::NumericLiteral(_)
            | ast::Expression::BigIntLiteral(_)
            | ast::Expression::RegExpLiteral(_)
            | ast::Expression::StringLiteral(_) => false,
            ast::Expression::TemplateLiteral(e) => e.contains(symbol),
            ast::Expression::Identifier(_) | ast::Expression::MetaProperty(_) => false,
            ast::Expression::Super(e) => e.contains(symbol),
            ast::Expression::ArrayExpression(e) => e.contains(symbol),
            ast::Expression::ArrowFunctionExpression(_) => false,
            ast::Expression::AssignmentExpression(e) => e.contains(symbol),
            ast::Expression::AwaitExpression(e) => e.contains(symbol),
            ast::Expression::BinaryExpression(e) => e.contains(symbol),
            ast::Expression::CallExpression(e) => e.contains(symbol),
            ast::Expression::ChainExpression(e) => e.contains(symbol),
            ast::Expression::ClassExpression(e) => e.contains(symbol),
            ast::Expression::ConditionalExpression(e) => e.contains(symbol),
            ast::Expression::FunctionExpression(e) => e.contains(symbol),
            ast::Expression::ImportExpression(e) => e.contains(symbol),
            ast::Expression::LogicalExpression(e) => e.contains(symbol),
            ast::Expression::NewExpression(e) => e.contains(symbol),
            ast::Expression::ObjectExpression(e) => e.contains(symbol),
            ast::Expression::ParenthesizedExpression(e) => e.contains(symbol),
            ast::Expression::SequenceExpression(e) => e.contains(symbol),
            ast::Expression::TaggedTemplateExpression(e) => e.contains(symbol),
            ast::Expression::ThisExpression(e) => e.contains(symbol),
            ast::Expression::UnaryExpression(e) => e.contains(symbol),
            ast::Expression::UpdateExpression(e) => e.contains(symbol),
            ast::Expression::YieldExpression(e) => e.contains(symbol),
            ast::Expression::PrivateInExpression(e) => e.contains(symbol),
            ast::Expression::JSXElement(_) | ast::Expression::JSXFragment(_) => unreachable!(),
            ast::Expression::TSAsExpression(e) => e.expression.contains(symbol),
            ast::Expression::TSSatisfiesExpression(e) => e.expression.contains(symbol),
            ast::Expression::TSTypeAssertion(e) => e.expression.contains(symbol),
            ast::Expression::TSNonNullExpression(e) => e.expression.contains(symbol),
            ast::Expression::TSInstantiationExpression(e) => e.expression.contains(symbol),
            ast::Expression::V8IntrinsicExpression(e) => {
                e.arguments.iter().any(|e| e.contains(symbol))
            }
            ast::Expression::ComputedMemberExpression(e) => e.contains(symbol),
            ast::Expression::StaticMemberExpression(e) => e.contains(symbol),
            ast::Expression::PrivateFieldExpression(e) => e.contains(symbol),
        }
    }
}

impl Contains for ast::BlockStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.body.iter().any(|st| st.contains(symbol))
    }
}

impl Contains for ast::DoWhileStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.test.contains(symbol) || self.body.contains(symbol)
    }
}

impl Contains for ast::ForInStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.right.contains(symbol) || self.left.contains(symbol) || self.body.contains(symbol)
    }
}

impl Contains for ast::ForStatementLeft<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::ForStatementLeft::VariableDeclaration(e) => e.contains(symbol),
            _ => self.as_assignment_target().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::ForOfStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.right.contains(symbol) || self.left.contains(symbol) || self.body.contains(symbol)
    }
}

impl Contains for ast::ForStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.init.as_ref().is_some_and(|e| e.contains(symbol))
            || self.test.as_ref().is_some_and(|e| e.contains(symbol))
            || self.update.as_ref().is_some_and(|e| e.contains(symbol))
            || self.body.contains(symbol)
    }
}

impl Contains for ast::ForStatementInit<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::ForStatementInit::VariableDeclaration(e) => e.contains(symbol),
            _ => self.as_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::IfStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.test.contains(symbol)
            || self.consequent.contains(symbol)
            || self
                .alternate
                .as_ref()
                .is_some_and(|st| st.contains(symbol))
    }
}

impl Contains for ast::LabeledStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.body.contains(symbol)
    }
}

impl Contains for ast::ReturnStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.argument.as_ref().is_some_and(|e| e.contains(symbol))
    }
}

impl Contains for ast::SwitchStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.discriminant.contains(symbol) || self.cases.iter().any(|e| e.contains(symbol))
    }
}

impl Contains for ast::SwitchCase<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.test.as_ref().is_some_and(|e| e.contains(symbol))
            || self.consequent.iter().any(|st| st.contains(symbol))
    }
}

impl Contains for ast::ThrowStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.argument.contains(symbol)
    }
}

impl Contains for ast::TryStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.block.contains(symbol)
            || self.handler.as_ref().is_some_and(|e| e.contains(symbol))
            || self
                .finalizer
                .as_ref()
                .is_some_and(|st| st.contains(symbol))
    }
}

impl Contains for ast::CatchClause<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.param
            .as_ref()
            .is_some_and(|e| e.pattern.contains(symbol))
            || self.body.contains(symbol)
    }
}

impl Contains for ast::WhileStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.test.contains(symbol) || self.body.contains(symbol)
    }
}

impl Contains for ast::WithStatement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.object.contains(symbol) || self.body.contains(symbol)
    }
}

impl Contains for ast::VariableDeclaration<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.declarations.iter().any(|e| e.contains(symbol))
    }
}

impl Contains for ast::VariableDeclarator<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.init.as_ref().is_some_and(|e| e.contains(symbol)) || self.id.contains(symbol)
    }
}

impl Contains for ast::TSEnumDeclaration<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        unreachable!()
    }
}

impl Contains for ast::TSModuleDeclaration<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        unreachable!()
    }
}

impl Contains for ast::TSImportEqualsDeclaration<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        unreachable!()
    }
}

impl Contains for ast::ImportDeclaration<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        false
    }
}

impl Contains for ast::ExportAllDeclaration<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        false
    }
}

impl Contains for ast::ExportDefaultDeclaration<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match &self.declaration {
            ast::ExportDefaultDeclarationKind::FunctionDeclaration(f) => f.contains(symbol),
            ast::ExportDefaultDeclarationKind::ClassDeclaration(c) => c.contains(symbol),
            ast::ExportDefaultDeclarationKind::TSInterfaceDeclaration(_) => false,
            _ => self.declaration.as_expression().unwrap().contains(symbol),
        }
    }
}

impl Contains for ast::ExportNamedDeclaration<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.declaration.as_ref().is_some_and(|e| match e {
            ast::Declaration::VariableDeclaration(e) => e.contains(symbol),
            ast::Declaration::FunctionDeclaration(f) => f.contains(symbol),
            ast::Declaration::ClassDeclaration(c) => c.contains(symbol),
            ast::Declaration::TSTypeAliasDeclaration(_)
            | ast::Declaration::TSInterfaceDeclaration(_) => false,
            ast::Declaration::TSEnumDeclaration(e) => e.contains(symbol),
            ast::Declaration::TSModuleDeclaration(e) => e.contains(symbol),
            ast::Declaration::TSImportEqualsDeclaration(e) => e.contains(symbol),
        })
    }
}

impl Contains for ast::TSExportAssignment<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        self.expression.contains(symbol)
    }
}

impl Contains for ast::TSNamespaceExportDeclaration<'_> {
    fn contains(&self, _symbol: ContainsSymbol) -> bool {
        false
    }
}

impl Contains for ast::Statement<'_> {
    fn contains(&self, symbol: ContainsSymbol) -> bool {
        match self {
            ast::Statement::BlockStatement(st) => st.contains(symbol),
            ast::Statement::BreakStatement(_)
            | ast::Statement::ContinueStatement(_)
            | ast::Statement::DebuggerStatement(_) => false,
            ast::Statement::DoWhileStatement(st) => st.contains(symbol),
            ast::Statement::EmptyStatement(_) => false,
            ast::Statement::ExpressionStatement(st) => st.expression.contains(symbol),
            ast::Statement::ForInStatement(st) => st.contains(symbol),
            ast::Statement::ForOfStatement(st) => st.contains(symbol),
            ast::Statement::ForStatement(st) => st.contains(symbol),
            ast::Statement::IfStatement(st) => st.contains(symbol),
            ast::Statement::LabeledStatement(st) => st.contains(symbol),
            ast::Statement::ReturnStatement(st) => st.contains(symbol),
            ast::Statement::SwitchStatement(st) => st.contains(symbol),
            ast::Statement::ThrowStatement(st) => st.contains(symbol),
            ast::Statement::TryStatement(st) => st.contains(symbol),
            ast::Statement::WhileStatement(st) => st.contains(symbol),
            ast::Statement::WithStatement(st) => st.contains(symbol),
            ast::Statement::VariableDeclaration(st) => st.contains(symbol),
            ast::Statement::FunctionDeclaration(st) => st.contains(symbol),
            ast::Statement::ClassDeclaration(st) => st.contains(symbol),
            ast::Statement::TSTypeAliasDeclaration(_)
            | ast::Statement::TSInterfaceDeclaration(_) => false,
            ast::Statement::TSEnumDeclaration(st) => st.contains(symbol),
            ast::Statement::TSModuleDeclaration(st) => st.contains(symbol),
            ast::Statement::TSImportEqualsDeclaration(st) => st.contains(symbol),
            ast::Statement::ImportDeclaration(st) => st.contains(symbol),
            ast::Statement::ExportAllDeclaration(st) => st.contains(symbol),
            ast::Statement::ExportDefaultDeclaration(st) => st.contains(symbol),
            ast::Statement::ExportNamedDeclaration(st) => st.contains(symbol),
            ast::Statement::TSExportAssignment(st) => st.contains(symbol),
            ast::Statement::TSNamespaceExportDeclaration(st) => st.contains(symbol),
        }
    }
}

/// ### [8.5.2 Static Semantics: ComputedPropertyContains](https://tc39.es/ecma262/#sec-static-semantics-computedpropertycontains)
///
/// The syntax-directed operation ComputedPropertyContains takes argument
/// symbol (a grammar symbol) and returns a Boolean.
pub(crate) trait ComputedPropertyContains {
    /// ### [8.5.2 Static Semantics: ComputedPropertyContains](https://tc39.es/ecma262/#sec-static-semantics-computedpropertycontains)
    fn computed_property_contains(&self, symbol: ContainsSymbol) -> bool;
}

impl ComputedPropertyContains for ast::ClassBody<'_> {
    fn computed_property_contains(&self, symbol: ContainsSymbol) -> bool {
        self.body.iter().any(|e| match e {
            ast::ClassElement::StaticBlock(_) => false,
            ast::ClassElement::MethodDefinition(e) => e.computed_property_contains(symbol),
            ast::ClassElement::PropertyDefinition(p) => p.key.contains(symbol),
            ast::ClassElement::AccessorProperty(p) => p.key.contains(symbol),
            ast::ClassElement::TSIndexSignature(_) => false,
        })
    }
}

impl ComputedPropertyContains for ast::MethodDefinition<'_> {
    fn computed_property_contains(&self, symbol: ContainsSymbol) -> bool {
        self.key.contains(symbol)
    }
}
