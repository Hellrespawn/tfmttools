#[derive(Clone, Debug)]
pub(crate) enum Value {
    Text(String),
    Integer(i64),
    Path(Vec<String>),
}
