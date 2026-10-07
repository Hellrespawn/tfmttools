use serde_json::{Value, json};
use tfmttools_core::history::history_schema_json;

#[test]
fn schema_matches_snapshot() {
    assert_eq!(
        history_schema_json().unwrap(),
        include_str!("../../../docs/history/schema-v1.json")
    );
}

#[test]
fn schema_describes_version_unions_and_decoder_optionals() {
    let schema: Value =
        serde_json::from_str(&history_schema_json().unwrap()).unwrap();
    assert_eq!(schema["properties"]["schema_version"]["const"], 1);
    assert_eq!(schema["additionalProperties"], false);
    assert!(
        schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("schema_version"))
    );
    assert!(schema["required"].as_array().unwrap().contains(&json!("records")));
    for name in ["Record", "ActionRecordMetadata", "StoredTagValueChange"] {
        assert_eq!(
            schema["$defs"][name]["additionalProperties"], false,
            "{name}"
        );
    }
    let record = &schema["$defs"]["Record"];
    assert!(!record["required"].as_array().unwrap().contains(&json!("id")));
    assert!(
        record["properties"]["id"]["type"]
            .as_array()
            .unwrap()
            .contains(&json!("null"))
    );
    assert_eq!(record["properties"]["timestamp"]["format"], "date-time");
    let changes = &schema["$defs"]["StoredTagValueChange"];
    for name in ["old_encoding", "new_encoding"] {
        assert!(
            !changes["required"].as_array().unwrap().contains(&json!(name))
        );
        assert_eq!(
            changes["properties"][name]["type"],
            json!(["string", "null"])
        );
    }
    for (name, types) in [
        ("StoredAction", vec![
            "move_file",
            "copy_file",
            "remove_file",
            "make_dir",
            "remove_dir",
            "edit_tag_values",
        ]),
        ("TemplateMetadata", vec![
            "file_or_name",
            "inline_template",
            "validation",
        ]),
    ] {
        let variants = schema["$defs"][name]["oneOf"].as_array().unwrap();
        assert_eq!(variants.len(), types.len());
        for (variant, expected) in variants.iter().zip(types) {
            assert_eq!(variant["properties"]["type"]["const"], expected);
            assert_eq!(variant["additionalProperties"], false);
            assert!(
                variant["required"]
                    .as_array()
                    .unwrap()
                    .contains(&json!("type"))
            );
        }
    }
}
