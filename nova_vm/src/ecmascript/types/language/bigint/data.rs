use crate::ecmascript::execution::agent::JsError;
use num_bigint_dig::BigInt;
// use num_traits::cast::ToPrimitive;

#[derive(Debug, Clone)]
pub struct BigIntHeapData {
    pub(super) data: BigInt,
}

impl TryInto<f64> for BigIntHeapData {
    type Error = JsError;

    fn try_into(self) -> Result<f64, Self::Error> {
        // self.data.to_f64()
        Err(JsError {})
    }
}
