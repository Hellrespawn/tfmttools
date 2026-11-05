use tfmttools_picotmpl::Template;

#[test]
fn compact_tree_preserves_structure_and_values() {
    let template = Template::compile(
        r#"
        name: "Not part of the path AST"
        arg prefix: string
        path: (
          / prefix "quoted \"text\"" /
          $album? ( $album / !$disc? ( "Single disc"))
          (($date ?: $year ?: "Unknown") | year | pad(4))
        )
        "#,
    )
    .unwrap();
    assert_eq!(
        template.dump_ast(),
        r#"Path
  Root
  Reference prefix
  Literal "quoted \"text\""
  Separator
  Conditional
    Condition
      Reference $album
    Then
      Sequence
        Reference $album
        Separator
        Conditional
          Condition
            Negate
              Reference $disc
          Then
            Sequence
              Literal "Single disc"
  Sequence
    Format year | pad(4)
      Sequence
        Fallback
          Reference $date
          Reference $year
          Literal "Unknown""#,
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
