use azteccraft::{
    history::{Command, EditOutcome, History},
    scene::{NodeId, Primitive, Transform},
    viewport::Camera,
};
use eframe::egui::{self, Color32, Key, RichText};
use glam::Vec3;

#[path = "file_ui.rs"]
mod files;

pub struct Editor {
    history: History,
    selected: Option<NodeId>,
    camera: Camera,
    name: String,
    channels: [String; 9],
    status: String,
    error: bool,
    aspect: f32,
    files: files::FileState,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            history: History::default(),
            selected: None,
            camera: Camera::default(),
            name: String::new(),
            channels: Default::default(),
            status: "Ready | Create a cube or plane to start".into(),
            error: false,
            aspect: 1.0,
            files: files::FileState::default(),
        }
    }
}

impl Editor {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut style = egui::Style {
            visuals: egui::Visuals::dark(),
            ..Default::default()
        };
        style.visuals.panel_fill = Color32::from_gray(48);
        style.visuals.window_fill = Color32::from_gray(52);
        style.visuals.selection.bg_fill = Color32::from_rgb(58, 92, 120);
        style.spacing.item_spacing = egui::vec2(6.0, 5.0);
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        cc.egui_ctx.set_theme(egui::Theme::Dark);
        cc.egui_ctx.set_style_of(egui::Theme::Dark, style);
        Self::default()
    }

    fn sync_fields(&mut self) {
        let Some(node) = self
            .selected
            .and_then(|id| self.history.scene().node(id).ok())
        else {
            self.selected = None;
            return;
        };
        self.name = node.name().into();
        let t = node.transform();
        let values = [
            t.translation.x,
            t.translation.y,
            t.translation.z,
            t.rotation_degrees.x,
            t.rotation_degrees.y,
            t.rotation_degrees.z,
            t.scale.x,
            t.scale.y,
            t.scale.z,
        ];
        self.channels = values.map(|value| value.to_string());
    }

    fn select(&mut self, selected: Option<NodeId>) {
        if !self.commit_pending() {
            return;
        }
        self.selected = selected;
        self.sync_fields();
    }

    fn execute(&mut self, command: Command) {
        if !self.commit_pending() {
            return;
        }
        match self.history.execute(command) {
            Ok(outcome) => {
                if let EditOutcome::Created(id) = outcome {
                    self.selected = Some(id);
                }
                self.sync_fields();
                self.status = if outcome == EditOutcome::Unchanged {
                    "No change"
                } else {
                    "Scene edited"
                }
                .into();
                self.error = false;
            }
            Err(error) => {
                self.status = error.to_string();
                self.error = true;
            }
        }
    }

    fn create(&mut self, primitive: Primitive) {
        if !self.commit_pending() {
            return;
        }
        let base = match primitive {
            Primitive::Cube => "pCube",
            Primitive::Plane => "pPlane",
        };
        let mut suffix = 1;
        while self
            .history
            .scene()
            .nodes()
            .iter()
            .any(|n| n.name() == format!("{base}{suffix}"))
        {
            suffix += 1;
        }
        self.execute(Command::Create {
            primitive,
            name: format!("{base}{suffix}"),
        });
    }

    fn history_step(&mut self, redo: bool) {
        if !self.commit_pending() {
            return;
        }
        let changed = if redo {
            self.history.redo()
        } else {
            self.history.undo()
        };
        if changed {
            self.sync_fields();
            self.status = if redo { "Redo" } else { "Undo" }.into();
            self.error = false;
        }
    }

    fn frame_selected(&mut self) {
        if !self.commit_pending() {
            return;
        }
        if let Some(node) = self
            .selected
            .and_then(|id| self.history.scene().node(id).ok())
            && let Err(error) = self.camera.frame(node, self.aspect)
        {
            self.status = error.to_string();
            self.error = true;
        }
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if ctx.egui_wants_keyboard_input() || egui::Popup::is_any_open(ctx) || self.files.is_modal()
        {
            return;
        }
        let (undo, redo, delete, frame) = ctx.input(|i| {
            (
                i.modifiers.command && !i.modifiers.shift && i.key_pressed(Key::Z),
                i.modifiers.command && i.modifiers.shift && i.key_pressed(Key::Z),
                i.modifiers.is_none() && i.key_pressed(Key::Delete),
                i.modifiers.is_none() && i.key_pressed(Key::F),
            )
        });
        if undo {
            self.history_step(false);
        }
        if redo {
            self.history_step(true);
        }
        if delete && let Some(id) = self.selected {
            self.execute(Command::Remove(id));
        }
        if frame {
            self.frame_selected();
        }
    }

    fn commit_pending(&mut self) -> bool {
        let Some(id) = self.selected else {
            return true;
        };
        let result = || -> Result<(Transform, bool, bool), String> {
            let node = self.history.scene().node(id).map_err(|e| e.to_string())?;
            let v: Vec<f32> = self
                .channels
                .iter()
                .map(|v| v.parse::<f32>())
                .collect::<Result<_, _>>()
                .map_err(|_| "Enter a valid number in every transform channel".to_string())?;
            let transform = Transform {
                translation: Vec3::new(v[0], v[1], v[2]),
                rotation_degrees: Vec3::new(v[3], v[4], v[5]),
                scale: Vec3::new(v[6], v[7], v[8]),
            };
            let rename = node.name() != self.name;
            let changed = node.transform() != transform;
            let mut candidate = self.history.scene().clone();
            candidate
                .rename(id, &self.name)
                .map_err(|e| e.to_string())?;
            candidate
                .set_transform(id, transform)
                .map_err(|e| e.to_string())?;
            Ok((transform, rename, changed))
        };
        let outcome = result().and_then(|(transform, rename, changed)| {
            if rename {
                self.history
                    .execute(Command::Rename {
                        id,
                        name: self.name.clone(),
                    })
                    .map_err(|e| e.to_string())?;
            }
            if changed {
                self.history
                    .execute(Command::SetTransform { id, transform })
                    .map_err(|e| e.to_string())?;
            }
            Ok(rename || changed)
        });
        match outcome {
            Ok(changed) => {
                self.sync_fields();
                if changed {
                    self.status = "Scene edited".into();
                    self.error = false;
                }
                true
            }
            Err(error) => {
                self.status = error;
                self.error = true;
                false
            }
        }
    }

    fn cancel_on_escape(&mut self, ctx: &egui::Context) {
        // egui clears text focus before this pass, so cancellation cannot rely on has_focus.
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.sync_fields();
            self.status = "Edit cancelled".into();
            self.error = false;
        }
    }

    fn menus(&mut self, ui: &mut egui::Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("New scene").clicked() {
                    ui.close();
                    self.request_new();
                }
                if ui.button("Open...").clicked() {
                    ui.close();
                    self.open_dialog();
                }
                if ui.button("Save As...").clicked() {
                    ui.close();
                    self.save_dialog();
                }
                unsupported(ui, "Export...");
                ui.separator();
                if ui.button("Close").clicked() {
                    ui.close();
                    self.request_close();
                }
            });
            ui.menu_button("Edit", |ui| {
                if ui
                    .add_enabled(
                        self.history.can_undo(),
                        egui::Button::new("Undo   Ctrl/Cmd Z"),
                    )
                    .clicked()
                {
                    self.history_step(false);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        self.history.can_redo(),
                        egui::Button::new("Redo   Ctrl/Cmd Shift Z"),
                    )
                    .clicked()
                {
                    self.history_step(true);
                    ui.close();
                }
                if ui
                    .add_enabled(self.selected.is_some(), egui::Button::new("Delete"))
                    .clicked()
                {
                    if let Some(id) = self.selected {
                        self.execute(Command::Remove(id));
                    }
                    ui.close();
                }
            });
            ui.menu_button("Create", |ui| {
                if ui.button("Polygon cube").clicked() {
                    self.create(Primitive::Cube);
                    ui.close();
                }
                if ui.button("Polygon plane").clicked() {
                    self.create(Primitive::Plane);
                    ui.close();
                }
                unsupported(ui, "Other primitives");
            });
            for label in ["Select", "Modify", "Mesh", "UV", "Windows", "Render"] {
                ui.menu_button(label, |ui| {
                    unsupported(ui, "Not implemented yet");
                });
            }
            ui.menu_button("Help", |ui| {
                ui.label("Local editor prototype");
                ui.label("Maya parity has not been verified.");
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            ui.label(RichText::new("Modeling").strong());
            ui.separator();
            if ui.button("Cube").clicked() {
                self.create(Primitive::Cube);
            }
            if ui.button("Plane").clicked() {
                self.create(Primitive::Plane);
            }
            ui.separator();
            if ui
                .add_enabled(self.history.can_undo(), egui::Button::new("Undo"))
                .clicked()
            {
                self.history_step(false);
            }
            if ui
                .add_enabled(self.history.can_redo(), egui::Button::new("Redo"))
                .clicked()
            {
                self.history_step(true);
            }
            if ui
                .add_enabled(
                    self.selected.is_some(),
                    egui::Button::new("Frame selected  F"),
                )
                .clicked()
            {
                self.frame_selected();
            }
            ui.separator();
            unsupported(ui, "Extrude");
            unsupported(ui, "Bevel");
            unsupported(ui, "Materials");
        });
    }

    fn outliner(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Outliner").strong());
        ui.separator();
        ui.weak("Scene root");
        let mut choice = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for node in self.history.scene().nodes() {
                if ui
                    .selectable_label(
                        self.selected == Some(node.id()),
                        format!("◇  {}", node.name()),
                    )
                    .clicked()
                {
                    choice = Some(node.id());
                }
            }
        });
        if let Some(id) = choice {
            self.select(Some(id));
        }
        if self.history.scene().nodes().is_empty() {
            ui.weak("No objects. Create a primitive above.");
        }
    }

    fn channels(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Channel Box").strong());
        ui.separator();
        let Some(id) = self.selected else {
            ui.weak("Select an object to edit its channels.");
            return;
        };
        ui.label("Object name");
        let response =
            ui.add(egui::TextEdit::singleline(&mut self.name).id(egui::Id::new("node_name")));
        let mut apply = response.lost_focus();
        ui.separator();
        egui::Grid::new("transform_channels")
            .num_columns(2)
            .spacing([8.0, 5.0])
            .show(ui, |ui| {
                for (index, (label, value)) in [
                    "Translate X",
                    "Translate Y",
                    "Translate Z",
                    "Rotate X",
                    "Rotate Y",
                    "Rotate Z",
                    "Scale X",
                    "Scale Y",
                    "Scale Z",
                ]
                .iter()
                .zip(&mut self.channels)
                .enumerate()
                {
                    ui.label(*label);
                    let response = ui.add(
                        egui::TextEdit::singleline(value)
                            .id(egui::Id::new(("channel", index)))
                            .desired_width(100.0),
                    );
                    apply |= response.lost_focus();
                    ui.end_row();
                }
            });
        ui.weak("Rotation in degrees | XYZ");
        if apply {
            self.commit_pending();
        }
        ui.separator();
        if ui.button("Delete selected").clicked() {
            self.execute(Command::Remove(id));
        }
        ui.separator();
        ui.weak("Inputs / construction history");
        unsupported(ui, "Not implemented yet");
    }
}

impl eframe::App for Editor {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
}

impl Editor {
    fn render(&mut self, ui: &mut egui::Ui) {
        self.handle_close(ui.ctx());
        if !self.files.is_modal() {
            self.cancel_on_escape(ui.ctx());
        }
        self.shortcuts(ui.ctx());
        egui::Panel::top("menu_shelf").show(ui, |ui| self.menus(ui));
        egui::Panel::bottom("timeline_status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Time slider");
                for label in ["|<", "<", "Play", ">", ">|"] {
                    unsupported(ui, label);
                }
                ui.weak("Animation not implemented");
            });
            ui.separator();
            ui.colored_label(
                if self.error {
                    Color32::from_rgb(255, 160, 130)
                } else {
                    Color32::LIGHT_GRAY
                },
                &self.status,
            );
        });
        egui::Panel::left("toolbox")
            .default_size(76.0)
            .resizable(false)
            .show(ui, |ui| {
                ui.label("Tools");
                ui.separator();
                ui.label("Select");
                for label in ["Move", "Rotate", "Scale", "Snap"] {
                    unsupported(ui, label);
                }
            });
        egui::Panel::left("outliner")
            .default_size(200.0)
            .size_range(150.0..=350.0)
            .show(ui, |ui| self.outliner(ui));
        egui::Panel::right("channels")
            .default_size(255.0)
            .size_range(230.0..=400.0)
            .show(ui, |ui| self.channels(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("persp");
                ui.separator();
                ui.weak("Alt + left: orbit | Middle: pan | Right / wheel: dolly");
            });
            match crate::viewport_ui::show(
                ui,
                &mut self.camera,
                self.history.scene(),
                self.selected,
            ) {
                Ok(output) => {
                    self.aspect = output.aspect;
                    if let Some(selection) = output.selection {
                        self.select(selection);
                    }
                }
                Err(error) => {
                    self.status = error.to_string();
                    self.error = true;
                }
            }
        });
        self.file_modal(ui.ctx());
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Title(self.document_title()));
    }
}

fn unsupported(ui: &mut egui::Ui, label: &str) {
    ui.add_enabled(false, egui::Button::new(label))
        .on_disabled_hover_text("Not implemented in this build");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(key: Key) -> egui::Event {
        egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }
    }

    fn frame(editor: &mut Editor, ctx: &egui::Context, events: Vec<egui::Event>) {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1440.0, 900.0),
                )),
                events,
                ..Default::default()
            },
            |ui| editor.render(ui),
        );
        output.textures_delta.clear();
    }

    #[test]
    fn real_escape_event_cancels_channel_and_name_after_egui_clears_focus() {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let id = editor.selected.unwrap();
        let before = editor.history.scene().clone();
        let ctx = egui::Context::default();
        frame(&mut editor, &ctx, vec![]);
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(("channel", 0_usize))));
        frame(&mut editor, &ctx, vec![]);
        editor.channels[0] = "3".into();
        frame(&mut editor, &ctx, vec![key(Key::Escape)]);
        assert_eq!(editor.channels[0], "0");
        assert_eq!(editor.history.scene(), &before);
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new("node_name")));
        frame(&mut editor, &ctx, vec![]);
        editor.name = "discarded".into();
        frame(&mut editor, &ctx, vec![key(Key::Escape)]);
        assert_eq!(editor.history.scene().node(id).unwrap().name(), "pCube1");
        editor.history_step(false);
        assert!(editor.history.scene().nodes().is_empty());
    }

    #[test]
    fn enter_commits_once_and_selection_commits_draft_to_previous_object() {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let old = editor.selected.unwrap();
        editor.create(Primitive::Plane);
        let next = editor.selected.unwrap();
        editor.select(Some(old));
        let ctx = egui::Context::default();
        frame(&mut editor, &ctx, vec![]);
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(("channel", 0_usize))));
        frame(&mut editor, &ctx, vec![]);
        editor.channels[0] = "2".into();
        frame(&mut editor, &ctx, vec![key(Key::Enter)]);
        assert_eq!(
            editor
                .history
                .scene()
                .node(old)
                .unwrap()
                .transform()
                .translation
                .x,
            2.0
        );
        editor.history_step(false);
        assert_eq!(
            editor
                .history
                .scene()
                .node(old)
                .unwrap()
                .transform()
                .translation
                .x,
            0.0
        );
        editor.channels[0] = "4".into();
        editor.select(Some(next));
        assert_eq!(
            editor
                .history
                .scene()
                .node(old)
                .unwrap()
                .transform()
                .translation
                .x,
            4.0
        );
        assert_eq!(
            editor.history.scene().node(next).unwrap().transform(),
            Transform::default()
        );
        assert_eq!(editor.channels[0], "0");
    }

    #[test]
    fn creation_commits_old_draft_and_invalid_draft_blocks_selection_and_actions() {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let old = editor.selected.unwrap();
        editor.channels[1] = "5".into();
        editor.create(Primitive::Plane);
        let next = editor.selected.unwrap();
        assert_eq!(
            editor
                .history
                .scene()
                .node(old)
                .unwrap()
                .transform()
                .translation
                .y,
            5.0
        );
        assert_eq!(
            editor.history.scene().node(next).unwrap().transform(),
            Transform::default()
        );
        editor.channels[0] = "nan".into();
        let before = editor.history.scene().clone();
        editor.select(Some(old));
        assert_eq!(editor.selected, Some(next));
        editor.create(Primitive::Cube);
        assert_eq!(editor.history.scene(), &before);
        assert_eq!(editor.channels[0], "nan");
        assert!(editor.error);
    }

    #[test]
    fn create_allocates_name_after_committing_pending_rename() {
        let mut editor = Editor::default();
        editor.create(Primitive::Cube);
        let old = editor.selected.unwrap();
        editor.name = "pCube2".into();
        editor.create(Primitive::Cube);
        let created = editor.selected.unwrap();
        assert_ne!(old, created);
        assert_eq!(editor.history.scene().node(old).unwrap().name(), "pCube2");
        assert_eq!(
            editor.history.scene().node(created).unwrap().name(),
            "pCube1"
        );
    }
}
