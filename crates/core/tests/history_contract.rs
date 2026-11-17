use serde_json::json;
use tfmttools_core::action::{Action, TagValueChange, TagValueKind};
use tfmttools_core::history::{
    ActionRecordMetadata, History, Record, StoredAction, TemplateMetadata,
};

#[test]
fn action_payloads_round_trip_through_executable_actions() {
    for value in [
        json!({"type":"move_file","source":"a","target":"b"}),
        json!({"type":"copy_file","source":"b","target":"c"}),
        json!({"type":"remove_file","path":"c"}),
        json!({"type":"make_dir","path":"dir"}),
        json!({"type":"remove_dir","path":"dir"}),
        json!({"type":"edit_tag_values","path":"b","changes":[
            {"key":"track_artist","kind":"text","old_value":"old","new_value":"new","old_encoding":"UTF8","new_encoding":"UTF16"},
            {"key":"audio_file_url","kind":"locator","old_value":"","new_value":"https://example.test","old_encoding":null,"new_encoding":null}
        ]}),
    ] {
        let stored: StoredAction =
            serde_json::from_value(value.clone()).unwrap();
        let executable = Action::try_from(&stored).unwrap();
        assert_eq!(
            serde_json::to_value(StoredAction::from(&executable)).unwrap(),
            value
        );
    }
}

#[test]
fn metadata_payloads_preserve_all_template_variants() {
    for kind in ["file_or_name", "inline_template", "validation"] {
        let value = json!({"template":{"type":kind,"value":"opaque\n$title"},"arguments":["--test"],"run_id":"test"});
        let metadata: ActionRecordMetadata =
            serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(metadata).unwrap(), value);
    }
}

#[test]
fn absent_encodings_remain_null_without_inference() {
    let stored: StoredAction = serde_json::from_value(json!({"type":"edit_tag_values","path":"a","changes":[{"key":"track_title","kind":"text","old_value":"","new_value":"title"}]})).unwrap();
    let Action::EditTagValues { changes, .. } =
        Action::try_from(&stored).unwrap()
    else {
        panic!("tag action")
    };
    assert_eq!(changes[0].old_encoding(), None);
    assert_eq!(changes[0].new_encoding(), None);
    let output = serde_json::to_value(stored).unwrap();
    assert!(output["changes"][0]["old_encoding"].is_null());
    assert!(output["changes"][0]["new_encoding"].is_null());
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
    for encoding in [
        "Latin1", "UTF16", "UTF16BE", "UTF8", "unknown", "latin1", "utf8",
        "UTF-8", "", " UTF8",
    ] {
        let mut value = base.clone();
        value["changes"][0]["new_encoding"] = json!(encoding);
        let stored: StoredAction = serde_json::from_value(value).unwrap();
        assert_eq!(
            Action::try_from(&stored).is_ok(),
            matches!(encoding, "Latin1" | "UTF16" | "UTF16BE" | "UTF8")
        );
    }
}

#[test]
fn decoder_rejects_unknown_fields_in_stored_payloads() {
    let action = json!({"type":"edit_tag_values","path":"a","changes":[{"key":"track_title","kind":"text","old_value":"","new_value":"title"}]});
    for pointer in ["", "/changes/0"] {
        let mut value = action.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!("data"));
        assert!(
            serde_json::from_value::<StoredAction>(value).is_err(),
            "{pointer}"
        );
    }
    let metadata = json!({"template":{"type":"validation","value":"characters"},"arguments":[],"run_id":"test"});
    for pointer in ["", "/template"] {
        let mut value = metadata.clone();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("unexpected".into(), json!("data"));
        assert!(
            serde_json::from_value::<ActionRecordMetadata>(value).is_err(),
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
        assert!(serde_json::from_value::<StoredAction>(action).is_err());
    }
    for template in [
        json!({"Script":"text"}),
        json!({"type":"Script","value":"text"}),
        json!({"value":"text"}),
    ] {
        assert!(serde_json::from_value::<TemplateMetadata>(template).is_err());
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
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: ActionRecordMetadata::new(
                TemplateMetadata::Validation { value: "characters".into() },
                vec![],
                "test".into(),
            ),
        })
        .unwrap();
    assert!(
        history
            .begin_attempt(
                id,
                0,
                &StoredAction::from(&action),
                &tfmttools_core::history::AttemptDetails {
                    paths: vec!["a".into()],
                    instructions: "Inspect".into()
                },
                None
            )
            .is_err()
    );
    history.close_run(id, false).unwrap();
    assert!(history.current_attempt().unwrap().is_none());
    assert!(history.records().iter().all(Record::is_empty));
}

#[test]
fn preview_has_explicit_progress() {
    let preview = Record::new(
        vec![StoredAction::MakeDir { path: "preview".into() }],
        ActionRecordMetadata::new(
            TemplateMetadata::Validation { value: "test".into() },
            vec![],
            "preview".into(),
        ),
    );
    assert_eq!(preview.id(), None);
    assert_eq!(preview.applied_count(), 1);
    assert!(preview.is_complete());
    assert!(preview.redo_allowed());
}
