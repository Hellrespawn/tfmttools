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
            .args(["rename", "--script", "path: (\"renamed\")"])
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
                "path: (\"blocked\", \"renamed\")"
            } else {
                "path: (\"renamed\")"
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
            "path: (\"renamed\")",
        ])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Would delete"), "{stdout}");
    assert!(!stdout.contains("Deleted"), "{stdout}");
    assert_eq!(std::fs::read(cover).unwrap(), b"cover");
}
