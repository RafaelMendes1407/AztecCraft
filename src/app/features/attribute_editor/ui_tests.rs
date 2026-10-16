use super::*;
use crate::app::tests::frame_with;
use crate::app::{channel_edit_id, text_edit_focused};
use crate::editor_operations::SidebarPanel::{AttributeEditor as AE, ChannelBox as CB};
use eframe::egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2, accesskit};

const SIZE: Vec2 = egui::vec2(960.0, 640.0);

fn setup() -> (Editor, egui::Context) {
    let context = egui::Context::default();
    context.enable_accesskit();
    let mut editor = Editor {
        renderer: Some(crate::viewport_renderer::tests::renderer_handle()),
        ..Editor::default()
    };
    editor.create(Primitive::Cube);
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    (editor, context)
}

fn key_event(key: Key, modifiers: Modifiers) -> Vec<Event> {
    vec![
        Event::ModifiersChanged(modifiers),
        Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers,
        },
        Event::ModifiersChanged(Modifiers::NONE),
    ]
}

/// Presses then releases `A` like a keyboard; returns the press frame's output.
fn press_a(editor: &mut Editor, context: &egui::Context, modifiers: Modifiers) -> egui::FullOutput {
    let output = frame_with(editor, context, SIZE, 1.0, key_event(Key::A, modifiers));
    frame_with(
        editor,
        context,
        SIZE,
        1.0,
        vec![Event::Key {
            key: Key::A,
            physical_key: None,
            pressed: false,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    output
}

fn ctrl_a(editor: &mut Editor, context: &egui::Context) -> egui::FullOutput {
    press_a(editor, context, Modifiers::CTRL)
}

fn text_drawn(output: &egui::FullOutput, text: &str) -> bool {
    output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::Shape::Text(drawn) if drawn.galley.job.text == text),
    )
}

/// Sidebar panels drawn this frame, by their header text.
fn panels_drawn(output: &egui::FullOutput) -> (bool, bool) {
    (
        text_drawn(output, "Channel Box"),
        text_drawn(output, "Attribute Editor"),
    )
}

fn label_center(output: &egui::FullOutput, label: &str) -> Pos2 {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some(label))
        .unwrap_or_else(|| panic!("missing control {label}"))
        .1
        .bounds()
        .map(|bounds| {
            Pos2::new(
                ((bounds.x0 + bounds.x1) / 2.0) as f32,
                ((bounds.y0 + bounds.y1) / 2.0) as f32,
            )
        })
        .unwrap()
}

#[test]
fn ctrl_a_target_follows_the_transition_table() {
    assert_eq!(ctrl_a_target(Some(CB)), AE);
    assert_eq!(ctrl_a_target(Some(AE)), CB);
    assert_eq!(ctrl_a_target(None), AE);
    for (current, target) in [(Some(CB), AE), (Some(AE), CB), (None, AE)] {
        let mut editor = Editor {
            sidebar_dock: current,
            ..Editor::default()
        };
        editor
            .apply_operation(Operation::SetSidebarPanelVisible {
                panel: ctrl_a_target(current),
                visible: true,
            })
            .unwrap();
        assert_eq!(editor.sidebar_dock, Some(target));
    }
}

#[test]
fn ctrl_a_cycles_the_dock_and_exactly_one_panel_is_drawn_per_frame() {
    let (mut editor, context) = setup();
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert_eq!(panels_drawn(&output), (true, false));
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(AE));
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert_eq!(panels_drawn(&output), (false, true));
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(CB));
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert_eq!(panels_drawn(&output), (true, false));
    editor
        .apply_operation(Operation::SetSidebarPanelVisible {
            panel: CB,
            visible: false,
        })
        .unwrap();
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert_eq!(panels_drawn(&output), (false, false));
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(AE));
}

#[test]
fn ctrl_a_ignores_other_modifiers_text_focus_and_modal() {
    let (mut editor, context) = setup();
    for modifiers in [
        Modifiers::NONE,
        Modifiers::SHIFT,
        Modifiers::ALT,
        Modifiers::MAC_CMD,
        Modifiers::CTRL | Modifiers::SHIFT,
    ] {
        press_a(&mut editor, &context, modifiers);
        assert_eq!(editor.sidebar_dock, Some(CB), "{modifiers:?}");
    }
    context.memory_mut(|memory| memory.request_focus(channel_edit_id(0)));
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert!(text_edit_focused(&context));
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(CB), "text focus keeps select-all");
    context.memory_mut(|memory| memory.surrender_focus(channel_edit_id(0)));
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert!(!text_edit_focused(&context));
    editor.new_document().unwrap();
    assert!(editor.files.is_modal());
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(CB), "modal blocks the shortcut");
}

#[test]
fn ctrl_a_with_a_pending_draft_commits_a_valid_one_and_refuses_an_invalid_one() {
    let (mut editor, context) = setup();
    let scene = editor.history.scene().clone();
    editor.channels[0] = "invalid".into();
    editor.channel_intents[0] = true;
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(CB));
    assert!(editor.error);
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.channels[0], "invalid");
    editor.channels[0] = "4".into();
    ctrl_a(&mut editor, &context);
    assert_eq!(editor.sidebar_dock, Some(AE));
    assert_eq!(
        editor.history.scene().nodes()[0].transform().translation[0],
        4.0
    );
}

#[test]
fn attribute_editor_edits_share_the_channel_drafts_and_history() {
    let (mut editor, context) = setup();
    ctrl_a(&mut editor, &context);
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert_eq!(panels_drawn(&output), (false, true));
    let scene = editor.history.scene().clone();
    // An untouched field never writes back.
    context.memory_mut(|memory| memory.request_focus(channel_edit_id(1)));
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    frame_with(
        &mut editor,
        &context,
        SIZE,
        1.0,
        key_event(Key::Enter, Modifiers::NONE),
    );
    assert_eq!(editor.history.scene(), &scene);
    // A typed edit commits through the same draft and widget id as the Channel Box.
    context.memory_mut(|memory| memory.request_focus(channel_edit_id(0)));
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    frame_with(
        &mut editor,
        &context,
        SIZE,
        1.0,
        vec![Event::Text("5".into())],
    );
    assert!(editor.has_channel_draft());
    frame_with(
        &mut editor,
        &context,
        SIZE,
        1.0,
        key_event(Key::Enter, Modifiers::NONE),
    );
    frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert_eq!(
        editor.history.scene().nodes()[0].transform().translation[0],
        5.0
    );
    editor.history_step(false);
    assert_eq!(editor.history.scene(), &scene);
    assert_eq!(editor.sidebar_dock, Some(AE));
    editor.history_step(true);
    assert_eq!(
        editor.history.scene().nodes()[0].transform().translation[0],
        5.0
    );
}

#[test]
fn clicking_a_shape_tab_selects_it_and_the_tab_survives_undo() {
    let (mut editor, context) = setup();
    ctrl_a(&mut editor, &context);
    let shape = editor.history.scene().shapes()[0].id();
    let name = editor.history.scene().shapes()[0].name().to_owned();
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let at = label_center(&output, &name);
    frame_with(
        &mut editor,
        &context,
        SIZE,
        1.0,
        vec![Event::PointerMoved(at)],
    );
    for pressed in [true, false] {
        frame_with(
            &mut editor,
            &context,
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
    assert_eq!(
        editor.attribute_editor_tab,
        Some(AttributeEditorTab::Shape(shape))
    );
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    assert!(text_drawn(&output, &format!("Shape: {name}")));
    let pin = output
        .platform_output
        .accesskit_update
        .as_ref()
        .unwrap()
        .nodes
        .iter()
        .find(|(_, node)| node.label() == Some("Pin Tab"))
        .expect("Pin Tab is drawn")
        .1
        .is_disabled();
    assert!(pin);
    editor
        .apply_operation(Operation::Rename {
            id: editor.selected.unwrap(),
            name: "renamed".into(),
        })
        .unwrap();
    editor.history_step(false);
    assert_eq!(
        editor.attribute_editor_tab,
        Some(AttributeEditorTab::Shape(shape))
    );
}

#[test]
fn the_status_line_toggle_drives_the_attribute_editor_dock() {
    let (mut editor, context) = setup();
    let output = frame_with(&mut editor, &context, SIZE, 1.0, vec![]);
    let at = label_center(&output, "Attribute Editor");
    for expected in [Some(AE), None] {
        frame_with(
            &mut editor,
            &context,
            SIZE,
            1.0,
            vec![Event::PointerMoved(at)],
        );
        for pressed in [true, false] {
            frame_with(
                &mut editor,
                &context,
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
        assert_eq!(editor.sidebar_dock, expected);
    }
    let _ = accesskit::Role::Button;
}
