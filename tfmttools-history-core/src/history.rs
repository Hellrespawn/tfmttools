use serde::Serialize;
use serde::de::DeserializeOwned;

use super::record::Record;
use crate::Result;

pub enum LoadHistoryResult {
    Loaded,
    New,
}

pub trait History<A, M>
where
    A: std::fmt::Debug + Serialize + DeserializeOwned,
    M: std::fmt::Debug + Serialize + DeserializeOwned,
{
    fn save(&mut self) -> Result<()>;

    fn push(&mut self, actions: Vec<A>, metadata: M) -> Result<()>;

    fn get_previous_record(&mut self) -> Result<Option<&Record<A, M>>>;

    fn get_records_to_undo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&Record<A, M>>>;

    fn get_records_to_redo(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&Record<A, M>>>;

    fn get_records_to_undo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&mut Record<A, M>>>;

    fn get_records_to_redo_mut(
        &mut self,
        amount: Option<usize>,
    ) -> Result<Vec<&mut Record<A, M>>>;

    fn get_n_records_to_undo(
        &mut self,
        amount: usize,
    ) -> Result<Vec<&Record<A, M>>> {
        self.get_records_to_undo(Some(amount))
    }

    fn get_n_records_to_undo_mut(
        &mut self,
        amount: usize,
    ) -> Result<Vec<&mut Record<A, M>>> {
        self.get_records_to_undo_mut(Some(amount))
    }

    fn get_n_records_to_redo(
        &mut self,
        amount: usize,
    ) -> Result<Vec<&Record<A, M>>> {
        self.get_records_to_redo(Some(amount))
    }

    fn get_n_records_to_redo_mut(
        &mut self,
        amount: usize,
    ) -> Result<Vec<&mut Record<A, M>>> {
        self.get_records_to_redo_mut(Some(amount))
    }

    fn get_all_records_to_undo(&mut self) -> Result<Vec<&Record<A, M>>> {
        self.get_records_to_undo(None)
    }

    fn get_all_records_to_undo_mut(
        &mut self,
    ) -> Result<Vec<&mut Record<A, M>>> {
        self.get_records_to_undo_mut(None)
    }

    fn get_all_records_to_redo(&mut self) -> Result<Vec<&Record<A, M>>> {
        self.get_records_to_redo(None)
    }

    fn get_all_records_to_redo_mut(
        &mut self,
    ) -> Result<Vec<&mut Record<A, M>>> {
        self.get_records_to_redo_mut(None)
    }

    fn remove(&mut self) -> Result<()>;
}
