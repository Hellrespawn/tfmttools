use tfmttools_core::history::HistoryMode;
use tfmttools_fs::{apply_patch, create_patch_pair};

#[test]
fn patches_restore_exact_bytes_in_both_directions() {
    for (before, after) in [
        (b"".as_slice(), b"abc\0xyz".as_slice()),
        (b"old\0bytes".as_slice(), b"new\0bytes\xff".as_slice()),
        (b"abc".as_slice(), b"".as_slice()),
    ] {
        let pair = create_patch_pair(before, after).unwrap();
        assert_eq!(
            apply_patch(before, &pair, HistoryMode::Redo).unwrap(),
            after
        );
        assert_eq!(
            apply_patch(after, &pair, HistoryMode::Undo).unwrap(),
            before
        );
        assert!(pair.forward.starts_with(b"BSDIFF40"));
        assert!(pair.reverse.starts_with(b"BSDIFF40"));
    }
}
#[test]
fn rejects_changed_input_and_corrupt_patches() {
    let pair = create_patch_pair(b"original", b"changed").unwrap();
    assert!(apply_patch(b"external", &pair, HistoryMode::Redo).is_err());
    assert!(apply_patch(b"external", &pair, HistoryMode::Undo).is_err());
    let mut bad = pair.clone();
    bad.format = "unknown".into();
    assert!(apply_patch(b"original", &bad, HistoryMode::Redo).is_err());
    let mut bad = pair.clone();
    bad.forward.truncate(31);
    assert!(apply_patch(b"original", &bad, HistoryMode::Redo).is_err());
    let mut bad = pair.clone();
    bad.after.length += 1;
    assert!(apply_patch(b"original", &bad, HistoryMode::Redo).is_err());
    let mut bad = pair.clone();
    bad.before.sha256[0] ^= 1;
    assert!(apply_patch(b"original", &bad, HistoryMode::Redo).is_err());
}
