use crate::{
    ecmascript::{
        execution::Agent,
        types::{
            language::value::{
                INTEGER_DISCRIMINANT, SMALL_STRING_DISCRIMINANT, STRING_DISCRIMINANT,
                SYMBOL_DISCRIMINANT,
            },
            String, Value,
        },
    },
    heap::{
        indexes::{StringIndex, SymbolIndex},
        CreateHeapData, GetHeapData,
    },
    Heap, SmallInteger, SmallString,
};

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
pub enum PropertyKey {
    Integer(SmallInteger) = INTEGER_DISCRIMINANT,
    SmallString(SmallString) = SMALL_STRING_DISCRIMINANT,
    String(StringIndex) = STRING_DISCRIMINANT,
    Symbol(SymbolIndex) = SYMBOL_DISCRIMINANT,
}

impl PropertyKey {
    // FIXME: This API is not necessarily in the right place.
    pub fn from_str(heap: &mut Heap, str: &str) -> Self {
        heap.create(str).into()
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
                let s = agent.heap.get(s);

                let Some(s) = s.as_str() else {
                    return false;
                };

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

impl From<StringIndex> for PropertyKey {
    fn from(value: StringIndex) -> Self {
        PropertyKey::String(value)
    }
}

impl From<SymbolIndex> for PropertyKey {
    fn from(value: SymbolIndex) -> Self {
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
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        match value {
            Value::Integer(x) => Ok(PropertyKey::Integer(x)),
            Value::SmallString(x) => Ok(PropertyKey::SmallString(x)),
            Value::String(x) => Ok(PropertyKey::String(x)),
            Value::Symbol(x) => Ok(PropertyKey::Symbol(x)),
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
