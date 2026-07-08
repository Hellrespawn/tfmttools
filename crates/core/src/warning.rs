#[derive(Debug, PartialEq)]
pub enum Warning {
    WhitespaceInTag { file: String, tag_name: String },
    DeprecatedPositionalArgs { template: String },
    DeprecatedLeadingComment { template: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warning_variants_are_constructible() {
        let _ = Warning::WhitespaceInTag {
            file: "song.mp3".to_owned(),
            tag_name: "track_artist".to_owned(),
        };
        let _ = Warning::DeprecatedPositionalArgs {
            template: "my_template".to_owned(),
        };
        let _ = Warning::DeprecatedLeadingComment {
            template: "old_template".to_owned(),
        };
    }
}
