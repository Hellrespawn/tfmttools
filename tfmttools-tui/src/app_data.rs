use camino::Utf8Path;
use tfmttools_core::action::{Action, Move};
use tfmttools_history::Record;

#[derive(Debug)]
pub enum PreviewData<'pd> {
    Rename(RenameData<'pd>),
    Undo(UndoRedoData<'pd>),
    Redo(UndoRedoData<'pd>),
}

impl<'pd> PreviewData<'pd> {
    pub fn rename(
        template_name: &'pd str,
        arguments: &'pd [String],
        move_actions: &'pd [Move],
        working_directory: &'pd Utf8Path,
    ) -> Self {
        PreviewData::Rename(RenameData {
            template_name,
            arguments,
            move_actions,
            working_directory,
        })
    }

    pub fn undo(records: &'pd [Record<Action>], amount: usize) -> Self {
        PreviewData::Undo(UndoRedoData { records, amount })
    }

    pub fn redo(records: &'pd [Record<Action>], amount: usize) -> Self {
        PreviewData::Redo(UndoRedoData { records, amount })
    }
}

#[derive(Debug)]
pub struct RenameData<'rd> {
    template_name: &'rd str,
    arguments: &'rd [String],
    move_actions: &'rd [Move],
    working_directory: &'rd Utf8Path,
}

impl<'rm> RenameData<'rm> {
    pub fn title(&self) -> String {
        format!(" {} ", self.template_name)
    }

    pub fn arguments(&self) -> &[String] {
        self.arguments
    }

    pub fn move_actions(&self) -> &[Move] {
        self.move_actions
    }

    pub fn working_directory(&self) -> &Utf8Path {
        self.working_directory
    }
}

#[derive(Debug)]
pub struct UndoRedoData<'urd> {
    records: &'urd [Record<Action>],
    amount: usize,
}

impl<'urd> UndoRedoData<'urd> {
    pub fn records(&self) -> &[Record<Action>] {
        self.records
    }

    pub fn amount(&self) -> usize {
        self.amount
    }

    pub fn actual(&self) -> usize {
        if self.amount > self.records.len() {
            self.records.len()
        } else {
            self.amount
        }
    }
}
