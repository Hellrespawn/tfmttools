use crate::actiongroup::ActionCount;
use crate::{Action, ActionGroup, DiskHandler, Result};
use log::{debug, info};
use std::fmt;
use std::path::Path;

/// History is responsible for saving and loading `ActionGroup`s
#[derive(PartialEq, Debug)]
pub struct History {
    disk_handler: DiskHandler,
    current_group: ActionGroup,
    applied_groups: Vec<ActionGroup>,
    undone_groups: Vec<ActionGroup>,
}

impl fmt::Display for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "History file at {}", self.disk_handler.path().display())?;

        writeln!(f, "Applied actions ({}):", self.applied_groups.len())?;

        for group in &self.applied_groups {
            writeln!(f, "{group}")?;
        }

        writeln!(f, "Undone actions ({}):", self.undone_groups.len())?;

        for group in &self.undone_groups {
            writeln!(f, "{group}")?;
        }

        Ok(())
    }
}

impl History {
    /// Load or create history file at `path`
    pub fn load(directory: &Path, name: &str) -> Result<Self> {
        let disk_handler = DiskHandler::init_dir(directory, name);
        let (applied_groups, undone_groups) = disk_handler.read()?;

        info!("Loading history from {}", disk_handler.path().display());

        Ok(History {
            disk_handler,
            current_group: ActionGroup::new(),
            applied_groups,
            undone_groups,
        })
    }

    /// Clears history
    pub fn clear(&mut self) -> Result<()> {
        self.current_group = ActionGroup::new();
        self.applied_groups = Vec::new();
        self.undone_groups = Vec::new();

        self.clear_on_disk()?;

        info!("History cleared.");

        Ok(())
    }

    /// Save history, if necessary
    pub fn save(&mut self) -> Result<bool> {
        if !self.changed() {
            info!("Nothing was changed.");
            return Ok(false);
        }

        if self.current_group.changed() {
            debug!("Current group was changed");
            let saved_group = std::mem::take(&mut self.current_group);

            self.applied_groups.push(saved_group);
        }

        self.update_hashes();

        self.save_to_disk()?;
        info!("Saved history to disk");

        Ok(true)
    }

    /// Gets the path of the current instance.
    pub fn path(&self) -> &Path {
        self.disk_handler.path()
    }

    /// Apply an action to the current `ActionGroup`.
    pub fn apply(&mut self, action: Action) -> Result<()> {
        self.current_group.apply(action)?;
        Ok(())
    }

    /// Undo `n` amount of `ActionGroup`s. Returns amount actually undone
    pub fn undo(&mut self, amount: usize) -> Result<Vec<ActionCount>> {
        let action_counts = History::undo_redo(
            &mut self.applied_groups,
            &mut self.undone_groups,
            true,
            amount,
        )?;

        self.save()?;

        Ok(action_counts)
    }

    /// Redo `n` amount of `ActionGroup`s. Returns amount actually redone
    pub fn redo(&mut self, amount: usize) -> Result<Vec<ActionCount>> {
        let action_counts = History::undo_redo(
            &mut self.undone_groups,
            &mut self.applied_groups,
            false,
            amount,
        )?;

        self.save()?;

        Ok(action_counts)
    }

    /// Returns true if the history was changed
    pub(crate) fn changed(&self) -> bool {
        self.current_group.changed()
            || self.applied_groups.iter().any(ActionGroup::changed)
            || self.undone_groups.iter().any(ActionGroup::changed)
    }

    fn save_to_disk(&self) -> Result<()> {
        self.disk_handler
            .write(&self.applied_groups, &self.undone_groups)
    }

    fn clear_on_disk(&self) -> Result<()> {
        self.disk_handler.clear()?;
        Ok(())
    }

    fn update_hashes(&mut self) {
        self.current_group.update_hash();

        for group in &mut self.applied_groups {
            group.update_hash();
        }
        for group in &mut self.undone_groups {
            group.update_hash();
        }
    }

    fn undo_redo(
        source_group: &mut Vec<ActionGroup>,
        target_group: &mut Vec<ActionGroup>,
        undo: bool,
        amount: usize,
    ) -> Result<Vec<ActionCount>> {
        if amount == 0 {
            return Ok(Vec::new());
        }

        let mut processed_groups = Vec::new();

        let mut action_counts = Vec::new();

        while let Some(mut group) = source_group.pop() {
            let action_count = group.to_action_count();

            if undo {
                group.undo()?;
            } else {
                group.redo()?;
            }

            processed_groups.push(group);

            action_counts.push(action_count);
            if action_counts.len() == amount {
                break;
            }
        }

        target_group.extend(processed_groups);

        Ok(action_counts)
    }
}
