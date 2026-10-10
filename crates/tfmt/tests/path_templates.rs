use std::process::{Command, Output};

use assert_fs::TempDir;
use camino::Utf8PathBuf;
use tfmttools_core::history::History;

fn root(directory: &TempDir) -> Utf8PathBuf {
    Utf8PathBuf::from_path_buf(directory.path().to_owned()).unwrap()
}

fn run(directory: &TempDir, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(directory.path())
        .args(["--simple", "--yes", "--config-directory"])
        .arg(root(directory).join("config"))
        .args(arguments)
        .output()
        .unwrap()
}

fn historical_rename() -> TempDir {
    let directory = TempDir::new().unwrap();
    let root = root(&directory);
    std::fs::create_dir(root.join("config")).unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3");
    std::fs::copy(fixture, root.join("original.mp3")).unwrap();
    let mut history = History::new(root.join("config/tfmt.hist"));
    history.load().unwrap();
    let action = tfmttools_core::action::Action::MoveFile {
        source: root.join("original.mp3"),
        target: root.join("renamed.mp3"),
    };
    let id = history
        .begin_run(
            tfmttools_core::history::OperationKind::Apply,
            None,
            Some(tfmttools_core::history::ActionRecordMetadata::new(
                tfmttools_core::history::TemplateMetadata::InlineTemplate {
                    value: "{{ artist }}/{{ title }}".into(),
                },
                vec![],
                "old-run".into(),
            )),
        )
        .unwrap();
    let mut prepared = tfmttools_fs::prepare_action(
        &action,
        tfmttools_core::history::OperationKind::Apply,
    )
    .unwrap();
    let attempt = history
        .begin_attempt(
            id,
            0,
            prepared.action(),
            prepared.details(),
            prepared.patches(),
        )
        .unwrap();
    prepared.retain_artifacts();
    prepared.execute().unwrap();
    history.confirm_attempt(attempt).unwrap();
    prepared.confirm().unwrap();
    history.close_run(id, true).unwrap();
    drop(history);
    directory
}

fn historical_rename_session() -> (TempDir, History) {
    let directory = historical_rename();
    let mut history = History::new(root(&directory).join("config/tfmt.hist"));
    history.load().unwrap();
    (directory, history)
}

#[test]
fn history_lock_blocks_commands_in_other_processes_and_releases_on_drop() {
    let (directory, history) = historical_rename_session();
    let before = std::fs::read(root(&directory).join("renamed.mp3")).unwrap();

    for arguments in
        [vec!["rename", "--script", "path: ({$title})"], vec!["undo"], vec![
            "clear-history",
        ]]
    {
        let output = run(&directory, &arguments);
        assert!(!output.status.success(), "{arguments:?}");
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains("using this history"), "{message}");
    }
    assert_eq!(
        std::fs::read(root(&directory).join("renamed.mp3")).unwrap(),
        before
    );
    assert!(root(&directory).join("config/tfmt.hist").is_file());

    drop(history);
    let output = run(&directory, &["undo"]);
    assert!(output.status.success(), "{output:?}");
    assert!(root(&directory).join("original.mp3").is_file());
}

#[test]
fn history_contention_prevents_tag_fixes() {
    let directory = TempDir::new().unwrap();
    let path = root(&directory).join("input.mp3");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../../tests/fixtures/cli/audio/Lindemann - Ich Weiß Es Nicht.mp3",
    );
    std::fs::copy(fixture, &path).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut history: History =
        History::new(root(&directory).join("config/tfmt.hist"));
    history.load().unwrap();

    let output = run(&directory, &["validate", "id3-encoding", "--fix"]);
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("using this history"), "{message}");
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("Updated"), "{stdout}");

    drop(history);
    let output = run(&directory, &["validate", "id3-encoding", "--fix"]);
    assert!(output.status.success(), "{output:?}");
    assert_ne!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn historical_jinja_reuse_requests_explicit_replacement() {
    let directory = historical_rename();
    let before = std::fs::read(root(&directory).join("renamed.mp3")).unwrap();
    let output = run(&directory, &["rename"]);
    assert!(!output.status.success());
    let message = String::from_utf8_lossy(&output.stderr);
    assert!(message.contains("migrat"), "{message}");
    assert!(message.contains("--script"), "{message}");
    assert_eq!(
        std::fs::read(root(&directory).join("renamed.mp3")).unwrap(),
        before
    );
    let output = run(&directory, &["rename", "--script", "path: ({$title})"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(root(&directory).join("Nemo.mp3")).unwrap(),
        before
    );
}

#[test]
fn historical_jinja_does_not_block_undo_and_redo() {
    let directory = historical_rename();
    let before = std::fs::read(root(&directory).join("renamed.mp3")).unwrap();
    let output = run(&directory, &["undo"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(root(&directory).join("original.mp3")).unwrap(),
        before
    );
    assert!(!root(&directory).join("renamed.mp3").exists());
    let output = run(&directory, &["redo"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(root(&directory).join("renamed.mp3")).unwrap(),
        before
    );
    assert!(!root(&directory).join("original.mp3").exists());
}

#[test]
fn template_arguments_are_validated_before_reading_audio() {
    let directory = TempDir::new().unwrap();
    std::fs::create_dir(root(&directory).join("config")).unwrap();
    let input = root(&directory).join("broken.mp3");
    std::fs::write(&input, "not audio").unwrap();
    for (template, args) in [
        (r#"arg unused: string path: ("Song")"#, vec!["bad?"]),
        (r#"arg unused: string(default: "bad?") path: ("Song")"#, vec![
            "valid",
        ]),
        (r#"path: ([$album? {$not_a_tag}] "Song")"#, vec![]),
    ] {
        let mut arguments = vec!["rename", "--script", template, "--"];
        arguments.extend(args);
        let output = run(&directory, &arguments);
        assert!(!output.status.success());
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(
            message.contains("unused") || message.contains("not_a_tag"),
            "{message}"
        );
        assert_eq!(std::fs::read_to_string(&input).unwrap(), "not audio");
        assert!(!root(&directory).join("Song.mp3").exists());
    }
}

#[test]
fn cleanup_preserves_target_with_parent_components_in_input_path() {
    let directory = TempDir::new().unwrap();
    std::fs::create_dir(root(&directory).join("config")).unwrap();
    std::fs::create_dir(root(&directory).join("input")).unwrap();
    let source = root(&directory).join("input/song.mp3");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3");
    std::fs::copy(fixture, &source).unwrap();
    let before = std::fs::read(&source).unwrap();
    let output = run(&directory, &[
        "rename",
        "-i",
        "input/../input",
        "--script",
        r#"path: ("input" / "Song")"#,
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let target = root(&directory).join("input/Song.mp3");
    assert!(target.exists(), "{}", String::from_utf8_lossy(&output.stdout));
    assert_eq!(std::fs::read(target).unwrap(), before);
}

#[test]
fn real_id3_number_pairs_supply_separate_totals() {
    use lofty::config::WriteOptions;
    use lofty::file::{AudioFile, TaggedFileExt};
    use lofty::tag::ItemKey;

    let directory = historical_rename();
    let source = root(&directory).join("renamed.mp3");
    let mut file = lofty::read_from_path(&source).unwrap();
    let tag = file.primary_tag_mut().unwrap();
    tag.insert_text(ItemKey::TrackNumber, "3".to_owned());
    tag.insert_text(ItemKey::TrackTotal, "12".to_owned());
    tag.insert_text(ItemKey::DiscNumber, "1".to_owned());
    tag.insert_text(ItemKey::DiscTotal, "2".to_owned());
    file.save_to_path(&source, WriteOptions::default()).unwrap();
    let reread = lofty::read_from_path(&source).unwrap();
    assert_eq!(
        reread.primary_tag().unwrap().get_string(ItemKey::TrackTotal),
        Some("12")
    );
    let before = std::fs::read(&source).unwrap();
    let output = run(&directory, &[
        "rename",
        "--script",
        r#"path: ({$track_number} "-" {$track_total ?? "missing"} "-" {$disc_total ?? "missing"})"#,
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let target = root(&directory).join("3-12-2.mp3");
    assert!(target.exists(), "{}", String::from_utf8_lossy(&output.stdout));
    assert_eq!(std::fs::read(target).unwrap(), before);
}

#[cfg(unix)]
#[test]
fn cleanup_resolution_failure_preserves_applied_action_history() {
    let directory = TempDir::new().unwrap();
    std::fs::create_dir(root(&directory).join("config")).unwrap();
    std::fs::create_dir(root(&directory).join("input")).unwrap();
    let source = root(&directory).join("input/song.mp3");
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3");
    std::fs::copy(fixture, &source).unwrap();
    std::os::unix::fs::symlink(
        "missing",
        root(&directory).join("input/broken-link.txt"),
    )
    .unwrap();
    let before = std::fs::read(&source).unwrap();
    let output = run(&directory, &[
        "rename",
        "-i",
        "input",
        "--script",
        r#"path: ("input" / "Song")"#,
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        std::fs::read(root(&directory).join("input/Song.mp3")).unwrap(),
        before
    );
    let output = run(&directory, &["undo"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(std::fs::read(source).unwrap(), before);
}

#[test]
fn named_template_ignores_invalid_unselected_template() {
    let directory = TempDir::new().unwrap();
    let templates = root(&directory).join("config");
    std::fs::create_dir_all(&templates).unwrap();
    std::fs::write(templates.join("selected.tfmt"), r#"path: ("Song")"#)
        .unwrap();
    std::fs::write(templates.join("broken.tfmt"), "{{ title }}").unwrap();
    let output = run(&directory, &["rename", "--template", "selected"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn duplicate_template_stems_are_ambiguous() {
    let directory = TempDir::new().unwrap();
    let templates = root(&directory).join("config");
    std::fs::create_dir_all(&templates).unwrap();
    for extension in ["tfmt", "j2"] {
        std::fs::write(
            templates.join(format!("selected.{extension}")),
            r#"path: ("Song")"#,
        )
        .unwrap();
    }
    let output = run(&directory, &["rename", "--template", "selected"]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("ambiguous"));
    let selected = templates.join("selected.tfmt");
    let output = run(&directory, &["rename", "--template", selected.as_str()]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn template_errors_use_lookup_name_instead_of_display_name() {
    let directory = historical_rename();
    let selected = root(&directory).join("config/selected.tfmt");
    for (source, expected) in [
        ("name: \"Display name\"\npath: ({$unknown_tag})", "unknown_tag"),
        (
            "name: \"Display name\"\narg required: string\npath: ({required})",
            "Missing required argument",
        ),
        (
            "name: \"Display name\"\npath: ({$title | year})",
            "Unable to extract a year",
        ),
    ] {
        std::fs::write(&selected, source).unwrap();
        let output = run(&directory, &["rename", "--template", "selected"]);
        assert!(!output.status.success());
        let message = String::from_utf8_lossy(&output.stderr);
        assert!(message.contains("Template 'selected' at 2:"), "{message}");
        assert!(message.contains(expected), "{message}");
        if expected == "Unable to extract a year" {
            assert!(message.contains("renamed.mp3"), "{message}");
        }
    }
}
