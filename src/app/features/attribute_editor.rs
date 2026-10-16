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
            AttributeEditorTab::Shape(shape) => self.is_direct_shape(node.transform_id(), shape),
            AttributeEditorTab::Material(shape) => {
                self.is_direct_shape(node.transform_id(), shape)
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

#[cfg(test)]
mod tests;
