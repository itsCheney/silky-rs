use std::fmt;

/// A script value. Variables of the value banks, locals and the stack hold
/// either an integer or a string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Int(i32),
    Str(String),
}

impl Default for Value {
    fn default() -> Self {
        Value::Int(0)
    }
}

impl Value {
    /// Integer value; strings count as 0.
    pub fn int(&self) -> i32 {
        match self {
            Value::Int(v) => *v,
            Value::Str(_) => 0,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            Value::Int(_) => None,
        }
    }

    pub fn is_true(&self) -> bool {
        match self {
            Value::Int(v) => *v != 0,
            Value::Str(s) => !s.is_empty(),
        }
    }
}

impl From<i32> for Value {
    fn from(v: i32) -> Self {
        Value::Int(v)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::Str(s.to_string())
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(v) => write!(f, "{v}"),
            Value::Str(s) => write!(f, "{s:?}"),
        }
    }
}
