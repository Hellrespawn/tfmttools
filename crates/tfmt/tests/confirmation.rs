use std::io::Write;
use std::process::{Command, Stdio};

use assert_fs::TempDir;

#[test]
fn piped_confirmation_requires_a_complete_affirmative_response() {
    for (input, accepts) in [
        ("y\n", true),
        (" YES \n", true),
        ("n\n", false),
        (" No \n", false),
        ("\n", false),
        ("", false),
        ("yikes\nno\n", false),
        ("notyes\nno\n", false),
        ("invalid\nyes\n", true),
    ] {
        let directory = TempDir::new().unwrap();
        let original = directory.path().join("original.mp3");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            &original,
        )
        .unwrap();
        let before = std::fs::read(&original).unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_tfmt"))
            .current_dir(directory.path())
            .args(["--simple", "-c"])
            .arg(directory.path().join("config"))
            .args(["rename", "--script", "template: (\"renamed\")"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success(), "{input:?}: {output:?}");
        let renamed = directory.path().join("renamed.mp3");
        assert_eq!(renamed.exists(), accepts, "{input:?}");
        let remaining = if accepts { &renamed } else { &original };
        assert_eq!(std::fs::read(remaining).unwrap(), before);
        if input == "yikes\nno\n" || input == "notyes\nno\n" {
            assert!(
                String::from_utf8_lossy(&output.stdout)
                    .contains("Please enter 'y' or 'n'")
            );
        }
    }
}

#[test]
fn cleanup_output_reports_effects_only_after_success() {
    for (input, dry_run, fails, cleanup) in [
        ("y\ny\n", false, false, true),
        ("y\nn\n", false, false, false),
        ("y\ny\n", true, false, false),
        ("y\ny\n", false, true, false),
    ] {
        let directory = TempDir::new().unwrap();
        let original = directory.path().join("original.mp3");
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            &original,
        )
        .unwrap();
        let before = std::fs::read(&original).unwrap();
        let leftover = directory.path().join("notes.txt");
        std::fs::write(&leftover, b"keep or archive").unwrap();
        if fails {
            std::fs::write(
                directory.path().join("blocked"),
                b"not a directory",
            )
            .unwrap();
        }
        let mut command = Command::new(env!("CARGO_BIN_EXE_tfmt"));
        command
            .current_dir(directory.path())
            .args(["--simple", "-c"])
            .arg(directory.path().join("config"));
        if dry_run {
            command.arg("--dry-run");
        }
        command.args([
            "rename",
            "--script",
            if fails {
                "template: (\"blocked\", \"renamed\")"
            } else {
                "template: (\"renamed\")"
            },
        ]);
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
        let output = child.wait_with_output().unwrap();
        assert_eq!(output.status.success(), !fails, "{output:?}");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(!stdout.contains("Deleted"), "{stdout}");
        assert!(!stdout.contains("Removed empty folders"), "{stdout}");
        assert_eq!(
            stdout.contains("Run completed."),
            !dry_run && !fails,
            "{stdout}"
        );
        assert_eq!(
            stdout.contains("Saved run #"),
            !dry_run && !fails,
            "{stdout}"
        );
        assert_eq!(!leftover.exists(), cleanup);
        if cleanup {
            let history = tfmttools_core::history::History::open_read_only(
                camino::Utf8PathBuf::try_from(
                    directory.path().join("config/tfmt.hist"),
                )
                .unwrap(),
            )
            .unwrap();
            let root = directory.path().canonicalize().unwrap();
            assert!(history.records()[0].actions().iter().any(|action| {
                matches!(action, tfmttools_core::history::StoredAction::RemoveDir { path }
                    if std::path::Path::new(path).canonicalize().ok().as_ref() == Some(&root))
            }));
            assert!(directory.path().is_dir());
        }
        let audio = if dry_run || fails {
            original
        } else {
            directory.path().join("renamed.mp3")
        };
        assert_eq!(std::fs::read(audio).unwrap(), before);
        if dry_run {
            assert!(!directory.path().join("config/tfmt.hist").exists());
        }
    }
}

#[test]
fn dry_run_image_cleanup_is_prospective_and_preserves_files() {
    let directory = TempDir::new().unwrap();
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
        directory.path().join("original.mp3"),
    )
    .unwrap();
    let cover = directory.path().join("cover.jpg");
    std::fs::write(&cover, b"cover").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(directory.path())
        .args([
            "--simple",
            "--yes",
            "--dry-run",
            "-c",
            "config",
            "rename",
            "--script",
            "template: (\"renamed\")",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would delete"), "{stdout}");
    assert!(!stdout.contains("Deleted"), "{stdout}");
    assert_eq!(std::fs::read(cover).unwrap(), b"cover");
}

#[test]
fn cleanup_preserves_relative_config_and_trash_directories() {
    for (config, trash) in [
        ("music/config", "music/trash"),
        ("music/../music/config", "music/../music/trash"),
        #[cfg(unix)]
        ("config-link", "trash-link"),
    ] {
        let directory = TempDir::new().unwrap();
        let root = directory.path();
        std::fs::create_dir_all(root.join("music/config")).unwrap();
        std::fs::create_dir_all(root.join("music/trash/old")).unwrap();
        #[cfg(unix)]
        if config == "config-link" {
            std::os::unix::fs::symlink("music/config", root.join(config))
                .unwrap();
            std::os::unix::fs::symlink("music/trash", root.join(trash))
                .unwrap();
        }
        std::fs::write(root.join("music/config/keep.jpg"), b"config").unwrap();
        std::fs::write(root.join("music/trash/old/keep.jpg"), b"trash")
            .unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            root.join("music/original.mp3"),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
            .current_dir(root)
            .args([
                "--simple",
                "--yes",
                "-c",
                config,
                "rename",
                "-i",
                "music",
                "--bin-directory",
                trash,
                "--script",
                "template: (\"renamed\")",
            ])
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            std::fs::read(root.join("music/config/keep.jpg")).unwrap(),
            b"config"
        );
        assert_eq!(
            std::fs::read(root.join("music/trash/old/keep.jpg")).unwrap(),
            b"trash"
        );
        assert!(root.join("music/config/tfmt.hist.lock").is_file());
        assert!(root.join("renamed.mp3").is_file());
    }
}

#[test]
fn cleanup_preserves_distinct_paths_and_undo_restores_them() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    std::fs::create_dir_all(root.join("music/a")).unwrap();
    for name in ["music/a/b.jpg", "music/a_b.jpg"] {
        std::fs::write(root.join(name), b"same cover").unwrap();
    }
    std::fs::copy(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
        root.join("music/original.mp3"),
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(root)
        .args([
            "--simple",
            "--yes",
            "-c",
            "config",
            "--run-id",
            "test",
            "rename",
            "-i",
            "music",
            "--script",
            "template: (\"renamed\")",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    for name in ["a/b.jpg", "a_b.jpg"] {
        assert_eq!(
            std::fs::read(root.join("config/trash/test").join(name)).unwrap(),
            b"same cover"
        );
        assert!(!root.join("music").join(name).exists());
    }
    let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
        .current_dir(root)
        .args(["--simple", "--yes", "-c", "config", "undo"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    for name in ["music/a/b.jpg", "music/a_b.jpg"] {
        assert_eq!(std::fs::read(root.join(name)).unwrap(), b"same cover");
    }
    assert!(root.join("music/original.mp3").is_file());
}

#[test]
fn cleanup_destination_collision_aborts_before_audio_moves() {
    for (relative, blocker) in
        [("cover.jpg", "cover.jpg"), ("a/cover.jpg", "a")]
    {
        let directory = TempDir::new().unwrap();
        let root = directory.path();
        std::fs::create_dir_all(root.join("music/a")).unwrap();
        std::fs::create_dir_all(root.join("config/trash/test")).unwrap();
        let blocked = root.join("config/trash/test").join(blocker);
        let cover = root.join("music").join(relative);
        std::fs::write(&blocked, b"old cover").unwrap();
        std::fs::write(&cover, b"new cover").unwrap();
        std::fs::copy(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/cli/audio/Nightwish - Nemo.mp3"),
            root.join("music/original.mp3"),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
            .current_dir(root)
            .args([
                "--simple",
                "--yes",
                "-c",
                "config",
                "--run-id",
                "test",
                "rename",
                "-i",
                "music",
                "--script",
                "template: (\"renamed\")",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{output:?}");
        assert!(root.join("music/original.mp3").is_file(), "{output:?}");
        assert!(!root.join("renamed.mp3").exists());
        assert_eq!(std::fs::read(&cover).unwrap(), b"new cover");
        assert_eq!(std::fs::read(&blocked).unwrap(), b"old cover");
        assert!(!root.join("config/tfmt.hist").exists());
    }
}
