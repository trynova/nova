use crate::{ecmascript::types::String, SmallString};

pub(crate) const ERROR_CLASS_NAME: String = String::from_small_string("Error");
pub(crate) const EMPTY_STRING: String = String::SmallString(SmallString::EMPTY);
pub(crate) const AT_KEY: String = String::from_small_string("at");
pub(crate) const CALLEE_KEY: String = String::from_small_string("callee");
pub(crate) const FILL_KEY: String = String::from_small_string("fill");
pub(crate) const FIND_KEY: String = String::from_small_string("find");
pub(crate) const FLAT_KEY: String = String::from_small_string("flat");
pub(crate) const FLAT_MAP_KEY: String = String::from_small_string("flatMap");
pub(crate) const KEYS_KEY: String = String::from_small_string("keys");
pub(crate) const LENGTH_KEY: String = String::from_small_string("length");
pub(crate) const NAME_KEY: String = String::from_small_string("name");
pub(crate) const VALUES_KEY: String = String::from_small_string("values");
