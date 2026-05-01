use camino::Utf8PathBuf;
use color_eyre::Result;
use color_eyre::eyre::bail;
use lofty::TextEncoding;
use lofty::id3::v2::{Frame, Id3v2Tag};
use lofty::tag::{ItemKey, Tag, TagItem, TagType};
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

use crate::cli::{TFMTOptions, ValidateArgs, ValidateOptions, ValidateType};
use crate::history::load_history;
use crate::ui::ProgressBar;

pub fn validate(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    validate_args: ValidateArgs,
) -> Result<()> {
    let validate_options =
        ValidateOptions::try_from(validate_args.common_args)?;

    if validate_args.command.is_none() && validate_args.fix {
        bail!("--fix requires a validation type.");
    }

    let file_paths = gather_file_paths(app_options, &validate_options);

    debug!("Read {} files.", file_paths.len());

    match (validate_args.command, validate_args.fix) {
        (None, false) => {
            check(app_options, file_paths, ValidationScope::All);
            Ok(())
        },
        (None, true) => unreachable!("bare validate --fix is rejected early"),
        (Some(ValidateType::Characters), false) => {
            check(app_options, file_paths, ValidationScope::Characters);
            Ok(())
        },
        (Some(ValidateType::Characters), true) => {
            fix_characters(fs_handler, app_options, file_paths)
        },
        (Some(ValidateType::Id3Encoding), false) => {
            check(app_options, file_paths, ValidationScope::Id3Encoding);
            Ok(())
        },
        (Some(ValidateType::Id3Encoding), true) => {
            fix_id3_encoding(fs_handler, app_options, file_paths)
        },
    }
}

fn check(
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
    scope: ValidationScope,
) {
    let result = validate_files(app_options, file_paths, scope);

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
    let actions = create_fix_actions(app_options, file_paths, |_, _, value| {
        let fixed = safe_interpolation_value(value);

        (fixed != value).then_some(FieldFix {
            new_value: fixed,
            old_encoding: None,
            new_encoding: None,
        })
    });

    apply_and_store_fix(
        fs_handler,
        app_options,
        actions,
        "validate characters --fix",
    )
}

fn fix_id3_encoding(
    fs_handler: &FsHandler,
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
) -> Result<()> {
    let file_paths = file_paths
        .into_iter()
        .filter(|path| path.extension() == Some("mp3"))
        .collect::<Vec<_>>();
    let actions =
        create_fix_actions(app_options, file_paths, |tag, item, value| {
            let source_encoding = lofty_id3v2_text_encoding(tag, item.key())?;
            rewrite_id3_text_as_utf16(value, source_encoding)
        });

    apply_and_store_fix(
        fs_handler,
        app_options,
        actions,
        "validate id3-encoding --fix",
    )
}

fn create_fix_actions(
    app_options: &TFMTOptions,
    file_paths: Vec<Utf8PathBuf>,
    fix_value: impl Fn(&Tag, &TagItem, &str) -> Option<FieldFix>,
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
                .filter_map(|item| {
                    tag_value_change(audio_file.tag(), item, &fix_value)
                })
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
    tag: &Tag,
    item: &TagItem,
    fix_value: &impl Fn(&Tag, &TagItem, &str) -> Option<FieldFix>,
) -> Option<TagValueChange> {
    let (kind, value) = tag_item_value(item)?;
    let fixed = fix_value(tag, item, value)?;

    Some(
        TagValueChange::new(
            format!("{:?}", item.key()),
            kind,
            value.to_owned(),
            fixed.new_value,
        )
        .with_encoding(fixed.old_encoding, fixed.new_encoding),
    )
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

        match AudioFile::new(path.clone()) {
            Ok(audio_file) => {
                trace!("Validated audio file encoding: {audio_file:?}");
                result.checked_files += 1;
                if scope.includes_characters() {
                    result.tag_issues.extend(validate_tag_values(&audio_file));
                }
                if scope.includes_id3_encoding() {
                    result
                        .id3_encoding_issues
                        .extend(validate_id3_encoding(&audio_file));
                }
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

#[derive(Debug, Clone, Copy)]
enum ValidationScope {
    All,
    Characters,
    Id3Encoding,
}

impl ValidationScope {
    fn includes_characters(self) -> bool {
        matches!(self, Self::All | Self::Characters)
    }

    fn includes_id3_encoding(self) -> bool {
        matches!(self, Self::All | Self::Id3Encoding)
    }
}

fn validate_tag_values(audio_file: &AudioFile) -> Vec<TagValueIssue> {
    audio_file
        .tag()
        .items()
        .filter_map(|item| {
            let (_, value) = tag_item_value(item)?;
            let forbidden_characters = forbidden_characters_in(value);

            (!forbidden_characters.is_empty()).then(|| {
                TagValueIssue {
                    path: audio_file.file().as_path().to_owned(),
                    key: format!("{:?}", item.key()),
                    value: value.to_owned(),
                    characters: forbidden_characters,
                }
            })
        })
        .collect()
}

fn validate_id3_encoding(audio_file: &AudioFile) -> Vec<Id3EncodingIssue> {
    audio_file
        .tag()
        .items()
        .filter_map(|item| {
            let (_, value) = tag_item_value(item)?;
            let encoding =
                lofty_id3v2_text_encoding(audio_file.tag(), item.key())?;

            should_rewrite_id3_text_as_utf16(value, encoding).then(|| {
                Id3EncodingIssue {
                    path: audio_file.file().as_path().to_owned(),
                    key: format!("{:?}", item.key()),
                    value: value.to_owned(),
                    encoding: text_encoding_name(encoding).to_owned(),
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

fn should_rewrite_id3_text_as_utf16(
    value: &str,
    encoding: TextEncoding,
) -> bool {
    encoding == TextEncoding::UTF8 && !value.is_ascii()
}

fn rewrite_id3_text_as_utf16(
    value: &str,
    source_encoding: TextEncoding,
) -> Option<FieldFix> {
    should_rewrite_id3_text_as_utf16(value, source_encoding).then(|| {
        FieldFix {
            new_value: value.to_owned(),
            old_encoding: Some(text_encoding_name(source_encoding).to_owned()),
            new_encoding: Some(
                text_encoding_name(TextEncoding::UTF16).to_owned(),
            ),
        }
    })
}

fn lofty_id3v2_text_encoding(
    tag: &Tag,
    item_key: ItemKey,
) -> Option<TextEncoding> {
    let id = item_key.map_key(TagType::Id3v2)?;
    let id3v2_tag = Id3v2Tag::from(tag.clone());

    id3v2_tag.into_iter().find_map(|frame| {
        match frame {
            Frame::Text(frame) if frame.id().as_str() == id => {
                Some(frame.encoding)
            },
            Frame::UserText(frame)
                if frame.description.as_ref() == id
                    || ItemKey::from_key(
                        TagType::Id3v2,
                        &frame.description,
                    ) == Some(item_key) =>
            {
                Some(frame.encoding)
            },
            _ => None,
        }
    })
}

fn text_encoding_name(encoding: TextEncoding) -> &'static str {
    match encoding {
        TextEncoding::Latin1 => "Latin1",
        TextEncoding::UTF16 => "UTF16",
        TextEncoding::UTF16BE => "UTF16BE",
        TextEncoding::UTF8 => "UTF8",
    }
}

#[derive(Debug)]
struct FieldFix {
    new_value: String,
    old_encoding: Option<String>,
    new_encoding: Option<String>,
}

#[derive(Debug, Default)]
struct ValidationResult {
    checked_files: usize,
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

    fn print(&self) {
        if self.is_valid() {
            println!("Validated {} audio files.", self.checked_files);
            return;
        }

        if !self.read_errors.is_empty() {
            println!();
            println!("Tag encoding/read errors:");
            for error in &self.read_errors {
                println!("\t{}: {}", error.path, error.error);
            }
        }

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
    error: TFMTError,
}

#[derive(Debug)]
struct TagValueIssue {
    path: Utf8PathBuf,
    key: String,
    value: String,
    characters: Vec<&'static str>,
}

#[derive(Debug)]
struct Id3EncodingIssue {
    path: Utf8PathBuf,
    key: String,
    value: String,
    encoding: String,
}

#[cfg(test)]
mod tests {
    use lofty::TextEncoding;

    use super::{
        forbidden_characters_in, rewrite_id3_text_as_utf16,
        safe_interpolation_value, should_rewrite_id3_text_as_utf16,
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
    fn rewrites_non_ascii_utf8_text_as_utf16() {
        let fix =
            rewrite_id3_text_as_utf16("Ich Weiß Es Nicht", TextEncoding::UTF8)
                .unwrap();

        assert_eq!(fix.new_value, "Ich Weiß Es Nicht");
        assert_eq!(fix.old_encoding.as_deref(), Some("UTF8"));
        assert_eq!(fix.new_encoding.as_deref(), Some("UTF16"));
    }

    #[test]
    fn skips_ascii_id3_text() {
        assert!(
            rewrite_id3_text_as_utf16("Nothing To Fix", TextEncoding::UTF8)
                .is_none()
        );
    }

    #[test]
    fn skips_id3_text_that_is_already_utf16() {
        assert!(
            rewrite_id3_text_as_utf16("Ich Weiß Es Nicht", TextEncoding::UTF16)
                .is_none()
        );
    }

    #[test]
    fn uses_same_id3_encoding_predicate_for_check_and_fix() {
        assert!(should_rewrite_id3_text_as_utf16(
            "Ich Weiß Es Nicht",
            TextEncoding::UTF8
        ));
        assert!(!should_rewrite_id3_text_as_utf16(
            "Nothing To Fix",
            TextEncoding::UTF8
        ));
        assert!(!should_rewrite_id3_text_as_utf16(
            "Ich Weiß Es Nicht",
            TextEncoding::UTF16
        ));
    }
}
