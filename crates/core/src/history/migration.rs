use serde_json::{Map, Value};

use super::legacy_tag_keys::canonicalize_legacy_tag_key;
use super::{CURRENT_SCHEMA_VERSION, HistoryError, Result, StoredHistory};

pub(super) fn upgrade_document(
    mut value: Value,
) -> Result<(StoredHistory, bool)> {
    let document = object(&mut value, "document")?;
    let version = match document.get("schema_version") {
        None => 0,
        Some(Value::Number(number)) if number.is_u64() => {
            number.as_u64().expect("unsigned integer checked")
        },
        Some(version) => {
            return Err(HistoryError::LoadError(format!(
                "Invalid history schema version {version}; expected an unsigned integer"
            )));
        },
    };
    let migrated = match version {
        0 => {
            value = migrate_v0_to_v1(value)?;
            true
        },
        CURRENT_SCHEMA_VERSION => false,
        _ => {
            return Err(HistoryError::LoadError(format!(
                "Unsupported history schema version {version}"
            )));
        },
    };
    let stored: StoredHistory =
        serde_json::from_value(value).map_err(|error| {
            HistoryError::LoadError(format!(
                "Invalid history document: {error}"
            ))
        })?;
    stored.validate()?;
    Ok((stored, migrated))
}

fn migrate_v0_to_v1(mut value: Value) -> Result<Value> {
    let document = object(&mut value, "document")?;
    let records = document
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| invalid("records", "expected an array"))?;
    for (index, record) in records.iter_mut().enumerate() {
        migrate_record(record, &format!("records[{index}]"))?;
    }
    document.insert("schema_version".into(), CURRENT_SCHEMA_VERSION.into());
    Ok(value)
}

fn migrate_record(value: &mut Value, location: &str) -> Result<()> {
    let record = object(value, location)?;
    rename_string(record.get_mut("state"), &format!("{location}.state"), &[
        ("Applied", "applied"),
        ("Undone", "undone"),
        ("Redone", "redone"),
        ("Superseded", "superseded"),
    ])?;
    let actions =
        record.get_mut("actions").and_then(Value::as_array_mut).ok_or_else(
            || invalid(&format!("{location}.actions"), "expected an array"),
        )?;
    for (index, action) in actions.iter_mut().enumerate() {
        migrate_action(action, &format!("{location}.actions[{index}]"))?;
    }
    let metadata_location = format!("{location}.metadata");
    let metadata = record
        .get_mut("metadata")
        .ok_or_else(|| invalid(&metadata_location, "missing metadata"))?;
    let metadata = object(metadata, &metadata_location)?;
    let template_location = format!("{metadata_location}.template");
    let template = metadata
        .get_mut("template")
        .ok_or_else(|| invalid(&template_location, "missing template"))?;
    let (variant, payload) = legacy_variant(template, &template_location)?;
    let name = match variant.as_str() {
        "FileOrName" => "file_or_name",
        "Script" => "inline_template",
        "Validation" => "validation",
        _ => {
            return Err(invalid(
                &template_location,
                "unknown legacy template variant",
            ));
        },
    };
    if !payload.is_string() {
        return Err(invalid(&template_location, "expected a string payload"));
    }
    *template = serde_json::json!({"type":name,"value":payload});
    Ok(())
}

fn migrate_action(value: &mut Value, location: &str) -> Result<()> {
    let (variant, mut payload) = legacy_variant(value, location)?;
    let (name, mut fields) = match variant.as_str() {
        "MoveFile" | "CopyFile" => {
            let fields = object(&mut payload, location)?;
            require_string(fields, "source", location)?;
            require_string(fields, "target", location)?;
            (
                if variant == "MoveFile" { "move_file" } else { "copy_file" },
                fields.clone(),
            )
        },
        "RemoveFile" | "MakeDir" | "RemoveDir" => {
            if !payload.is_string() {
                return Err(invalid(location, "expected a path string"));
            }
            let name = match variant.as_str() {
                "RemoveFile" => "remove_file",
                "MakeDir" => "make_dir",
                "RemoveDir" => "remove_dir",
                _ => unreachable!(),
            };
            (name, Map::from_iter([("path".into(), payload)]))
        },
        "EditTagValues" => {
            let fields = object(&mut payload, location)?;
            require_string(fields, "path", location)?;
            let changes = fields
                .get_mut("changes")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| invalid(location, "expected changes array"))?;
            for (index, change) in changes.iter_mut().enumerate() {
                let change_location = format!("{location}.changes[{index}]");
                let change = object(change, &change_location)?;
                rename_string(
                    change.get_mut("kind"),
                    &format!("{change_location}.kind"),
                    &[("Text", "text"), ("Locator", "locator")],
                )?;
                let key = require_string(change, "key", &change_location)?;
                let canonical =
                    canonicalize_legacy_tag_key(key).ok_or_else(|| {
                        invalid(
                            &change_location,
                            &format!("unknown legacy tag key '{key}'"),
                        )
                    })?;
                change.insert("key".into(), canonical.into());
            }
            ("edit_tag_values", fields.clone())
        },
        _ => return Err(invalid(location, "unknown legacy action variant")),
    };
    if fields.contains_key("type") {
        return Err(invalid(
            location,
            "legacy payload contains conflicting type field",
        ));
    }
    fields.insert("type".into(), name.into());
    *value = Value::Object(fields);
    Ok(())
}

fn legacy_variant(value: &Value, location: &str) -> Result<(String, Value)> {
    let fields =
        value.as_object().filter(|fields| fields.len() == 1).ok_or_else(
            || invalid(location, "expected exactly one legacy variant key"),
        )?;
    let (name, payload) = fields.iter().next().expect("one field checked");
    Ok((name.clone(), payload.clone()))
}

fn object<'a>(
    value: &'a mut Value,
    location: &str,
) -> Result<&'a mut Map<String, Value>> {
    value.as_object_mut().ok_or_else(|| invalid(location, "expected an object"))
}

fn require_string<'a>(
    fields: &'a Map<String, Value>,
    field: &str,
    location: &str,
) -> Result<&'a str> {
    fields.get(field).and_then(Value::as_str).ok_or_else(|| {
        invalid(&format!("{location}.{field}"), "expected a string")
    })
}

fn rename_string(
    value: Option<&mut Value>,
    location: &str,
    names: &[(&str, &str)],
) -> Result<()> {
    let value = value.ok_or_else(|| invalid(location, "missing field"))?;
    let name =
        value.as_str().ok_or_else(|| invalid(location, "expected a string"))?;
    let replacement = names
        .iter()
        .find_map(|&(old, new)| (name == old).then_some(new))
        .ok_or_else(|| {
            invalid(location, &format!("unknown legacy variant '{name}'"))
        })?;
    *value = replacement.into();
    Ok(())
}

fn invalid(location: &str, reason: &str) -> HistoryError {
    HistoryError::LoadError(format!(
        "History migration failed at {location}: {reason}"
    ))
}
