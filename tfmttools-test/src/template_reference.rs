use std::collections::HashMap;
use std::path::Path;

use color_eyre::Result;
use serde::Deserialize;
use tfmttools_fs::FileOrName;

#[derive(Deserialize)]
pub struct TemplateReference {
    pub template: FileOrName,
    pub reference: HashMap<String, String>,
    pub arguments: Option<Vec<String>>,
}

impl TemplateReference {
    pub fn from_file(path: &Path) -> Result<Self> {
        let body = fs_err::read_to_string(path)?;

        let template_reference: TemplateReference =
            serde_json::from_str(&body)?;

        Ok(template_reference)
    }
}
