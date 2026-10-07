use camino::Utf8PathBuf;
use serde_json::Value;
use tempfile::TempDir;
use tfmttools_core::history::History;

pub fn load(
    value: &Value,
) -> Result<History, tfmttools_core::history::HistoryError> {
    let directory = TempDir::new().unwrap();
    let path =
        Utf8PathBuf::from_path_buf(directory.path().join("history.json"))
            .unwrap();
    std::fs::write(&path, serde_json::to_vec(value).unwrap()).unwrap();
    let mut history = History::new(path);
    history.load()?;
    Ok(history)
}
