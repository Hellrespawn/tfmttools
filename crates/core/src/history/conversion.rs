use super::{
    HistoryError, StoredAction, StoredTagValueChange, StoredTagValueKind,
};
use crate::action::{Action, TagValueChange, TagValueKind};
use crate::item_keys::{canonical_tag_name, parse_item_key};

impl From<&Action> for StoredAction {
    fn from(action: &Action) -> Self {
        match action {
            Action::MoveFile { source, target } => {
                Self::MoveFile {
                    source: source.to_string(),
                    target: target.to_string(),
                }
            },
            Action::CopyFile { source, target } => {
                Self::CopyFile {
                    source: source.to_string(),
                    target: target.to_string(),
                }
            },
            Action::RemoveFile(path) => {
                Self::RemoveFile { path: path.to_string() }
            },
            Action::MakeDir(path) => Self::MakeDir { path: path.to_string() },
            Action::RemoveDir(path) => {
                Self::RemoveDir { path: path.to_string() }
            },
            Action::EditTagValues { path, changes } => {
                Self::EditTagValues {
                    path: path.to_string(),
                    changes: changes
                        .iter()
                        .map(StoredTagValueChange::from)
                        .collect(),
                }
            },
        }
    }
}

impl TryFrom<&StoredAction> for Action {
    type Error = HistoryError;

    fn try_from(action: &StoredAction) -> Result<Self, Self::Error> {
        Ok(match action {
            StoredAction::MoveFile { source, target } => {
                Self::MoveFile { source: source.into(), target: target.into() }
            },
            StoredAction::CopyFile { source, target } => {
                Self::CopyFile { source: source.into(), target: target.into() }
            },
            StoredAction::RemoveFile { path } => Self::RemoveFile(path.into()),
            StoredAction::MakeDir { path } => Self::MakeDir(path.into()),
            StoredAction::RemoveDir { path } => Self::RemoveDir(path.into()),
            StoredAction::EditTagValues { path, changes } => {
                Self::EditTagValues {
                    path: path.into(),
                    changes: changes
                        .iter()
                        .enumerate()
                        .map(|(index, change)| {
                            TagValueChange::try_from(change).map_err(|error| {
                                HistoryError::LoadError(format!(
                                    "changes[{index}]: {error}"
                                ))
                            })
                        })
                        .collect::<Result<_, _>>()?,
                }
            },
        })
    }
}

impl From<&TagValueChange> for StoredTagValueChange {
    fn from(change: &TagValueChange) -> Self {
        Self {
            key: change.key().to_owned(),
            kind: match change.kind() {
                TagValueKind::Text => StoredTagValueKind::Text,
                TagValueKind::Locator => StoredTagValueKind::Locator,
            },
            old_value: change.old_value().to_owned(),
            new_value: change.new_value().to_owned(),
            old_encoding: change.old_encoding().map(str::to_owned),
            new_encoding: change.new_encoding().map(str::to_owned),
        }
    }
}

impl TryFrom<&StoredTagValueChange> for TagValueChange {
    type Error = HistoryError;

    fn try_from(change: &StoredTagValueChange) -> Result<Self, Self::Error> {
        let key = parse_item_key(&change.key)
            .map_err(|error| HistoryError::LoadError(error.to_string()))?;
        if canonical_tag_name(key) != Some(change.key.as_str()) {
            return Err(HistoryError::LoadError(format!(
                "Noncanonical tag key '{}'",
                change.key
            )));
        }
        for encoding in
            [change.old_encoding.as_deref(), change.new_encoding.as_deref()]
                .into_iter()
                .flatten()
        {
            if !matches!(encoding, "Latin1" | "UTF16" | "UTF16BE" | "UTF8") {
                return Err(HistoryError::LoadError(format!(
                    "Unknown tag encoding '{encoding}'"
                )));
            }
        }
        Ok(Self::new(
            change.key.clone(),
            match change.kind {
                StoredTagValueKind::Text => TagValueKind::Text,
                StoredTagValueKind::Locator => TagValueKind::Locator,
            },
            change.old_value.clone(),
            change.new_value.clone(),
        )
        .with_encoding(
            change.old_encoding.clone(),
            change.new_encoding.clone(),
        ))
    }
}
