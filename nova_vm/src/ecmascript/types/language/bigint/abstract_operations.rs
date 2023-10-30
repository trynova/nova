use crate::{
    ecmascript::{execution::Agent, types::BigInt},
    heap::GetHeapData,
};

/// ### [6.1.6.2.12 BigInt::lessThan ( x, y )](https://tc39.es/ecma262/#sec-numeric-types-bigint-lessThan)
///
/// The abstract operation BigInt::lessThan takes arguments x (a BigInt) and
/// y (a BigInt) and returns a Boolean. It performs the following steps when
/// called:
pub(crate) fn less_than(agent: &mut Agent, x: BigInt, y: BigInt) -> bool {
    // 1. If ℝ(x) < ℝ(y), return true; otherwise return false.
    match (x, y) {
        (BigInt::BigInt(_), BigInt::SmallBigInt(_)) => false,
        (BigInt::SmallBigInt(_), BigInt::BigInt(_)) => true,
        (BigInt::BigInt(b1), BigInt::BigInt(b2)) => {
            let (b1, b2) = (agent.heap.get(b1), agent.heap.get(b2));
            b1.data < b2.data
        }
        (BigInt::SmallBigInt(b1), BigInt::SmallBigInt(b2)) => b1.into_i64() < b2.into_i64(),
    }
}
