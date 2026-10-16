use crate::app::*;

impl Editor {
    pub(in crate::app) fn history_step(&mut self, redo: bool) {
        self.native_operation(if redo {
            Operation::Redo
        } else {
            Operation::Undo
        });
    }
}
