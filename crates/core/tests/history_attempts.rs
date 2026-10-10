mod support;
use camino::Utf8PathBuf;
use tfmttools_core::history::{
    AttemptDetails, AttemptOutcome, History, StoredAction,
};

fn setup() -> (tempfile::TempDir, History) {
    let dir = tempfile::tempdir().unwrap();
    let h = History::new(
        Utf8PathBuf::try_from(dir.path().join("history")).unwrap(),
    );
    (dir, h)
}
fn action(name: &str) -> StoredAction {
    StoredAction::MakeDir { path: name.into() }
}
fn attempt(
    h: &mut History,
    run: tfmttools_core::history::RunId,
    pos: usize,
    name: &str,
) -> tfmttools_core::history::AttemptId {
    h.begin_attempt(
        run,
        pos,
        &action(name),
        &AttemptDetails {
            paths: vec![name.into()],
            instructions: "Check directory".into(),
        },
        None,
    )
    .unwrap()
}
#[test]
fn confirmed_prefix_survives_error_and_not_applied_resolution() {
    let (_dir, mut h) = setup();
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("partial"),
        })
        .unwrap();
    for (pos, name) in ["a", "b"].iter().enumerate() {
        let id = attempt(&mut h, run, pos, name);
        h.confirm_attempt(id).unwrap();
    }
    let failed = attempt(&mut h, run, 2, "c");
    h.load().unwrap();
    assert_eq!(h.records()[0].len(), 2);
    assert!(
        h.begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("blocked")
        })
        .is_err()
    );
    let r = h.resolve_attempt(failed, AttemptOutcome::NotApplied).unwrap();
    assert_eq!(r.applied_count(), 2);
    assert!(!r.is_complete());
    assert!(!r.redo_allowed());
    assert!(h.current_attempt().unwrap().is_none());
    assert_eq!(
        h.records_for(tfmttools_core::history::HistoryMode::Undo, None).len(),
        1
    );
}
#[test]
fn manual_applied_resolution_and_partial_undo_use_cursor() {
    let (_dir, mut h) = setup();
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("apply"),
        })
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    let r = h.resolve_attempt(id, AttemptOutcome::Applied).unwrap();
    assert_eq!(r.len(), 1);
    let undo = h
        .begin_run(tfmttools_core::history::RunRequest::Replay {
            record_id: r.id().unwrap(),
            direction: tfmttools_core::history::HistoryMode::Undo,
        })
        .unwrap();
    let id = attempt(&mut h, undo, 0, "a");
    h.resolve_attempt(id, AttemptOutcome::Applied).unwrap();
    assert_eq!(h.records()[0].applied_count(), 0);
    assert!(
        h.records_for(tfmttools_core::history::HistoryMode::Redo, None)
            .is_empty()
    );
}
#[test]
fn abandoned_run_closes_without_actions_or_attempt() {
    let (_dir, mut h) = setup();
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("abandoned"),
        })
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    h.confirm_attempt(id).unwrap();
    h.load().unwrap();
    let r = h.close_abandoned_run().unwrap().unwrap();
    assert_eq!(r.applied_count(), 1);
    assert!(!r.is_complete());
    assert!(h.close_abandoned_run().unwrap().is_none());
}
#[test]
fn failed_confirmation_is_atomic_and_stale_attempts_cannot_resolve_new_work() {
    let (dir, mut h) = setup();
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("failure"),
        })
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    let c = rusqlite::Connection::open(dir.path().join("history")).unwrap();
    c.execute_batch("CREATE TRIGGER fail_confirmation BEFORE DELETE ON attempts BEGIN SELECT RAISE(ABORT,'injected'); END;").unwrap();
    assert!(h.confirm_attempt(id).is_err());
    assert!(h.records()[0].is_empty());
    assert_eq!(h.current_attempt().unwrap().unwrap().id, id);
    c.execute_batch("DROP TRIGGER fail_confirmation").unwrap();
    h.resolve_attempt(id, AttemptOutcome::NotApplied).unwrap();
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("next"),
        })
        .unwrap();
    let next = attempt(&mut h, run, 0, "b");
    assert_ne!(id, next);
    assert!(h.resolve_attempt(id, AttemptOutcome::Applied).is_err());
    assert_eq!(h.current_attempt().unwrap().unwrap().id, next);
}
#[test]
fn partial_undo_remains_selectable_and_failed_replay_disables_redo() {
    let (_dir, mut h) = setup();
    let r = support::history::apply(
        &mut h,
        vec![action("a"), action("b")],
        support::history::metadata("full"),
    );
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Replay {
            record_id: r.id().unwrap(),
            direction: tfmttools_core::history::HistoryMode::Undo,
        })
        .unwrap();
    let id = attempt(&mut h, run, 1, "b");
    h.confirm_attempt(id).unwrap();
    let r = h.close_run(run, false).unwrap();
    assert_eq!(r.applied_count(), 1);
    assert_eq!(
        h.records_for(tfmttools_core::history::HistoryMode::Undo, None)[0]
            .applied_count(),
        1
    );
    assert!(
        h.records_for(tfmttools_core::history::HistoryMode::Redo, None)
            .is_empty()
    );
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Replay {
            record_id: r.id().unwrap(),
            direction: tfmttools_core::history::HistoryMode::Undo,
        })
        .unwrap();
    let id = attempt(&mut h, run, 0, "a");
    h.confirm_attempt(id).unwrap();
    h.close_run(run, true).unwrap();
    assert!(
        h.records_for(tfmttools_core::history::HistoryMode::Undo, None)
            .is_empty()
    );
    assert!(
        h.records_for(tfmttools_core::history::HistoryMode::Redo, None)
            .is_empty()
    );
}
#[test]
fn loading_rejects_attempt_that_disagrees_with_record_cursor() {
    let (dir, mut h) = setup();
    let run = h
        .begin_run(tfmttools_core::history::RunRequest::Apply {
            metadata: support::history::metadata("tampered"),
        })
        .unwrap();
    attempt(&mut h, run, 0, "a");
    drop(h);
    let path = Utf8PathBuf::try_from(dir.path().join("history")).unwrap();
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute("UPDATE attempts SET action_position=3", []).unwrap();
    drop(c);
    assert!(History::new(path).load().is_err());
}
#[test]
fn resolving_unknown_attempt_does_not_create_history() {
    let (dir, mut h) = setup();
    assert!(
        h.resolve_attempt(
            tfmttools_core::history::AttemptId(42),
            AttemptOutcome::NotApplied
        )
        .is_err()
    );
    assert!(!dir.path().join("history").exists());
}

#[test]
fn attempt_details_are_checked_before_insertion_and_when_loaded() {
    for details in [
        AttemptDetails { paths: vec![], instructions: "Inspect".into() },
        AttemptDetails {
            paths: vec!["".into()],
            instructions: "Inspect".into(),
        },
        AttemptDetails { paths: vec!["a".into()], instructions: "".into() },
    ] {
        let (dir, mut h) = setup();
        let run = h
            .begin_run(tfmttools_core::history::RunRequest::Apply {
                metadata: support::history::metadata("details"),
            })
            .unwrap();
        assert!(h.begin_attempt(run, 0, &action("a"), &details, None).is_err());
        assert!(h.current_attempt().unwrap().is_none());
        attempt(&mut h, run, 0, "a");
        let db =
            rusqlite::Connection::open(dir.path().join("history")).unwrap();
        db.execute("UPDATE attempts SET details=?1", [serde_json::to_string(
            &details,
        )
        .unwrap()])
            .unwrap();
        assert!(h.load().is_err());
    }
}

#[test]
fn invalid_patch_requests_are_mutation_errors() {
    use tfmttools_core::history::{
        BinaryPatchPair, ByteIdentity, HistoryError,
    };
    let tag: StoredAction = serde_json::from_value(serde_json::json!({
        "type":"edit_tag_values", "path":"song.mp3", "changes":[{
            "key":"track_title", "kind":"text", "old_value":"a", "new_value":"b"
        }]
    }))
    .unwrap();
    let valid = BinaryPatchPair {
        format: "bsdiff40-v1".into(),
        before: ByteIdentity { length: 1, sha256: [1; 32] },
        after: ByteIdentity { length: 2, sha256: [2; 32] },
        forward: [b"BSDIFF40".as_slice(), &[0; 24]].concat(),
        reverse: [b"BSDIFF40".as_slice(), &[0; 24]].concat(),
    };
    for case in 0..5 {
        let (_dir, mut h) = setup();
        let run = h
            .begin_run(tfmttools_core::history::RunRequest::Apply {
                metadata: support::history::metadata("patch"),
            })
            .unwrap();
        let mut pair = valid.clone();
        match case {
            0 => pair.format = "unknown".into(),
            1 => pair.forward.truncate(8),
            2 => pair.reverse[0] = 0,
            3 => pair.before.length = u64::MAX,
            _ => (),
        }
        let selected = if case == 4 { action("dir") } else { tag.clone() };
        let result = h.begin_attempt(
            run,
            0,
            &selected,
            &AttemptDetails {
                paths: vec!["path".into()],
                instructions: "Inspect".into(),
            },
            Some(&pair),
        );
        if case == 3 {
            assert!(
                matches!(result, Err(HistoryError::SaveError(_))),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(HistoryError::MutError(_))),
                "{result:?}"
            );
        }
        assert!(h.current_attempt().unwrap().is_none());
    }
}

#[test]
fn malformed_persisted_patches_are_load_errors_with_identity() {
    use tfmttools_core::history::{BinaryPatchPair, ByteIdentity};
    let tag: StoredAction = serde_json::from_value(serde_json::json!({
        "type":"edit_tag_values", "path":"song.mp3", "changes":[{
            "key":"track_title", "kind":"text", "old_value":"a", "new_value":"b"
        }]
    }))
    .unwrap();
    let valid = BinaryPatchPair {
        format: "bsdiff40-v1".into(),
        before: ByteIdentity { length: 1, sha256: [1; 32] },
        after: ByteIdentity { length: 2, sha256: [2; 32] },
        forward: [b"BSDIFF40".as_slice(), &[0; 24]].concat(),
        reverse: [b"BSDIFF40".as_slice(), &[0; 24]].concat(),
    };

    for attempted in [true, false] {
        for corruption in [
            "SET format='unknown'",
            "SET forward=X'00'",
            "SET reverse=zeroblob(32)",
            "SET before_length=-1",
            "SET after_hash=X'01'",
            "delete",
            "non_tag",
        ] {
            let (dir, mut h) = setup();
            let run = h
                .begin_run(tfmttools_core::history::RunRequest::Apply {
                    metadata: support::history::metadata("stored-patch"),
                })
                .unwrap();
            let id = h
                .begin_attempt(
                    run,
                    0,
                    &tag,
                    &AttemptDetails {
                        paths: vec!["song.mp3".into()],
                        instructions: "Inspect".into(),
                    },
                    Some(&valid),
                )
                .unwrap();
            if !attempted {
                h.confirm_attempt(id).unwrap();
                h.close_run(run, true).unwrap();
            }
            let db =
                rusqlite::Connection::open(dir.path().join("history")).unwrap();
            db.execute_batch("PRAGMA ignore_check_constraints=ON").unwrap();
            let table = if attempted { "attempt_patches" } else { "patches" };
            let sql = match corruption {
                "delete" => format!("DELETE FROM {table}"),
                "non_tag" => {
                    format!(
                        "UPDATE {} SET payload='{{\"type\":\"make_dir\",\"path\":\"dir\"}}'",
                        if attempted { "attempts" } else { "actions" }
                    )
                },
                _ => format!("UPDATE {table} {corruption}"),
            };
            db.execute_batch(&sql).unwrap();
            let error = h.load().unwrap_err();
            assert!(
                matches!(
                    error,
                    tfmttools_core::history::HistoryError::LoadError(_)
                ),
                "{corruption}: {error:?}"
            );
            if matches!(
                corruption,
                "SET forward=X'00'"
                    | "SET reverse=zeroblob(32)"
                    | "delete"
                    | "non_tag"
            ) {
                assert!(
                    error.to_string().contains(if attempted {
                        "Attempt"
                    } else {
                        "Record"
                    }),
                    "{error}"
                );
            }
        }
    }
}
