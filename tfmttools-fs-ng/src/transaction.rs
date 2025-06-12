use tfmttools_core::error::{TFMTError, TFMTResult};

use crate::action::Action;

pub struct TransactionError {
    execution_error: TFMTError,
    rollback_error: Option<TFMTError>,
}

pub struct Transaction {
    actions: Vec<Box<dyn Action>>,
    execution_count: usize,
}

impl Transaction {
    pub fn new() -> Self {
        Self { actions: Vec::new(), execution_count: 0 }
    }

    pub fn add_action(&mut self, action: Box<dyn Action>) {
        self.actions.push(action);
    }

    pub fn add_actions(&mut self, actions: Vec<Box<dyn Action>>) {
        self.actions.extend(actions);
    }

    pub fn run(&mut self) -> TFMTResult<(), TransactionError> {
        let result = self.execute();

        if let Err(execution_error) = result {
            let rollback_error = self.rollback().err();

            Err(TransactionError { execution_error, rollback_error })
        } else {
            Ok(())
        }
    }

    fn execute(&mut self) -> TFMTResult {
        for (i, action) in self.actions.iter().enumerate() {
            self.execution_count = i;
            action.apply()?;
        }

        Ok(())
    }

    fn rollback(&mut self) -> TFMTResult {
        for i in (0..self.execution_count).rev() {
            self.actions[i].undo()?;
        }

        Ok(())
    }
}
