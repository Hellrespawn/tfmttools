use tfmttools_picotmpl::Template;

#[test]
fn compact_tree_preserves_structure_and_values() {
    let template = Template::compile(
        r#"
        name: "Not part of the path AST"
        arg prefix: string
        path: (
          / {prefix} "quoted \"text\"" /
          [$album? {$album} / [!$disc? "Single disc"]]
          {$date ?? $year ?? "Unknown" | year | pad(4)}
        )
        "#,
    )
    .unwrap();
    assert_eq!(
        template.dump_ast(),
        concat!(
            "Path\n",
            "  Root\n",
            "  Interpolation prefix\n",
            "  Literal \"quoted \\\"text\\\"\"\n",
            "  Separator\n",
            "  Guard $album\n",
            "    Interpolation $album\n",
            "    Separator\n",
            "    Guard !$disc\n",
            "      Literal \"Single disc\"\n",
            "  Interpolation $date ?? $year ?? \"Unknown\" | year | pad(4)",
        )
    );
}

#[test]
fn literals_preserve_spaces_and_escape_backslashes() {
    let template = Template::compile(r#"path: ("  a \\ b  ")"#).unwrap();
    assert_eq!(
        template.dump_ast(),
        r#"Path
  Literal "  a \\ b  ""#
    );
}
