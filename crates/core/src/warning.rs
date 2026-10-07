#[derive(Debug, PartialEq)]
pub enum Warning {
    WhitespaceInTag { file: String, tag_name: String },
}
