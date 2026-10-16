use crate::app::*;

impl Editor {
    pub(in crate::app) fn frame_selected(&mut self) {
        self.native_operation(Operation::FrameSelected);
    }
}
