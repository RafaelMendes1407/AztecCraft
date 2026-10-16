use super::*;
use crate::app::tests::frame_with;
use crate::editor_operations::SelectionMode;
use azteccraft::scene::NodeId;
use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, Rect, Vec2, accesskit};

const SIZE: Vec2 = egui::vec2(960.0, 640.0);
const MODES: [&str; 3] = [
    "Hierarchy selection",
    "Object selection",
    "Component selection",
];
const TOGGLES: [&str; 5] = [
    "Modeling Toolkit",
    "HumanIK",
    "Attribute Editor",
    "Tool Settings",
    "Channel Box / Layer Editor",
];

struct Control {
    role: accesskit::Role,
    label: String,
    id: accesskit::NodeId,
    rect: Rect,
    disabled: bool,
}

fn controls(output: &egui::FullOutput) -> Vec<Control> {
    let update = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accesskit enabled");
    update
        .nodes
        .iter()
        .filter_map(|(id, node)| {
            let bounds = node.bounds()?;
            Some(Control {
                role: node.role(),
                label: node.label()?.to_owned(),
                id: *id,
                rect: Rect::from_min_max(
                    Pos2::new(bounds.x0 as f32, bounds.y0 as f32),
                    Pos2::new(bounds.x1 as f32, bounds.y1 as f32),
                ),
                disabled: node.is_disabled(),
            })
        })
        .collect()
}

fn strip(output: &egui::FullOutput) -> Vec<Control> {
    let mut found: Vec<Control> = controls(output)
        .into_iter()
        .filter(|c| {
            c.role == accesskit::Role::ComboBox && c.label == "Menu set"
                || MODES.contains(&c.label.as_str())
                || TOGGLES.contains(&c.label.as_str())
        })
        .collect();
    found.sort_by(|a, b| a.rect.left().total_cmp(&b.rect.left()));
    found
}

fn setup() -> (Editor, egui::Context) {
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut editor = Editor {
        renderer: Some(crate::viewport_renderer::tests::renderer_handle()),
        ..Editor::default()
    };
    editor.create(azteccraft::scene::Primitive::Cube);
    (editor, context)
}

fn key(key: Key) -> Event {
    Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    }
}

fn click(editor: &mut Editor, context: &egui::Context, at: Pos2) {
    frame_with(editor, context, SIZE, 1.0, vec![Event::PointerMoved(at)]);
    for pressed in [true, false] {
        frame_with(
            editor,
            context,
            SIZE,
            1.0,
            vec![Event::PointerButton {
                pos: at,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            }],
        );
    }
}

/// Every state the strip must leave alone when a disabled control is used.
fn snapshot(editor: &Editor) -> (uuid::Uuid, String, bool, SelectionMode, Option<NodeId>) {
    (
        editor.scene_token,
        editor.status.clone(),
        editor.error,
        editor.selection_mode,
        editor.selected,
    )
}

#[test]
fn strip_fits_the_minimum_window_at_every_scale_and_enlarged_text() {
    for (ppp, font) in [(1.0, 14.0), (1.25, 14.0), (2.0, 14.0), (1.0, 18.0)] {
        let (mut editor, context) = setup();
        let mut style = egui::Style {
            override_font_id: Some(egui::FontId::proportional(font)),
            ..Default::default()
        };
        style.spacing.button_padding = egui::vec2(8.0, 4.0);
        context.set_style_of(egui::Theme::Dark, style);
        context.set_theme(egui::Theme::Dark);
        frame_with(&mut editor, &context, SIZE, ppp, vec![]);
        let output = frame_with(&mut editor, &context, SIZE, ppp, vec![]);
        let found = strip(&output);
        assert_eq!(found.len(), 9, "ppp {ppp} font {font}");
        let margin = egui::Frame::side_top_panel(&context.global_style()).total_margin();
        for control in &found {
            assert!(
                control.rect.left() >= margin.left - 0.5
                    && control.rect.right() <= SIZE.x - margin.right + 0.5,
                "ppp {ppp} font {font}: {} at {:?}",
                control.label,
                control.rect
            );
        }
        for pair in found.windows(2) {
            assert!(
                pair[0].rect.right() <= pair[1].rect.left() + 0.5,
                "ppp {ppp} font {font}: {} overlaps {}",
                pair[0].label,
                pair[1].label
            );
        }
    }
}

#[test]
fn strip_elements_are_in_documented_order() {
    let (mut editor, context) = setup();
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let labels: Vec<String> = strip(&output).into_iter().map(|c| c.label).collect();
    let expected: Vec<&str> = ["Menu set"]
        .into_iter()
        .chain(MODES)
        .chain(TOGGLES)
        .collect();
    assert_eq!(labels, expected);
}

#[test]
fn sidebar_cluster_hugs_the_right_edge_at_a_wide_window() {
    let wide = egui::vec2(1440.0, 900.0);
    let (mut editor, context) = setup();
    let output = frame_with(&mut editor, &context, wide, 1.0, vec![]);
    let last = strip(&output).pop().expect("strip controls");
    assert_eq!(last.label, TOGGLES[4]);
    let margin = egui::Frame::side_top_panel(&context.global_style()).total_margin();
    let edge = wide.x - margin.right;
    let spacing = context.global_style().spacing.item_spacing.x;
    assert!(
        (edge - last.rect.right()).abs() <= spacing,
        "right edge {} vs strip edge {edge}",
        last.rect.right()
    );
}

#[test]
fn clicking_disabled_strip_buttons_changes_nothing() {
    let (mut editor, context) = setup();
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let before = snapshot(&editor);
    for control in strip(&output)
        .iter()
        .filter(|c| c.role == accesskit::Role::Button)
    {
        assert!(control.disabled, "{}", control.label);
        click(&mut editor, &context, control.rect.center());
        assert_eq!(snapshot(&editor), before, "{}", control.label);
    }
}

#[test]
fn open_combo_popup_blocks_shortcuts_and_disabled_sets_stay_disabled() {
    let (mut editor, context) = setup();
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let combo = strip(&output).remove(0);
    let before = snapshot(&editor);
    click(&mut editor, &context, combo.rect.center());
    assert!(egui::Popup::is_any_open(&context));
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    for set in ["Rigging", "Animation", "FX", "Rendering"] {
        let entry = controls(&output)
            .into_iter()
            .find(|c| c.label == set)
            .unwrap_or_else(|| panic!("set {set}"));
        assert!(entry.disabled, "{set}");
    }
    frame_with(&mut editor, &context, SIZE, 1.0, vec![key(Key::F11)]);
    assert_eq!(editor.selection_mode, before.3, "F11 reached a shortcut");
    assert_eq!(snapshot(&editor), before);
}

#[test]
fn shortcut_baseline_changes_mode_without_the_popup() {
    let (mut editor, context) = setup();
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let before = editor.selection_mode;
    frame_with(&mut editor, &context, SIZE, 1.0, vec![key(Key::F11)]);
    assert_ne!(editor.selection_mode, before);
}

#[test]
fn every_strip_control_has_its_label_and_disabled_state() {
    let (mut editor, context) = setup();
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let found = strip(&output);
    for name in MODES.iter().chain(TOGGLES.iter()) {
        let control = found
            .iter()
            .find(|c| c.label == *name)
            .unwrap_or_else(|| panic!("missing AccessKit label {name}"));
        assert_eq!(control.role, accesskit::Role::Button, "{name}");
        assert!(control.disabled, "{name}");
    }
    let combo = found
        .iter()
        .find(|c| c.role == accesskit::Role::ComboBox)
        .expect("menu set combo");
    assert!(!combo.disabled);
}

/// Tabs until the menu set combo has keyboard focus, failing if a disabled button takes it.
fn tab_to_combo(editor: &mut Editor, context: &egui::Context) {
    for _ in 0..40 {
        let output = frame_with(editor, context, SIZE, 1.0, vec![key(Key::Tab)]);
        let focus = output
            .platform_output
            .accesskit_update
            .as_ref()
            .map(|update| update.focus);
        let found = strip(&output);
        let focused = found.iter().find(|c| Some(c.id) == focus);
        match focused {
            Some(c) if c.role == accesskit::Role::ComboBox => return,
            Some(c) => panic!("Tab focused disabled strip button {}", c.label),
            None => {}
        }
    }
    panic!("Tab never reached the combo");
}

#[test]
fn keyboard_activates_the_enabled_combo_and_skips_disabled_buttons() {
    let (mut editor, context) = setup();
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    tab_to_combo(&mut editor, &context);
    let mut reached_shelf = false;
    for _ in 0..40 {
        let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![key(Key::Tab)]);
        let focus = output
            .platform_output
            .accesskit_update
            .as_ref()
            .map(|u| u.focus);
        let focused = controls(&output).into_iter().find(|c| Some(c.id) == focus);
        if let Some(c) = focused {
            assert!(
                !MODES.contains(&c.label.as_str()) && !TOGGLES.contains(&c.label.as_str()),
                "Tab focused disabled {}",
                c.label
            );
            if c.label == "Cube" {
                reached_shelf = true;
                break;
            }
        }
    }
    assert!(
        reached_shelf,
        "Tab never passed the disabled buttons to the shelf"
    );
    let before = snapshot(&editor);
    for activate in [Key::Space, Key::Enter] {
        tab_to_combo(&mut editor, &context);
        frame_with(&mut editor, &context, SIZE, 1.0, vec![key(activate)]);
        assert!(egui::Popup::is_any_open(&context), "{activate:?} opens");
        frame_with(&mut editor, &context, SIZE, 1.0, vec![key(Key::Escape)]);
        assert!(!egui::Popup::is_any_open(&context));
    }
    // Existing behaviour, tracked in #95: cancel_on_escape (panels.rs) is not gated on
    // Popup::is_any_open. A fix should flip this assertion deliberately.
    assert_eq!(editor.status, "Edit cancelled");
    let after = snapshot(&editor);
    assert_eq!(
        (after.0, after.2, after.3, after.4),
        (before.0, before.2, before.3, before.4)
    );
}
