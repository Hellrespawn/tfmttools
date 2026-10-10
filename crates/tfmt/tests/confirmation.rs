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
