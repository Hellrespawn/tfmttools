use crate::util::calculate_hash;
use crate::{Change, ChangeType, Result};
use serde::{Deserialize, Serialize};
use std::fmt;

pub struct ChangeCount {
    pub mv: u64,
    pub mkdir: u64,
    pub rmdir: u64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub(crate) struct ChangeList {
    changes: Vec<Change>,
    hash: u64,
}

impl fmt::Display for ChangeList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut move_count = 0;
        let mut mkdir_count = 0;
        let mut rmdir_count = 0;

        for change in &self.changes {
            match change.change_type() {
                ChangeType::Mv { .. } => move_count += 1,
                ChangeType::MkDir(_) => mkdir_count += 1,
                ChangeType::RmDir(_) => rmdir_count += 1,
            }
        }

        writeln!(
            f,
            "mv: {move_count}, mkdir: {mkdir_count}, rmdir: {rmdir_count}"
        )?;

        for change in &self.changes {
            writeln!(f, "{change}")?;
        }

        Ok(())
    }
}

impl PartialEq for ChangeList {
    fn eq(&self, other: &Self) -> bool {
        self.changes == other.changes
    }
}

impl ChangeList {
    pub(crate) fn new() -> Self {
        let changes = Vec::new();
        let hash = calculate_hash(&changes);
        ChangeList { changes, hash }
    }

    pub(crate) fn update_hash(&mut self) {
        self.hash = calculate_hash(&self.changes);
    }

    pub(crate) fn to_change_count(&self) -> ChangeCount {
        let mut change_count = ChangeCount {
            mv: 0,
            mkdir: 0,
            rmdir: 0,
        };

        for change in &self.changes {
            match change.change_type() {
                ChangeType::Mv { .. } => change_count.mv += 1,
                ChangeType::MkDir(_) => change_count.mkdir += 1,
                ChangeType::RmDir(_) => change_count.rmdir += 1,
            }
        }

        change_count
    }

    // pub(crate) fn to_string_short(&self) -> String {
    //     let string = self.to_string();
    //     let lines: Vec<&str> = string.lines().collect();
    //     format!("{}{}{}", lines[0], lines[1], lines[self.0.len() + 2])
    // }

    pub(crate) fn changed(&self) -> bool {
        self.hash != calculate_hash(&self.changes)
    }

    pub(crate) fn apply(&mut self, mut change: Change) -> Result<()> {
        change.apply()?;
        self.changes.push(change);
        Ok(())
    }

    pub(crate) fn undo(&mut self) -> Result<()> {
        // Undo happens in reverse order
        for change in self.changes.iter_mut().rev() {
            change.undo()?;
        }

        Ok(())
    }

    pub(crate) fn redo(&mut self) -> Result<()> {
        // Redo happens in original order
        for change in &mut self.changes {
            change.apply()?;
        }

        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn push(&mut self, change: Change) {
        self.changes.push(change);
    }
}
