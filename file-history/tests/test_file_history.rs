use assert_fs::prelude::*;
use assert_fs::TempDir;
use color_eyre::Result;
use file_history::{Change, History};
use predicates::prelude::*;

// TODO Write undo/redo tests

const FILE_NAME: &str = "test.histfile";

#[test]
fn test_new_history_doesnt_create_file() -> Result<()> {
    let dir = TempDir::new()?;

    let history = History::load(dir.path(), FILE_NAME)?;

    assert!(!history.path().exists());

    Ok(())
}

#[test]
fn test_unchanged_history_doesnt_save() -> Result<()> {
    let dir = TempDir::new()?;

    let mut history = History::load(dir.path(), FILE_NAME)?;

    assert!(matches!(history.save(), Ok(false)));

    assert!(!history.path().exists());

    Ok(())
}

#[test]
fn test_apply_action() -> Result<()> {
    let dir = TempDir::new()?;
    let path = dir.child("testdir");

    let mut history = History::load(dir.path(), FILE_NAME)?;

    let action = Change::mkdir(&path);

    // Before: doesn't exist
    path.assert(predicate::path::missing());

    // Applied: exists
    history.apply(action)?;
    path.assert(predicate::path::exists());

    Ok(())
}

#[test]
fn test_undo_action() -> Result<()> {
    let dir = TempDir::new()?;
    let path = dir.child("testdir");

    let mut history = History::load(dir.path(), FILE_NAME)?;

    let action = Change::mkdir(&path);

    // Before: doesn't exist
    path.assert(predicate::path::missing());

    // Applied: exists
    history.apply(action)?;
    path.assert(predicate::path::exists());

    history.save()?;

    // Undone: doesn't exist
    history.undo(1)?;
    path.assert(predicate::path::missing());

    Ok(())
}

#[test]
fn test_redo_action() -> Result<()> {
    let dir = TempDir::new()?;
    let path = dir.child("testdir");

    let mut history = History::load(dir.path(), FILE_NAME)?;

    let action = Change::mkdir(&path);

    // Before: doesn't exist
    path.assert(predicate::path::missing());

    // Applied: exists
    history.apply(action)?;
    path.assert(predicate::path::exists());

    history.save()?;

    // Undone: doesn't exist
    history.undo(1)?;
    path.assert(predicate::path::missing());

    // Redone: exists
    history.redo(1)?;
    path.assert(predicate::path::exists());

    Ok(())
}

#[test]
fn test_read_write_from_disk() -> Result<()> {
    let dir = TempDir::new()?;
    let path = dir.child("testdir");

    let mut history = History::load(dir.path(), FILE_NAME)?;

    let action = Change::mkdir(&path);

    // Before: doesn't exist
    path.assert(predicate::path::missing());

    // Applied: exists
    history.apply(action)?;
    path.assert(predicate::path::exists());

    history.save()?;

    // Undone: doesn't exist
    history.undo(1)?;
    path.assert(predicate::path::missing());

    // Redone: exists
    history.redo(1)?;
    path.assert(predicate::path::exists());

    let second_history = History::load(dir.path(), FILE_NAME)?;

    assert_eq!(history, second_history);

    Ok(())
}
