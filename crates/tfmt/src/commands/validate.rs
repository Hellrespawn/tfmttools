mod inspection;

use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::bail;
use inspection::{
    Id3EncodingIssue, TagValueIssue, ValidationScope, inspect_file,
};
use tfmttools_core::action::Action;
use tfmttools_core::history::{
    ActionRecordMetadata, History, TemplateMetadata,
};
use tfmttools_core::util::{FSMode, Utf8PathExt};
use tfmttools_fs::{FsError, PathIteratorOptions};
use tracing::debug;

use super::CommandOutcome;
use crate::cli::{TFMTOptions, ValidateArgs, ValidateOptions, ValidateType};
use crate::history::load_history_for_mode;
use crate::ui::{ProgressBar, current_dir_utf8};

pub fn validate(
    app_options: &TFMTOptions,
    validate_args: ValidateArgs,
) -> Result<CommandOutcome> {
    let relative_input = validate_args
        .common_args
        .custom_input_directory
        .as_ref()
        .is_some_and(|path| path.is_relative());
    let cwd = current_dir_utf8()?;
    let validate_options =
        ValidateOptions::try_from(validate_args.common_args)?;

    if validate_args.command.is_none() && validate_args.fix {
        bail!("--fix requires a validation type.");
    }

    let options = PathIteratorOptions::with_depth(
        validate_options.input_directory().as_path(),
        validate_options.recursion_depth(),
    );
    let gather = || -> Result<Vec<Utf8PathBuf>> {
        let paths =
            super::audio_discovery::gather_audio_paths(app_options, &options)?;
        Ok(paths
            .into_iter()
            .map(|path| {
                if relative_input {
                    path.strip_prefix(cwd.as_path()).unwrap_or(&path).to_owned()
                } else {
                    path
                }
            })
            .collect())
    };

    let scope = match validate_args.command {
        None => ValidationScope::All,
        Some(ValidateType::Characters) => ValidationScope::Characters,
        Some(ValidateType::Id3Encoding) => ValidationScope::Id3Encoding,
    };
    if !validate_args.fix {
        return Ok(check(app_options, gather()?, scope));
    }

    let (mut history, _) = load_history_for_mode(
        &app_options.history_file_path()?,
        app_options.fs_mode(),
    )?;
    crate::history::execution::check_interrupted(
        &mut history,
        app_options.fs_mode(),
    )?;
    let file_paths = gather()?;
    debug!("Read {} files.", file_paths.len());

    let result = validate_files(app_options, file_paths, scope);
    if !result.read_errors.is_empty() {
        result.print_read_errors();
        bail!(
            "Cannot apply tag fixes because audio inspection was incomplete; no tag fixes were applied"
        );
    }
    let command = match scope {
        ValidationScope::Characters => "validate characters --fix",
        ValidationScope::Id3Encoding => "validate id3-encoding --fix",
        ValidationScope::All => {
            unreachable!("bare validate --fix is rejected early")
        },
    };
    apply_and_store_fix(&mut history, app_options, &result.actions, command)?;
    Ok(CommandOutcome::Success)
}
fn check(
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
    scope: ValidationScope,
) -> CommandOutcome {
    let result = validate_files(app_options, file_paths, scope);

    result.print();

    if result.is_valid() {
        CommandOutcome::Success
    } else {
        CommandOutcome::ValidationFailed
    }
}

fn apply_and_store_fix(
    history: &mut History,
    app_options: &TFMTOptions,
    actions: &[Action],
    command: &str,
) -> Result<()> {
    if actions.is_empty() {
        println!("No tag values changed.");
        return Ok(());
    }

    if matches!(app_options.fs_mode(), FSMode::DryRun) {
        report_actions(actions, true);
        return Ok(());
    }

    let metadata = ActionRecordMetadata::new(
        TemplateMetadata::Validation { value: command.to_owned() },
        Vec::new(),
        app_options.run_id().to_owned(),
    );
    crate::history::execution::execute_recorded(
        history,
        app_options.fs_mode(),
        actions.to_vec(),
        metadata,
    )?;
    report_actions(actions, false);
    println!("Saved run #{} to history.", app_options.run_id());

    Ok(())
}

fn report_actions(actions: &[Action], dry_run: bool) {
    let verb = if dry_run { "Would update" } else { "Updated" };

    for action in actions {
        if let Action::EditTagValues { path, changes } = action {
            println!("{verb} {path}:");
            for change in changes {
                println!(
                    "\t[{}] '{}' => '{}'",
                    change.key(),
                    change.old_value(),
                    change.new_value()
                );
            }
        }
    }
}

fn validate_files(
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
    scope: ValidationScope,
) -> ValidationResult {
    let bar = ProgressBar::bar(
        app_options.display_mode(),
        "Validating files...",
        "Validated files.",
        file_paths.len() as u64,
        false,
    );

    let mut result = ValidationResult::default();

    for path in file_paths {
        bar.inc_found();

        match inspect_file(&path, scope) {
            Ok(inspection) => {
                result.checked_files += 1;
                result.tag_issues.extend(inspection.tag_issues);
                result
                    .id3_encoding_issues
                    .extend(inspection.id3_encoding_issues);
                if !inspection.changes.is_empty() {
                    result.actions.push(Action::EditTagValues {
                        path,
                        changes: inspection.changes,
                    });
                }
            },
            Err(error) => {
                result.read_errors.push(ValidationReadError { path, error })
            },
        }

        #[cfg(feature = "debug")]
        crate::debug::delay();
    }

    bar.finish();

    result
}

#[derive(Debug, Default)]
struct ValidationResult {
    checked_files: usize,
    actions: Vec<Action>,
    read_errors: Vec<ValidationReadError>,
    tag_issues: Vec<TagValueIssue>,
    id3_encoding_issues: Vec<Id3EncodingIssue>,
}

impl ValidationResult {
    fn is_valid(&self) -> bool {
        self.read_errors.is_empty()
            && self.tag_issues.is_empty()
            && self.id3_encoding_issues.is_empty()
    }

    fn print_read_errors(&self) {
        if !self.read_errors.is_empty() {
            println!();
            println!("Tag encoding/read errors:");
            for error in &self.read_errors {
                println!("\t{}: {}", error.path, error.error);
            }
        }
    }

    fn print(&self) {
        if self.is_valid() {
            println!("Validated {} audio files.", self.checked_files);
            return;
        }

        self.print_read_errors();

        if !self.tag_issues.is_empty() {
            println!();
            println!(
                "Some tag values contain characters that may not work well in filenames."
            );
            println!(
                "Run `tfmt validate characters --fix` to strip or replace them."
            );
            println!("This changes file tags.");
            for issue in &self.tag_issues {
                println!(
                    "\t{} [{}] contains '{}': {}",
                    issue.path,
                    issue.key,
                    issue.characters.join("', '"),
                    issue.value
                );
            }
        }

        if !self.id3_encoding_issues.is_empty() {
            println!();
            println!(
                "Some ID3 text frames contain non-ASCII characters but are not encoded as UTF-16."
            );
            println!("UTF-16 is recommended for compatibility.");
            println!(
                "Run `tfmt validate id3-encoding --fix` to rewrite matching frames as UTF-16."
            );
            println!("This changes file tags.");
            for issue in &self.id3_encoding_issues {
                println!(
                    "\t{} [{}] {}: {}",
                    issue.path, issue.key, issue.encoding, issue.value
                );
            }
        }
    }
}

#[derive(Debug)]
struct ValidationReadError {
    path: Utf8PathBuf,
    error: FsError,
}
