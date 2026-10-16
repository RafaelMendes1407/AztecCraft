use crate::app::*;

impl Editor {
    /// Whether `shape` is a mesh shape parented directly under `transform`.
    pub(in crate::app) fn is_direct_shape(&self, transform: NativeId, shape: NativeId) -> bool {
        self.history.scene().dag_edges().iter().any(|edge| {
            edge.parent() == Some(transform)
                && edge.child() == shape
                && self.history.scene().shape(shape).is_ok()
        })
    }

    /// Position of `shape` among the transform's direct mesh shapes, in tab order.
    fn direct_shape_rank(&self, transform: NativeId, shape: NativeId) -> Option<usize> {
        self.direct_shapes(transform).position(|id| id == shape)
    }

    pub(in crate::app) fn direct_shapes(
        &self,
        transform: NativeId,
    ) -> impl Iterator<Item = NativeId> + '_ {
        let scene = self.history.scene();
        scene
            .dag_edges()
            .iter()
            .filter(move |edge| edge.parent() == Some(transform))
            .filter_map(|edge| scene.shape(edge.child()).ok())
            .map(|shape| shape.id())
    }

    /// Whether `tab` names the selected transform or one of its direct shapes or their materials.
    fn attribute_editor_tab_resolves(&self, tab: AttributeEditorTab) -> bool {
        let Some(node) = self
            .selected
            .and_then(|id| self.history.scene().node(id).ok())
        else {
            return false;
        };
        match tab {
            AttributeEditorTab::Transform(id) => id == node.id(),
            AttributeEditorTab::Shape(shape) => self
                .direct_shape_rank(node.transform_id(), shape)
                .is_some_and(|rank| rank < MAX_SHAPE_TABS),
            AttributeEditorTab::Material(shape) => {
                self.direct_shape_rank(node.transform_id(), shape)
                    .is_some_and(|rank| rank < MAX_SHAPE_TABS)
                    && self.history.scene().material_for_shape(shape).is_ok()
            }
        }
    }

    /// The active tab: the stored one while it still resolves, else the selected transform's.
    /// `None` without a selected node.
    pub(in crate::app) fn effective_attribute_editor_tab(&self) -> Option<AttributeEditorTab> {
        let selected = self
            .selected
            .filter(|id| self.history.scene().node(*id).is_ok())?;
        Some(
            self.attribute_editor_tab
                .filter(|tab| self.attribute_editor_tab_resolves(*tab))
                .unwrap_or(AttributeEditorTab::Transform(selected)),
        )
    }

    /// Drops a stored tab that no longer resolves; runs from `sync_fields` and scene replacement.
    pub(in crate::app) fn normalize_attribute_editor_tab(&mut self) {
        if self
            .attribute_editor_tab
            .is_some_and(|tab| !self.attribute_editor_tab_resolves(tab))
        {
            self.attribute_editor_tab = None;
        }
    }

    /// Re-binds the single material selection to the active shape or material tab, so the tab
    /// and the shape the Diffuse editor targets cannot diverge (the Channel Box combo may have
    /// moved the selection while this panel was hidden).
    pub(in crate::app) fn bind_material_shape_to_tab(&mut self) {
        if let Some(AttributeEditorTab::Shape(shape) | AttributeEditorTab::Material(shape)) =
            self.effective_attribute_editor_tab()
            && self.material_shape != Some(shape)
        {
            self.material_shape = Some(shape);
            self.sync_fields();
        }
    }

    pub(in crate::app) fn select_attribute_editor_tab(
        &mut self,
        tab: AttributeEditorTab,
    ) -> Result<OperationReply, OperationError> {
        if !self.sidebar_panel_visible(SidebarPanel::AttributeEditor) {
            return Err(OperationError::InvalidInput(
                "Attribute Editor tab requires the Attribute Editor to be the active sidebar panel"
                    .into(),
            ));
        }
        if let (AttributeEditorTab::Shape(shape) | AttributeEditorTab::Material(shape), Some(node)) = (
            tab,
            self.selected
                .and_then(|id| self.history.scene().node(id).ok()),
        ) && self
            .direct_shape_rank(node.transform_id(), shape)
            .is_some_and(|rank| rank >= MAX_SHAPE_TABS)
        {
            return Err(OperationError::InvalidInput(format!(
                "Attribute Editor shows at most {MAX_SHAPE_TABS} shape tabs per transform"
            )));
        }
        if !self.attribute_editor_tab_resolves(tab) {
            return Err(OperationError::InvalidInput(
                "Attribute Editor tab does not resolve for the selected node".into(),
            ));
        }
        let before = self.effective_attribute_editor_tab();
        self.attribute_editor_tab = Some(tab);
        if let AttributeEditorTab::Shape(shape) | AttributeEditorTab::Material(shape) = tab {
            self.material_shape = Some(shape);
            self.sync_fields();
        }
        Ok(if before != self.effective_attribute_editor_tab() {
            OperationReply::Changed
        } else {
            OperationReply::Unchanged
        })
    }
}

/// Most shape tabs drawn; a scene can parent an unbounded number of shapes to one transform.
pub(in crate::app) const MAX_SHAPE_TABS: usize = 16;

/// Ctrl+A target: Channel Box and Attribute Editor swap, and a hidden dock opens the Attribute
/// Editor. The result goes through `SetSidebarPanelVisible`, so the transition table decides.
pub(in crate::app) fn ctrl_a_target(dock: Option<SidebarPanel>) -> SidebarPanel {
    match dock {
        Some(SidebarPanel::AttributeEditor) => SidebarPanel::ChannelBox,
        Some(SidebarPanel::ChannelBox) | None => SidebarPanel::AttributeEditor,
    }
}

impl Editor {
    pub(in crate::app) fn attribute_editor(&mut self, ui: &mut egui::Ui) {
        ui.label(RichText::new("Attribute Editor").strong());
        let read_only_reason = self.history.scene().authoring_read_only_reason();
        let authoring = read_only_reason.is_none();
        if let Some(reason) = read_only_reason {
            ui.weak(reason.to_string());
        }
        if !self.history.scene().curves().is_empty() {
            ui.weak("Curves: select in the Outliner and frame with F. Curve attributes are unavailable.");
        }
        self.appearance_summary(ui);
        ui.separator();
        let Some(active) = self.effective_attribute_editor_tab() else {
            ui.weak("Select an object to edit its attributes.");
            return;
        };
        self.attribute_editor_tabs(ui, active);
        ui.separator();
        match active {
            AttributeEditorTab::Transform(_) => {
                self.transform_section(ui, authoring);
            }
            AttributeEditorTab::Shape(shape) => self.shape_section(ui, shape),
            AttributeEditorTab::Material(shape) => {
                ui.label(RichText::new("Materials").strong());
                ui.label("Common Material Attributes");
                self.material_section(ui, shape);
            }
        }
    }

    fn attribute_editor_tabs(&mut self, ui: &mut egui::Ui, active: AttributeEditorTab) {
        let scene = self.history.scene();
        let Some(node) = self.selected.and_then(|id| scene.node(id).ok()) else {
            return;
        };
        let shapes: Vec<_> = scene
            .dag_edges()
            .iter()
            .filter(|edge| edge.parent() == Some(node.transform_id()))
            .filter_map(|edge| scene.shape(edge.child()).ok())
            .collect();
        // Same order and cap as `direct_shape_rank`, which gates which tabs can be selected.
        let mut tabs = vec![(
            AttributeEditorTab::Transform(node.id()),
            node.name().to_owned(),
        )];
        for shape in shapes.iter().take(MAX_SHAPE_TABS) {
            tabs.push((
                AttributeEditorTab::Shape(shape.id()),
                shape.name().to_owned(),
            ));
            if scene.material_for_shape(shape.id()).is_ok() {
                tabs.push((
                    AttributeEditorTab::Material(shape.id()),
                    format!("{} material", shape.name()),
                ));
            }
        }
        let omitted = shapes.len().saturating_sub(MAX_SHAPE_TABS);
        ui.horizontal_wrapped(|ui| {
            for (tab, title) in tabs {
                if ui.selectable_label(tab == active, title).clicked() && tab != active {
                    self.attribute_editor_tab_request = Some(tab);
                }
            }
            ui.add_enabled(false, egui::Button::new("Pin Tab"))
                .on_disabled_hover_text("Pin Tab: not implemented yet");
        });
        if omitted > 0 {
            ui.weak(format!("{omitted} more shapes are not shown as tabs."));
        }
    }

    fn shape_section(&self, ui: &mut egui::Ui, shape: NativeId) {
        let Ok(shape) = self.history.scene().shape(shape) else {
            ui.weak("Shape unavailable.");
            return;
        };
        ui.label(format!("Shape: {}", shape.name()));
        ui.label(format!(
            "{} points, {} edges, {} faces",
            shape.mesh().point_count(),
            shape.mesh().edges().len(),
            shape.mesh().faces().len()
        ));
        ui.weak("Shape attributes are read-only in this slice.");
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod ui_tests;
