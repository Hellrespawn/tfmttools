use std::collections::BTreeMap;
use std::convert::Infallible;

use tfmttools_picotmpl::{ArgumentPolicy, Scalar, Template};

const SOURCE: &str = include_str!("fixtures/stef.tfmt");

fn tags() -> BTreeMap<String, Scalar> {
    [
        ("artist", "Example Artist"),
        ("album_artist", "Example Artist"),
        ("album", "Example Album"),
        ("date", "2024-03-10"),
        ("album_sort", "2"),
        ("disc_number", "1"),
        ("track_number", "3"),
        ("title", "Example Song"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_owned(), Scalar::Text(value.to_owned())))
    .collect()
}

fn render(
    source: &str,
    tags: &BTreeMap<String, Scalar>,
    args: &[String],
) -> Vec<String> {
    let forbidden: Vec<_> = "<>\":|?*~/\\".chars().collect();
    let template =
        Template::compile(source, ArgumentPolicy::new(&forbidden)).unwrap();
    template
        .bind(args)
        .unwrap()
        .render(|name| Ok::<_, Infallible>(tags.get(name).cloned()))
        .unwrap()
        .components()
        .to_vec()
}

#[test]
fn stef_complete_layout() {
    assert_eq!(render(SOURCE, &tags(), &["Music".to_owned()]), [
        "Music",
        "Example Artist",
        "2024.02 - Example Album",
        "103 - Example Artist - Example Song"
    ],);
}

#[test]
fn stef_missing_optional_tags() {
    for (missing, album_directory, filename) in [
        ("album", None, "103 - Example Artist - Example Song"),
        ("date", Some("Example Album"), "103 - Example Artist - Example Song"),
        (
            "album_sort",
            Some("2024 - Example Album"),
            "103 - Example Artist - Example Song",
        ),
        ("album_artist", Some("2024.02 - Example Album"), "103 - Example Song"),
        ("artist", Some("2024.02 - Example Album"), "103 - Example Song"),
        (
            "disc_number",
            Some("2024.02 - Example Album"),
            "03 - Example Artist - Example Song",
        ),
        (
            "track_number",
            Some("2024.02 - Example Album"),
            "1Example Artist - Example Song",
        ),
    ] {
        let mut tags = tags();
        tags.remove(missing);
        let mut expected = vec!["Example Artist".to_owned()];
        if let Some(directory) = album_directory {
            expected.push(directory.to_owned());
        }
        expected.push(filename.to_owned());
        assert_eq!(render(SOURCE, &tags, &[]), expected, "{missing}");
    }
}

#[test]
fn stef_empty_tags_behave_like_missing_tags() {
    let mut missing = tags();
    missing.remove("album");
    let mut empty = tags();
    empty.insert("album".to_owned(), Scalar::Text(String::new()));
    assert_eq!(render(SOURCE, &missing, &[]), render(SOURCE, &empty, &[]));
    assert_eq!(render(SOURCE, &empty, &[]), [
        "Example Artist",
        "103 - Example Artist - Example Song"
    ]);
}

#[test]
fn stef_zero_is_present() {
    let mut tags = tags();
    tags.insert("album_sort".to_owned(), Scalar::Integer(0));
    tags.insert("track_number".to_owned(), Scalar::Integer(0));
    assert_eq!(render(SOURCE, &tags, &[]), [
        "Example Artist",
        "2024.00 - Example Album",
        "100 - Example Artist - Example Song"
    ],);
}

#[test]
fn stef_directory_prefix_is_structural() {
    assert_eq!(render(SOURCE, &tags(), &["Music\\Artists".to_owned()]), [
        "Music",
        "Artists",
        "Example Artist",
        "2024.02 - Example Album",
        "103 - Example Artist - Example Song"
    ],);
}

#[test]
fn stef_outside_string_whitespace_does_not_change_output() {
    let expanded = SOURCE
        .replace("path: (", "path: (\n # path comment\n")
        .replace("[$album?", "[ $album ?\n # album comment\n");
    assert_eq!(render(&expanded, &tags(), &[]), [
        "Example Artist",
        "2024.02 - Example Album",
        "103 - Example Artist - Example Song"
    ],);
}
