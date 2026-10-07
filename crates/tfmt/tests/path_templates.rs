use std::process::{Command, Output};

use assert_fs::TempDir;
use camino::Utf8PathBuf;
use tfmttools_core::action::Action;
use tfmttools_core::history::{ActionRecordMetadata, TemplateMetadata};
use tfmttools_history::History;

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
    std::fs::copy(fixture, root.join("renamed.mp3")).unwrap();
    let mut history = History::new(root.join("config/tfmt.hist"));
    history
        .push(
            vec![Action::MoveFile {
                source: root.join("original.mp3"),
                target: root.join("renamed.mp3"),
            }],
            ActionRecordMetadata::new(
                TemplateMetadata::Script("{{ artist }}/{{ title }}".to_owned()),
                Vec::new(),
                "old-run".to_owned(),
            ),
        )
        .unwrap();
    history.save().unwrap();
    directory
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
fn script_arguments_are_validated_before_reading_audio() {
    let directory = TempDir::new().unwrap();
    std::fs::create_dir(root(&directory).join("config")).unwrap();
    let input = root(&directory).join("broken.mp3");
    std::fs::write(&input, "not audio").unwrap();
    for (script, args) in [
        (r#"arg unused: string path: ("Song")"#, vec!["bad?"]),
        (r#"arg unused: string(default: "bad?") path: ("Song")"#, vec![
            "valid",
        ]),
        (r#"path: ([$album? {$not_a_tag}] "Song")"#, vec![]),
    ] {
        let mut arguments = vec!["rename", "--script", script, "--"];
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
        r#"path: ({$tracknumber} "-" {$tracktotal ?? "missing"} "-" {$disctotal ?? "missing"})"#,
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
