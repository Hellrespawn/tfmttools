use color_eyre::Result;
use tfmttools_test::TestCase;

#[test]
fn test_rename_simple_input() -> Result<()> {
    let case = TestCase::load("simple_input")?;

    case.assert_apply(true);

    Ok(())
}

#[test]
fn test_rename_undo_simple_input() -> Result<()> {
    let case = TestCase::load("simple_input")?;

    case.assert_apply(false);
    case.assert_undo(true);

    Ok(())
}

#[test]
fn test_rename_redo_simple_input() -> Result<()> {
    let case = TestCase::load("simple_input")?;

    case.assert_apply(false);
    case.assert_undo(false);
    case.assert_redo(true);

    Ok(())
}

#[test]
fn test_rename_typical_input() -> Result<()> {
    let case = TestCase::load("typical_input")?;

    case.assert_apply(true);

    Ok(())
}

#[test]
fn test_rename_undo_typical_input() -> Result<()> {
    let case = TestCase::load("typical_input")?;

    case.assert_apply(false);
    case.assert_undo(true);

    Ok(())
}

#[test]
fn test_rename_redo_typical_input() -> Result<()> {
    let case = TestCase::load("typical_input")?;

    case.assert_apply(false);
    case.assert_undo(false);
    case.assert_redo(true);

    Ok(())
}
