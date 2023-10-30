use std::fmt;
use std::path::Path;

use log::{debug, info};

use crate::changelist::ChangeCount;
use crate::{Change, ChangeList, DiskHandler, Result};

/// History is responsible for saving and loading `ChangeLists`s
#[derive(PartialEq, Debug)]
pub struct History {
    disk_handler: DiskHandler,
    current_list: ChangeList,
    applied_lists: Vec<ChangeList>,
    undone_lists: Vec<ChangeList>,
}

impl fmt::Display for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "History file at {}", self.disk_handler.path().display())?;

        writeln!(f, "Applied changes ({}):", self.applied_lists.len())?;

        for list in &self.applied_lists {
            writeln!(f, "{list}")?;
        }

        writeln!(f, "Undone changes ({}):", self.undone_lists.len())?;

        for list in &self.undone_lists {
            writeln!(f, "{list}")?;
        }

        Ok(())
    }
}

impl History {
    /// Load or create history file at `path`
    pub fn load(directory: &Path, name: &str) -> Result<Self> {
        let disk_handler = DiskHandler::init_dir(directory, name);
        let (applied_lists, undone_lists) = disk_handler.read()?;

        info!("Loading history from {}", disk_handler.path().display());

        Ok(History {
            disk_handler,
            current_list: ChangeList::new(),
            applied_lists,
            undone_lists,
        })
    }

    /// Clears history
    pub fn clear(&mut self) -> Result<()> {
        self.current_list = ChangeList::new();
        self.applied_lists = Vec::new();
        self.undone_lists = Vec::new();

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

        if self.current_list.changed() {
            debug!("Current list was changed");
            let saved_list = std::mem::take(&mut self.current_list);

            self.applied_lists.push(saved_list);
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

    /// Apply a change to the current `ChangeList`.
    pub fn apply(&mut self, change: Change) -> Result<()> {
        self.current_list.apply(change)?;
        Ok(())
    }

    /// Undo `n` amount of `ChangeList`s. Returns amount actually undone
    pub fn undo(&mut self, amount: usize) -> Result<Vec<ChangeCount>> {
        let change_counts = History::undo_redo(
            &mut self.applied_lists,
            &mut self.undone_lists,
            true,
            amount,
        )?;

        self.save()?;

        Ok(change_counts)
    }

    /// Redo `n` amount of `ChangeList`s. Returns amount actually redone
    pub fn redo(&mut self, amount: usize) -> Result<Vec<ChangeCount>> {
        let change_counts = History::undo_redo(
            &mut self.undone_lists,
            &mut self.applied_lists,
            false,
            amount,
        )?;

        self.save()?;

        Ok(change_counts)
    }

    /// Returns true if the history was changed
    pub(crate) fn changed(&self) -> bool {
        self.current_list.changed()
            || self.applied_lists.iter().any(ChangeList::changed)
            || self.undone_lists.iter().any(ChangeList::changed)
    }

    fn save_to_disk(&self) -> Result<()> {
        self.disk_handler.write(&self.applied_lists, &self.undone_lists)
    }

    fn clear_on_disk(&self) -> Result<()> {
        self.disk_handler.clear()?;
        Ok(())
    }

    fn update_hashes(&mut self) {
        self.current_list.update_hash();

        for list in &mut self.applied_lists {
            list.update_hash();
        }
        for list in &mut self.undone_lists {
            list.update_hash();
        }
    }

    fn undo_redo(
        source_list: &mut Vec<ChangeList>,
        target_list: &mut Vec<ChangeList>,
        undo: bool,
        amount: usize,
    ) -> Result<Vec<ChangeCount>> {
        if amount == 0 {
            return Ok(Vec::new());
        }

        let mut processed_lists = Vec::new();

        let mut change_counts = Vec::new();

        while let Some(mut list) = source_list.pop() {
            let change_count = list.to_change_count();

            if undo {
                list.undo()?;
            } else {
                list.redo()?;
            }

            processed_lists.push(list);

            change_counts.push(change_count);
            if change_counts.len() == amount {
                break;
            }
        }

        target_list.extend(processed_lists);

        Ok(change_counts)
    }
}
