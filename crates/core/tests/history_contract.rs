use serde_json::{Value, json};
use tfmttools_core::action::{Action, TagValueChange, TagValueKind};
use tfmttools_core::history::{
    ActionRecordMetadata, History, OperationKind, Record, StoredAction,
    TemplateMetadata,
};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredHistory {
    schema_version: u64,
    records: Vec<Record>,
}

fn fixture() -> Value {
    serde_json::from_str(include_str!("fixtures/history/v1-all-variants.json"))
        .unwrap()
}

#[test]
fn current_output_preserves_all_record_data_and_variants() {
    let mut expected = fixture();
    let stored: StoredHistory =
        serde_json::from_value(expected.clone()).unwrap();
    let output = serde_json::to_value(&stored).unwrap();
    for (actual, expected_record) in output["records"]
        .as_array()
        .unwrap()
        .iter()
        .zip(expected["records"].as_array_mut().unwrap())
    {
        let actual_time = chrono::DateTime::parse_from_rfc3339(
            actual["timestamp"].as_str().unwrap(),
        )
        .unwrap();
        let expected_time = chrono::DateTime::parse_from_rfc3339(
            expected_record["timestamp"].as_str().unwrap(),
        )
        .unwrap();
        assert_eq!(actual_time, expected_time);
        expected_record["timestamp"] = actual["timestamp"].clone();
    }
    assert_eq!(output, expected);
    for record in &stored.records {
        for action in record.iter() {
            let executable = Action::try_from(action).unwrap();
            assert_eq!(
                serde_json::to_value(StoredAction::from(&executable)).unwrap(),
                serde_json::to_value(action).unwrap()
            );
        }
    }
    assert_eq!(stored.records[0].metadata().arguments(), &["opaque"]);
    assert_eq!(stored.records[3].id(), None);
}

#[test]
fn absent_optionals_remain_null_without_encoding_inference() {
    let stored: StoredHistory = serde_json::from_str(include_str!(
        "fixtures/history/v1-null-optionals.json"
    ))
    .unwrap();
    assert_eq!(stored.records[0].id(), None);
    let Action::EditTagValues { changes, .. } =
        Action::try_from(&stored.records[0].actions()[0]).unwrap()
    else {
        panic!("tag action")
    };
    assert_eq!(changes[0].old_encoding(), None);
    assert_eq!(changes[0].new_encoding(), None);
    let output = serde_json::to_value(&stored).unwrap();
    assert!(output["records"][0]["id"].is_null());
    assert!(
        output["records"][0]["actions"][0]["changes"][0]["old_encoding"]
            .is_null()
    );
}

#[test]
fn conversion_requires_canonical_writable_keys_and_known_encodings() {
    let base = json!({"type":"edit_tag_values","path":"a","changes":[{"key":"track_artist","kind":"text","old_value":"","new_value":""}]});
    for key in ["artist", "TrackArtist", "date", "not_a_tag"] {
        let mut value = base.clone();
        value["changes"][0]["key"] = json!(key);
        let stored: StoredAction = serde_json::from_value(value).unwrap();
        assert!(Action::try_from(&stored).is_err(), "{key}");
    }
    for encoding in ["Latin1", "UTF16", "UTF16BE", "UTF8", "unknown"] {
        let mut value = base.clone();
        value["changes"][0]["new_encoding"] = json!(encoding);
        let stored: StoredAction = serde_json::from_value(value).unwrap();
        assert_eq!(Action::try_from(&stored).is_ok(), encoding != "unknown");
    }
}

#[test]
fn decoder_rejects_unknown_fields_at_every_object_boundary() {
    for pointer in [
        "",
        "/records/0",
        "/records/0/metadata",
        "/records/0/metadata/template",
        "/records/0/actions/0",
        "/records/0/actions/5/changes/0",
    ] {
        let mut value = fixture();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!("data"));
        assert!(
            serde_json::from_value::<StoredHistory>(value).is_err(),
            "{pointer}"
        );
    }
}

#[test]
fn decoder_rejects_legacy_or_malformed_discriminators_and_payloads() {
    for action in [
        json!({"MoveFile":{"source":"a","target":"b"}}),
        json!({"type":"unknown"}),
        json!({"source":"a","target":"b"}),
        json!({"type":"move_file","source":1,"target":"b"}),
        json!({"type":"remove_file"}),
    ] {
        let mut value = fixture();
        value["records"][0]["actions"] = json!([action]);
        assert!(serde_json::from_value::<StoredHistory>(value).is_err());
    }
    for template in [
        json!({"Script":"text"}),
        json!({"type":"Script","value":"text"}),
        json!({"value":"text"}),
    ] {
        let mut value = fixture();
        value["records"][0]["metadata"]["template"] = template;
        assert!(serde_json::from_value::<StoredHistory>(value).is_err());
    }
}

#[test]
fn invalid_recording_does_not_mutate_history() {
    let dir = tempfile::tempdir().unwrap();
    let mut history = History::new(
        camino::Utf8PathBuf::try_from(dir.path().join("h.hist")).unwrap(),
    );
    let action = Action::EditTagValues {
        path: "a".into(),
        changes: vec![TagValueChange::new(
            "artist".into(),
            TagValueKind::Text,
            "a".into(),
            "b".into(),
        )],
    };
    let id = history
        .begin_operation(
            OperationKind::Apply,
            None,
            Some(ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "characters".into() },
                vec![],
                "test".into(),
            )),
        )
        .unwrap();
    assert!(
        history.set_operation_plan(id, &[StoredAction::from(&action)]).is_err()
    );
    history.cancel_unstarted(id).unwrap();
    assert!(history.pending_operations().unwrap().is_empty());
    assert!(history.is_empty());
}
