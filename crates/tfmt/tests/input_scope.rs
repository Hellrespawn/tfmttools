use std::process::Command;

use assert_fs::TempDir;

#[test]
fn commands_find_the_same_input_through_relative_and_absolute_paths() {
    let dir = TempDir::new().unwrap();
    let input = dir.path().join("input with spaces");
    std::fs::create_dir(&input).unwrap();
    std::fs::write(input.join("broken.mp3"), b"invalid audio").unwrap();
    let mut scopes =
        vec![std::path::PathBuf::from("input with spaces"), input.clone()];
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
