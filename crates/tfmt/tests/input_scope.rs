use std::process::Command;

use assert_fs::TempDir;

#[test]
fn commands_find_the_same_input_through_relative_and_absolute_paths() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("input with spaces");
    std::fs::create_dir(&input).unwrap();
    std::fs::write(input.join("broken.mp3"), b"invalid audio").unwrap();
    let mut scopes = vec![
        std::path::PathBuf::from("input with spaces"),
        std::path::PathBuf::from("./input with spaces"),
        input.clone(),
    ];
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&input, dir.path().join("linked input"))
            .unwrap();
        scopes.push("linked input".into());
    }
    for scope in scopes {
        for command in [vec!["validate", "characters"], vec![
            "rename",
            "--script",
            "path: (\"renamed\")",
        ]] {
            let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
                .current_dir(dir.path())
                .args(["--simple", "--yes", "-c", "config"])
                .arg(command[0])
                .arg("-i")
                .arg(&scope)
                .args(&command[1..])
                .output()
                .unwrap();
            assert!(!output.status.success(), "{output:?}");
            let report = format!(
                "{}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(report.contains("broken.mp3"), "{report}");
            if command[0] == "validate" {
                assert!(
                    report.contains(&format!("{}/broken.mp3", scope.display())),
                    "{report}"
                );
                if scope.is_relative() {
                    assert!(
                        !report.contains(&dir.path().display().to_string()),
                        "{report}"
                    );
                }
            }
        }
    }
}

#[test]
fn missing_relative_directory_retains_its_supplied_diagnostic_scope() {
    let dir = TempDir::new().unwrap();
    for scope in ["missing", "./missing"] {
        let output = Command::new(env!("CARGO_BIN_EXE_tfmt"))
            .current_dir(dir.path())
            .args([
                "--simple",
                "-c",
                "config",
                "validate",
                "-i",
                scope,
                "characters",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success(), "{output:?}");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains(&format!(
                "Unable to discover audio files in {scope}"
            )),
            "{stderr}"
        );
        assert!(
            !stderr.contains(&dir.path().display().to_string()),
            "{stderr}"
        );
        assert!(stderr.contains("os error"), "{stderr}");
    }
}

#[test]
fn recursion_depth_is_limited_to_256() {
    let dir = TempDir::new().unwrap();
    for command in ["rename", "validate"] {
        for depth in ["0", "256", "257", "18446744073709551615"] {
            let mut invocation = Command::new(env!("CARGO_BIN_EXE_tfmt"));
            invocation.current_dir(dir.path()).args([
                "--simple",
                "--dry-run",
                "-c",
                "config",
                command,
                "--recursion-depth",
                depth,
            ]);
            if command == "rename" {
                invocation.args(["--script", "path: (\"renamed\")"]);
            }
            let output = invocation.output().unwrap();
            let accepted = matches!(depth, "0" | "256");
            assert_eq!(output.status.success(), accepted, "{output:?}");
            if !accepted {
                assert_eq!(output.status.code(), Some(1), "{output:?}");
                assert!(
                    String::from_utf8_lossy(&output.stderr)
                        .contains("Recursion depth must not exceed 256"),
                    "{output:?}"
                );
            }
        }
    }
}
