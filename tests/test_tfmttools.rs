use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use color_eyre::Result;
use once_cell::sync::Lazy;
use std::fs;
use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};
use test_harness::test_runner;

const TEST_DATA_DIRECTORY: &str = "tests/testdata/";

static INITIAL_CONFIG_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec!["config/simple_input.tfmt", "config/typical_input.tfmt"]
        .into_iter()
        .map(normalize_separators)
        .collect()
});

static INITIAL_FILE_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        "files/Die Antwoord - Gucci Coochie (feat. Dita Von Teese).mp3",
        "files/Dune - MASTER BOOT RECORD.mp3",
        "files/SET MIDI=SYNTH1 MAPG MODE1 - MASTER BOOT RECORD.mp3",
        "files/Under Siege - Amon Amarth.mp3",
        "files/Welcome To Heaven - Damjan Mravunac.ogg",
        "files/While Your Lips Are Still Red - Nightwish.mp3",
    ]
    .into_iter()
    .map(normalize_separators)
    .collect()
});

static TYPICAL_INPUT_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
    "myname/Die Antwoord/2016 - Mount Ninji and da Nice Time Kid/05 - Gucci Coochie (feat. Dita Von Teese).mp3",
    "myname/MASTER BOOT RECORD/WAREZ/Dune.mp3",
    "myname/MASTER BOOT RECORD/2016.03 - CEDIT AUTOEXEC.BAT/05 - SET MIDI=SYNTH1 MAPG MODE1.mp3",
    "myname/Amon Amarth/2013 - Deceiver of the Gods/105 - Under Siege.mp3",
    "myname/The Talos Principle/2015 - The Talos Principle OST/01 - Damjan Mravunac - Welcome To Heaven.ogg",
    "myname/Nightwish/While Your Lips Are Still Red.mp3",
]    .into_iter()
.map(normalize_separators)
.collect()
});

struct TestEnv {
    tempdir: TempDir,
}

impl TestEnv {
    fn new() -> Result<Self> {
        let env = TestEnv {
            tempdir: TempDir::new()?,
        };

        env.populate_templates()?;
        env.populate_files()?;

        Ok(env)
    }

    fn populate_templates(&self) -> Result<()> {
        let paths: Vec<PathBuf> =
            fs::read_dir(TestEnv::get_test_data_dir().join("template"))?
                .flat_map(|r| r.map(|d| d.path()))
                .collect();

        fs::create_dir(self.get_config_dir())?;

        for template_path in &paths {
            // Templates are selected by is_file, should always have a filename
            // so path.file_name().unwrap() should be safe.

            assert!(template_path.file_name().is_some());
            let file_name = template_path.file_name().unwrap();

            fs::copy(template_path, self.get_config_dir().join(file_name))?;
        }

        Ok(())
    }

    fn populate_files(&self) -> Result<()> {
        let paths: Vec<PathBuf> =
            fs::read_dir(TestEnv::get_test_data_dir().join("music"))?
                .flat_map(|r| r.map(|d| d.path()))
                .collect();

        fs::create_dir(self.get_files_dir())?;

        for audiofile_path in &paths {
            // Audio files are selected by is_file, should always have a
            // filename so path.file_name().unwrap() should be safe.

            assert!(audiofile_path.file_name().is_some());

            fs::copy(
                audiofile_path,
                self.get_files_dir()
                    .join(audiofile_path.file_name().unwrap()),
            )?;
        }

        self.assert_files_exist(
            &INITIAL_CONFIG_REFERENCE,
            "assert initial config files exists",
        );

        self.assert_files_exist(
            &INITIAL_FILE_REFERENCE,
            "assert initial audio files exist",
        );

        Ok(())
    }

    fn get_test_data_dir() -> PathBuf {
        PathBuf::from(TEST_DATA_DIRECTORY)
    }

    fn path(&self) -> &Path {
        self.tempdir.path()
    }

    fn get_config_dir(&self) -> PathBuf {
        self.path().join("config")
    }

    fn get_files_dir(&self) -> PathBuf {
        self.path().join("files")
    }

    fn assert_files_exist(&self, reference: &[String], message: &str) {
        self.print_tempdir(message);

        for path in reference {
            let child = self.tempdir.child(path);

            assert!(child.path().exists(), "{message} failed on '{path}' ");
        }
    }

    fn assert_files_missing(&self, reference: &[String], message: &str) {
        self.print_tempdir(message);

        for path in reference {
            let child = self.tempdir.child(path);

            assert!(!child.path().exists(), "{message} failed on '{path}' ");
        }
    }

    fn print_tempdir(&self, message: &str) {
        fn inner(path: &Path, depth: usize) {
            for result in
                std::fs::read_dir(path).expect("Unable to read tempdir.")
            {
                if depth == 0 {
                    return;
                }

                if let Ok(entry) = result {
                    let path = entry.path();

                    if path.is_dir() {
                        inner(&path, depth - 1)
                    } else if path.is_file() {
                        println!("{}", path.display())
                    }
                } else {
                    continue;
                }
            }
        }

        println!("\n-- {message} --");
        inner(self.tempdir.path(), 4);
        println!("-----------\n");
    }
}

fn rename_typical_input(env: &TestEnv) {
    let config_dir = env.get_config_dir();

    let mut cmd = Command::cargo_bin("tfmt").unwrap();

    let assert = cmd
        .arg("--config")
        .arg(config_dir)
        .arg("rename")
        .arg("typical_input")
        .arg("myname")
        .current_dir(env.tempdir.path())
        .assert();

    println!("{}", String::from_utf8_lossy(&assert.get_output().stdout));

    assert.success();
}

fn undo(env: &TestEnv) {
    let config_dir = env.get_config_dir();

    let mut cmd = Command::cargo_bin("tfmt").unwrap();

    let assert = cmd
        .arg("--config")
        .arg(config_dir)
        .arg("undo")
        .current_dir(env.tempdir.path())
        .assert();

    println!("{}", String::from_utf8_lossy(&assert.get_output().stdout));

    assert.success();
}

fn redo(env: &TestEnv) {
    let config_dir = env.get_config_dir();

    let mut cmd = Command::cargo_bin("tfmt").unwrap();

    let assert = cmd
        .arg("--config")
        .arg(config_dir)
        .arg("redo")
        .current_dir(env.tempdir.path())
        .assert();

    println!("{}", String::from_utf8_lossy(&assert.get_output().stdout));

    assert.success();
}

#[test]
fn test_rename_simple_input() -> Result<()> {
    let reference: Vec<String> = vec![
        "MASTER BOOT RECORD/Dune.mp3",
        "MASTER BOOT RECORD/SET MIDI=SYNTH1 MAPG MODE1.mp3",
        "Amon Amarth/Under Siege.mp3",
        "Damjan Mravunac/Welcome To Heaven.ogg",
        "Nightwish/While Your Lips Are Still Red.mp3",
        "Die Antwoord/Gucci Coochie (feat. Dita Von Teese).mp3",
    ]
    .into_iter()
    .map(normalize_separators)
    .collect();

    test_runner(
        TestEnv::new,
        |_| Ok(()),
        |env| {
            let config_dir = env.get_config_dir();

            let mut cmd = Command::cargo_bin("tfmt").unwrap();

            let assert = cmd
                .arg("--config")
                .arg(config_dir)
                .arg("rename")
                .arg("simple_input")
                .current_dir(env.tempdir.path())
                .assert();

            println!(
                "{}",
                String::from_utf8_lossy(&assert.get_output().stdout)
            );

            assert.success();

            env.assert_files_missing(
                &INITIAL_FILE_REFERENCE,
                "assert initial files are missing",
            );
            env.assert_files_exist(&reference, "assert reference files exist");

            Ok(())
        },
    )
}

#[test]
fn test_rename_typical_input() -> Result<()> {
    test_runner(
        TestEnv::new,
        |_| Ok(()),
        |env| {
            rename_typical_input(env);

            env.assert_files_missing(
                &INITIAL_FILE_REFERENCE,
                "assert initial files are missing",
            );
            env.assert_files_exist(
                &TYPICAL_INPUT_REFERENCE,
                "assert reference files exist",
            );

            Ok(())
        },
    )
}

#[test]
fn test_undo_typical_input() -> Result<()> {
    test_runner(
        TestEnv::new,
        |_| Ok(()),
        |env| {
            rename_typical_input(env);
            env.assert_files_missing(
                &INITIAL_FILE_REFERENCE,
                "assert initial files are missing",
            );
            env.assert_files_exist(
                &TYPICAL_INPUT_REFERENCE,
                "assert reference files exist",
            );

            undo(env);
            env.assert_files_exist(
                &INITIAL_FILE_REFERENCE,
                "assert initial files have returned",
            );
            env.assert_files_missing(
                &TYPICAL_INPUT_REFERENCE,
                "assert reference files are removed",
            );

            Ok(())
        },
    )
}

#[test]
fn test_redo_typical_input() -> Result<()> {
    test_runner(
        TestEnv::new,
        |_| Ok(()),
        |env| {
            rename_typical_input(env);
            env.assert_files_missing(
                &INITIAL_FILE_REFERENCE,
                "assert initial files are missing",
            );
            env.assert_files_exist(
                &TYPICAL_INPUT_REFERENCE,
                "assert reference files exist",
            );

            undo(env);
            env.assert_files_exist(
                &INITIAL_FILE_REFERENCE,
                "assert initial files have returned",
            );
            env.assert_files_missing(
                &TYPICAL_INPUT_REFERENCE,
                "assert reference files are removed",
            );

            redo(env);
            env.assert_files_missing(
                &INITIAL_FILE_REFERENCE,
                "assert initial files are missing again",
            );
            env.assert_files_exist(
                &TYPICAL_INPUT_REFERENCE,
                "assert reference files are exist again",
            );

            Ok(())
        },
    )
}

/// Normalizes separators for the platform in `string`.
pub(crate) fn normalize_separators(string: &str) -> String {
    string.replace(
        if MAIN_SEPARATOR == '/' { '\\' } else { '/' },
        MAIN_SEPARATOR_STR,
    )
}
