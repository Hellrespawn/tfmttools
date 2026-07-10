use tfmttools_core::action::{CaseInsensitivePathSet, RenameAction};
use tfmttools_core::util::Utf8PathExt;

#[must_use]
pub fn existing_target_paths(
    rename_actions: &[RenameAction],
) -> CaseInsensitivePathSet {
    let mut set = CaseInsensitivePathSet::new();

    for rename_action in rename_actions {
        if rename_action.target().exists() {
            set.insert(rename_action.target());
        }
    }

    set
}

#[cfg(test)]
mod tests {
    use assert_fs::TempDir;
    use camino::Utf8PathBuf;
    use color_eyre::Result;
    use tfmttools_core::util::Utf8File;

    use super::*;

    #[test]
    fn includes_only_targets_that_exist_on_disk() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let existing = temp_dir.path().join("existing.mp3");
        let missing = temp_dir.path().join("missing.mp3");
        fs_err::write(&existing, "x")?;

        let existing = Utf8File::new(Utf8PathBuf::try_from(existing)?);
        let missing = Utf8File::new(Utf8PathBuf::try_from(missing)?);

        let rename_actions = vec![
            RenameAction::new(Utf8File::new("a.mp3"), existing.clone()),
            RenameAction::new(Utf8File::new("b.mp3"), missing.clone()),
        ];

        let set = existing_target_paths(&rename_actions);

        assert!(set.contains(&existing));
        assert!(!set.contains(&missing));

        Ok(())
    }
}
