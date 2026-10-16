use super::*;
use azteccraft::ui_icons::UiIcon;

impl Editor {
    pub(super) fn menus(
        &mut self,
        ui: &mut egui::Ui,
        selection_intent: &mut Option<SelectionModeIntent>,
        delete_intent: &mut bool,
    ) {
        let read_only_reason = self
            .history
            .scene()
            .authoring_read_only_reason()
            .map(|reason| reason.to_string());
        let authoring = read_only_reason.is_none();
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
                let mut save =
                    ui.add_enabled(self.files.save_available(), egui::Button::new("Save"));
                if let Some(reason) = read_only_reason.as_deref() {
                    save = save.on_disabled_hover_text(reason);
                }
                if save.clicked() {
                    ui.close();
                    self.native_operation(Operation::SaveScene);
                }
                let mut save_as = ui.add_enabled(
                    self.files.save_as_available(),
                    egui::Button::new("Save As..."),
                );
                if let Some(reason) = read_only_reason.as_deref() {
                    save_as = save_as.on_disabled_hover_text(reason);
                }
                if save_as.clicked() {
                    ui.close();
                    self.save_as_dialog();
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
                    .add_enabled(
                        authoring
                            && self.selected.is_some()
                            && self.selection_mode == SelectionMode::Object,
                        egui::Button::new("Delete"),
                    )
                    .on_disabled_hover_text(
                        "Object selection required; face deletion is unsupported",
                    )
                    .clicked()
                {
                    *delete_intent = true;
                    ui.close();
                }
            });
            ui.menu_button("Create", |ui| {
                if ui
                    .add_enabled(authoring, egui::Button::new("Polygon cube"))
                    .clicked()
                {
                    self.create(Primitive::Cube);
                    ui.close();
                }
                if ui
                    .add_enabled(authoring, egui::Button::new("Polygon plane"))
                    .clicked()
                {
                    self.create(Primitive::Plane);
                    ui.close();
                }
                if ui
                    .add_enabled(authoring, egui::Button::new("Polygon sphere"))
                    .clicked()
                {
                    self.create(Primitive::Sphere);
                    ui.close();
                }
                unsupported(ui, "Other primitives");
            });
            ui.menu_button("Select", |ui| {
                if ui.button("Object / Component   F8").clicked() {
                    selection_intent.get_or_insert(SelectionModeIntent::Toggle);
                    ui.close();
                }
                ui.menu_button("Components", |ui| {
                    if ui
                        .selectable_label(self.selection_mode == SelectionMode::Face, "Face   F11")
                        .clicked()
                    {
                        *selection_intent = Some(SelectionModeIntent::Face);
                        ui.close();
                    }
                    unsupported(ui, "Vertices / edges / UVs");
                });
            });
            for label in ["Modify", "Mesh", "UV", "Windows", "Render"] {
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
            if crate::tool_icons::button(ui, "Cube", UiIcon::PolyCube, authoring).clicked() {
                self.create(Primitive::Cube);
            }
            if crate::tool_icons::button(ui, "Plane", UiIcon::PolyPlane, authoring).clicked() {
                self.create(Primitive::Plane);
            }
            ui.separator();
            if crate::tool_icons::button(ui, "Undo", UiIcon::Undo, self.history.can_undo())
                .clicked()
            {
                self.history_step(false);
            }
            if crate::tool_icons::button(ui, "Redo", UiIcon::Redo, self.history.can_redo())
                .clicked()
            {
                self.history_step(true);
            }
            if crate::tool_icons::button(
                ui,
                "Frame selected  F",
                UiIcon::FrameSelected,
                self.selected.is_some(),
            )
            .clicked()
            {
                self.frame_selected();
            }
            ui.separator();
            ui.add_enabled(false, egui::Button::new("Extrude"))
                .on_disabled_hover_text(
                    "Extrude construction history and manipulator workflow are unsupported",
                );
            unsupported(ui, "Bevel");
            unsupported(ui, "Materials");
        });
    }
}
