use std::collections::HashMap;

use color_eyre::Result;
use color_eyre::eyre::eyre;
use tfmttools_core::action::{RenameAction, validate_rename_actions};
use tfmttools_core::util::{Utf8File, Utf8PathExt};
use tfmttools_core::warning::Warning;

use super::{RenamePlan, RenameSession};
use crate::ui::{ItemName, PreviewList, current_dir_utf8};

pub fn preview(session: &RenameSession, plan: &RenamePlan) -> Result<()> {
    validate_rename_action_errors(&plan.actions)?;
    preview_rename_actions(
        session,
        &plan.actions,
        &plan.unchanged_files,
        &plan.warnings,
    )
}

fn validate_rename_action_errors(
    rename_actions: &[RenameAction],
) -> Result<()> {
    let validation_errors = validate_rename_actions(rename_actions);

    if validation_errors.is_empty() {
        Ok(())
    } else {
        let error_string = validation_errors
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        Err(eyre!("Had validation errors:\n{error_string}"))
    }
}

fn preview_rename_actions(
    session: &RenameSession,
    rename_actions: &[RenameAction],
    unchanged_files: &[Utf8File],
    warnings: &[Warning],
) -> Result<()> {
    let working_directory = current_dir_utf8()?;

    let iter = rename_actions.iter().map(|rename_action| {
        super::shared::strip_path_prefix(
            rename_action.target().as_path(),
            working_directory.as_path(),
        )
    });

    let preview_list =
        PreviewList::new(iter, session.app_options().preview_list_size())
            .with_item_name(ItemName::simple("destination"));

    if !unchanged_files.is_empty() {
        println!("There are {} unchanged files.\n", unchanged_files.len());
    }

    preview_list.print()?;

    if let Some(report) = format_warnings(warnings) {
        println!();
        println!("{report}");
    }

    Ok(())
}

fn format_warnings(warnings: &[Warning]) -> Option<String> {
    if warnings.is_empty() {
        return None;
    }

    let mut lines: Vec<String> = Vec::new();

    for warning in warnings {
        match warning {
            Warning::DeprecatedPositionalArgs { template } => {
                lines.push(format!(
                    "  \u{26a0} Template '{template}': uses positional args[N] without frontmatter; declare arguments to migrate."
                ));
            },
            Warning::DeprecatedLeadingComment { template } => {
                lines.push(format!(
                    "  \u{26a0} Template '{template}': uses a leading comment as its description; move it to frontmatter's `description` field."
                ));
            },
            Warning::WhitespaceInTag { .. } => {},
        }
    }

    let mut whitespace_by_tag: HashMap<&str, usize> = HashMap::new();
    for warning in warnings {
        if let Warning::WhitespaceInTag { tag_name, .. } = warning {
            *whitespace_by_tag.entry(tag_name.as_str()).or_insert(0) += 1;
        }
    }

    let mut whitespace_entries: Vec<(&str, usize)> =
        whitespace_by_tag.into_iter().collect();
    whitespace_entries.sort_by_key(|(tag, _)| *tag);

    for (tag, count) in whitespace_entries {
        let file_word = if count == 1 { "file" } else { "files" };
        lines.push(format!(
            "  \u{26a0} {count} {file_word}: leading/trailing whitespace in `{tag}`"
        ));
    }

    Some(format!("Warnings:\n{}", lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use tfmttools_core::warning::Warning;

    use super::*;

    #[test]
    fn format_warnings_empty_returns_none() {
        assert!(format_warnings(&[]).is_none());
    }

    #[test]
    fn format_warnings_groups_whitespace_by_tag() {
        let warnings = vec![
            Warning::WhitespaceInTag {
                file: "a.mp3".to_owned(),
                tag_name: "track_artist".to_owned(),
            },
            Warning::WhitespaceInTag {
                file: "b.mp3".to_owned(),
                tag_name: "track_artist".to_owned(),
            },
            Warning::WhitespaceInTag {
                file: "c.mp3".to_owned(),
                tag_name: "album".to_owned(),
            },
        ];

        let output = format_warnings(&warnings).unwrap();
        assert!(output.contains("2 files") && output.contains("track_artist"));
        assert!(output.contains("1 file") && output.contains("album"));
    }

    #[test]
    fn format_warnings_includes_template_warnings() {
        let warnings = vec![Warning::DeprecatedPositionalArgs {
            template: "my_template".to_owned(),
        }];

        let output = format_warnings(&warnings).unwrap();
        assert!(output.contains("my_template"));
        assert!(output.contains("positional"));
    }
}
