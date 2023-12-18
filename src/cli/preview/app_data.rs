use camino::Utf8Path;

use crate::action::{Action, Move};

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

    pub fn title(&self) -> String {
        match self {
            PreviewData::Rename(data) => data.title(),
            PreviewData::Undo(_) => unimplemented!(),
            PreviewData::Redo(_) => unimplemented!(),
        }
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
    _actions: &'urd [Action],
}
