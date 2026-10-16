use crate::app::*;

impl Editor {
    pub(in crate::app) fn sync_fields(&mut self) {
        self.material_draft = None;
        self.channel_intents = [false; 9];
        self.name_intent = false;
        let Some(node) = self
            .selected
            .and_then(|id| self.history.scene().node(id).ok())
        else {
            self.selected = None;
            self.selected_path.clear();
            self.remembered_face = None;
            self.name.clear();
            self.channels = Default::default();
            self.material_shape = None;
            return;
        };
        if self.remembered_face.as_ref().is_some_and(|target| {
            Some(target.transform) != self.selected
                || target
                    .path
                    .split_last()
                    .is_none_or(|(_, path)| path != self.selected_path)
                || self.history.scene().resolve_face_target(target).is_err()
        }) {
            self.remembered_face = None;
        }
        self.name = node.name().into();
        let values = node.transform().values();
        self.channels = values.map(|value| value.to_string());
        let transform = node.transform_id();
        let direct = |shape| {
            self.history.scene().dag_edges().iter().any(|edge| {
                edge.parent() == Some(transform)
                    && edge.child() == shape
                    && self.history.scene().shape(shape).is_ok()
            })
        };
        if !self.material_shape.is_some_and(direct) {
            let mut shapes = self
                .history
                .scene()
                .dag_edges()
                .iter()
                .filter(|edge| edge.parent() == Some(transform))
                .filter_map(|edge| self.history.scene().shape(edge.child()).ok())
                .map(|shape| shape.id());
            self.material_shape = shapes.next().filter(|_| shapes.next().is_none());
        }
        if let Some(shape) = self.material_shape
            && let Ok(value) = self.history.scene().material_for_shape(shape)
        {
            self.material_draft = Some(MaterialDraft {
                scene_token: self.scene_token,
                shape,
                original: value.diffuse,
                text: value.diffuse.to_string(),
                intent: false,
                focus: material_edit_id(),
            });
        }
    }
    pub(in crate::app) fn create(&mut self, primitive: Primitive) {
        self.native_operation(Operation::Create {
            primitive,
            name: None,
        });
    }
    pub(in crate::app) fn appearance_scope(
        &self,
    ) -> Result<
        (&'static str, azteccraft::native_scene::AppearanceSummary),
        azteccraft::native_scene::NativeError,
    > {
        if let Some(id) = self.selected
            && let Some(summary) = self.history.scene().transform_appearance_readiness(id)?
        {
            return Ok(("Direct shapes", summary));
        }
        Ok(("Scene totals", self.history.scene().appearance_readiness()))
    }
    pub(in crate::app) fn appearance_summary(&self, ui: &mut egui::Ui) {
        let Ok((scope, summary)) = self.appearance_scope() else {
            ui.weak("Appearance source state unavailable");
            return;
        };
        ui.label(egui::RichText::new(format!("{scope} ({})", summary.shapes)).small());
        ui.add(
            egui::Label::new(
                egui::RichText::new(format!(
                    "Source bindings: {} checked, {} unavailable, {} unbound",
                    summary.checked_source_bindings, summary.unavailable, summary.unbound
                ))
                .small(),
            )
            .wrap(),
        );
        let issues = summary.issues;
        let reasons = [
            (issues.missing_membership, "missing membership"),
            (issues.overlapping_membership, "overlapping membership"),
            (issues.unsupported_shader, "unsupported shader"),
            (issues.unsupported_graph, "unsupported graph"),
            (issues.ambiguous_instance, "ambiguous instance"),
            (issues.unsupported_settings, "unsupported settings"),
            (issues.uv_mismatch, "UV mismatch"),
            (issues.malformed_data, "malformed data"),
        ]
        .into_iter()
        .filter_map(|(present, label)| present.then_some(label))
        .collect::<Vec<_>>()
        .join(", ");
        if !reasons.is_empty() {
            ui.add(egui::Label::new(egui::RichText::new(reasons).small()).wrap());
        }
        ui.add(
            egui::Label::new(
                egui::RichText::new(
                    "Evaluation unavailable; filtering unproved; no color processor.",
                )
                .weak()
                .small(),
            )
            .wrap(),
        );
    }
    pub(in crate::app) fn channels(&mut self, ui: &mut egui::Ui, delete_intent: &mut bool) {
        ui.label(RichText::new("Channel Box").strong());
        let read_only_reason = self.history.scene().authoring_read_only_reason();
        let authoring = read_only_reason.is_none();
        if let Some(reason) = read_only_reason {
            ui.weak(reason.to_string());
        }
        if !self.history.scene().curves().is_empty() {
            ui.weak("Curves: select in the Outliner and frame with F. Viewport curve picking and CV editing are unavailable.");
        }
        self.appearance_summary(ui);
        ui.separator();
        if self.selected.is_none() {
            ui.weak("Select an object to edit its channels.");
            return;
        }
        ui.label("Object name");
        let response = ui.add_enabled(
            authoring,
            egui::TextEdit::singleline(&mut self.name)
                .interactive(authoring)
                .id(name_edit_id()),
        );
        self.name_intent |= response.changed();
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
                    let response = ui.add_enabled(
                        authoring,
                        egui::TextEdit::singleline(value)
                            .interactive(authoring)
                            .id(channel_edit_id(index))
                            .desired_width(100.0),
                    );
                    self.channel_intents[index] |= response.changed();
                    apply |= response.lost_focus();
                    ui.end_row();
                }
            });
        let angular = match &self.history.scene().units().angular {
            AngularUnit::Degree => "degrees",
            AngularUnit::Radian => "radians",
            AngularUnit::Unsupported(unit) => unit,
        };
        ui.weak(format!("Rotation in {angular} | XYZ"));
        if apply {
            self.commit_pending();
        }
        self.material_controls(ui);
        ui.separator();
        if ui
            .add_enabled(
                authoring && self.selection_mode == SelectionMode::Object,
                egui::Button::new("Delete selected"),
            )
            .on_disabled_hover_text("Face deletion is unsupported; switch to Object selection")
            .clicked()
        {
            *delete_intent = true;
        }
        ui.separator();
        ui.weak("Inputs / construction history");
        unsupported(ui, "Not implemented yet");
    }
}
