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
