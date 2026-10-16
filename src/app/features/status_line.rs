use crate::app::{Editor, SelectionModeIntent};
use crate::editor_operations::SelectionMode;
use crate::tool_icons::icon_button;
use azteccraft::ui_icons::UiIcon;
use eframe::egui::{self, Align, Layout};

const MENU_SETS: [&str; 5] = ["Modeling", "Rigging", "Animation", "FX", "Rendering"];
const ACTIVE_MENU_SET: &str = "Modeling";
const MENU_SET_LABEL: &str = "Menu set";
const MENU_SET_UNAVAILABLE: &str = "not implemented yet";

struct StripButton {
    label: &'static str,
    icon: UiIcon,
    hover: &'static str,
}

struct ModeButton {
    label: &'static str,
    icon: UiIcon,
    hover: &'static str,
    /// `None` while the mode is unsupported.
    target: Option<(SelectionModeIntent, SelectionMode)>,
}

const SELECTION_MODES: [ModeButton; 3] = [
    ModeButton {
        label: "Hierarchy selection",
        icon: UiIcon::SelectHierarchy,
        hover: "Hierarchy selection: not implemented yet",
        target: None,
    },
    ModeButton {
        label: "Object selection",
        icon: UiIcon::SelectObject,
        hover: "Object selection (F8 toggles)",
        target: Some((SelectionModeIntent::Object, SelectionMode::Object)),
    },
    ModeButton {
        label: "Component selection",
        icon: UiIcon::SelectComponent,
        hover: "Component selection (Face only; Vertex, Edge, UV unsupported)  F11",
        target: Some((SelectionModeIntent::Face, SelectionMode::Face)),
    },
];

/// Left to right as drawn at the right end of the strip.
const SIDEBAR_TOGGLES: [StripButton; 5] = [
    StripButton {
        label: "Modeling Toolkit",
        icon: UiIcon::SidebarModelingToolkit,
        hover: "Modeling Toolkit: not implemented yet",
    },
    StripButton {
        label: "HumanIK",
        icon: UiIcon::SidebarHumanIk,
        hover: "HumanIK: not implemented yet",
    },
    StripButton {
        label: "Attribute Editor",
        icon: UiIcon::SidebarAttributeEditor,
        hover: "Attribute Editor: not implemented yet",
    },
    StripButton {
        label: "Tool Settings",
        icon: UiIcon::SidebarToolSettings,
        hover: "Tool Settings: not implemented yet",
    },
    StripButton {
        label: "Channel Box / Layer Editor",
        icon: UiIcon::SidebarChannelBox,
        hover: "Channel Box / Layer Editor: not implemented yet",
    },
];

impl Editor {
    pub(in crate::app) fn status_line(
        &self,
        ui: &mut egui::Ui,
        selection_intent: &mut Option<SelectionModeIntent>,
    ) {
        let admitted = !self.files.is_modal() && !self.files.is_closing();
        ui.horizontal(|ui| {
            menu_set_selector(ui);
            ui.separator();
            for button in &SELECTION_MODES {
                let selected = button
                    .target
                    .is_some_and(|(_, mode)| mode == self.selection_mode);
                let enabled = admitted && button.target.is_some();
                let response = icon_button(
                    ui,
                    button.label,
                    button.icon,
                    enabled,
                    selected,
                    button.hover,
                );
                if let (true, Some((intent, _))) = (response.clicked(), button.target) {
                    selection_intent.get_or_insert(intent);
                }
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                for button in SIDEBAR_TOGGLES.iter().rev() {
                    strip_button(ui, button);
                }
            });
        });
    }
}

fn strip_button(ui: &mut egui::Ui, button: &StripButton) {
    icon_button(ui, button.label, button.icon, false, false, button.hover);
}

fn menu_set_selector(ui: &mut egui::Ui) {
    let combo = egui::ComboBox::from_id_salt("menu_set")
        .selected_text(ACTIVE_MENU_SET)
        .show_ui(ui, |ui| {
            for set in MENU_SETS {
                let active = set == ACTIVE_MENU_SET;
                let entry = ui.add_enabled(active, egui::Button::selectable(active, set));
                if !active {
                    entry.on_disabled_hover_text(MENU_SET_UNAVAILABLE);
                }
            }
        });
    combo.response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, MENU_SET_LABEL)
    });
}

#[cfg(test)]
#[path = "status_line/tests.rs"]
mod tests;
