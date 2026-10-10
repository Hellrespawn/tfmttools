use tfmttools_core::history::{
    ActionRecordMetadata, History, OperationId, OperationKind, PreparedAction,
    Record, RecoveryDescriptor, StoredAction, TemplateMetadata,
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
    let id = history
        .begin_operation(OperationKind::Apply, None, Some(metadata))
        .unwrap();
    finish(history, id, OperationKind::Apply, actions)
}

pub fn replay(
    history: &mut History,
    record_id: usize,
    kind: OperationKind,
) -> Record {
    let record =
        history.records().iter().find(|r| r.id() == Some(record_id)).unwrap();
    let actions = if kind == OperationKind::Undo {
        record.iter().rev().cloned().collect()
    } else {
        record.iter().cloned().collect()
    };
    let id = history.begin_operation(kind, Some(record_id), None).unwrap();
    finish(history, id, kind, actions)
}

fn finish(
    history: &mut History,
    id: OperationId,
    kind: OperationKind,
    actions: Vec<StoredAction>,
) -> Record {
    history.set_operation_plan(id, &actions).unwrap();
    // Core tests exercise journal persistence with directory descriptors.
    // Filesystem installation and recovery are covered in the fs crate.
    for action in actions {
        let (path, creates) = match &action {
            StoredAction::MakeDir { path } => {
                (path.clone(), kind != OperationKind::Undo)
            },
            StoredAction::RemoveDir { path } => {
                (path.clone(), kind == OperationKind::Undo)
            },
            _ => panic!("This fixture helper records directory actions only"),
        };
        let position = history
            .append_prepared(id, &PreparedAction {
                action,
                recovery: RecoveryDescriptor::Directory {
                    path,
                    before_exists: !creates,
                    after_exists: creates,
                },
                patches: None,
            })
            .unwrap();
        history.complete_action(id, position).unwrap();
    }
    let entries = history.pending_operations().unwrap()[0].entries.len();
    let record = history.finish_operation(id).unwrap();
    for position in 0..entries {
        history.complete_cleanup(id, position).unwrap();
    }
    record
}
