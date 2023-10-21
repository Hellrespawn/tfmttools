use crate::cli::args::Args;
use color_eyre::eyre::eyre;
use color_eyre::Result;
use file_history::Action;
use std::collections::HashMap;
use std::path::Path;

pub(crate) fn validate_actions(actions: &[Action]) -> Result<()> {
    validate_collisions(actions)?;
    validate_existing_files(actions)?;

    Ok(())
}

fn validate_collisions(actions: &[Action]) -> Result<()> {
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
        Err(eyre!(format_collisions(&collisions)))
    }
}

fn format_collisions(collisions: &HashMap<&Path, Vec<&Path>>) -> String {
    let length = collisions.len();
    let mut string = format!(
        "{} collision{} {} detected{}:\n",
        length,
        if length > 1 { "s" } else { "" },
        if length > 1 { "were" } else { "was" },
        if length > Args::DEFAULT_PREVIEW_AMOUNT {
            format!("! Showing {}", Args::DEFAULT_PREVIEW_AMOUNT)
        } else {
            String::new()
        },
    );

    for (i, (path, collisions)) in collisions.iter().enumerate() {
        if i >= Args::DEFAULT_PREVIEW_AMOUNT {
            break;
        }
        let length = collisions.len();
        string += &format!(
            "{} is pointed to by {} file{}{}:\n",
            path.display(),
            length,
            if length > 1 { "s" } else { "" },
            if length > Args::DEFAULT_PREVIEW_AMOUNT {
                format!("! Showing {}", Args::DEFAULT_PREVIEW_AMOUNT)
            } else {
                String::new()
            },
        );

        for (i, path) in collisions.iter().enumerate() {
            if i >= Args::DEFAULT_PREVIEW_AMOUNT {
                break;
            }
            string += &format!("{}\n", path.display());
        }
        string += "\n";
    }

    string
}

fn validate_existing_files(actions: &[Action]) -> Result<()> {
    let existing: Vec<&Path> = actions
        .iter()
        .filter_map(|action| {
            let (_, target) = action.get_src_tgt_unchecked();
            target.exists().then_some(target)
        })
        .collect();

    let length = existing.len();

    if !existing.is_empty() {
        let string = format!(
            "{} file{} already exist{}{}:\n{}",
            length,
            if length > 1 { "s" } else { "" },
            if length > 1 { "" } else { "s" },
            if length > Args::DEFAULT_PREVIEW_AMOUNT {
                format!("! Showing {}", Args::DEFAULT_PREVIEW_AMOUNT)
            } else {
                String::new()
            },
            existing
                .iter()
                .take(Args::DEFAULT_PREVIEW_AMOUNT)
                .map(|p| p.display().to_string())
                .collect::<Vec<String>>()
                .join("\n")
        );
        return Err(eyre!(string));
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn validate_collisions_test() -> Result<()> {
        let reference = [
            ("/a/b/c.file", "/b/c/d.file"),
            ("/c/d/e.file", "/b/c/d.file"),
        ]
        .map(|(source, target)| Action::mv(source, target));

        if let Ok(()) = validate_collisions(&reference) {
            return Err(eyre!(
                "validate_collisions should have returned an error!"
            ));
        }

        let reference = [
            ("/a/b/c.file", "/b/c/d.file"),
            ("/c/d/e.file", "/d/e/f.file"),
        ]
        .map(|(source, target)| Action::mv(source, target));

        if let Err(err) = validate_collisions(&reference) {
            return Err(eyre!(
                "validate_collisions returned an error when it shouldn't!\n{}",
                err
            ));
        }

        Ok(())
    }
}
