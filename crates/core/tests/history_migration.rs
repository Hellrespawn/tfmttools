mod support;

use serde_json::{Value, json};
use tfmttools_core::history::{RecordState, StoredAction};

fn legacy() -> Value {
    serde_json::from_str(include_str!("fixtures/history/v0-all-variants.json"))
        .unwrap()
}

#[test]
fn opaque_strings_and_order_survive_migration() {
    let input = legacy();
    let history = support::load(&input).unwrap();
    let records = history.records();
    assert_eq!(records.iter().map(|r| r.state()).collect::<Vec<_>>(), [
        RecordState::Applied,
        RecordState::Undone,
        RecordState::Redone,
        RecordState::Superseded
    ]);
    let output = serde_json::to_value(records).unwrap();
    assert_eq!(
        output[0]["actions"],
        json!([
            {"type":"move_file","source":"TrackArtist","target":"Applied"},
            {"type":"copy_file","source":"Applied","target":"Script"},
            {"type":"remove_file","path":"Script"},
            {"type":"make_dir","path":"dir"},
            {"type":"remove_dir","path":"dir"},
            {"type":"edit_tag_values","path":"Applied","changes":[
                {"key":"track_artist","kind":"text","old_value":"TrackArtist","new_value":"Script","old_encoding":"UTF8","new_encoding":"UTF16"},
                {"key":"audio_file_url","kind":"locator","old_value":"Applied","new_value":"Title","old_encoding":null,"new_encoding":null}
            ]}
        ])
    );
    assert_eq!(
        output[0]["metadata"],
        json!({"template":{"type":"inline_template","value":"TrackArtist Applied Script"},"arguments":["TrackArtist","Applied"],"run_id":"Script"})
    );
    assert_eq!(
        output[1]["metadata"]["template"],
        json!({"type":"file_or_name","value":"Title"})
    );
    assert_eq!(
        output[2]["metadata"]["template"],
        json!({"type":"validation","value":"characters"})
    );
    for (index, record) in records.iter().enumerate() {
        assert_eq!(
            record.id(),
            input["records"][index]["id"]
                .as_u64()
                .map(|id| usize::try_from(id).unwrap())
        );
        assert_eq!(
            record.timestamp().fixed_offset(),
            chrono::DateTime::parse_from_rfc3339(
                input["records"][index]["timestamp"].as_str().unwrap()
            )
            .unwrap()
        );
    }
}

#[test]
fn historical_mapping_covers_all_keys_aliases_and_case_forms() {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "fixtures/history/legacy-tag-spellings.json"
    ))
    .unwrap();
    assert_eq!(rows.len(), 105);
    for row in rows {
        let mut spellings = row["spellings"].as_array().unwrap().clone();
        spellings.push(row["historical"].clone());
        spellings.push(row["canonical"].clone());
        for spelling in spellings {
            for name in [
                spelling.as_str().unwrap().to_owned(),
                spelling.as_str().unwrap().to_ascii_uppercase(),
            ] {
                let mut value = legacy();
                value["records"][0]["actions"][5]["EditTagValues"]["changes"]
                    [0]["key"] = json!(name);
                let history = support::load(&value)
                    .unwrap_or_else(|error| panic!("{name}: {error}"));
                let StoredAction::EditTagValues { changes, .. } =
                    &history.records()[0].actions()[5]
                else {
                    panic!("tag edit")
                };
                assert_eq!(
                    changes[0].key,
                    row["canonical"].as_str().unwrap(),
                    "{name}"
                );
            }
        }
    }
}

#[test]
fn explicit_zero_migrates_mixed_legacy_and_canonical_keys() {
    let value = serde_json::from_str(include_str!(
        "fixtures/history/v0-mixed-tag-keys.json"
    ))
    .unwrap();
    let history = support::load(&value).unwrap();
    let StoredAction::EditTagValues { changes, .. } =
        &history.records()[0].actions()[0]
    else {
        panic!("tag edit")
    };
    assert_eq!(changes.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), [
        "album_title_sort_order",
        "disc_number",
        "track_title",
        "track_artist"
    ]);
}

#[test]
fn invalid_versions_never_fall_back_to_legacy() {
    for version in [
        json!(null),
        json!("1"),
        json!(1.0),
        json!(-1),
        json!(2),
        json!(u64::MAX),
        serde_json::from_str::<Value>("18446744073709551616").unwrap(),
    ] {
        let mut value = legacy();
        value["schema_version"] = version.clone();
        let error = support::load(&value).unwrap_err().to_string();
        assert!(error.contains("version"), "{version}: {error}");
    }
    let mut value = legacy();
    value["schema_version"] = json!(1);
    assert!(support::load(&value).is_err());
}

#[test]
fn malformed_legacy_shapes_and_fields_are_rejected() {
    for action in [
        json!({"Unknown":"a"}),
        json!({"MakeDir":"a","RemoveDir":"a"}),
        json!({"MoveFile":{"source":"a","target":"b","type":"copy_file"}}),
        json!({"MoveFile":{"source":"a","target":"b","extra":1}}),
        json!({"MakeDir":{"path":"a"}}),
    ] {
        let mut value = legacy();
        value["records"][0]["actions"][0] = action;
        assert!(support::load(&value).is_err());
    }
    for template in [
        json!({"Unknown":"text"}),
        json!({"Script":"text","Validation":"text"}),
        json!({"Script":{"value":"text"}}),
    ] {
        let mut value = legacy();
        value["records"][0]["metadata"]["template"] = template;
        assert!(support::load(&value).is_err());
    }
    for pointer in [
        "",
        "/records/0",
        "/records/0/metadata",
        "/records/0/actions/5/EditTagValues/changes/0",
    ] {
        let mut value = legacy();
        value
            .pointer_mut(pointer)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert("extra".into(), json!(1));
        assert!(support::load(&value).is_err(), "{pointer}");
    }
    for (pointer, invalid) in [
        ("/records/0/state", json!("unknown")),
        ("/records/0/actions/5/EditTagValues/changes/0/kind", json!("unknown")),
        ("/records/0/actions/5/EditTagValues/changes/0/key", json!("date")),
    ] {
        let mut value = legacy();
        *value.pointer_mut(pointer).unwrap() = invalid;
        assert!(support::load(&value).is_err());
    }
}

#[test]
fn bad_final_superseded_change_rejects_entire_document_with_location() {
    let mut value = legacy();
    value["records"][3]["actions"] = json!([{"EditTagValues":{"path":"a","changes":[{"key":"TrackArtist","kind":"Text","old_value":"a","new_value":"b"},{"key":"unknown","kind":"Text","old_value":"a","new_value":"b"}]}}]);
    let error = support::load(&value).unwrap_err().to_string();
    assert!(error.contains("records[3].actions[0].changes[1]"), "{error}");
}
