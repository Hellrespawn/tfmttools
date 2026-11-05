use tfmttools_picotmpl::{ArgKind, Template};

fn compile(source: &str) -> Result<Template, tfmttools_picotmpl::Diagnostic> {
    Template::compile(source)
}

#[test]
fn definitions_and_forward_argument_references() {
    let template =
        compile("template: (prefix $TITLE) arg prefix: string(default: \"\")")
            .unwrap();
    assert_eq!(template.arguments().len(), 1);
    assert_eq!(template.arguments()[0].kind, ArgKind::String);
    assert_eq!(template.arguments()[0].default.as_deref(), Some(""));
    assert_eq!(template.tag_references()[0].name, "TITLE");
}

#[test]
fn quoted_whitespace_and_comments() {
    let source = r#"
        # a comment
        name: "Quoted \"name\""
        description: "Windows \\ Unix /"
        template: (" - # [{}] " $title) # another comment
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
        r#"template: ($album? ( $date? ( ($date | year))) !$album? ( "Singles" /) $title)"#,
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
        "template: (undeclared)",
        "arg prefix: path(required: false) template: ($title)",
        "template: (!undeclared? ( $title))",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn duplicate_definitions() {
    for source in [
        r#"name: "A" name: "B" template: ("x")"#,
        r#"description: "A" description: "B" template: ("x")"#,
        r#"template: ("A") template: ("B")"#,
        r#"arg prefix: string arg PREFIX: path template: ("x")"#,
        r#"arg prefix: string(default: "", default: "x") template: ("x")"#,
        r#"arg prefix: string(description: "", description: "x") template: ("x")"#,
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn bad_formatter_syntax() {
    for source in [
        "template: (($title | unknown))",
        "template: (($track | pad()))",
        "template: (($track | pad(-1)))",
        "template: (($date | year(2)))",
        "template: (($track | pad(2, 3)))",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn diagnostic_unicode_positions() {
    let source = "name: \"Été\"\ntemplate: (\"é\" @)";
    let error = compile(source).unwrap_err();
    assert_eq!(&source[error.span.start..error.span.end], "@");
    assert_eq!(error.line_column(source), (2, 16));
}

#[test]
fn compiled_template_outlives_source() {
    let template = {
        let source = String::from("name: \"Owned\" template: ($title)");
        compile(&source).unwrap()
    };
    assert_eq!(template.metadata().name.as_deref(), Some("Owned"));
    assert_eq!(template.tag_references()[0].name, "title");
}

#[test]
fn binding_diagnostic_uses_retained_source() {
    let template = {
        let source =
            String::from("name: \"Été\"\narg prefix: int\ntemplate: (prefix)");
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
            String::from("name: \"Été\"\ntemplate: (\"é\" ($date | year))");
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
        "2:25: Unable to extract a year from \"unknown\"",
    );
}

#[test]
fn malformed_documents_and_strings() {
    for source in [
        "",
        "name: \"A\"",
        "template: ($title",
        "template: ($title? ( \"A\")",
        "template: (\"unterminated)",
        r#"template: ("invalid\n")"#,
        "template: (bare)",
        "template: ($)",
        "arg 1prefix: string template: ($title)",
        "arg prefix: unknown template: ($title)",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
}

#[test]
fn multiple_fallbacks_and_chained_formatters() {
    assert!(
        compile(r#"template: ((($a ?: $b ?: "Unknown") | year | pad(6)))"#,)
            .is_ok()
    );
}

#[test]
fn padding_width_is_bounded() {
    assert!(compile("template: (($track | pad(1024)))").is_ok());
    assert!(compile("template: (($track | pad(1025)))").is_err());
}

#[test]
fn lexical_boundaries_and_full_document_consumption() {
    for source in [
        "tem plate: (\"x\")",
        "template: ($ title)",
        "template: (($track | pad(1 2)))",
        "template: (\"x\") trailing",
        "arg prefix: string(default: \"x\", DEFAULT: \"y\") template: (prefix)",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    let template = compile("NAME: \"Été\"\u{2003}arg PREFIX: string(default: \"x\",) template: (prefix $custom-tag)").unwrap();
    assert_eq!(template.metadata().name.as_deref(), Some("Été"));
    assert_eq!(template.arguments()[0].name, "prefix");
    assert_eq!(template.tag_references()[0].name, "custom-tag");
    let source = "name: \"é\"\ntemplate: (";
    let error = compile(source).unwrap_err();
    assert_eq!(error.span.start, source.len());
    assert_eq!(error.span.end, source.len());
    assert_eq!(error.line_column(source), (2, 12));
}

#[test]
fn booleans_are_rejected_only_in_invalid_contexts() {
    for source in [
        r#"template: ((!$album) ? "missing" : $title)"#,
        r#"template: ((!$a ?: !$b) ? "absent" : "present")"#,
    ] {
        compile(source).unwrap();
    }
    for source in [
        "template: (!$album)",
        "template: ($artist ? !$album)",
        r#"template: ("Album: " !$album)"#,
        r#"template: ((!$album ?: "Unknown"))"#,
        r#"template: ((!$album | pad(2)) ? "missing")"#,
        r#"template: ("always" ?: !$album)"#,
        r#"template: ("always" ? "ok" : !$album)"#,
    ] {
        let error = compile(source).unwrap_err();
        assert!(error.message.to_lowercase().contains("boolean"), "{error}");
        assert!(error.span.start < error.span.end);
        assert!(source.get(error.span.start..error.span.end).is_some());
    }
}

#[test]
fn conditional_tokens_and_association() {
    for source in [
        "template: ($a?$b?:$c:$d)",
        r#"template: ($a?$b?"both":"a":"none")"#,
        r#"template: ("{} [] ?? ?:")"#,
    ] {
        compile(source).unwrap();
    }
    for expression in [
        "$a ? : $b",
        "$a ?? $b",
        "$a ?",
        "$a ? $b :",
        ": $a",
        "!",
        "{$a}",
        "[$a? $b]",
    ] {
        let source = format!("template: ({expression})");
        let error = compile(&source).unwrap_err();
        assert!(source.get(error.span.start..error.span.end).is_some());
    }
}

#[test]
fn unicode_diagnostic_columns() {
    let source = "template: (\"é\" !)";
    let error = compile(source).unwrap_err();
    assert_eq!(&source[error.span.start..error.span.end], ")");
    assert_eq!(error.line_column(source), (1, 17));
}

#[test]
fn old_syntax_has_migration_hints() {
    for (source, hint) in [
        ("template: ({$title})", "parentheses"),
        ("template: ([$album? $title])", "condition ?"),
        ("template: ($artist ?? $title)", "?:"),
    ] {
        let error = compile(source).unwrap_err();
        assert!(error.message.contains(hint), "{error}");
    }
}

#[test]
fn template_definition_is_distinct_from_path_argument_type() {
    let template =
        compile("arg prefix: path template: (prefix $title)").unwrap();
    assert_eq!(template.arguments()[0].kind, ArgKind::Path);
    let rendered = template
        .bind(&["Music/Albums".to_owned()])
        .unwrap()
        .render(|_| {
            Ok::<_, std::convert::Infallible>(Some(
                tfmttools_picotmpl::Scalar::Text("Song".to_owned()),
            ))
        })
        .unwrap();
    assert_eq!(rendered.components(), ["Music", "Albums", "Song"]);
    assert!(compile("path: ($title)").is_err());
}
