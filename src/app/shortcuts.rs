use super::*;

impl Editor {
    pub(super) fn shortcuts(&mut self, ctx: &egui::Context) -> Option<ActiveTool> {
        if ctx.egui_wants_keyboard_input() || egui::Popup::is_any_open(ctx) || self.files.is_modal()
        {
            return None;
        }
        let tool_intent = ctx.input(|i| {
            i.events.iter().find_map(|event| match event {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                    ..
                } if modifiers.is_none() => match key {
                    Key::Q => Some(ActiveTool::Select),
                    Key::W => Some(ActiveTool::Move),
                    _ => None,
                },
                _ => None,
            })
        });
        let (undo, redo, frame, save, save_as) = ctx.input(|i| {
            (
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::Z),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::Z),
                i.modifiers.is_none() && i.key_pressed(Key::F),
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::S),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::S),
            )
        });
        if undo {
            self.history_step(false);
        }
        if redo {
            self.history_step(true);
        }
        if frame {
            self.frame_selected();
        }
        if save && self.files.save_available() {
            self.native_operation(Operation::SaveScene);
        }
        if save_as && self.files.save_as_available() {
            self.save_as_dialog();
        }
        tool_intent
    }
    pub(super) fn cancel_on_escape(&mut self, ctx: &egui::Context) {
        // egui clears text focus before this pass, so cancellation cannot rely on has_focus.
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            let cancelled = self.cancel_move();
            self.consume_pointer_sequence();
            if let Err(error) = cancelled {
                self.status = error.to_string();
                self.error = true;
                return;
            }
            self.sync_fields();
            self.status = "Edit cancelled".into();
            self.error = false;
        }
    }
}
