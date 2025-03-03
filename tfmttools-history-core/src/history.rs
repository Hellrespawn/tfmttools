use super::record::Record;
use crate::{RecordState, Result};

#[derive(Debug, Clone, Copy)]
pub enum LoadHistoryResult {
    Loaded,
    New,
}

pub trait History<A, M> {
    fn save(&mut self) -> Result<()>;

    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()>;

    fn get_previous_record(&mut self) -> Result<Option<Record<A, M>>>;

    fn get_records_to_undo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>>;

    fn get_records_to_redo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<Record<A, M>>>;

    fn get_n_records_to_undo(
        &mut self,
        amount: usize,
    ) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_undo(Some(amount))
    }

    fn get_n_records_to_redo(
        &mut self,
        amount: usize,
    ) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_redo(Some(amount))
    }

    fn get_all_records_to_undo(&mut self) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_undo(None)
    }

    fn get_all_records_to_redo(&mut self) -> Result<Vec<Record<A, M>>> {
        self.get_records_to_redo(None)
    }

    fn set_record_state(
        &mut self,
        record: Record<A, M>,
        state: RecordState,
    ) -> Result<Record<A, M>>;

    fn remove(&mut self) -> Result<()>;

    fn is_empty(&mut self) -> bool;
}
