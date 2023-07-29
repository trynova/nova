use crate::{
    execution::Agent,
    heap::GetHeapData,
    types::{String, Value},
    SmallString,
};

#[derive(Debug, Clone, Copy)]
pub struct PropertyKey(Value);

impl Default for PropertyKey {
    fn default() -> Self {
        Self(Value::SmallString(SmallString::from_str_unchecked(
            "unknown",
        )))
    }
}

impl PropertyKey {
    pub(crate) fn new(value: Value) -> Self {
        debug_assert!(matches!(
            value,
            Value::IntegerNumber(_) | Value::String(_) | Value::SmallString(_)
        ));
        Self(value)
    }

    pub fn into_value(self) -> Value {
        self.0
    }

    pub fn is_array_index(self) -> bool {
        // TODO: string check
        matches!(self.into_value(), Value::IntegerNumber(_))
    }

    pub(self) fn is_str_eq_num(s: &str, n: i64) -> bool {
        let (s, mut n) = if s.starts_with("-") {
            if n > 0 {
                return false;
            }
            (&s[1..], -n as usize)
        } else {
            if n < 0 {
                return false;
            }
            (s, n as usize)
        };

        if Some(s.len()) != n.checked_ilog10().map(|n| n as usize) {
            return false;
        }

        for c in s.as_bytes().iter().rev() {
            let code = (n % 10) as u8 + '0' as u8;

            if *c != code {
                return false;
            }

            n /= 10;
        }

        true
    }

    pub fn equals(self, agent: &mut Agent, y: Self) -> bool {
        let x = self.into_value();
        let y = y.into_value();

        match (x, y) {
            // Assumes the interner is working correctly.
            (Value::String(s1), Value::String(s2)) => s1 == s2,
            (Value::String(s), Value::IntegerNumber(n)) => {
                let realm = agent.current_realm();
                let realm = realm.borrow();
                let s = realm.heap.get(s);

                let Some(s) = s.as_str() else {
					return false;
				};

                Self::is_str_eq_num(s, n.into_i64())
            }
            _ => unreachable!(),
        }
    }
}

impl From<String> for PropertyKey {
    fn from(value: String) -> Self {
        Self(value.into_value())
    }
}

impl TryFrom<Value> for PropertyKey {
    type Error = ();
    fn try_from(value: Value) -> Result<Self, Self::Error> {
        if value.is_string() || value.is_symbol() || value.is_number() {
            Ok(Self(value))
        } else {
            Err(())
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
