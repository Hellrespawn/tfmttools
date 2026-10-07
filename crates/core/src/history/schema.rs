use schemars::{Schema, SchemaGenerator, json_schema};
use serde_json::Value;

use super::{CURRENT_SCHEMA_VERSION, StoredHistory};

/// Generate the published schema snapshot from the current storage model.
pub fn history_schema_json() -> Result<String, serde_json::Error> {
    let mut schema =
        serde_json::to_value(schemars::schema_for!(StoredHistory))?;
    sort_objects(&mut schema);
    Ok(format!("{}\n", serde_json::to_string_pretty(&schema)?))
}

pub(super) fn version_schema(_: &mut SchemaGenerator) -> Schema {
    json_schema!({"type": "integer", "const": CURRENT_SCHEMA_VERSION})
}

fn sort_objects(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            for field in fields.values_mut() {
                sort_objects(field);
            }
            fields.sort_keys();
        },
        Value::Array(elements) => elements.iter_mut().for_each(sort_objects),
        _ => {},
    }
}
