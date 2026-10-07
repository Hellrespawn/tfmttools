use tfmttools_picotmpl::{ArgKind, ArgumentPolicy, Template};

fn compile(source: &str) -> Result<Template, tfmttools_picotmpl::Diagnostic> {
    Template::compile(source, ArgumentPolicy::new(&[]))
}

#[test]
fn definitions_and_forward_argument_references() {
    let template =
        compile("path: ({prefix} {$TITLE}) arg prefix: string(default: \"\")")
            .unwrap();
    assert_eq!(template.arguments().len(), 1);
    assert_eq!(template.arguments()[0].kind, ArgKind::String);
    assert_eq!(template.arguments()[0].default.as_deref(), Some(""));
    assert_eq!(template.tag_references()[0].name, "title");
}

#[test]
fn quoted_whitespace_and_comments() {
    let source = r#"
        # a comment
        name: "Quoted \"name\""
        description: "Windows \\ Unix /"
        path: (" - # [{}] " {$title}) # another comment
    "#;
    let template = compile(source).unwrap();
    assert_eq!(template.metadata().name.as_deref(), Some("Quoted \"name\""));
    assert_eq!(
        template.metadata().description.as_deref(),
        Some("Windows \\ Unix /")
    );
    assert_eq!(template.tag_references().len(), 1);
}

#[test]
fn nested_guards_and_negative_guards() {
    let template = compile(
        r#"path: ([$album? [$date? {$date | year}]] [!$album? "Singles" /] {$title})"#,
    )
    .unwrap();
    let names: Vec<_> = template
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
fn compiled_template_outlives_source() {
    let template = {
        let source = String::from("name: \"Owned\" path: ({$title})");
        compile(&source).unwrap()
    };
    assert_eq!(template.metadata().name.as_deref(), Some("Owned"));
    assert_eq!(template.tag_references()[0].name, "title");
}

#[test]
fn binding_diagnostic_uses_retained_source() {
    let template = {
        let source =
            String::from("name: \"Été\"\narg prefix: int\npath: ({prefix})");
        compile(&source).unwrap()
    };
    let error = template.bind(&["invalid".to_owned()]).unwrap_err();
    assert_eq!(
        template.format_diagnostic(&error),
        "2:1: Argument 'prefix' requires an integer",
    );
}

#[test]
fn bound_render_diagnostic_outlives_template_and_source() {
    let bound = {
        let source =
            String::from("name: \"Été\"\npath: (\"é\" {$date | year})");
        compile(&source).unwrap().bind(&[]).unwrap()
    };
    let error = bound
        .render(|_| {
            Ok::<_, std::convert::Infallible>(Some(
                tfmttools_picotmpl::Scalar::Text("unknown".to_owned()),
            ))
        })
        .unwrap_err();
    let error = match error {
        tfmttools_picotmpl::RenderError::Template(error) => error,
        tfmttools_picotmpl::RenderError::Resolver { source, .. } => {
            match source {}
        },
    };
    assert_eq!(
        bound.format_diagnostic(&error),
        "2:21: Unable to extract a year from \"unknown\"",
    );
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
