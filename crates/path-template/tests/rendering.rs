use std::convert::Infallible;
use std::io;

use path_template::{ArgumentPolicy, BoundScript, RenderError, Scalar, Script};

fn bind(source: &str) -> BoundScript {
    Script::compile(source, ArgumentPolicy::new(&[])).unwrap().bind(&[]).unwrap()
}

fn text(value: &str) -> Option<Scalar> {
    Some(Scalar::Text(value.to_owned()))
}

#[test]
fn quoted_literals_and_adjacent_values() {
    let path = bind(r#"path: ("A" {$title} " B")"#)
        .render(|_| Ok::<_, Infallible>(text("T")))
        .unwrap();
    assert_eq!(path.components(), ["AT B"]);
}

#[test]
fn zero_is_present() {
    let path = bind("path: ([$track? {$track}])")
        .render(|_| Ok::<_, Infallible>(Some(Scalar::Integer(0))))
        .unwrap();
    assert_eq!(path.components(), ["0"]);
}

#[test]
fn missing_and_empty_are_absent() {
    for value in [None, text("")] {
        let path = bind(r#"path: ([$album? "Album"] [!$album? "Single"])"#)
            .render(|_| Ok::<_, Infallible>(value.clone()))
            .unwrap();
        assert_eq!(path.components(), ["Single"]);
    }
}

#[test]
fn fallback_is_lazy() {
    let mut calls = Vec::new();
    let path = bind(r#"path: ({$albumartist ?? $artist ?? "Unknown"})"#)
        .render(|name| {
            calls.push(name.to_owned());
            Ok::<_, Infallible>(text("Album Artist"))
        })
        .unwrap();
    assert_eq!(path.components(), ["Album Artist"]);
    assert_eq!(calls, ["albumartist"]);
    let path = bind(r#"path: ({$albumartist ?? $artist ?? "Unknown"})"#)
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap();
    assert_eq!(path.components(), ["Unknown"]);
}

#[test]
fn skipped_guard_does_not_lookup_contents() {
    let mut calls = Vec::new();
    let path = bind("path: ([$album? {$date | year} /] {$title})")
        .render(|name| {
            calls.push(name.to_owned());
            Ok::<_, Infallible>(if name == "title" { text("Song") } else { None })
        })
        .unwrap();
    assert_eq!(path.components(), ["Song"]);
    assert_eq!(calls, ["album", "title"]);
}

#[test]
fn resolver_error_has_reference_span() {
    let source = "path: ({$title})";
    let error = bind(source)
        .render(|_| Err::<Option<Scalar>, _>(io::Error::from(io::ErrorKind::PermissionDenied)))
        .unwrap_err();
    let RenderError::Resolver { name, span, source: error } = error else {
        panic!("Expected the original resolver error");
    };
    assert_eq!(name, "title");
    assert_eq!(&source[span.start..span.end], "$title");
    assert_eq!(error.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn all_references_are_visible_before_rendering() {
    let script = Script::compile(
        "path: ([$album? {$title}] {$title})",
        ArgumentPolicy::new(&[]),
    ).unwrap();
    let names: Vec<_> = script.tag_references().iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["album", "title", "title"]);
}

#[test]
fn whitespace_only_argument_is_preserved() {
    let script = Script::compile(
        r#"arg suffix: string path: ("A" {suffix} "B")"#,
        ArgumentPolicy::new(&[]),
    ).unwrap();
    let path = script.bind(&[" ".to_owned()]).unwrap()
        .render(|_| Ok::<_, Infallible>(None)).unwrap();
    assert_eq!(path.components(), ["A B"]);
}

#[test]
fn supplied_empty_value_overrides_default() {
    let script = Script::compile(
        r#"arg suffix: string(default: "fallback") path: ("A" {suffix} "B")"#,
        ArgumentPolicy::new(&[]),
    ).unwrap();
    let path = script.bind(&[String::new()]).unwrap()
        .render(|_| Ok::<_, Infallible>(None)).unwrap();
    assert_eq!(path.components(), ["AB"]);
}
