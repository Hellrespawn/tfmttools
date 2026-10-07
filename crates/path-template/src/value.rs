#[derive(Clone, Debug)]
pub(crate) enum Value {
    Text(String),
    Integer(i64),
    Path(Vec<String>),
}

impl Value {
    pub(crate) fn is_present(&self) -> bool {
        match self {
            Self::Text(text) => !text.is_empty(),
            Self::Integer(_) => true,
            Self::Path(components) => !components.is_empty(),
        }
    }
}

impl From<Scalar> for Value {
    fn from(value: Scalar) -> Self {
        match value {
            Scalar::Text(text) => Self::Text(text),
            Scalar::Integer(integer) => Self::Integer(integer),
        }
    }
}
/// A caller supplied metadata value. Missing values use `None`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Scalar {
    Text(String),
    Integer(i64),
}
