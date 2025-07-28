// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use num_traits::Num;
use oxc_ast::ast::RegExpFlags;
use wtf8::Wtf8Buf;

use crate::{
    ecmascript::{
        builtins::{
            ordinary::shape::ObjectShape,
            regexp::{RegExp, reg_exp_create_literal},
        },
        execution::Agent,
        types::{BigInt, IntoValue, Number, PropertyKey, String, Value},
    },
    engine::{
        Executable, ExecutableHeapData, FunctionExpression, Instruction,
        bytecode::executable::ArrowFunctionExpression,
        context::{Bindable, NoGcScope},
    },
    heap::CreateHeapData,
};

use super::{IndexType, JumpIndex};

/// Context for executable creation only. This struct contains the things
/// needed for adding instructions and nothing else, no scope tracking and
/// other metadata is included.
pub(super) struct ExecutableContext<'agent, 'gc, 'scope> {
    pub(crate) agent: &'agent mut Agent,
    pub(crate) gc: NoGcScope<'gc, 'scope>,
    /// true if the current last instruction is a terminal instruction and no
    /// jumps point past it.
    current_instruction_pointer_is_unreachable: bool,
    /// Instructions being built
    instructions: Vec<u8>,
    /// Constants being built
    constants: Vec<Value<'gc>>,
    /// Object Shapes being built
    shapes: Vec<ObjectShape<'gc>>,
    /// Function expressions being built
    function_expressions: Vec<FunctionExpression<'gc>>,
    /// Arrow function expressions being built
    arrow_function_expressions: Vec<ArrowFunctionExpression>,
    class_initializer_bytecodes: Vec<(Option<Executable<'gc>>, bool)>,
}

impl<'agent, 'gc, 'scope> ExecutableContext<'agent, 'gc, 'scope> {
    pub(super) fn new(agent: &'agent mut Agent, gc: NoGcScope<'gc, 'scope>) -> Self {
        Self {
            agent,
            gc,
            current_instruction_pointer_is_unreachable: false,
            instructions: Vec::new(),
            constants: Vec::new(),
            shapes: Vec::new(),
            function_expressions: Vec::new(),
            arrow_function_expressions: Vec::new(),
            class_initializer_bytecodes: Vec::new(),
        }
    }

    pub(super) fn get_agent_and_gc(&mut self) -> (&mut Agent, NoGcScope<'gc, 'scope>) {
        (&mut self.agent, self.gc)
    }

    pub(crate) fn get_agent(&self) -> &Agent {
        self.agent
    }

    pub(crate) fn get_agent_mut(&mut self) -> &mut Agent {
        self.agent
    }

    pub(super) fn create_bigint(&mut self, literal: &str, radix: u32) -> BigInt<'gc> {
        if let Ok(result) = i64::from_str_radix(literal, radix) {
            BigInt::from_i64(self.agent, result).bind(self.gc)
        } else {
            BigInt::from_num_bigint(
                self.agent,
                num_bigint::BigInt::from_str_radix(literal, radix).unwrap(),
            )
            .bind(self.gc)
        }
    }

    pub(super) fn create_number(&mut self, value: f64) -> Number<'gc> {
        Number::from_f64(self.agent, value, self.gc)
    }

    pub(crate) fn create_property_key(&mut self, literal: &str) -> PropertyKey<'gc> {
        PropertyKey::from_str(self.agent, literal, self.gc)
    }

    pub(crate) fn create_regexp(&mut self, literal: &str, flags: RegExpFlags) -> RegExp<'gc> {
        let pattern = self.create_string(literal);
        reg_exp_create_literal(self.agent, pattern, Some(flags), self.gc)
    }

    pub(super) fn create_string(&mut self, literal: &str) -> String<'gc> {
        String::from_str(self.agent, literal, self.gc)
    }

    pub(super) fn create_string_from_owned(&mut self, owned: std::string::String) -> String<'gc> {
        String::from_string(self.agent, owned, self.gc)
    }

    pub(super) fn create_string_from_wtf8_buf(&mut self, buf: Wtf8Buf) -> String<'gc> {
        String::from_wtf8_buf(self.agent, buf, self.gc)
    }

    /// Returns true if the current instruction pointer is a unreachable.
    pub(super) fn is_unreachable(&self) -> bool {
        self.current_instruction_pointer_is_unreachable
    }

    pub(super) fn finish(self) -> Executable<'gc> {
        self.agent.heap.create(ExecutableHeapData {
            instructions: self.instructions.into_boxed_slice(),
            constants: self.constants.unbind().into_boxed_slice(),
            shapes: self.shapes.unbind().into_boxed_slice(),
            function_expressions: self.function_expressions.unbind().into_boxed_slice(),
            arrow_function_expressions: self.arrow_function_expressions.into_boxed_slice(),
            class_initializer_bytecodes: self
                .class_initializer_bytecodes
                .into_iter()
                .map(|(exe, b)| (exe.unbind(), b))
                .collect(),
        })
    }

    pub(super) fn add_instruction(&mut self, instruction: Instruction) {
        debug_assert_eq!(instruction.argument_count(), 0);
        debug_assert!(
            !instruction.has_constant_index()
                && !instruction.has_function_expression_index()
                && !instruction.has_identifier_index()
        );
        self.push_instruction(instruction);
    }

    pub(super) fn add_instruction_with_jump_slot(&mut self, instruction: Instruction) -> JumpIndex {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_jump_slot());
        self.push_instruction(instruction);
        self.add_jump_index()
    }

    pub(super) fn add_jump_instruction_to_index(
        &mut self,
        instruction: Instruction,
        jump_index: JumpIndex,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_jump_slot());
        self.push_instruction(instruction);
        self.add_double_index(jump_index.index);
    }

    pub(super) fn get_jump_index_to_here(&mut self) -> JumpIndex {
        self.current_instruction_pointer_is_unreachable = false;
        JumpIndex {
            index: self.instructions.len(),
        }
    }

    pub(super) fn add_constant(&mut self, constant: Value<'gc>) -> usize {
        let duplicate = self
            .constants
            .iter()
            .enumerate()
            .find(|item| item.1.eq(&constant))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.constants.len();
            self.constants.push(constant);
            index
        })
    }

    pub(super) fn add_identifier(&mut self, identifier: String<'gc>) -> usize {
        let duplicate = self
            .constants
            .iter()
            .enumerate()
            .find(|item| String::try_from(*item.1) == Ok(identifier))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.constants.len();
            self.constants.push(identifier.into_value());
            index
        })
    }

    pub(super) fn add_shape(&mut self, shape: ObjectShape<'gc>) -> usize {
        let duplicate = self
            .shapes
            .iter()
            .enumerate()
            .find(|item| item.1.eq(&shape))
            .map(|(idx, _)| idx);

        duplicate.unwrap_or_else(|| {
            let index = self.shapes.len();
            self.shapes.push(shape);
            index
        })
    }

    pub(super) fn add_instruction_with_immediate(
        &mut self,
        instruction: Instruction,
        immediate: usize,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        self.push_instruction(instruction);
        self.add_index(immediate);
    }

    pub(super) fn add_instruction_with_constant(
        &mut self,
        instruction: Instruction,
        constant: impl Into<Value<'gc>>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_constant_index());
        self.push_instruction(instruction);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    pub(super) fn add_instruction_with_identifier(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_identifier_index());
        self.push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
    }

    pub(super) fn add_instruction_with_identifier_and_constant(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
        constant: impl Into<Value<'gc>>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_identifier_index() && instruction.has_constant_index());
        self.push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
        let constant = self.add_constant(constant.into());
        self.add_index(constant);
    }

    pub(super) fn add_instruction_with_identifier_and_immediate(
        &mut self,
        instruction: Instruction,
        identifier: String<'gc>,
        immediate: usize,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_identifier_index());
        self.push_instruction(instruction);
        let identifier = self.add_identifier(identifier);
        self.add_index(identifier);
        self.add_index(immediate);
    }

    pub(super) fn add_instruction_with_immediate_and_immediate(
        &mut self,
        instruction: Instruction,
        immediate1: usize,
        immediate2: usize,
    ) {
        debug_assert_eq!(instruction.argument_count(), 2);
        self.push_instruction(instruction);
        self.add_index(immediate1);
        self.add_index(immediate2)
    }

    pub(super) fn add_instruction_with_function_expression(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self.push_instruction(instruction);
        self.function_expressions.push(function_expression);
        let index = self.function_expressions.len() - 1;
        self.add_index(index);
    }

    /// Add an Instruction that takes a function expression and an immediate
    /// as its bytecode parameters.
    ///
    /// Returns the function expression's index.
    pub(super) fn add_instruction_with_function_expression_and_immediate(
        &mut self,
        instruction: Instruction,
        function_expression: FunctionExpression<'gc>,
        immediate: usize,
    ) -> IndexType {
        debug_assert_eq!(instruction.argument_count(), 2);
        debug_assert!(instruction.has_function_expression_index());
        self.push_instruction(instruction);
        let index = self.function_expressions.len();
        self.function_expressions.push(function_expression);
        self.add_index(index);
        self.add_index(immediate);
        // Note: add_index would have panicked if this was not a lossless
        // conversion.
        index as IndexType
    }

    pub(super) fn add_instruction_with_shape(
        &mut self,
        instruction: Instruction,
        shape: ObjectShape<'gc>,
    ) {
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_shape_index());
        self.push_instruction(instruction);
        let shape = self.add_shape(shape);
        self.add_index(shape);
    }

    pub(super) fn add_arrow_function_expression(
        &mut self,
        arrow_function_expression: ArrowFunctionExpression,
    ) {
        let instruction = Instruction::InstantiateArrowFunctionExpression;
        debug_assert_eq!(instruction.argument_count(), 1);
        debug_assert!(instruction.has_function_expression_index());
        self.push_instruction(instruction);
        self.arrow_function_expressions
            .push(arrow_function_expression);
        let index = self.arrow_function_expressions.len() - 1;
        self.add_index(index);
    }

    pub(super) fn set_jump_target(&mut self, source: JumpIndex, target: JumpIndex) {
        assert!(target.index < u32::MAX as usize);
        let bytes: [u8; 4] = (target.index as u32).to_ne_bytes();
        self.instructions[source.index..source.index + 4].copy_from_slice(&bytes);
    }

    pub(super) fn set_jump_target_here(&mut self, jump: JumpIndex) {
        if self.current_instruction_pointer_is_unreachable
            && jump.index == self.instructions.len().saturating_sub(4)
        {
            // OPTIMISATION: An unconditional jump to next instruction
            // can be popped from the bytecode stream.
            self.instructions
                .truncate(self.instructions.len().saturating_sub(5));
            // After popping the Jump off, we're no longer on unreachable
            // ground.
            self.current_instruction_pointer_is_unreachable = false;
            return;
        }
        self.set_jump_target(
            jump,
            JumpIndex {
                index: self.instructions.len(),
            },
        );
        self.current_instruction_pointer_is_unreachable = false;
    }

    pub(super) fn get_next_class_initializer_index(&self) -> IndexType {
        IndexType::try_from(self.class_initializer_bytecodes.len()).unwrap()
    }

    pub(super) fn set_function_expression_bytecode(
        &mut self,
        index: IndexType,
        executable: Executable<'gc>,
    ) {
        self.function_expressions[index as usize].compiled_bytecode = Some(executable);
    }

    pub(super) fn add_class_initializer_bytecode(
        &mut self,
        executable: Executable<'gc>,
        has_constructor_parent: bool,
    ) {
        self.class_initializer_bytecodes
            .push((Some(executable), has_constructor_parent));
    }

    pub(super) fn add_class_initializer(&mut self, has_constructor_parent: bool) {
        self.class_initializer_bytecodes
            .push((None, has_constructor_parent));
    }

    fn push_instruction(&mut self, instruction: Instruction) {
        self.instructions.push(instruction.as_u8());
        self.current_instruction_pointer_is_unreachable = instruction.is_terminal();
    }

    fn add_index(&mut self, index: usize) {
        let index = IndexType::try_from(index).expect("Immediate value is too large");
        let bytes: [u8; 2] = index.to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_double_index(&mut self, index: usize) {
        let index = u32::try_from(index).expect("Immediate value is too large");
        let bytes: [u8; 4] = index.to_ne_bytes();
        self.instructions.extend_from_slice(&bytes);
    }

    fn add_jump_index(&mut self) -> JumpIndex {
        self.add_double_index(0);
        JumpIndex {
            index: self.instructions.len() - core::mem::size_of::<u32>(),
        }
    }
}
