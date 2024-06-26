use crate::{
    ecmascript::{
        abstract_operations::type_conversion::parse_string_to_integer_property_key,
        execution::Agent,
        types::{
            language::{
                string::HeapString,
                value::{
                    INTEGER_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
                    SYMBOL_DISCRIMINANT,
                },
            },
            String, Symbol, Value,
        },
    },
    SmallInteger, SmallString,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PropertyKey {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    String(HeapString) = STRING_DISCRIMINANT,
    Symbol(Symbol) = SYMBOL_DISCRIMINANT,
    // TODO: PrivateKey
}

impl PropertyKey {
    // FIXME: This API is not necessarily in the right place.
    pub fn from_str(agent: &mut Agent, str: &str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_str(agent, str).into())
    }

    pub fn from_static_str(agent: &mut Agent, str: &'static str) -> Self {
        parse_string_to_integer_property_key(str)
            .unwrap_or_else(|| String::from_static_str(agent, str).into())
    }

    pub fn from_string(agent: &mut Agent, string: std::string::String) -> Self {
        parse_string_to_integer_property_key(&string)
            .unwrap_or_else(|| String::from_string(agent, string).into())
    }

    pub fn into_value(self) -> Value {
        self.into()
    }

    pub fn is_array_index(self) -> bool {
        // TODO: string check
        matches!(self.into_value(), Value::Integer(_))
    }

    pub(self) fn is_str_eq_num(s: &str, n: i64) -> bool {
        // TODO: Come up with some advanced algorithm.
        s == n.to_string()
    }

    pub fn equals(self, agent: &mut Agent, y: Self) -> bool {
        let x = self;

        match (x, y) {
            // Assumes the interner is working correctly.
            (PropertyKey::String(s1), PropertyKey::String(s2)) => s1 == s2,
            (PropertyKey::SmallString(s1), PropertyKey::SmallString(s2)) => {
                s1.as_str() == s2.as_str()
            }
            (PropertyKey::String(s), PropertyKey::Integer(n)) => {
                let s = agent[s].as_str();

                Self::is_str_eq_num(s, n.into_i64())
            }
            (PropertyKey::SmallString(s), PropertyKey::Integer(n)) => {
                Self::is_str_eq_num(s.as_str(), n.into_i64())
            }
            (PropertyKey::Integer(n1), PropertyKey::Integer(n2)) => n1.into_i64() == n2.into_i64(),
            (PropertyKey::Integer(_), _) => y.equals(agent, self),
            _ => false,
        }
    }
}

impl From<u32> for PropertyKey {
    fn from(value: u32) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<u16> for PropertyKey {
    fn from(value: u16) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<u8> for PropertyKey {
    fn from(value: u8) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i32> for PropertyKey {
    fn from(value: i32) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i16> for PropertyKey {
    fn from(value: i16) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<i8> for PropertyKey {
    fn from(value: i8) -> Self {
        PropertyKey::Integer(value.into())
    }
}

impl From<SmallInteger> for PropertyKey {
    fn from(value: SmallInteger) -> Self {
        PropertyKey::Integer(value)
    }
}

impl From<SmallString> for PropertyKey {
    fn from(value: SmallString) -> Self {
        PropertyKey::SmallString(value)
    }
}

impl From<HeapString> for PropertyKey {
    fn from(value: HeapString) -> Self {
        PropertyKey::String(value)
    }
}

impl From<Symbol> for PropertyKey {
    fn from(value: Symbol) -> Self {
        PropertyKey::Symbol(value)
    }
}

impl From<String> for PropertyKey {
    fn from(value: String) -> Self {
        match value {
            String::String(x) => PropertyKey::String(x),
            String::SmallString(x) => PropertyKey::SmallString(x),
        }
    }
}

impl From<PropertyKey> for Value {
    fn from(value: PropertyKey) -> Self {
        match value {
            PropertyKey::Integer(x) => Value::Integer(x),
            PropertyKey::SmallString(x) => Value::SmallString(x),
            PropertyKey::String(x) => Value::String(x),
            PropertyKey::Symbol(x) => Value::Symbol(x),
        }
    }
}

impl TryFrom<Value> for PropertyKey {
    type Error = ();

    #[inline(always)]
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(x) => Ok(PropertyKey::Integer(x)),
            Value::Float(x) => {
                if x == -0.0f32 {
                    Ok(PropertyKey::Integer(0.into()))
                } else if x.fract() == 0.0
                    && (SmallInteger::MIN_NUMBER..=SmallInteger::MAX_NUMBER).contains(&(x as i64))
                {
                    unreachable!("Value::Float should not contain safe integers");
                } else {
                    Err(())
                }
            }
            Value::SmallString(x) => Ok(PropertyKey::SmallString(x)),
            Value::String(x) => Ok(PropertyKey::String(x)),
            Value::Symbol(x) => Ok(PropertyKey::Symbol(x)),
            Value::SmallBigInt(x)
                if (SmallInteger::MIN_NUMBER..=SmallInteger::MAX_NUMBER)
                    .contains(&x.into_i64()) =>
            {
                Ok(PropertyKey::Integer(x.into_inner()))
            }
            _ => Err(()),
        }
    }
}

#[test]
fn compare_num_str() {
    assert!(PropertyKey::is_str_eq_num("23", 23));
    assert!(PropertyKey::is_str_eq_num("-23", -23));
    assert!(PropertyKey::is_str_eq_num("-120543809", -120543809));
    assert!(PropertyKey::is_str_eq_num("985493", 985493));
    assert!(PropertyKey::is_str_eq_num("0", 0));
    assert!(PropertyKey::is_str_eq_num("5", 5));
    assert!(PropertyKey::is_str_eq_num("-5", -5));
    assert!(PropertyKey::is_str_eq_num("9302", 9302));
    assert!(PropertyKey::is_str_eq_num("19", 19));

    assert!(!PropertyKey::is_str_eq_num("19", 91));
    assert!(!PropertyKey::is_str_eq_num("-19", 19));
}
