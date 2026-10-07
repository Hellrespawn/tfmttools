use std::convert::Infallible;
use std::io;
use std::path::PathBuf;

use tfmttools_picotmpl::{
    ArgumentPolicy, BoundTemplate, RenderError, Scalar, Template,
};

fn bind(source: &str) -> BoundTemplate {
    Template::compile(source, ArgumentPolicy::new(&[]))
        .unwrap()
        .bind(&[])
        .unwrap()
}

fn text(value: &str) -> Scalar {
    Scalar::Text(value.to_owned())
}

#[test]
fn quoted_literals_and_adjacent_values() {
    let path = bind(r#"path: ("A" {$title} " B")"#)
        .render(|_| Ok::<_, Infallible>(Some(text("T"))))
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
    for value in [None, Some(text(""))] {
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
            Ok::<_, Infallible>(Some(text("Album Artist")))
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
            Ok::<_, Infallible>(if name == "title" {
                Some(text("Song"))
            } else {
                None
            })
        })
        .unwrap();
    assert_eq!(path.components(), ["Song"]);
    assert_eq!(calls, ["album", "title"]);
}

#[test]
fn resolver_error_has_reference_span() {
    let source = "path: ({$title})";
    let error = bind(source)
        .render(|_| {
            Err::<Option<Scalar>, _>(io::Error::from(
                io::ErrorKind::PermissionDenied,
            ))
        })
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
    let template = Template::compile(
        "path: ([$album? {$title}] {$title})",
        ArgumentPolicy::new(&[]),
    )
    .unwrap();
    let names: Vec<_> =
        template.tag_references().iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["album", "title", "title"]);
}

#[test]
fn whitespace_only_argument_is_preserved() {
    let template = Template::compile(
        r#"arg suffix: string path: ("A" {suffix} "B")"#,
        ArgumentPolicy::new(&[]),
    )
    .unwrap();
    let path = template
        .bind(&[" ".to_owned()])
        .unwrap()
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap();
    assert_eq!(path.components(), ["A B"]);
}

#[test]
fn supplied_empty_value_overrides_default() {
    let template = Template::compile(
        r#"arg suffix: string(default: "fallback") path: ("A" {suffix} "B")"#,
        ArgumentPolicy::new(&[]),
    )
    .unwrap();
    let path = template
        .bind(&[String::new()])
        .unwrap()
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap();
    assert_eq!(path.components(), ["AB"]);
}

#[test]
fn year_extraction_and_padding() {
    for date in ["2024-03-10", "10-03-2024", "2024", "released in 2024"] {
        let path = bind("path: ({$date | year})")
            .render(|_| Ok::<_, Infallible>(Some(text(date))))
            .unwrap();
        assert_eq!(path.components(), ["2024"], "{date}");
    }
    let path = bind("path: ({$date | year | pad(6)})")
        .render(|_| Ok::<_, Infallible>(Some(text("2024"))))
        .unwrap();
    assert_eq!(path.components(), ["002024"]);
}

#[test]
fn date_pattern_precedence() {
    let path = bind("path: ({$date | year})")
        .render(|_| Ok::<_, Infallible>(Some(text("1999 then 2024-03-10"))))
        .unwrap();
    assert_eq!(path.components(), ["2024"]);
}

#[test]
fn bad_date_has_formatter_span() {
    let source = "path: ({$date | year})";
    let error = bind(source)
        .render(|_| Ok::<_, Infallible>(Some(text("unknown"))))
        .unwrap_err();
    let RenderError::Template(error) = error;
    assert_eq!(&source[error.span.start..error.span.end], "year");
    assert!(error.message.contains("year"));
}

#[test]
fn absent_formatter_input_emits_nothing() {
    let path = bind(r#"path: ({$date | year} "Song")"#)
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap();
    assert_eq!(path.components(), ["Song"]);
}

#[test]
fn padding_is_minimum_width_and_zero_is_preserved() {
    for (value, width, expected) in [
        (Scalar::Integer(3), 2, "03"),
        (Scalar::Text("123".to_owned()), 2, "123"),
        (Scalar::Integer(0), 2, "00"),
        (Scalar::Integer(3), 0, "3"),
        (Scalar::Integer(-2), 3, "0-2"),
        (Scalar::Text("é".to_owned()), 2, "0é"),
    ] {
        let path = bind(&format!("path: ({{$track | pad({width})}})"))
            .render(|_| Ok::<_, Infallible>(Some(value.clone())))
            .unwrap();
        assert_eq!(path.components(), [expected]);
    }
}

#[test]
fn formatter_applies_to_selected_fallback() {
    let path = bind("path: ({$a ?? $b | pad(2)})")
        .render(|name| {
            Ok::<_, Infallible>(if name == "b" {
                Some(text("3"))
            } else {
                None
            })
        })
        .unwrap();
    assert_eq!(path.components(), ["03"]);
}

#[test]
fn formatters_reject_path_arguments_in_all_alternatives() {
    for source in [
        "arg prefix: path(default: \"\") path: ({prefix | year})",
        "arg prefix: path(default: \"\") path: ({$a ?? prefix | pad(2)})",
    ] {
        assert!(Template::compile(source, ArgumentPolicy::new(&[])).is_err());
    }
}

#[test]
fn native_separators() {
    let path = bind(r#"path: ("Artist" / "Song")"#)
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap();
    assert_eq!(path.components(), ["Artist", "Song"]);
    assert_eq!(path.to_path_buf(), PathBuf::from("Artist").join("Song"));
    assert!(!path.is_rooted());
}

#[test]
fn prefix_components() {
    let template = Template::compile(
        r#"arg prefix: path path: ({prefix} "Artist" / "Song")"#,
        ArgumentPolicy::new(&[]),
    )
    .unwrap();
    for prefix in ["Music/Artists", "Music\\Artists", "/Music//Artists/"] {
        let path = template
            .bind(&[prefix.to_owned()])
            .unwrap()
            .render(|_| Ok::<_, Infallible>(None))
            .unwrap();
        assert_eq!(path.components(), ["Music", "Artists", "Artist", "Song"]);
        assert!(!path.is_rooted());
    }
}

#[test]
fn empty_prefix_has_no_boundary() {
    let path = bind(
        r#"arg prefix: path(default: "") path: ({prefix} "Artist" / "Song")"#,
    )
    .render(|_| Ok::<_, Infallible>(None))
    .unwrap();
    assert_eq!(path.components(), ["Artist", "Song"]);
}

#[test]
fn argument_after_component_text_is_error() {
    let template = Template::compile(
        r#"arg prefix: path path: ("A" {prefix} "B")"#,
        ArgumentPolicy::new(&[]),
    )
    .unwrap();
    let error = template
        .bind(&["Music".to_owned()])
        .unwrap()
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap_err();
    assert!(error.to_string().contains("boundary"));
}

#[test]
fn explicit_separator_after_path_argument_is_error() {
    let template = Template::compile(
        r#"arg prefix: path path: ({prefix} / "Song")"#,
        ArgumentPolicy::new(&[]),
    )
    .unwrap();
    let error = template
        .bind(&["Music".to_owned()])
        .unwrap()
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap_err();
    assert!(error.to_string().contains("empty component"));
}

#[test]
fn invalid_boundaries() {
    for source in [
        r#"path: ("A" / / "B")"#,
        r#"path: ("A" /)"#,
        "path: ()",
        "path: ({$missing})",
        r#"path: ("A" / {$missing})"#,
        r#"path: (/ / "A")"#,
    ] {
        assert!(
            bind(source).render(|_| Ok::<_, Infallible>(None)).is_err(),
            "{source}"
        );
    }
}

#[test]
fn root_separator() {
    let path = bind(r#"path: (/ "Artist" / "Song")"#)
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap();
    assert!(path.is_rooted());
    assert_eq!(path.components(), ["Artist", "Song"]);
    assert_eq!(
        path.to_path_buf(),
        PathBuf::from(std::path::MAIN_SEPARATOR_STR)
            .join("Artist")
            .join("Song"),
    );
}

#[test]
fn prepared_tag_cannot_inject_boundary() {
    for value in ["Artist/Album", "Artist\\Album"] {
        let error = bind("path: ({$artist})")
            .render(|_| Ok::<_, Infallible>(Some(text(value))))
            .unwrap_err();
        assert!(error.to_string().contains("separator"));
    }
}

#[test]
fn absent_initial_component_cannot_select_root() {
    for source in [
        r#"path: ({$artist} / "Song")"#,
        r#"path: ("" / "Song")"#,
        r#"arg prefix: path(default: "") path: ({prefix} / "Song")"#,
        r#"path: ([$artist? "Artist"] / "Song")"#,
        r#"path: ([$artist? / "Song"])"#,
    ] {
        let result = bind(source).render(|_| Ok::<_, Infallible>(None));
        assert!(result.is_err(), "{source}");
    }
    let result = bind(r#"path: ([$artist? / "Song"])"#)
        .render(|_| Ok::<_, Infallible>(Some(text("Artist"))));
    assert!(result.is_err());
}
