use std::convert::Infallible;

use tfmttools_picotmpl::{Scalar, Template};

fn render(expression: &str) -> Vec<String> {
    Template::compile(&format!("template: ({expression})"))
        .unwrap()
        .bind(&[])
        .unwrap()
        .render(|_| Ok::<_, Infallible>(None))
        .unwrap()
        .components()
        .to_vec()
}

#[test]
fn integer_expression_literals() {
    for (expression, expected) in [
        ("0", "0"),
        ("007", "7"),
        ("-1", "-1"),
        ("-9223372036854775808", "-9223372036854775808"),
        ("9223372036854775807", "9223372036854775807"),
        ("0 ?: 9", "0"),
        ("7 | pad(2)", "07"),
    ] {
        assert_eq!(render(expression), [expected], "{expression}");
    }
    assert_eq!(
        Template::compile("template: (7)").unwrap().dump_ast(),
        "Path\n  Integer 7"
    );
}

#[test]
fn integer_literal_diagnostics() {
    for expression in ["9223372036854775808", "-9223372036854775809"] {
        let source = format!("template: ({expression})");
        let error = Template::compile(&source).unwrap_err();
        assert_eq!(&source[error.span.start..error.span.end], expression);
        assert!(error.message.to_lowercase().contains("integer"), "{error}");
    }
    for expression in ["-", "- 1", "+1", "1.0", "1e2", "12abc"] {
        assert!(
            Template::compile(&format!("template: ({expression})")).is_err(),
            "{expression}"
        );
    }
    let path = Template::compile("template: (12 title) arg title: string")
        .unwrap()
        .bind(&["Song".to_owned()])
        .unwrap()
        .render(|_| Ok::<_, Infallible>(Some(Scalar::Integer(0))))
        .unwrap();
    assert_eq!(path.components(), ["12Song"]);
}

fn condition(expression: &str, expected: bool) {
    assert_eq!(
        render(&format!(r#"({expression}) ? "yes" : "no""#)),
        [if expected { "yes" } else { "no" }],
        "{expression}"
    );
}

#[test]
fn comparison_values_and_types() {
    for (expression, expected) in [
        ("1 == 1", true),
        ("1 == 2", false),
        ("1 != 1", false),
        ("1 != 2", true),
        ("1 < 2", true),
        ("1 < 1", false),
        ("1 <= 1", true),
        ("2 <= 1", false),
        ("2 > 1", true),
        ("1 > 1", false),
        ("1 >= 1", true),
        ("1 >= 2", false),
        (r#"1 == "1""#, false),
        (r#""a" == "a""#, true),
        (r#""A" == "a""#, false),
        (r#""a" != "b""#, true),
        ("(!$a) == (!$b)", true),
        ("(!$a) == (!!$a)", false),
        ("(!!$a) == $b", false),
        ("$a == $b", true),
        (r#"$a == """#, true),
        ("$a == 0", false),
        ("$a != 0", true),
        ("$a < 0", false),
        ("0 >= $a", false),
        (r#"(1 2) == "12""#, true),
        (r#"(1) == "1""#, false),
        ("() == $a", true),
        ("($a $b) == $a", true),
        (r#"($a ?: "Unknown") == "Unknown""#, true),
        (r#"$a ?: "Unknown" == "Unknown""#, true),
        ("(!$a ? 2 : 3) > 1", true),
        ("(1 == 1) == (!0)", false),
    ] {
        condition(expression, expected);
    }
}

#[test]
fn comparison_errors_and_validation() {
    for expression in [
        "1 < 2 < 3",
        "1 ==",
        "== 1",
        "1 === 1",
        "1 = 1",
        "1 == 1",
        r#""prefix" (1 == 1)"#,
        "(1 == 1) | pad(2)",
        r#"1 ? "ok" : 1 == 1"#,
        r#""ok" ?: (1 == 1)"#,
        r#""1" < 2 ? "x""#,
        r#"(!1) < 2 ? "x""#,
        r#"("x" /) == "x" ? "x""#,
        r#"(1 ? 1 : ("x" /)) == 1 ? "x""#,
        r#"1 ? "ok" : ("x" < 2 ? "bad")"#,
    ] {
        assert!(
            Template::compile(&format!("template: ({expression})")).is_err(),
            "{expression}"
        );
    }
    for expression in [r#"prefix == "" ? "x""#, r#"(1 ?: prefix) == 1 ? "x""#] {
        assert!(
            Template::compile(&format!(
                "arg prefix: path template: ({expression})"
            ))
            .is_err()
        );
    }
    let source = r#"template: ($number >= 1 ? "yes" : "no")"#;
    let bound = Template::compile(source).unwrap().bind(&[]).unwrap();
    let error = bound
        .render(|_| Ok::<_, Infallible>(Some(Scalar::Text("2".to_owned()))))
        .unwrap_err();
    let tfmttools_picotmpl::RenderError::Template(error) = error;
    assert_eq!(&source[error.span.start..error.span.end], ">=");
    assert!(error.message.contains("integer"), "{error}");
    // Absence makes ordering false, even when the other dynamic value is text.
    let path = Template::compile(r#"template: ($a < $b ? "yes" : "no")"#)
        .unwrap()
        .bind(&[])
        .unwrap()
        .render(|name| {
            Ok::<_, Infallible>(
                (name == "b").then(|| Scalar::Text("x".to_owned())),
            )
        })
        .unwrap();
    assert_eq!(path.components(), ["no"]);
}

#[test]
fn comparisons_resolve_operands_once_in_order() {
    let source = r#"template: ($a != $b ? "different" : "same")"#;
    let template = Template::compile(source).unwrap();
    assert_eq!(
        template
            .tag_references()
            .iter()
            .map(|r| r.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    let bound = template.bind(&[]).unwrap();
    let mut calls = Vec::new();
    let path = bound
        .render(|name| {
            calls.push(name.to_owned());
            Ok::<_, Infallible>(Some(Scalar::Integer(1)))
        })
        .unwrap();
    assert_eq!(calls, ["a", "b"]);
    assert_eq!(path.components(), ["same"]);
    let error = bound
        .render(|name| if name == "b" { Err("broken") } else { Ok(None) })
        .unwrap_err();
    let tfmttools_picotmpl::RenderError::Resolver { name, span, source: cause } =
        error
    else {
        panic!("expected resolver error")
    };
    assert_eq!(name, "b");
    assert_eq!(cause, "broken");
    assert_eq!(&source[span.start..span.end], "$b");
    assert_eq!(
        template.dump_ast(),
        "Path\n  Conditional\n    Condition\n      Compare !=\n        Reference $a\n        Reference $b\n    Then\n      Literal \"different\"\n    Else\n      Literal \"same\""
    );
}

#[test]
fn logical_presence_and_precedence() {
    for (expression, expected) in [
        ("$a && $b", false),
        ("$a || $b", false),
        ("0 && 1", true),
        ("$a && 0", false),
        ("0 && $a", false),
        ("0 || $a", true),
        ("$a || 0", true),
        (r#""" || "0""#, true),
        (r#""0" && 0"#, true),
        ("!$a && !$b", true),
        ("!!$a || !!$b", false),
        ("1 == 1 || 1 == 2 && 1 == 2", true),
        ("(1 == 1 || 1 == 2) && 1 == 2", false),
        ("$a ?: 0 && 1", true),
        ("0 | pad(2) == \"00\" && 1", true),
        (r#"("dir" /) && 1"#, true),
        ("(1 < 2) != (1 > 2) && 0", true),
    ] {
        condition(expression, expected);
    }
    assert_eq!(render(r#"$a || 0 ? $b && 1 ? "both" : "a" : "none""#), ["a"]);
    let template =
        Template::compile(r#"template: ($a || $b || $c ? "yes" : "no")"#)
            .unwrap();
    assert_eq!(
        template.dump_ast(),
        "Path\n  Conditional\n    Condition\n      Logical ||\n        Logical ||\n          Reference $a\n          Reference $b\n        Reference $c\n    Then\n      Literal \"yes\"\n    Else\n      Literal \"no\""
    );
}

#[test]
fn logical_short_circuit() {
    for (operator, left, expected_calls, expected) in [
        ("&&", None, vec!["a"], "no"),
        ("||", Some(Scalar::Integer(0)), vec!["a"], "yes"),
        ("&&", Some(Scalar::Integer(0)), vec!["a", "b"], "yes"),
        ("||", None, vec!["a", "b"], "yes"),
    ] {
        let template = Template::compile(&format!(
            r#"template: ($a {operator} $b ? "yes" : "no")"#
        ))
        .unwrap();
        assert_eq!(
            template
                .tag_references()
                .iter()
                .map(|r| r.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "b"]
        );
        let mut calls = Vec::new();
        let bound = template.bind(&[]).unwrap();
        let path = bound
            .render(|name| {
                calls.push(name.to_owned());
                Ok::<_, Infallible>(if name == "a" {
                    left.clone()
                } else {
                    Some(Scalar::Integer(1))
                })
            })
            .unwrap();
        assert_eq!(calls, expected_calls);
        assert_eq!(path.components(), [expected]);
        let result = bound.render(|name| {
            if name == "a" { Ok(left.clone()) } else { Err("broken") }
        });
        assert_eq!(result.is_err(), expected_calls.len() == 2);
    }
}

#[test]
fn logical_syntax_and_validation() {
    for expression in [
        r#"1 & 2 ? "x""#,
        r#"1 || ? "x""#,
        r#"&& 1 ? "x""#,
        r#"1 ||| 2 ? "x""#,
        "1 && 2",
        r#""x" (1 || 2)"#,
        "(1 && 2) | pad(2)",
        r#"1 ? "ok" : 1 && 2"#,
        r#"0 || undeclared ? "x""#,
        r#"$a && ("x" | unknown) ? "x""#,
        r#"1 < 2 < 3 && 1 ? "x""#,
    ] {
        assert!(
            Template::compile(&format!("template: ({expression})")).is_err(),
            "{expression}"
        );
    }
    condition(r#""0"|pad(2)=="00"||!0!=!1&&0"#, true);
    condition("!0!=!1||0&&1", true);
}

#[test]
fn text_formatter_results() {
    for (expression, expected) in [
        (r#"" Ab C " | trim | lower | replace(" ", "_")"#, "ab_c"),
        (r#""Straße" | upper"#, "STRASSE"),
        ("\"\u{2003}Ab\u{00a0}\" | trim", "Ab"),
        (r#""AB" | LoWeR"#, "ab"),
        (r#""aaaaa" | replace("aa", "b")"#, "bba"),
        (r#""aAa" | replace("a", "")"#, "A"),
        (r#"12 | lower | upper | trim | replace("1", "x")"#, "x2"),
        (
            r#"($missing | trim | upper | replace("a", "b")) ?: "fallback""#,
            "fallback",
        ),
        (r#"("" | pad(2) | lower) ?: "fallback""#, "fallback"),
        (r#"("  " | trim) ?: "fallback""#, "fallback"),
        (r#""  " | trim | pad(2)"#, "00"),
        (r#""abc" | replace("abc", "") | pad(2)"#, "00"),
        (r#""a-b" | replace("-", "/")"#, "a/b"),
        (r#""a\\b" | replace("\\", "\"")"#, "a\"b"),
        (r#""2024-01-02" | lower | year | pad(6)"#, "002024"),
    ] {
        assert_eq!(render(expression), [expected], "{expression}");
    }
    condition(r#""  " | trim"#, false);
    let template = Template::compile(
        r#"template: ($title | trim | lower | upper | replace("a", "b"))"#,
    )
    .unwrap();
    assert_eq!(
        template.dump_ast(),
        "Path\n  Format trim | lower | upper | replace(\"a\", \"b\")\n    Reference $title"
    );
}

#[test]
fn text_formatter_parameter_errors() {
    for expression in [
        r#""x" | replace("", "x")"#,
        r#""x" | replace("x")"#,
        r#""x" | replace("x", "y", "z")"#,
        r#""x" | replace(1, "y")"#,
        r#""x" | replace("x", 1)"#,
        r#""x" | replace($a, "y")"#,
        r#""x" | replace("x" "y")"#,
        r#""x" | replace("x", "y""#,
        r#""x" | replace("x", "y",)"#,
        r#""x" | replace("x", "\n")"#,
        r#""x" | lower()"#,
        r#""x" | upper(1)"#,
        r#""x" | trim("x")"#,
        r#""x" | unknown"#,
        r#""x" | pad("2")"#,
        r#""x" | pad(-1)"#,
        r#""x" | pad(1025)"#,
        r#""x" | year()"#,
        r#""x" | year(2)"#,
        "(!$a) | lower",
        r#"("x" /) | trim"#,
        r#"(1 ? "x" : ("y" /)) | upper"#,
        r#""x" | replace("x", "y") | pad()"#,
    ] {
        assert!(
            Template::compile(&format!("template: ({expression})")).is_err(),
            "{expression}"
        );
    }
    for parameter in ["", "\n", "\u{202e}"] {
        let source =
            format!(r#"template: ("x" | replace("{parameter}", "y"))"#);
        let error = Template::compile(&source).unwrap_err();
        assert_eq!(
            &source[error.span.start..error.span.end],
            format!("\"{parameter}\"")
        );
    }
    let source = "template: (\"x\" | replace(\"x\", \"\u{2066}\"))";
    let error = Template::compile(source).unwrap_err();
    assert_eq!(&source[error.span.start..error.span.end], "\"\u{2066}\"");
    assert!(
        Template::compile(
            r#"arg p: path template: (("x" ?: p) | replace("x", "y"))"#
        )
        .is_err()
    );
}

#[test]
fn year_formatter_preserves_adjacent_groups() {
    for expression in [
        r#""2024" | year (" suffix")"#,
        r#""2024" | YEAR(" suffix")"#,
        r#""2024" | year ((" suffix"))"#,
    ] {
        assert_eq!(render(expression), ["2024 suffix"], "{expression}");
    }
    assert_eq!(render(r#""2024" | year (1 2)"#), ["202412"]);
    assert_eq!(render(r#""2024" | year (0 ?: 1)"#), ["20240"]);
}
