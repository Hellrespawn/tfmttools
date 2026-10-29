use tfmttools_picotmpl::Template;

fn compile(source: &str) -> Result<Template, tfmttools_picotmpl::Diagnostic> {
    Template::compile(source)
}

#[test]
fn no_default_requires_argument() {
    let template = compile("arg edition: int path: ({$title})").unwrap();
    let error = template.bind(&[]).unwrap_err();
    assert!(error.message.contains("edition"));
}

#[test]
fn empty_default_makes_prefix_optional() {
    let template =
        compile("arg prefix: path(default: \"\") path: ({$title})").unwrap();
    assert!(template.bind(&[]).is_ok());
}

#[test]
fn defaults_do_not_replace_supplied_empty_values() {
    let template =
        compile("arg suffix: string(default: \"fallback\") path: ({$title})")
            .unwrap();
    assert!(template.bind(&[String::new()]).is_ok());
}

#[test]
fn integer_validation() {
    let template = compile("arg edition: int path: ({$title})").unwrap();
    for value in ["0", "-2", "9223372036854775807", "-9223372036854775808"] {
        assert!(template.bind(&[value.to_owned()]).is_ok(), "{value}");
    }
    for value in ["", "no", "9223372036854775808", "-9223372036854775809", " 2"]
    {
        assert!(template.bind(&[value.to_owned()]).is_err(), "{value}");
    }
}

#[test]
fn declaration_order_and_excess_arguments() {
    let template =
        compile("arg count: int arg label: string path: ({$title})").unwrap();
    assert!(template.bind(&["2".to_owned(), "label".to_owned()]).is_ok());
    assert!(template.bind(&["label".to_owned(), "2".to_owned()]).is_err());
    assert!(
        template
            .bind(&["2".to_owned(), "label".to_owned(), "extra".to_owned()])
            .is_err()
    );
    assert!(
        compile("path: ({$title})")
            .unwrap()
            .bind(&["extra".to_owned()])
            .is_err()
    );
}

#[test]
fn filename_characters_are_preserved_until_after_interpolation() {
    for (kind, value) in [("string", "A/B\\C?"), ("path", "A/B?/")] {
        let template =
            compile(&format!("arg value: {kind} path: ({{value}} \"Song\")"))
                .unwrap();
        assert!(template.bind(&[value.to_owned()]).is_ok());
    }
}

#[test]
fn invalid_defaults_even_when_overridden() {
    for source in [
        "arg count: int(default: \"\") path: ({$title})",
        "arg suffix: string(default: \"bad\n\") path: ({$title})",
    ] {
        assert!(compile(source).is_err(), "{source}");
    }
    assert!(
        compile("arg prefix: path(default: \"A/B\") path: ({$title})").is_ok()
    );
}

#[test]
fn invalid_unused_argument() {
    let template = compile("arg unused: string path: ({$title})").unwrap();
    let error = template.bind(&["bad\n".to_owned()]).unwrap_err();
    assert!(error.message.contains("unused"));
}
