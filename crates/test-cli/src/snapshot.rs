use camino::Utf8Path;
use snapbox::assert::Action;
use snapbox::data::DataFormat;
use snapbox::{Assert, Data, Redactions};

use crate::case::TestContext;

fn update_action(ci: bool, requested: Option<&str>) -> Action {
    if !ci && requested == Some("overwrite") {
        Action::Overwrite
    } else {
        Action::Verify
    }
}

pub(crate) fn compare_stream(
    actual: &[u8],
    expected: &Utf8Path,
    context: &TestContext,
) -> Result<(), String> {
    compare_with_action(
        actual,
        expected,
        context,
        update_action(
            std::env::var_os("CI").is_some(),
            std::env::var("TFMT_SNAPSHOTS").ok().as_deref(),
        ),
    )
}

fn compare_with_action(
    actual: &[u8],
    expected: &Utf8Path,
    context: &TestContext,
    action: Action,
) -> Result<(), String> {
    let expected =
        Data::try_read_from(expected.as_std_path(), Some(DataFormat::Text))
            .map_err(|e| e.to_string())?;
    let mut redactions = Redactions::new();
    redactions
        .insert("[CONFIG]", context.config_work_dir().into_std_path_buf())
        .map_err(|e| e.to_string())?;
    redactions
        .insert("[WORK]", context.work_dir_path().into_std_path_buf())
        .map_err(|e| e.to_string())?;
    Assert::new()
        .action(action)
        .redact_with(redactions)
        .try_eq(Some(&"command output"), Data::binary(actual), expected)
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::case::TestContext;

    #[test]
    fn updates_are_explicit_and_never_enabled_in_ci() {
        let context = TestContext::new().unwrap();
        let path = context.work_dir_path().join("expected.stdout");
        std::fs::write(&path, "old\n").unwrap();
        for action in [
            update_action(false, None),
            update_action(true, Some("overwrite")),
            update_action(false, Some("ignore")),
        ] {
            assert!(
                compare_with_action(b"new\n", &path, &context, action).is_err()
            );
            assert_eq!(std::fs::read_to_string(&path).unwrap(), "old\n");
        }
        assert!(
            compare_with_action(
                b"new\n",
                &path,
                &context,
                update_action(false, Some("overwrite"))
            )
            .is_ok()
        );
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new\n");
    }

    #[test]
    fn comparisons_normalize_only_expected_dynamic_values() {
        let context = TestContext::new().unwrap();
        let path = context.work_dir_path().join("expected.stdout");
        std::fs::write(&path, "[WORK]/song.mp3\n").unwrap();
        let actual = format!("{}/song.mp3\r\n", context.work_dir_path());
        assert!(compare_stream(actual.as_bytes(), &path, &context).is_ok());
        assert!(
            compare_stream(b"[WORK]/wrong.mp3\n", &path, &context).is_err()
        );
        assert!(
            compare_stream(b" [WORK]/song.mp3\n", &path, &context).is_err()
        );
        std::fs::write(&path, "").unwrap();
        assert!(compare_stream(b"", &path, &context).is_ok());
        assert!(compare_stream(b"\n", &path, &context).is_err());
        std::fs::remove_file(&path).unwrap();
        assert!(compare_stream(b"", &path, &context).is_err());
    }
}
