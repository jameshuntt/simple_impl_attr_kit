//! What a key can hold.

use crate::AttrBag;

/// The value after a key, or the absence of one.
///
/// `key` alone is `Bool(true)`. `key = <literal>` is the literal's kind.
/// `key = <anything else>` is kept as [`Tokens`](Self::Tokens): a type, a
/// path, a call, an expression, verbatim. `key(a, b)` and `key = [a, b]`
/// are [`List`](Self::List); `key(inner = 1, flag)` is
/// [`Nested`](Self::Nested); a key written twice becomes a `List` of its
/// values.
#[derive(Debug, Clone, PartialEq)]
pub enum AttrValue {
    /// `key = "text"`
    Str(String),
    /// `key` or `key = true` / `key = false`
    Bool(bool),
    /// `key = 3`, `key = -3`
    Int(i64),
    /// `key = 1.5`, `key = -0.25`
    Float(f64),
    /// `key = 'c'`
    Char(char),
    /// `key = b"bytes"`
    Bytes(Vec<u8>),
    /// `key = some::Path`, `key = Vec<u8>`, `key = f(1, 2)`: the tokens as written.
    Tokens(String),
    /// `key(a, "b", 3)`, `key = [a, "b"]`, or a key given more than once.
    List(Vec<AttrValue>),
    /// `key(inner = "x", flag)`: a bag of its own.
    Nested(AttrBag),
}

impl AttrValue {
    /// The string, if this is a `Str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::Str(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// The bool, if this is a `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    /// The integer, if this is an `Int`.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The float, if this is a `Float`; an `Int` is widened.
    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(value) => Some(*value),
            Self::Int(value) => Some(*value as f64),
            _ => None,
        }
    }

    /// The char, if this is a `Char`.
    pub fn as_char(&self) -> Option<char> {
        match self {
            Self::Char(value) => Some(*value),
            _ => None,
        }
    }

    /// The bytes, if this is `Bytes`.
    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Bytes(value) => Some(value.as_slice()),
            _ => None,
        }
    }

    /// The token text, if this is `Tokens`.
    pub fn as_tokens(&self) -> Option<&str> {
        match self {
            Self::Tokens(value) => Some(value.as_str()),
            _ => None,
        }
    }

    /// The elements, if this is a `List`.
    pub fn as_list(&self) -> Option<&[AttrValue]> {
        match self {
            Self::List(values) => Some(values.as_slice()),
            _ => None,
        }
    }

    /// The inner bag, if this is `Nested`.
    pub fn as_nested(&self) -> Option<&AttrBag> {
        match self {
            Self::Nested(bag) => Some(bag),
            _ => None,
        }
    }

    /// The kind, for messages.
    pub fn kind_name(&self) -> &'static str {
        match self {
            Self::Str(_) => "string",
            Self::Bool(_) => "bool",
            Self::Int(_) => "integer",
            Self::Float(_) => "float",
            Self::Char(_) => "char",
            Self::Bytes(_) => "byte string",
            Self::Tokens(_) => "tokens",
            Self::List(_) => "list",
            Self::Nested(_) => "nested attribute",
        }
    }
}

impl From<String> for AttrValue {
    fn from(value: String) -> Self {
        Self::Str(value)
    }
}

impl From<&str> for AttrValue {
    fn from(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl From<bool> for AttrValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<i64> for AttrValue {
    fn from(value: i64) -> Self {
        Self::Int(value)
    }
}

impl From<f64> for AttrValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<char> for AttrValue {
    fn from(value: char) -> Self {
        Self::Char(value)
    }
}

impl From<Vec<AttrValue>> for AttrValue {
    fn from(values: Vec<AttrValue>) -> Self {
        Self::List(values)
    }
}
