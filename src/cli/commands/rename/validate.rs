use std::collections::HashMap;

use camino::Utf8Path;
use color_eyre::eyre::eyre;
use color_eyre::Result;

use crate::action::Move;
use crate::cli::ui::table::Table;
use crate::config::Config;

pub(crate) fn validate_move_actions(
    config: &Config,
    move_actions: &[Move],
) -> Result<()> {
    validate_double_separators(config, move_actions)?;
    validate_collisions(config, move_actions)?;
    validate_existing_files(config, move_actions)?;

    Ok(())
}

fn validate_double_separators(
    config: &Config,
    move_actions: &[Move],
) -> Result<()> {
    let move_actions_with_double_separators = move_actions
        .iter()
        .filter(|move_action| {
            move_action
                .target()
                .to_string()
                .contains(&std::path::MAIN_SEPARATOR_STR.repeat(2))
        })
        .collect::<Vec<_>>();

    let paths = move_actions_with_double_separators
        .iter()
        .take(config.preview_amount())
        .map(|m| m.target())
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

        if move_actions_with_double_separators.len() > paths.len() {
            table.push_string(format!(
                "And {} more...",
                move_actions_with_double_separators.len() - paths.len()
            ));
        }

        Err(eyre!(table.to_string()))
    }
}

fn validate_collisions(config: &Config, move_actions: &[Move]) -> Result<()> {
    let mut map = HashMap::new();

    for move_action in move_actions {
        let source = move_action.source();

        map.entry(move_action.target()).or_insert_with(Vec::new).push(source);
    }

    let collisions: HashMap<&Utf8Path, Vec<&Utf8Path>> =
        map.into_iter().filter(|(_, v)| v.len() > 1).collect();

    if collisions.is_empty() {
        Ok(())
    } else {
        Err(eyre!(format_collisions(config, &collisions)))
    }
}

fn format_collisions(
    config: &Config,
    collisions_map: &HashMap<&Utf8Path, Vec<&Utf8Path>>,
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
    path: &Utf8Path,
    collisions: &[&Utf8Path],
) -> String {
    let mut table = Table::new();

    let length = collisions.len();

    table.set_heading(format!(
        "{} is pointed to by {} file{}",
        path,
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

fn validate_existing_files(
    config: &Config,
    move_actions: &[Move],
) -> Result<()> {
    let existing: Vec<&Utf8Path> = move_actions
        .iter()
        .filter_map(|move_action| {
            let target = move_action.target();
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
    use camino::Utf8PathBuf;

    use super::*;

    #[test]
    fn validate_collisions_test() -> Result<()> {
        let config = Config::new(&Utf8PathBuf::new(), false)?;

        let reference =
            [("/a/b/c.file", "/b/c/d.file"), ("/c/d/e.file", "/b/c/d.file")]
                .map(|(source, target)| {
                    Move::new(
                        Utf8PathBuf::from(source),
                        Utf8PathBuf::from(target),
                    )
                });

        if let Ok(()) = validate_collisions(&config, &reference) {
            return Err(eyre!(
                "validate_collisions should have returned an error!"
            ));
        }

        let reference =
            [("/a/b/c.file", "/b/c/d.file"), ("/c/d/e.file", "/d/e/f.file")]
                .map(|(source, target)| {
                    Move::new(
                        Utf8PathBuf::from(source),
                        Utf8PathBuf::from(target),
                    )
                });

        if let Err(err) = validate_collisions(&config, &reference) {
            return Err(eyre!(
                "validate_collisions returned an error when it shouldn't!\n{}",
                err
            ));
        }

        Ok(())
    }
}
