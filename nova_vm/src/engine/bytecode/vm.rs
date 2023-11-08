use oxc_span::Atom;
use oxc_syntax::operator::BinaryOperator;

use crate::ecmascript::{
    abstract_operations::{
        testing_and_comparison::is_same_type,
        type_conversion::{to_number, to_numeric, to_primitive, to_string},
    },
    execution::{
        agent::{resolve_binding, ExceptionType},
        Agent, EnvironmentIndex, JsResult,
    },
    types::{Base, BigInt, Number, Reference, Value},
};

use super::{Executable, IndexType, Instr, Instruction, InstructionIter};

#[derive(Debug)]
pub(crate) struct Vm {
    /// Instruction pointer.
    ip: usize,
    stack: Vec<Value>,
    reference_stack: Vec<Reference>,
    exception_jump_target_stack: Vec<usize>,
    result: Value,
    exception: Option<Value>,
    reference: Option<Reference>,
}

impl Vm {
    fn new() -> Self {
        Self {
            ip: 0,
            stack: Vec::with_capacity(32),
            reference_stack: Vec::new(),
            exception_jump_target_stack: Vec::new(),
            result: Value::Undefined,
            exception: None,
            reference: None,
        }
    }

    fn fetch_identifier<'a>(&self, exe: &'a Executable, index: usize) -> &'a Atom {
        &exe.identifiers[index]
    }

    fn fetch_constant(&self, exe: &Executable, index: usize) -> Value {
        exe.constants[index]
    }

    /// Executes an executable using the virtual machine.
    pub(crate) fn execute(agent: &mut Agent, executable: &Executable) -> JsResult<Value> {
        let mut vm = Vm::new();

        let mut iter = InstructionIter::new(&executable.instructions);

        while let Some(instr) = iter.next() {
            eprintln!("{:?} {:?}", instr.kind, instr.args);

            match instr.kind {
                Instruction::ResolveBinding => {
                    let identifier =
                        vm.fetch_identifier(executable, instr.args[0].unwrap() as usize);

                    let reference = resolve_binding(agent, &identifier, None)?;

                    vm.result = match reference.base {
                        Base::Value(value) => value,
                        _ => {
                            return Err(agent.throw_exception(
                                ExceptionType::ReferenceError,
                                "Unable to resolve identifier.",
                            ));
                        }
                    };

                    vm.reference = Some(reference);
                }
                Instruction::LoadConstant => {
                    let constant = vm.fetch_constant(executable, instr.args[0].unwrap() as usize);
                    vm.stack.push(constant);
                }
                Instruction::Load => {
                    vm.stack.push(vm.result);
                }
                Instruction::Return => {
                    return Ok(vm.result);
                }
                Instruction::Store => {
                    vm.result = vm.stack.pop().unwrap();
                }
                Instruction::StoreConstant => {
                    let constant = vm.fetch_constant(executable, instr.args[0].unwrap() as usize);
                    vm.result = constant;
                }
                Instruction::UnaryMinus => {
                    vm.result = Number::unary_minus(vm.result.try_into().unwrap(), agent).into();
                }
                Instruction::ToNumber => {
                    vm.result = to_number(agent, vm.result).map(|number| number.into())?;
                }
                Instruction::ToNumeric => {
                    vm.result = to_numeric(agent, vm.result)?;
                }
                Instruction::ApplyStringOrNumericBinaryOperator(op_text) => {
                    let lval = vm.stack.pop().unwrap();
                    let rval = vm.stack.pop().unwrap();
                    vm.result =
                        apply_string_or_numeric_binary_operator(agent, lval, op_text, rval)?;
                }
                Instruction::PushReference => {
                    vm.reference_stack.push(vm.reference.take().unwrap());
                }
                Instruction::PopReference => {
                    vm.reference_stack.pop();
                }
                Instruction::PutValue => {
                    let reference = vm.reference_stack.last_mut().unwrap();
                    reference.base = Base::Value(vm.stack.pop().unwrap());
                }
                other => todo!("{other:?}"),
            }
        }

        Ok(vm.result)
    }
}

/// ### [13.15.3 ApplyStringOrNumericBinaryOperator ( lval, opText, rval )](https://tc39.es/ecma262/#sec-applystringornumericbinaryoperator)
///
/// The abstract operation ApplyStringOrNumericBinaryOperator takes
/// arguments lval (an ECMAScript language value), opText (**, *, /, %, +,
/// -, <<, >>, >>>, &, ^, or |), and rval (an ECMAScript language value) and
/// returns either a normal completion containing either a String, a BigInt,
/// or a Number, or a throw completion.
#[inline]
fn apply_string_or_numeric_binary_operator(
    agent: &mut Agent,
    mut lval: Value,
    op_text: BinaryOperator,
    mut rval: Value,
) -> JsResult<Value> {
    // 1. If opText is +, then
    if op_text == BinaryOperator::Addition {
        // a. Let lprim be ? ToPrimitive(lval).
        let lprim = to_primitive(agent, lval, None)?;

        // b. Let rprim be ? ToPrimitive(rval).
        let rprim = to_primitive(agent, rval, None)?;

        // c. If lprim is a String or rprim is a String, then
        if lprim.is_string() || rprim.is_string() {
            // i. Let lstr be ? ToString(lprim).
            let lstr = to_string(agent, lprim)?;

            // ii. Let rstr be ? ToString(rprim).
            let rstr = to_string(agent, rprim)?;

            // iii. Return the string-concatenation of lstr and rstr.
            todo!("Concatenate the strings.")
        }

        // d. Set lval to lprim.
        lval = lprim;

        // e. Set rval to rprim.
        rval = rprim;
    }

    // 2. NOTE: At this point, it must be a numeric operation.

    // 3. Let lnum be ? ToNumeric(lval).
    let lnum = to_numeric(agent, lval)?;

    // 4. Let rnum be ? ToNumeric(rval).
    let rnum = to_numeric(agent, rval)?;

    // 5. If Type(lnum) is not Type(rnum), throw a TypeError exception.
    if !is_same_type(lnum, rnum) {
        return Err(agent.throw_exception(
            ExceptionType::TypeError,
            "The left and right-hand sides do not have the same type.",
        ));
    }

    // 6. If lnum is a BigInt, then
    if let (Ok(lnum), Ok(rnum)) = (BigInt::try_from(lnum), BigInt::try_from(rnum)) {
        match op_text {
            // a. If opText is **, return ? BigInt::exponentiate(lnum, rnum).
            BinaryOperator::Exponential => {
                return BigInt::exponentiate(agent, lnum, rnum).map(|bigint| bigint.into())
            }
            // b. If opText is /, return ? BigInt::divide(lnum, rnum).
            BinaryOperator::Division => todo!(),
            // c. If opText is %, return ? BigInt::remainder(lnum, rnum).
            BinaryOperator::Remainder => todo!(),
            // d. If opText is >>>, return ? BigInt::unsignedRightShift(lnum, rnum).
            BinaryOperator::ShiftRightZeroFill => todo!(),
            _ => unreachable!(),
        }
    }

    // 7. Let operation be the abstract operation associated with opText and
    // Type(lnum) in the following table:
    // 8. Return operation(lnum, rnum).
    // NOTE: We do step 8. explicitly in branch.
    Ok(match op_text {
        // opText	Type(lnum)	operation
        // **	Number	Number::exponentiate
        BinaryOperator::Exponential if lnum.is_number() => {
            Number::exponentiate(lnum.try_into().unwrap(), agent, rnum.try_into().unwrap()).into()
        }
        // *	Number	Number::multiply
        // *	BigInt	BigInt::multiply
        // /	Number	Number::divide
        // %	Number	Number::remainder
        // +	Number	Number::add
        BinaryOperator::Addition if lnum.is_number() => {
            Number::add(agent, lnum.try_into().unwrap(), rnum.try_into().unwrap()).into()
        }
        // +	BigInt	BigInt::add
        // -	Number	Number::subtract
        // -	BigInt	BigInt::subtract
        // <<	Number	Number::leftShift
        // <<	BigInt	BigInt::leftShift
        // >>	Number	Number::signedRightShift
        // >>	BigInt	BigInt::signedRightShift
        // >>>	Number	Number::unsignedRightShift
        // &	Number	Number::bitwiseAND
        // &	BigInt	BigInt::bitwiseAND
        // ^	Number	Number::bitwiseXOR
        // ^	BigInt	BigInt::bitwiseXOR
        // |	Number	Number::bitwiseOR
        // |	BigInt	BigInt::bitwiseOR
        _ => todo!(),
    })
}
