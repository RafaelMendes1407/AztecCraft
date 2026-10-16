use crate::app::Editor;
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
    enabled: bool,
    hover: &'static str,
}

const SELECTION_MODES: [StripButton; 3] = [
    StripButton {
        label: "Hierarchy selection",
        icon: UiIcon::SelectHierarchy,
        enabled: false,
        hover: "Hierarchy selection: not implemented yet",
    },
    StripButton {
        label: "Object selection",
        icon: UiIcon::SelectObject,
        enabled: false,
        hover: "Object selection: not implemented yet",
    },
    StripButton {
        label: "Component selection",
        icon: UiIcon::SelectComponent,
        enabled: false,
        hover: "Component selection: not implemented yet",
    },
];

/// Left to right as drawn at the right end of the strip.
const SIDEBAR_TOGGLES: [StripButton; 5] = [
    StripButton {
        label: "Modeling Toolkit",
        icon: UiIcon::SidebarModelingToolkit,
        enabled: false,
        hover: "Modeling Toolkit: not implemented yet",
    },
    StripButton {
        label: "HumanIK",
        icon: UiIcon::SidebarHumanIk,
        enabled: false,
        hover: "HumanIK: not implemented yet",
    },
    StripButton {
        label: "Attribute Editor",
        icon: UiIcon::SidebarAttributeEditor,
        enabled: false,
        hover: "Attribute Editor: not implemented yet",
    },
    StripButton {
        label: "Tool Settings",
        icon: UiIcon::SidebarToolSettings,
        enabled: false,
        hover: "Tool Settings: not implemented yet",
    },
    StripButton {
        label: "Channel Box / Layer Editor",
        icon: UiIcon::SidebarChannelBox,
        enabled: false,
        hover: "Channel Box / Layer Editor: not implemented yet",
    },
];

impl Editor {
    pub(in crate::app) fn status_line(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            menu_set_selector(ui);
            ui.separator();
            for button in &SELECTION_MODES {
                strip_button(ui, button);
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
    icon_button(
        ui,
        button.label,
        button.icon,
        button.enabled,
        false,
        button.hover,
    );
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
