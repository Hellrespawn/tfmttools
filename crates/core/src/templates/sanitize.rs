use crate::action::FORBIDDEN_CHARACTERS;
use crate::error::{TFMTError, TFMTResult};

/// Apply filename character rules to a completed component.
pub fn sanitize_component(value: &str, strict: bool) -> TFMTResult<String> {
    if let Some(character) = tfmttools_picotmpl::prohibited_character(value) {
        return Err(TFMTError::Template(format!(
            "Component {value:?} contains prohibited character {character:?}"
        )));
    }
    let sanitized = sanitize_tag_value(value);
    if strict && sanitized != value {
        return Err(TFMTError::Template(format!(
            "Component {value:?} contains forbidden characters (sanitized: {sanitized:?})"
        )));
    }
    if sanitized.is_empty() || matches!(sanitized.as_str(), "." | "..") {
        return Err(TFMTError::Template(format!(
            "Character replacement leaves invalid component {value:?}"
        )));
    }
    Ok(sanitized)
}

/// Apply the shared replacement rules; metadata inspection also uses this.
#[must_use]
pub fn sanitize_tag_value(value: &str) -> String {
    let mut value = value.to_owned();
    for rule in
        FORBIDDEN_CHARACTERS.iter().filter(|r| !r.leading() && !r.trailing())
    {
        value = value.replace(rule.char(), rule.replacement().unwrap_or(""));
    }
    let leading: Vec<_> = FORBIDDEN_CHARACTERS
        .iter()
        .filter(|r| r.leading())
        .flat_map(|r| {
            r.char().chars().map(move |c| (c, r.replacement().unwrap_or("")))
        })
        .collect();
    let trailing: Vec<_> = FORBIDDEN_CHARACTERS
        .iter()
        .filter(|r| r.trailing())
        .flat_map(|r| {
            r.char().chars().map(move |c| (c, r.replacement().unwrap_or("")))
        })
        .collect();
    let value = replace_edge_characters(&value, &leading, true);
    replace_edge_characters(&value, &trailing, false)
}

fn replace_edge_characters(
    value: &str,
    rules: &[(char, &str)],
    leading: bool,
) -> String {
    let matches = |c: char| rules.iter().any(|(character, _)| *character == c);
    let boundary = if leading {
        value.find(|c| !matches(c)).unwrap_or(value.len())
    } else {
        value
            .char_indices()
            .rfind(|(_, c)| !matches(*c))
            .map_or(0, |(i, c)| i + c.len_utf8())
    };
    let (prefix, suffix) = value.split_at(boundary);
    let (edge, rest) =
        if leading { (prefix, suffix) } else { (suffix, prefix) };
    let mut replaced = String::new();
    for character in edge.chars() {
        replaced
            .push_str(rules.iter().find(|(c, _)| *c == character).unwrap().1);
    }
    if leading { replaced + rest } else { rest.to_owned() + &replaced }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_rules_keep_replacements_and_leave_interior_characters_alone() {
        let rules = [(' ', ""), ('~', "—")];
        assert_eq!(
            replace_edge_characters(" ~~a~b~~ ", &rules, true),
            "——a~b~~ "
        );
        assert_eq!(
            replace_edge_characters(" ~~a~b~~ ", &rules, false),
            " ~~a~b——"
        );
        assert_eq!(replace_edge_characters("~~", &rules, true), "——");
        assert_eq!(replace_edge_characters("~~", &rules, false), "——");
        assert_eq!(replace_edge_characters("", &rules, true), "");
        assert_eq!(replace_edge_characters("é~", &rules, false), "é—");
        assert_eq!(sanitize_tag_value(" AC/DC: Live. . "), "AC-DC Live");
    }
}
