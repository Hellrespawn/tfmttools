use path_template::{ArgKind, ArgumentPolicy, Script};

fn compile(source: &str) -> Result<Script, path_template::Diagnostic> {
    Script::compile(source, ArgumentPolicy::new(&[]))
}

#[test]
fn definitions_and_forward_argument_references() {
    let script =
        compile("path: ({prefix} {$TITLE}) arg prefix: string(default: \"\")")
            .unwrap();
    assert_eq!(script.arguments().len(), 1);
    assert_eq!(script.arguments()[0].kind, ArgKind::String);
    assert_eq!(script.arguments()[0].default.as_deref(), Some(""));
    assert_eq!(script.tag_references()[0].name, "title");
}

#[test]
fn quoted_whitespace_and_comments() {
    let source = r#"
        # a comment
        name: "Quoted \"name\""
        description: "Windows \\ Unix /"
        path: (" - # [{}] " {$title}) # another comment
    "#;
    let script = compile(source).unwrap();
    assert_eq!(script.metadata().name.as_deref(), Some("Quoted \"name\""));
    assert_eq!(
        script.metadata().description.as_deref(),
        Some("Windows \\ Unix /")
    );
    assert_eq!(script.tag_references().len(), 1);
}

#[test]
fn nested_guards_and_negative_guards() {
    let script = compile(
        r#"path: ([$album? [$date? {$date | year}]] [!$album? "Singles" /] {$title})"#,
    )
    .unwrap();
    let names: Vec<_> = script
        .tag_references()
        .iter()
        .map(|reference| reference.name.as_str())
        .collect();
    assert_eq!(names, ["album", "date", "date", "album", "title"]);
}

#[test]
fn unknown_argument_and_option() {
    for source in [
        "path: ({undeclared})",
        "arg prefix: path(required: false) path: ({$title})",
        "path: ([!undeclared? {$title}])",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn duplicate_definitions() {
    for source in [
        r#"name: "A" name: "B" path: ("x")"#,
        r#"description: "A" description: "B" path: ("x")"#,
        r#"path: ("A") path: ("B")"#,
        r#"arg prefix: string arg PREFIX: path path: ("x")"#,
        r#"arg prefix: string(default: "", default: "x") path: ("x")"#,
        r#"arg prefix: string(description: "", description: "x") path: ("x")"#,
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn bad_formatter_syntax() {
    for source in [
        "path: ({$title | unknown})",
        "path: ({$track | pad()})",
        "path: ({$track | pad(-1)})",
        "path: ({$date | year(2)})",
        "path: ({$track | pad(2, 3)})",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn diagnostic_unicode_positions() {
    let source = "name: \"Été\"\npath: (\"é\" @)";
    let error = compile(source).unwrap_err();
    assert_eq!(&source[error.span.start..error.span.end], "@");
    assert_eq!(error.line_column(source), (2, 12));
}

#[test]
fn compiled_script_outlives_source() {
    let script = {
        let source = String::from("name: \"Owned\" path: ({$title})");
        compile(&source).unwrap()
    };
    assert_eq!(script.metadata().name.as_deref(), Some("Owned"));
    assert_eq!(script.tag_references()[0].name, "title");
}

#[test]
fn malformed_documents_and_strings() {
    for source in [
        "",
        "name: \"A\"",
        "path: ({$title}",
        "path: ([$title? \"A\")",
        "path: (\"unterminated)",
        r#"path: ("invalid\n")"#,
        r#"path: ("A/B")"#,
        r#"path: ("A\\B")"#,
        "path: (bare)",
        "path: ({$})",
        "arg 1prefix: string path: ({$title})",
        "arg prefix: unknown path: ({$title})",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn multiple_fallbacks_and_chained_formatters() {
    assert!(
        compile(r#"path: ({$a ?? $b ?? "Unknown" | year | pad(6)})"#,).is_ok()
    );
}

#[test]
fn padding_width_is_bounded() {
    assert!(compile("path: ({$track | pad(1024)})").is_ok());
    assert!(compile("path: ({$track | pad(1025)})").is_err());
}
