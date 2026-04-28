use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::eyre;
use lofty::tag::TagItem;
use tfmttools_core::action::{
    Action, FORBIDDEN_CHARACTERS, TagValueChange, TagValueKind,
};
use tfmttools_core::audiofile::AudioFile;
use tfmttools_core::error::TFMTError;
use tfmttools_core::history::{ActionRecordMetadata, TemplateMetadata};
use tfmttools_core::util::{FSMode, Utf8PathExt};
use tfmttools_fs::{
    ActionExecutor, FsHandler, PathIterator, PathIteratorOptions,
};
use tfmttools_history::{History, HistoryError};
use tracing::{debug, trace};

use crate::cli::{
    ConfirmMode, FixEncodingArgs, TFMTOptions, ValidateArgs,
    ValidateFixSubcommand, ValidateOptions, ValidateSubcommand,
};
use crate::history::load_history;
use crate::ui::{ConfirmationPrompt, ProgressBar};

pub fn validate(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    validate_args: ValidateArgs,
) -> Result<()> {
    let validate_options =
        ValidateOptions::try_from(validate_args.common_args)?;
    let file_paths = gather_file_paths(app_options, &validate_options);

    debug!("Read {} files.", file_paths.len());

    match validate_args.command.unwrap_or(ValidateSubcommand::Check) {
        ValidateSubcommand::Check => {
            check(app_options, file_paths);
            Ok(())
        },
        ValidateSubcommand::Fix(fix_args) => {
            match fix_args.command {
                ValidateFixSubcommand::Encoding(args) => {
                    fix_encoding(fs_handler, app_options, file_paths, &args)
                },
                ValidateFixSubcommand::Characters => {
                    fix_characters(fs_handler, app_options, file_paths)
                },
            }
        },
    }
}

fn check(app_options: &TFMTOptions, file_paths: Vec<Utf8PathBuf>) {
    let result = validate_files(app_options, file_paths);

    result.print();

    if !result.is_valid() {
        std::process::exit(1);
    }
}

fn fix_characters(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
) -> Result<()> {
    let actions = create_fix_actions(app_options, file_paths, |value| {
        let fixed = safe_interpolation_value(value);

        (fixed != value).then_some(FieldFix { new_value: fixed })
    });

    apply_and_store_fix(
        fs_handler,
        app_options,
        actions,
        "validate fix characters",
    )
}

fn fix_encoding(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
    args: &FixEncodingArgs,
) -> Result<()> {
    let encoding = SourceEncoding::parse(&args.encoding)?;
    let actions = create_fix_actions(app_options, file_paths, |value| {
        fix_encoding_value(value, encoding)
    });
    let problems = actions
        .iter()
        .flat_map(|action| {
            match action {
                Action::EditTagValues { path, changes } => {
                    changes
                        .iter()
                        .flat_map(|change| {
                            encoding_problems(change.old_value())
                                .into_iter()
                                .map(move |problem| (path, change, problem))
                        })
                        .collect::<Vec<_>>()
                },
                _ => Vec::new(),
            }
        })
        .collect::<Vec<_>>();

    if !problems.is_empty() {
        println!("Encoding conversion is lossy.");
        for (path, change, problem) in &problems {
            println!("\t{path} [{}]: {problem}", change.key());
        }

        if !confirm_lossy_encoding_fix(app_options)? {
            println!("Aborting!");
            return Ok(());
        }
    }

    apply_and_store_fix(
        fs_handler,
        app_options,
        actions,
        "validate fix encoding",
    )
}

fn create_fix_actions(
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
    fix_value: impl Fn(&str) -> Option<FieldFix>,
) -> Vec<Action> {
    let bar = ProgressBar::bar(
        app_options.display_mode(),
        "Determining tag fixes...",
        "Determined tag fixes.",
        file_paths.len() as u64,
        false,
    );
    let mut actions = Vec::new();

    for path in file_paths {
        bar.inc_found();

        if let Ok(audio_file) = AudioFile::new(path.clone()) {
            let changes = audio_file
                .tag()
                .items()
                .filter_map(|item| tag_value_change(item, &fix_value))
                .collect::<Vec<_>>();

            if !changes.is_empty() {
                actions.push(Action::EditTagValues { path, changes });
            }
        }
    }

    bar.finish();

    actions
}

fn tag_value_change(
    item: &TagItem,
    fix_value: &impl Fn(&str) -> Option<FieldFix>,
) -> Option<TagValueChange> {
    let (kind, value) = tag_item_value(item)?;
    let fixed = fix_value(value)?;

    Some(TagValueChange::new(
        format!("{:?}", item.key()),
        kind,
        value.to_owned(),
        fixed.new_value,
    ))
}

fn tag_item_value(item: &TagItem) -> Option<(TagValueKind, &str)> {
    item.value().text().map(|value| (TagValueKind::Text, value)).or_else(|| {
        item.value().locator().map(|value| (TagValueKind::Locator, value))
    })
}

fn apply_and_store_fix(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    actions: Vec<Action>,
    command: &str,
) -> Result<()> {
    if actions.is_empty() {
        println!("No tag values changed.");
        return Ok(());
    }

    report_actions(&actions, matches!(app_options.fs_mode(), FSMode::DryRun));

    if matches!(app_options.fs_mode(), FSMode::DryRun) {
        return Ok(());
    }

    let applied_actions =
        ActionExecutor::new(fs_handler).apply_actions(actions)?;
    let (mut history, _) = load_history(&app_options.history_file_path()?)?;
    store_history(app_options, &mut history, applied_actions, command)?;

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

fn store_history(
    app_options: &TFMTOptions,
    history: &mut History<Action, ActionRecordMetadata>,
    actions: Vec<Action>,
    command: &str,
) -> Result<()> {
    let metadata = ActionRecordMetadata::new(
        TemplateMetadata::Validation(command.to_owned()),
        Vec::new(),
        app_options.run_id().to_owned(),
    );

    history.push(actions, metadata)?;

    match history.save() {
        Err(err @ HistoryError::SaveErrorWithBackup { .. }) => {
            eprintln!("{err}");
        },
        result => {
            result?;
            println!("Saved run #{} to history.", app_options.run_id());
        },
    }

    Ok(())
}

fn confirm_lossy_encoding_fix(app_options: &TFMTOptions) -> Result<bool> {
    Ok(matches!(app_options.confirm_mode(), ConfirmMode::NoConfirm)
        || ConfirmationPrompt::new("Apply lossy encoding fixes?").prompt()?)
}

fn gather_file_paths(
    app_options: &TFMTOptions,
    validate_options: &ValidateOptions,
) -> Vec<Utf8PathBuf> {
    let spinner = ProgressBar::spinner(
        app_options.display_mode(),
        "audio",
        "total files",
        "Gathering files...",
        "Gathered files.",
    );

    let options = PathIteratorOptions::with_depth(
        validate_options.input_directory().as_path(),
        validate_options.recursion_depth(),
    );

    let file_paths = PathIterator::new(&options)
        .flatten()
        .inspect(|_| spinner.inc_total())
        .filter(|path| AudioFile::path_predicate(path))
        .inspect(|_| {
            spinner.inc_found();

            #[cfg(feature = "debug")]
            crate::debug::delay();
        })
        .collect::<Vec<_>>();

    spinner.finish();

    file_paths
}

fn validate_files(
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
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

        match AudioFile::new(path.clone()) {
            Ok(audio_file) => {
                trace!("Validated audio file encoding: {audio_file:?}");
                result.checked_files += 1;
                result.tag_errors.extend(validate_tag_values(&audio_file));
            },
            Err(error) => {
                result.read_errors.push(ValidationReadError { path, error });
            },
        }

        #[cfg(feature = "debug")]
        crate::debug::delay();
    }

    bar.finish();

    result
}

fn validate_tag_values(audio_file: &AudioFile) -> Vec<TagValueError> {
    audio_file
        .tag()
        .items()
        .filter_map(|item| {
            let (_, value) = tag_item_value(item)?;
            let forbidden_characters = forbidden_characters_in(value);

            (!forbidden_characters.is_empty()).then(|| {
                TagValueError {
                    path: audio_file.file().as_path().to_owned(),
                    key: format!("{:?}", item.key()),
                    value: value.to_owned(),
                    forbidden_characters,
                }
            })
        })
        .collect()
}

fn safe_interpolation_value(value: &str) -> String {
    let value = FORBIDDEN_CHARACTERS.iter().fold(
        value.to_owned(),
        |string, forbidden_character| {
            string.replace(
                forbidden_character.char(),
                forbidden_character.replacement().unwrap_or(""),
            )
        },
    );

    value.trim_end_matches('.').to_owned()
}

fn forbidden_characters_in(value: &str) -> Vec<&'static str> {
    let mut forbidden_characters = Vec::new();

    for forbidden_character in FORBIDDEN_CHARACTERS.iter() {
        if value.contains(forbidden_character.char()) {
            forbidden_characters.push(forbidden_character.char());
        }
    }

    forbidden_characters
}

#[derive(Debug, Clone, Copy)]
enum SourceEncoding {
    Iso88591,
}

impl SourceEncoding {
    fn parse(encoding: &str) -> Result<Self> {
        match encoding.to_ascii_lowercase().as_str() {
            "iso-8859-1" | "iso8859-1" | "latin1" | "latin-1" => {
                Ok(Self::Iso88591)
            },
            _ => Err(eyre!("Unsupported encoding: {encoding}")),
        }
    }
}

fn fix_encoding_value(
    value: &str,
    encoding: SourceEncoding,
) -> Option<FieldFix> {
    let bytes = encode_source_bytes(value, encoding);
    let new_value = match String::from_utf8(bytes.clone()) {
        Ok(value) => value,
        Err(_) => String::from_utf8_lossy(&bytes).into_owned(),
    };

    (new_value != value).then_some(FieldFix { new_value })
}

fn encode_source_bytes(value: &str, encoding: SourceEncoding) -> Vec<u8> {
    match encoding {
        SourceEncoding::Iso88591 => {
            value
                .chars()
                .map(|character| {
                    u8::try_from(u32::from(character)).unwrap_or(b'?')
                })
                .collect()
        },
    }
}

fn encoding_problems(value: &str) -> Vec<String> {
    let mut problems = value
        .chars()
        .filter(|character| u32::from(*character) > 0xff)
        .map(|character| {
            format!("'{character}' cannot be represented in ISO-8859-1")
        })
        .collect::<Vec<_>>();

    let bytes = encode_source_bytes(value, SourceEncoding::Iso88591);
    if let Err(err) = String::from_utf8(bytes) {
        problems.push(format!("invalid UTF-8 byte sequence: {err}"));
    }

    problems
}

#[derive(Debug)]
struct FieldFix {
    new_value: String,
}

#[derive(Debug, Default)]
struct ValidationResult {
    checked_files: usize,
    read_errors: Vec<ValidationReadError>,
    tag_errors: Vec<TagValueError>,
}

impl ValidationResult {
    fn is_valid(&self) -> bool {
        self.read_errors.is_empty() && self.tag_errors.is_empty()
    }

    fn print(&self) {
        if self.is_valid() {
            println!("Validated {} audio files.", self.checked_files);
            return;
        }

        println!("Validation failed.");

        if !self.read_errors.is_empty() {
            println!();
            println!("Tag encoding/read errors:");
            for error in &self.read_errors {
                println!("\t{}: {}", error.path, error.error);
            }
        }

        if !self.tag_errors.is_empty() {
            println!();
            println!("Forbidden characters in tag values:");
            for error in &self.tag_errors {
                println!(
                    "\t{} [{}] contains '{}': {}",
                    error.path,
                    error.key,
                    error.forbidden_characters.join("', '"),
                    error.value
                );
            }
        }
    }
}

#[derive(Debug)]
struct ValidationReadError {
    path: Utf8PathBuf,
    error: TFMTError,
}

#[derive(Debug)]
struct TagValueError {
    path: Utf8PathBuf,
    key: String,
    value: String,
    forbidden_characters: Vec<&'static str>,
}

#[cfg(test)]
mod tests {
    use super::{
        SourceEncoding, encoding_problems, fix_encoding_value,
        forbidden_characters_in, safe_interpolation_value,
    };

    #[test]
    fn finds_forbidden_characters() {
        assert_eq!(forbidden_characters_in("AC/DC: Live"), vec![":", "/"]);
    }

    #[test]
    fn replaces_forbidden_characters_like_templates() {
        assert_eq!(safe_interpolation_value("AC/DC: Live."), "AC-DC Live");
    }

    #[test]
    fn fixes_latin1_mojibake_when_lossless() {
        let fix =
            fix_encoding_value("FranÃ§ois", SourceEncoding::Iso88591).unwrap();

        assert_eq!(fix.new_value, "François");
    }

    #[test]
    fn reports_lossy_encoding_problems() {
        assert!(!encoding_problems("Beyoncé").is_empty());
    }
}
