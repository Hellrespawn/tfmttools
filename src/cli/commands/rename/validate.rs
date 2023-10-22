use crate::cli::ui::table::Table;
use crate::cli::Config;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use file_history::Action;
use std::collections::HashMap;
use std::path::Path;

pub(crate) fn validate_actions(
    config: &Config,
    actions: &[Action],
) -> Result<()> {
    validate_double_separators(config, actions)?;
    validate_collisions(config, actions)?;
    validate_existing_files(config, actions)?;

    Ok(())
}

fn validate_double_separators(
    config: &Config,
    actions: &[Action],
) -> Result<()> {
    let double_actions = actions
        .iter()
        .filter(|a| {
            a.get_src_tgt_unchecked()
                .1
                .to_string_lossy()
                .contains(&std::path::MAIN_SEPARATOR_STR.repeat(2))
        })
        .collect::<Vec<_>>();

    let paths = double_actions
        .iter()
        .take(config.preview_amount())
        .map(|a| a.get_src_tgt_unchecked().1)
        .collect::<Vec<_>>();

    if paths.is_empty() {
        Ok(())
    } else {
        let mut table = Table::new();

        table.set_heading(
            "The following destinations contain double directory separators"
                .to_owned(),
        );

        for path in &paths {
            table.push_path(path);
        }

        if double_actions.len() > paths.len() {
            table.push_string(format!(
                "And {} more...",
                double_actions.len() - paths.len()
            ));
        }

        Err(eyre!(table.to_string()))
    }
}

fn validate_collisions(config: &Config, actions: &[Action]) -> Result<()> {
    let mut map = HashMap::new();

    for action in actions {
        let (source, target) = action.get_src_tgt_unchecked();
        map.entry(target).or_insert_with(Vec::new).push(source);
    }

    let collisions: HashMap<&Path, Vec<&Path>> =
        map.into_iter().filter(|(_, v)| v.len() > 1).collect();

    if collisions.is_empty() {
        Ok(())
    } else {
        Err(eyre!(format_collisions(config, &collisions)))
    }
}

fn format_collisions(
    config: &Config,
    collisions_map: &HashMap<&Path, Vec<&Path>>,
) -> String {
    let length = collisions_map.len();

    let mut string = format!(
        "{} collision{} detected:\n",
        length,
        if length > 1 { "s were" } else { " was" }
    );

    for (path, collisions) in
        collisions_map.iter().take(config.preview_amount())
    {
        string += &format_collision(config, path, collisions);
    }

    if length > config.preview_amount() {
        string +=
            &format!("\nAnd {} more...", length - config.preview_amount());
    }

    string
}

fn format_collision(
    config: &Config,
    path: &Path,
    collisions: &[&Path],
) -> String {
    let mut table = Table::new();

    let length = collisions.len();

    table.set_heading(format!(
        "{} is pointed to by {} file{}",
        path.display(),
        length,
        if length > 1 { "s" } else { "" },
    ));

    let iter = collisions.iter().take(config.preview_amount());

    for path in iter {
        table.push_path(path);
    }

    if length > config.preview_amount() {
        table.push_string(format!(
            "And {} more...",
            length - config.preview_amount()
        ));
    }

    table.to_string()
}

fn validate_existing_files(config: &Config, actions: &[Action]) -> Result<()> {
    let existing: Vec<&Path> = actions
        .iter()
        .filter_map(|action| {
            let (_, target) = action.get_src_tgt_unchecked();
            target.exists().then_some(target)
        })
        .collect();

    let length = existing.len();

    if existing.is_empty() {
        Ok(())
    } else {
        let mut table = Table::new();

        table.set_heading(format!(
            "{} file{} already exist{}",
            length,
            if length > 1 { "s" } else { "" },
            if length > 1 { "" } else { "s" },
        ));

        for path in existing.iter().take(config.preview_amount()) {
            table.push_path(path);
        }

        if length > config.preview_amount() {
            table.push_string(format!(
                "And {} more...",
                length - config.preview_amount()
            ));
        }

        Err(eyre!(table.to_string()))
    }
}

#[cfg(test)]
mod test {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn validate_collisions_test() -> Result<()> {
        let config = Config::new(&PathBuf::new(), false)?;

        let reference = [
            ("/a/b/c.file", "/b/c/d.file"),
            ("/c/d/e.file", "/b/c/d.file"),
        ]
        .map(|(source, target)| Action::mv(source, target));

        if let Ok(()) = validate_collisions(&config, &reference) {
            return Err(eyre!(
                "validate_collisions should have returned an error!"
            ));
        }

        let reference = [
            ("/a/b/c.file", "/b/c/d.file"),
            ("/c/d/e.file", "/d/e/f.file"),
        ]
        .map(|(source, target)| Action::mv(source, target));

        if let Err(err) = validate_collisions(&config, &reference) {
            return Err(eyre!(
                "validate_collisions returned an error when it shouldn't!\n{}",
                err
            ));
        }

        Ok(())
    }
}
