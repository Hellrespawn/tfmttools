use std::path::{Path, PathBuf, MAIN_SEPARATOR, MAIN_SEPARATOR_STR};

use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use color_eyre::Result;
use fs_err as fs;
use once_cell::sync::Lazy;

const TEST_DATA_DIRECTORY: &str = "tests/testdata/";

static INITIAL_CONFIG_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec!["config/simple_input.tfmt", "config/typical_input.tfmt"]
        .into_iter()
        .map(normalize_separators)
        .collect()
});

static INITIAL_FILE_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        "files/Amon Amarth - Under Siege.mp3",
        "files/Damjan Mravunac - Welcome To Heaven.ogg",
        "files/Die Antwoord - Gucci Coochie (feat. Dita Von Teese).mp3",
        "files/MASTER BOOT RECORD - Dune.mp3",
        "files/MASTER BOOT RECORD - RAMDRIVE.SYS.mp3",
        "files/MASTER BOOT RECORD - SET MIDI=SYNTH1 MAPG MODE1.mp3",
        "files/Nightwish - Elvenpath (Live).mp3",
        "files/Nightwish - Nemo.mp3",
        "files/Nightwish - While Your Lips Are Still Red.mp3",
    ]
    .into_iter()
    .map(normalize_separators)
    .collect()
});

static SIMPLE_INPUT_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
        "Amon Amarth/Under Siege.mp3",
        "Damjan Mravunac/Welcome To Heaven.ogg",
        "Die Antwoord/Gucci Coochie (feat. Dita Von Teese).mp3",
        "MASTER BOOT RECORD/Dune.mp3",
        "MASTER BOOT RECORD/MYTH.NFO.mp3",
        "MASTER BOOT RECORD/RAMDRIVE.SYS.mp3",
        "MASTER BOOT RECORD/SET MIDI=SYNTH1 MAPG MODE1.mp3",
        "Nightwish/Elvenpath (Live).mp3",
        "Nightwish/Nemo.mp3",
        "Nightwish/While Your Lips Are Still Red.mp3",
    ]
    .into_iter()
    .map(normalize_separators)
    .collect()
});

static TYPICAL_INPUT_REFERENCE: Lazy<Vec<String>> = Lazy::new(|| {
    vec![
    "output_dir/Die Antwoord/2016 - Mount Ninji and da Nice Time Kid/05 - Gucci Coochie (feat. Dita Von Teese).mp3",
    "output_dir/MASTER BOOT RECORD/WAREZ/Dune.mp3",
    "output_dir/MASTER BOOT RECORD/2016.03 - C-EDIT AUTOEXEC.BAT/05 - SET MIDI=SYNTH1 MAPG MODE1.mp3",
    "output_dir/MASTER BOOT RECORD/2017.01 - C-COPY . A -V/04 - MYTH.NFO.mp3",
    "output_dir/MASTER BOOT RECORD/2020.01 - FLOPPY DISK OVERDRIVE/07 - RAMDRIVE.SYS.mp3",
    "output_dir/Amon Amarth/2013 - Deceiver of the Gods/105 - Under Siege.mp3",
    "output_dir/The Talos Principle/2015 - The Talos Principle OST/01 - Damjan Mravunac - Welcome To Heaven.ogg",
    "output_dir/Nightwish/2004 - Once/03 - Nemo.mp3",
    "output_dir/Nightwish/2019 - Decades Live in Buenos Aires/12 - Elvenpath (Live).mp3",
    "output_dir/Nightwish/While Your Lips Are Still Red.mp3",
]    .into_iter()
.map(normalize_separators)
.collect()
});

struct TestEnv {
    tempdir: TempDir,
}

impl TestEnv {
    fn new() -> Result<Self> {
        let env = TestEnv { tempdir: TempDir::new()? };

        env.populate_templates()?;
        env.populate_files()?;

        Ok(env)
    }

    fn populate_templates(&self) -> Result<()> {
        let paths: Vec<PathBuf> =
            fs::read_dir(TestEnv::get_test_data_dir().join("template"))?
                .flat_map(|result| result.map(|entry| entry.path()))
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
                .flat_map(|result| result.map(|entry| entry.path()))
                .collect();

        fs::create_dir(self.get_files_dir())?;

        for audiofile_path in &paths {
            // Audio files are selected by is_file, should always have a
            // filename so path.file_name().unwrap() should be safe.

            assert!(audiofile_path.file_name().is_some());

            fs::copy(
                audiofile_path,
                self.get_files_dir().join(audiofile_path.file_name().unwrap()),
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

    fn get_template_dir(&self) -> PathBuf {
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
            for result in fs::read_dir(path).expect("Unable to read tempdir.") {
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

    fn add_default_rename_args(&self, cmd: &mut Command) {
        cmd.arg("--config")
            .arg(self.get_config_dir())
            .arg("rename")
            .arg("--template-directory")
            .arg(self.get_template_dir())
            .arg("--force");
    }
}

fn rename_typical_input(env: &TestEnv) {
    let mut cmd = Command::cargo_bin("tfmt").unwrap();

    env.add_default_rename_args(&mut cmd);

    let assert = cmd
        .arg("typical_input")
        .arg("output_dir/")
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
        .arg("--force")
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
        .arg("--force")
        .current_dir(env.tempdir.path())
        .assert();

    println!("{}", String::from_utf8_lossy(&assert.get_output().stdout));

    assert.success();
}

#[test]
fn test_rename_simple_input() -> Result<()> {
    let env = TestEnv::new()?;

    let mut cmd = Command::cargo_bin("tfmt").unwrap();

    env.add_default_rename_args(&mut cmd);

    let assert =
        cmd.arg("simple_input").current_dir(env.tempdir.path()).assert();

    println!("{}", String::from_utf8_lossy(&assert.get_output().stdout));

    assert.success();

    env.assert_files_missing(
        &INITIAL_FILE_REFERENCE,
        "assert initial files are missing",
    );
    env.assert_files_exist(
        &SIMPLE_INPUT_REFERENCE,
        "assert reference files exist",
    );

    Ok(())
}

#[test]
fn test_rename_typical_input() -> Result<()> {
    let env = TestEnv::new()?;

    rename_typical_input(&env);

    env.assert_files_missing(
        &INITIAL_FILE_REFERENCE,
        "assert initial files are missing",
    );
    env.assert_files_exist(
        &TYPICAL_INPUT_REFERENCE,
        "assert reference files exist",
    );

    Ok(())
}

#[test]
fn test_undo_typical_input() -> Result<()> {
    let env = TestEnv::new()?;

    rename_typical_input(&env);
    env.assert_files_missing(
        &INITIAL_FILE_REFERENCE,
        "assert initial files are missing",
    );
    env.assert_files_exist(
        &TYPICAL_INPUT_REFERENCE,
        "assert reference files exist",
    );

    undo(&env);
    env.assert_files_exist(
        &INITIAL_FILE_REFERENCE,
        "assert initial files have returned",
    );
    env.assert_files_missing(
        &TYPICAL_INPUT_REFERENCE,
        "assert reference files are removed",
    );

    Ok(())
}

#[test]
fn test_redo_typical_input() -> Result<()> {
    let env = TestEnv::new()?;

    rename_typical_input(&env);
    env.assert_files_missing(
        &INITIAL_FILE_REFERENCE,
        "assert initial files are missing",
    );
    env.assert_files_exist(
        &TYPICAL_INPUT_REFERENCE,
        "assert reference files exist",
    );

    undo(&env);
    env.assert_files_exist(
        &INITIAL_FILE_REFERENCE,
        "assert initial files have returned",
    );
    env.assert_files_missing(
        &TYPICAL_INPUT_REFERENCE,
        "assert reference files are removed",
    );

    redo(&env);
    env.assert_files_missing(
        &INITIAL_FILE_REFERENCE,
        "assert initial files are missing again",
    );
    env.assert_files_exist(
        &TYPICAL_INPUT_REFERENCE,
        "assert reference files are exist again",
    );

    Ok(())
}

/// Normalizes separators for the platform in `string`.
pub fn normalize_separators(string: &str) -> String {
    string.replace(
        if MAIN_SEPARATOR == '/' { '\\' } else { '/' },
        MAIN_SEPARATOR_STR,
    )
}
