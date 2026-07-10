use std::sync::LazyLock;

use regex::Regex;

use super::Frontmatter;
use crate::error::{TFMTError, TFMTResult};
use crate::warning::Warning;

const FRONTMATTER_FENCE: &str = "+++";

/// Splits a raw template source into its Jinja body and (optional)
/// frontmatter block, and computes any deprecation warnings implied by
/// the pre-split source. Pure: operates only on the given string.
pub fn parse_template_source(
    label: &str,
    source: String,
) -> TFMTResult<(String, Option<Frontmatter>, Vec<Warning>)> {
    let (body, frontmatter) = split_frontmatter(label, source)?;

    let warnings = if frontmatter.is_none() {
        deprecation_warnings(label, &body)
    } else {
        Vec::new()
    };

    Ok((body, frontmatter, warnings))
}

fn deprecation_warnings(label: &str, body: &str) -> Vec<Warning> {
    let mut warnings = Vec::new();

    if body_uses_indexed_args(body) {
        warnings.push(Warning::DeprecatedPositionalArgs {
            template: label.to_owned(),
        });
    }

    if description(body).is_some() {
        warnings.push(Warning::DeprecatedLeadingComment {
            template: label.to_owned(),
        });
    }

    warnings
}

fn split_frontmatter(
    label: &str,
    source: String,
) -> TFMTResult<(String, Option<Frontmatter>)> {
    // The regex crate doesn't support look-around, so the opening and
    // closing fences are matched with two separate anchored patterns
    // instead of one monolithic `open ... \r?\n ... close` capture. The
    // closing fence is found by searching for a line consisting solely
    // of `+++` (optionally followed by trailing spaces/tabs) starting
    // right after the opening fence. This lets the closing fence
    // immediately follow the opening fence's own newline when the
    // frontmatter block has no content (e.g. "+++\n+++\n"), since
    // `find_at` treats the position right after that newline as a valid
    // line start rather than requiring a second, independent `\r?\n`
    // between the two fences.
    static RE_OPENING_FENCE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\A\+\+\+[ \t]*\r?\n").unwrap());

    static RE_CLOSING_FENCE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?m)^\+\+\+[ \t]*\r?$").unwrap());

    if !source.starts_with(FRONTMATTER_FENCE) {
        return Ok((source, None));
    }

    let Some(opening) = RE_OPENING_FENCE.find(&source) else {
        return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
    };

    let Some(closing) = RE_CLOSING_FENCE.find_at(&source, opening.end()) else {
        return Err(TFMTError::UnterminatedFrontmatter(label.to_owned()));
    };

    let toml_text = &source[opening.end()..closing.start()];

    let frontmatter = Frontmatter::parse(toml_text, label)?;

    let mut body_start = closing.end();

    if let Some(rest) = source[body_start..].strip_prefix("\r\n") {
        body_start = source.len() - rest.len();
    } else if let Some(rest) = source[body_start..].strip_prefix('\n') {
        body_start = source.len() - rest.len();
    }

    let body = source[body_start..].to_owned();

    if body_uses_indexed_args(&body) {
        return Err(TFMTError::IndexedArgsWithFrontmatter(label.to_owned()));
    }

    Ok((body, Some(frontmatter)))
}

fn body_uses_indexed_args(body: &str) -> bool {
    static RE_ARGS_INDEX: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"\bargs\s*\[").unwrap());

    RE_ARGS_INDEX.is_match(body)
}

fn description(source: &str) -> Option<String> {
    const COMMENT_START: &str = "{#";
    const COMMENT_END: &str = "#}";

    if source.trim().starts_with(COMMENT_START) {
        source.split_once(COMMENT_END).map(|(left, _)| {
            left.replace(COMMENT_START, "")
                .replace(COMMENT_END, "")
                .trim()
                .to_owned()
        })
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_frontmatter_returns_none_when_absent() {
        let source = "{{ artist }}/{{ title }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn split_frontmatter_parses_present_block() {
        let source = "+++\nname = \"Test\"\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) = split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert_eq!(frontmatter.unwrap().name(), Some("Test"));
    }

    #[test]
    fn split_frontmatter_handles_empty_toml_block() {
        let source = "+++\n+++\n{{ artist }}".to_owned();

        let (body, frontmatter) = split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert!(frontmatter.is_some());
        assert_eq!(frontmatter.unwrap().name(), None);
    }

    #[test]
    fn split_frontmatter_handles_empty_toml_block_crlf() {
        let source = "+++\r\n+++\r\n{{ artist }}".to_owned();

        let (body, frontmatter) = split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ artist }}");
        assert!(frontmatter.is_some());
        assert_eq!(frontmatter.unwrap().name(), None);
    }

    #[test]
    fn split_frontmatter_errors_when_unterminated() {
        let source = "+++\nname = \"Test\"\n{{ artist }}".to_owned();

        let error = split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::UnterminatedFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_errors_when_body_uses_indexed_args() {
        let source = "+++\nname = \"Test\"\n+++\n{{ args[0] }}".to_owned();

        let error = split_frontmatter("test", source).unwrap_err();

        assert!(matches!(error, TFMTError::IndexedArgsWithFrontmatter(_)));
    }

    #[test]
    fn split_frontmatter_allows_kwargs_identifier_with_frontmatter() {
        let source = "+++\nname = \"Test\"\n+++\n{{ kwargs[0] }}".to_owned();

        let (body, frontmatter) = split_frontmatter("test", source).unwrap();

        assert_eq!(body, "{{ kwargs[0] }}");
        assert!(frontmatter.is_some());
    }

    #[test]
    fn split_frontmatter_allows_indexed_args_without_frontmatter() {
        let source = "{{ args[0] }}".to_owned();

        let (body, frontmatter) =
            split_frontmatter("test", source.clone()).unwrap();

        assert_eq!(body, source);
        assert!(frontmatter.is_none());
    }

    #[test]
    fn parse_template_source_without_frontmatter_using_indexed_args_returns_warning()
     {
        let (_, _, warnings) =
            parse_template_source("script", "{{ args[0] }}".to_owned())
                .unwrap();

        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            Warning::DeprecatedPositionalArgs { ref template }
            if template == "script"
        ));
    }

    #[test]
    fn parse_template_source_without_frontmatter_with_leading_comment_returns_warning()
     {
        let (_, _, warnings) = parse_template_source(
            "script",
            "{# A description #}\n{{ artist }}".to_owned(),
        )
        .unwrap();

        assert_eq!(warnings.len(), 1);
        assert!(matches!(
            warnings[0],
            Warning::DeprecatedLeadingComment { ref template }
            if template == "script"
        ));
    }

    #[test]
    fn parse_template_source_with_frontmatter_returns_no_warnings() {
        let (_, _, warnings) = parse_template_source(
            "script",
            "+++\nname = \"Test\"\n+++\n{{ artist }}".to_owned(),
        )
        .unwrap();

        assert!(warnings.is_empty());
    }
}
