use tfmttools_core::history::{
    ActionRecordMetadata, AttemptDetails, History, OperationKind, Record,
    StoredAction, TemplateMetadata,
};
pub fn metadata(run: &str) -> ActionRecordMetadata {
    ActionRecordMetadata::new(
        TemplateMetadata::Validation { value: "test".into() },
        vec![],
        run.into(),
    )
}
pub fn apply(
    history: &mut History,
    actions: Vec<StoredAction>,
    metadata: ActionRecordMetadata,
) -> Record {
    let id =
        history.begin_run(OperationKind::Apply, None, Some(metadata)).unwrap();
    for (pos, action) in actions.into_iter().enumerate() {
        let a = history
            .begin_attempt(
                id,
                pos,
                &action,
                &AttemptDetails {
                    paths: vec!["test".into()],
                    instructions: "Inspect test action".into(),
                },
                None,
            )
            .unwrap();
        history.confirm_attempt(a).unwrap();
    }
    history.close_run(id, true).unwrap()
}
pub fn replay(
    history: &mut History,
    record_id: usize,
    kind: OperationKind,
) -> Record {
    let record = history
        .records()
        .iter()
        .find(|r| r.id() == Some(record_id))
        .unwrap()
        .clone();
    let positions: Vec<_> = if kind == OperationKind::Undo {
        (0..record.applied_count()).rev().collect()
    } else {
        (record.applied_count()..record.len()).collect()
    };
    let id = history.begin_run(kind, Some(record_id), None).unwrap();
    for pos in positions {
        let a = history
            .begin_attempt(
                id,
                pos,
                &record.actions()[pos],
                &AttemptDetails {
                    paths: vec!["test".into()],
                    instructions: "Inspect test action".into(),
                },
                None,
            )
            .unwrap();
        history.confirm_attempt(a).unwrap();
    }
    history.close_run(id, true).unwrap()
}
