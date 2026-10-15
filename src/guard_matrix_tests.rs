//! Guard matrix: today's behavior of every `Operation` through both dispatch pipelines.
//!
//! Each cell runs one sample operation on a freshly built editor in one state, through either the
//! native pipeline (`apply_operation_guarded`, provenance Native) or the remote pipeline
//! (`remote_operation_guarded`), and records the result plus every observable side effect that
//! differs from the state's baseline. The text lives in `tests/snapshots/guard_matrix.txt`; it is
//! generated from the code, never edited by hand (`UPDATE_SNAPSHOTS=1`).
//!
//! Adding an `Operation` variant fails to compile in `expand`, which matches without a wildcard.
//! The new arm must list the samples that exercise that variant.

use super::*;
use crate::editor_operations::{FileDecision, MoveDistance, RemoteRequest, TransformInput};
use azteccraft::history::{MoveFinish, MoveId};
use azteccraft::native_scene::{NativeScene, TransformTarget, WorldAxis};
use std::fmt::Write as _;

#[allow(dead_code)]
mod material_wire {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_binary.rs"
    ));
}
#[allow(dead_code)]
mod material_fixture {
    use super::material_wire as wire;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_appearance.rs"
    ));
}
mod ascii_fixture {
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/maya_ascii.rs"
    ));
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Pipeline {
    Native,
    Remote,
}

impl Pipeline {
    fn name(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Remote => "remote",
        }
    }
}

/// Editor conditions. `StaleToken` exists only for the remote pipeline, which alone carries a
/// scene token; `NoInstall*` run only the three operations that thread `may_install`.
#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Closing,
    Modal,
    LocalMove,
    RemoteMove,
    NameDraft,
    MaterialDraft,
    ChannelDraft,
    TextFocus,
    NativeInputPending,
    StaleToken,
    ReadOnlyAscii,
    BinaryDirty,
    NoInstall,
    NoInstallModal,
}

const STATES: [State; 15] = [
    State::Idle,
    State::Closing,
    State::Modal,
    State::LocalMove,
    State::RemoteMove,
    State::NameDraft,
    State::MaterialDraft,
    State::ChannelDraft,
    State::TextFocus,
    State::NativeInputPending,
    State::StaleToken,
    State::ReadOnlyAscii,
    State::BinaryDirty,
    State::NoInstall,
    State::NoInstallModal,
];

impl State {
    fn name(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Closing => "closing",
            Self::Modal => "modal",
            Self::LocalMove => "local_move",
            Self::RemoteMove => "remote_move",
            Self::NameDraft => "name_draft",
            Self::MaterialDraft => "material_draft",
            Self::ChannelDraft => "channel_draft",
            Self::TextFocus => "text_focus",
            Self::NativeInputPending => "native_input_pending",
            Self::StaleToken => "stale_token",
            Self::ReadOnlyAscii => "read_only_ascii",
            Self::BinaryDirty => "binary_dirty",
            Self::NoInstall => "no_install",
            Self::NoInstallModal => "no_install_modal",
        }
    }

    fn reachable(self, pipeline: Pipeline) -> bool {
        !(self == Self::StaleToken && pipeline == Pipeline::Native)
    }

    fn install_only(self) -> bool {
        matches!(self, Self::NoInstall | Self::NoInstallModal)
    }
}

/// Mutation applied after the state is built and before the operation runs.
#[derive(Clone, Copy)]
enum Pre {
    None,
    FaceMode,
}

struct Sample {
    label: &'static str,
    pre: Pre,
    install_probe: bool,
    operation: Operation,
}

fn sample(label: &'static str, operation: Operation) -> Sample {
    Sample {
        label,
        pre: Pre::None,
        install_probe: false,
        operation,
    }
}

struct Rig {
    editor: Editor,
    context: egui::Context,
    directory: tempfile::TempDir,
    token: Option<Uuid>,
    may_install: bool,
}

/// Identifiers the sample operations refer to, read from the built editor.
struct Targets {
    node: azteccraft::scene::NodeId,
    path: Vec<NativeId>,
    shape: NativeId,
    move_id: MoveId,
    pending_id: Uuid,
    ascii: std::path::PathBuf,
    save_as: std::path::PathBuf,
}

impl Targets {
    fn transform_target(&self) -> TransformTarget {
        TransformTarget {
            node: self.node,
            path: self.path.clone(),
        }
    }

    fn face(&self) -> FaceTarget {
        let mut path = self.path.clone();
        path.push(self.shape);
        FaceTarget {
            transform: self.node,
            shape: self.shape,
            path,
            face_index: 0,
        }
    }
}

fn targets(rig: &Rig) -> Targets {
    let editor = &rig.editor;
    let scene = editor.history.scene();
    let node = editor.selected.expect("every state selects a node");
    let shape = scene
        .shapes()
        .first()
        .map(|shape| shape.id())
        .or_else(|| scene.curves().first().map(|curve| curve.id()))
        .expect("every state has a shape or curve");
    Targets {
        node,
        path: editor.selected_path.clone(),
        shape,
        move_id: editor
            .history
            .active_move()
            .map_or_else(|| MoveId::from_decimal("999").unwrap(), |active| active.id),
        pending_id: editor
            .files
            .pending_inspection()
            .map_or(Uuid::nil(), |pending| pending.id.parse().unwrap()),
        ascii: rig.directory.path().join("curve.ma"),
        save_as: rig.directory.path().join("saved.mb"),
    }
}

fn cube_editor() -> Editor {
    let mut editor = Editor::default();
    editor.create(Primitive::Cube);
    let id = editor.selected.unwrap();
    editor
        .apply_operation(Operation::Rename {
            id,
            name: "renamed".into(),
        })
        .unwrap();
    editor.apply_operation(Operation::Undo).unwrap();
    editor
}

fn material_editor() -> Editor {
    let scene = azteccraft::native_scene::binary::decode(
        material_fixture::appearance_specimen_with_users(2).into(),
    )
    .unwrap();
    let mut editor = Editor {
        history: History::new(scene),
        ..Editor::default()
    };
    editor.selected = Some(editor.history.scene().nodes()[0].id());
    editor.selected_path = vec![editor.history.scene().nodes()[0].transform_id()];
    editor.sync_fields();
    editor
}

/// A dirty, writable Maya binary document: the only fixture whose Save paths reach the installer.
fn binary_editor(directory: &std::path::Path) -> Editor {
    let path = directory.join("original.mb");
    std::fs::write(&path, material_wire::specimen()).unwrap();
    let mut editor = Editor::default();
    editor
        .apply_operation(Operation::OpenScene { path })
        .unwrap();
    let node = editor.history.scene().nodes()[0].id();
    let path = editor.history.scene().transform_paths().unwrap()[0]
        .path
        .clone();
    editor
        .apply_operation(Operation::Select {
            node: Some(node),
            instance_path: Some(path),
        })
        .unwrap();
    let mut transform = editor
        .transform_input(editor.history.scene().nodes()[0].transform())
        .unwrap();
    transform.translation[0] += 1.0;
    editor
        .apply_operation(Operation::SetTransform {
            id: node,
            transform,
        })
        .unwrap();
    editor
}

fn pending_confirmation(editor: &mut Editor, operation: Operation) {
    assert!(matches!(
        editor.apply_operation(operation).unwrap(),
        OperationReply::PendingConfirmation { .. }
    ));
}

fn build(state: State) -> Rig {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(directory.path().join("curve.ma"), ascii_fixture::specimen()).unwrap();
    let context = egui::Context::default();
    let mut editor = match state {
        State::MaterialDraft => material_editor(),
        _ => cube_editor(),
    };
    let mut token = None;
    let mut may_install = true;
    match state {
        State::Idle => {}
        State::Closing => {
            pending_confirmation(&mut editor, Operation::RequestClose);
            let pending = editor.files.pending_inspection().unwrap();
            editor
                .apply_operation(Operation::ResolveFileAction {
                    pending_id: pending.id.parse().unwrap(),
                    decision: FileDecision::Discard,
                })
                .unwrap();
            assert!(editor.files.is_closing());
        }
        State::Modal => pending_confirmation(&mut editor, Operation::NewScene),
        State::NoInstallModal => {
            editor = binary_editor(directory.path());
            pending_confirmation(&mut editor, Operation::NewScene);
            may_install = false;
        }
        State::LocalMove | State::RemoteMove => {
            let provenance = if state == State::LocalMove {
                MoveProvenance::Native
            } else {
                MoveProvenance::Remote
            };
            editor.active_tool = ActiveTool::Move;
            let target = TransformTarget {
                node: editor.selected.unwrap(),
                path: editor.selected_path.clone(),
            };
            editor
                .apply_operation_guarded(
                    provenance,
                    Operation::BeginMove {
                        target,
                        axis: WorldAxis::X,
                    },
                    &|| true,
                )
                .unwrap();
            assert!(editor.history.active_move().is_some());
        }
        State::NameDraft => editor.name = "draft name".into(),
        State::MaterialDraft => {
            editor.material_draft.as_mut().unwrap().text = "0.5".into();
            assert!(editor.has_draft());
        }
        State::ChannelDraft => {
            editor.channels[0] = "5".into();
            assert!(editor.has_channel_draft());
        }
        State::TextFocus => {
            context.memory_mut(|memory| memory.request_focus(name_edit_id()));
        }
        State::NativeInputPending => editor.native_input_pending = true,
        State::StaleToken => token = Some(Uuid::nil()),
        State::ReadOnlyAscii => {
            editor = Editor::default();
            editor
                .apply_operation(Operation::OpenScene {
                    path: directory.path().join("curve.ma"),
                })
                .unwrap();
            let node = editor.history.scene().nodes()[0].id();
            let path = editor.history.scene().transform_paths().unwrap()[0]
                .path
                .clone();
            editor
                .apply_operation(Operation::Select {
                    node: Some(node),
                    instance_path: Some(path),
                })
                .unwrap();
            assert!(
                editor
                    .history
                    .scene()
                    .authoring_read_only_reason()
                    .is_some()
            );
        }
        State::BinaryDirty => editor = binary_editor(directory.path()),
        State::NoInstall => {
            editor = binary_editor(directory.path());
            may_install = false;
        }
    }
    Rig {
        editor,
        context,
        directory,
        token,
        may_install,
    }
}

/// One entry per operation kind. `Operation` is not `Clone`, so each cell rebuilds its samples.
fn seeds(t: &Targets) -> Vec<Operation> {
    vec![
        Operation::SetActiveTool(ActiveTool::Move),
        Operation::BeginMove {
            target: t.transform_target(),
            axis: WorldAxis::X,
        },
        Operation::UpdateMove {
            id: t.move_id,
            distance: MoveDistance::Centimeters(1.0),
        },
        Operation::FinishMove {
            id: t.move_id,
            finish: MoveFinish::Commit,
        },
        Operation::SetMaterialDiffuse {
            shape: t.shape,
            value: 0.25,
        },
        Operation::Inspect,
        Operation::Create {
            primitive: Primitive::Plane,
            name: None,
        },
        Operation::Select {
            node: Some(t.node),
            instance_path: Some(t.path.clone()),
        },
        Operation::SetSelectionMode(SelectionMode::Face),
        Operation::SelectFace {
            target: Some(t.face()),
        },
        Operation::NewScene,
        Operation::RequestClose,
        Operation::OpenScene {
            path: t.ascii.clone(),
        },
        Operation::SaveScene,
        Operation::SaveSceneAs {
            path: t.save_as.clone(),
        },
        Operation::ResolveFileAction {
            pending_id: t.pending_id,
            decision: FileDecision::Cancel,
        },
        Operation::Rename {
            id: t.node,
            name: "renamed".into(),
        },
        Operation::SetTransform {
            id: t.node,
            transform: TransformInput {
                translation: [1.0, 2.0, 3.0],
                rotation_degrees: [0.0; 3],
                scale: [1.0; 3],
            },
        },
        Operation::EditNode {
            id: t.node,
            name: None,
            translation: [Some(1.0), None, None],
            rotation_degrees: [None; 3],
            scale: [None; 3],
        },
        Operation::ExtrudeFace {
            target: t.face(),
            distance: 0.5,
        },
        Operation::Delete(t.node),
        Operation::Undo,
        Operation::Redo,
        Operation::FrameSelected,
        Operation::SetGridVisible { visible: false },
        Operation::Orbit {
            radians: [0.1, 0.0],
        },
        Operation::Pan {
            world_units: [1.0, 0.0],
        },
        Operation::Dolly { factor: 1.1 },
    ]
}

/// Samples for one operation kind. No wildcard arm: a new `Operation` variant must be added here.
fn expand(seed: Operation, t: &Targets) -> Vec<Sample> {
    match seed {
        Operation::SetActiveTool(_) => vec![
            sample(
                "set_active_tool(move)",
                Operation::SetActiveTool(ActiveTool::Move),
            ),
            sample(
                "set_active_tool(select)",
                Operation::SetActiveTool(ActiveTool::Select),
            ),
        ],
        Operation::BeginMove { .. } => vec![sample(
            "begin_move",
            Operation::BeginMove {
                target: t.transform_target(),
                axis: WorldAxis::X,
            },
        )],
        Operation::UpdateMove { .. } => vec![sample(
            "update_move",
            Operation::UpdateMove {
                id: t.move_id,
                distance: MoveDistance::Centimeters(1.0),
            },
        )],
        Operation::FinishMove { .. } => vec![sample(
            "finish_move(commit)",
            Operation::FinishMove {
                id: t.move_id,
                finish: MoveFinish::Commit,
            },
        )],
        Operation::SetMaterialDiffuse { .. } => vec![sample(
            "set_material_diffuse",
            Operation::SetMaterialDiffuse {
                shape: t.shape,
                value: 0.25,
            },
        )],
        Operation::Inspect => vec![sample("inspect", Operation::Inspect)],
        Operation::Create { .. } => vec![sample(
            "create(plane)",
            Operation::Create {
                primitive: Primitive::Plane,
                name: None,
            },
        )],
        Operation::Select { .. } => vec![
            sample(
                "select(node)",
                Operation::Select {
                    node: Some(t.node),
                    instance_path: Some(t.path.clone()),
                },
            ),
            sample(
                "select(none)",
                Operation::Select {
                    node: None,
                    instance_path: None,
                },
            ),
            sample(
                "select(none_with_path)",
                Operation::Select {
                    node: None,
                    instance_path: Some(t.path.clone()),
                },
            ),
        ],
        Operation::SetSelectionMode(_) => vec![sample(
            "set_selection_mode(face)",
            Operation::SetSelectionMode(SelectionMode::Face),
        )],
        Operation::SelectFace { .. } => vec![
            sample(
                "select_face(object_mode)",
                Operation::SelectFace {
                    target: Some(t.face()),
                },
            ),
            Sample {
                pre: Pre::FaceMode,
                ..sample(
                    "select_face(face_mode)",
                    Operation::SelectFace {
                        target: Some(t.face()),
                    },
                )
            },
        ],
        Operation::NewScene => vec![sample("new_scene", Operation::NewScene)],
        Operation::RequestClose => vec![sample("request_close", Operation::RequestClose)],
        Operation::OpenScene { .. } => vec![
            sample(
                "open_scene(ascii_curve)",
                Operation::OpenScene {
                    path: t.ascii.clone(),
                },
            ),
            sample(
                "open_scene(empty_path)",
                Operation::OpenScene {
                    path: std::path::PathBuf::new(),
                },
            ),
        ],
        Operation::SaveScene => vec![Sample {
            install_probe: true,
            ..sample("save_scene", Operation::SaveScene)
        }],
        Operation::SaveSceneAs { .. } => vec![Sample {
            install_probe: true,
            ..sample(
                "save_scene_as",
                Operation::SaveSceneAs {
                    path: t.save_as.clone(),
                },
            )
        }],
        Operation::ResolveFileAction { .. } => vec![
            sample(
                "resolve_file_action(cancel)",
                Operation::ResolveFileAction {
                    pending_id: t.pending_id,
                    decision: FileDecision::Cancel,
                },
            ),
            sample(
                "resolve_file_action(discard)",
                Operation::ResolveFileAction {
                    pending_id: t.pending_id,
                    decision: FileDecision::Discard,
                },
            ),
            Sample {
                install_probe: true,
                ..sample(
                    "resolve_file_action(save)",
                    Operation::ResolveFileAction {
                        pending_id: t.pending_id,
                        decision: FileDecision::Save,
                    },
                )
            },
        ],
        Operation::Rename { .. } => vec![sample(
            "rename",
            Operation::Rename {
                id: t.node,
                name: "renamed".into(),
            },
        )],
        Operation::SetTransform { .. } => vec![sample(
            "set_transform",
            Operation::SetTransform {
                id: t.node,
                transform: TransformInput {
                    translation: [1.0, 2.0, 3.0],
                    rotation_degrees: [0.0; 3],
                    scale: [1.0; 3],
                },
            },
        )],
        Operation::EditNode { .. } => vec![
            sample(
                "edit_node(translate_x)",
                Operation::EditNode {
                    id: t.node,
                    name: None,
                    translation: [Some(1.0), None, None],
                    rotation_degrees: [None; 3],
                    scale: [None; 3],
                },
            ),
            sample(
                "edit_node(empty)",
                Operation::EditNode {
                    id: t.node,
                    name: None,
                    translation: [None; 3],
                    rotation_degrees: [None; 3],
                    scale: [None; 3],
                },
            ),
        ],
        Operation::ExtrudeFace { .. } => vec![sample(
            "extrude_face",
            Operation::ExtrudeFace {
                target: t.face(),
                distance: 0.5,
            },
        )],
        Operation::Delete(_) => vec![sample("delete", Operation::Delete(t.node))],
        Operation::Undo => vec![sample("undo", Operation::Undo)],
        Operation::Redo => vec![sample("redo", Operation::Redo)],
        Operation::FrameSelected => vec![sample("frame_selected", Operation::FrameSelected)],
        Operation::SetGridVisible { .. } => vec![sample(
            "set_grid_visible(false)",
            Operation::SetGridVisible { visible: false },
        )],
        Operation::Orbit { .. } => vec![sample(
            "orbit",
            Operation::Orbit {
                radians: [0.1, 0.0],
            },
        )],
        Operation::Pan { .. } => vec![sample(
            "pan",
            Operation::Pan {
                world_units: [1.0, 0.0],
            },
        )],
        Operation::Dolly { .. } => vec![sample("dolly", Operation::Dolly { factor: 1.1 })],
    }
}

fn samples(rig: &Rig) -> Vec<Sample> {
    let t = targets(rig);
    seeds(&t)
        .into_iter()
        .flat_map(|seed| expand(seed, &t))
        .collect()
}

struct Observed {
    scene: NativeScene,
    fields: Vec<(&'static str, String)>,
}

fn observe(editor: &Editor, roots: &[&std::path::Path]) -> Observed {
    let scene = editor.history.scene();
    let selected = editor
        .selected
        .and_then(|id| scene.node(id).ok())
        .map_or("-", |node| node.name());
    let active_move = match editor.history.active_move() {
        None => "none",
        Some(active) if active.provenance == MoveProvenance::Native => "native",
        Some(_) => "remote",
    };
    let pending = editor.files.pending_inspection();
    let material = editor.material_draft.as_ref().map_or_else(
        || "-".to_owned(),
        |draft| format!("text={} intent={}", draft.text, draft.intent),
    );
    let flags = |values: &[bool; 9]| {
        values
            .iter()
            .map(|v| if *v { '1' } else { '0' })
            .collect::<String>()
    };
    let fields = vec![
        ("token", editor.scene_token.to_string()),
        ("nodes", scene.nodes().len().to_string()),
        ("selected", selected.to_owned()),
        ("selected_path_len", editor.selected_path.len().to_string()),
        ("selection_mode", format!("{:?}", editor.selection_mode)),
        (
            "face_remembered",
            editor.remembered_face.is_some().to_string(),
        ),
        ("active_tool", format!("{:?}", editor.active_tool)),
        ("grid", editor.grid_visible.to_string()),
        ("camera", format!("{:?}", editor.camera)),
        (
            "status",
            format!(
                "{:?}",
                super::snapshots::normalize_text(&editor.status, roots)
            ),
        ),
        ("status_is_error", editor.error.to_string()),
        ("active_move", active_move.to_owned()),
        ("native_move_drag", editor.native_move.is_some().to_string()),
        (
            "pointer_consumed_frame",
            editor.move_consumed_frame.to_string(),
        ),
        (
            "pointer_suppress_primary",
            editor.move_suppress_primary.to_string(),
        ),
        ("has_draft", editor.has_draft().to_string()),
        ("has_channel_draft", editor.has_channel_draft().to_string()),
        ("name_intent", editor.name_intent.to_string()),
        ("material_draft", material),
        ("name_field", format!("{:?}", editor.name)),
        ("channel_fields", editor.channels.join(",")),
        ("channel_intents", flags(&editor.channel_intents)),
        ("modal", editor.files.is_modal().to_string()),
        ("closing", editor.files.is_closing().to_string()),
        (
            "pending_action",
            pending.map_or("-".to_owned(), |pending| format!("{:?}", pending.kind)),
        ),
        ("dirty", editor.is_dirty().to_string()),
        ("can_undo", editor.history.can_undo().to_string()),
        ("can_redo", editor.history.can_redo().to_string()),
    ];
    Observed {
        scene: scene.clone(),
        fields,
    }
}

fn describe(result: &Result<OperationReply, OperationError>, roots: &[&std::path::Path]) -> String {
    match result {
        Ok(reply) => format!(
            "Ok({})",
            serde_json::to_value(reply).unwrap()["outcome"]
                .as_str()
                .unwrap()
        ),
        Err(error) => {
            let debug = format!("{error:?}");
            let variant: String = debug
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric())
                .collect();
            format!(
                "Err({variant}: {})",
                super::snapshots::normalize_text(&error.to_string(), roots)
            )
        }
    }
}

fn run_cell(state: State, pipeline: Pipeline, index: usize) -> (String, String) {
    let mut rig = build(state);
    let roots = [rig.directory.path()];
    let roots: Vec<std::path::PathBuf> = roots.iter().map(|path| path.to_path_buf()).collect();
    let root_refs: Vec<&std::path::Path> = roots.iter().map(|path| path.as_path()).collect();
    let mut chosen = samples(&rig).swap_remove(index);
    let label = chosen.label;
    if matches!(chosen.pre, Pre::FaceMode) {
        rig.editor.selection_mode = SelectionMode::Face;
    }
    let before = observe(&rig.editor, &root_refs);
    let may_install = rig.may_install;
    let operation = std::mem::replace(&mut chosen.operation, Operation::Inspect);
    let result = match pipeline {
        Pipeline::Native => {
            rig.editor
                .apply_operation_guarded(MoveProvenance::Native, operation, &move || may_install)
        }
        Pipeline::Remote => {
            let expected = rig.token.unwrap_or(rig.editor.scene_token);
            rig.editor.remote_operation_guarded(
                RemoteRequest {
                    expected_scene_token: Some(expected),
                    operation,
                },
                &rig.context,
                &move || may_install,
            )
        }
    };
    let after = observe(&rig.editor, &root_refs);
    let mut effects = Vec::new();
    if before.scene != after.scene {
        effects.push("scene changed".to_owned());
    }
    for ((name, left), (_, right)) in before.fields.iter().zip(&after.fields) {
        if left != right {
            if *name == "token" {
                effects.push("token replaced".into());
            } else if *name == "camera" {
                effects.push("camera changed".into());
            } else {
                effects.push(format!("{name}: {left} -> {right}"));
            }
        }
    }
    let effects = if effects.is_empty() {
        "no state change".to_owned()
    } else {
        effects.join("; ")
    };
    (
        label.to_owned(),
        format!("{} | {effects}", describe(&result, &root_refs)),
    )
}

fn baseline_text(state: State) -> String {
    let rig = build(state);
    let roots = [rig.directory.path()];
    let observed = observe(&rig.editor, &roots);
    let mut lines = vec![String::new()];
    for (name, value) in observed
        .fields
        .iter()
        .filter(|(name, _)| *name != "token" && *name != "camera")
    {
        let item = format!("{name}={value}");
        let line = lines.last_mut().unwrap();
        if line.len() + item.len() > 100 {
            lines.push(item);
        } else {
            if !line.is_empty() {
                line.push(' ');
            }
            line.push_str(&item);
        }
    }
    lines
        .iter()
        .map(|line| format!("    {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn matrix() -> (String, usize) {
    let mut text = String::new();
    text.push_str(
        "# Guard matrix (generated by guard_matrix_tests.rs; regenerate with UPDATE_SNAPSHOTS=1)\n\
         # cell: operation => result | effects relative to the state's baseline\n\
         # native = apply_operation_guarded(Native), remote = remote_operation_guarded\n",
    );
    let mut cells = 0;
    for state in STATES {
        for pipeline in [Pipeline::Native, Pipeline::Remote] {
            if !state.reachable(pipeline) {
                continue;
            }
            let rig = build(state);
            let all = samples(&rig);
            let selected: Vec<usize> = all
                .iter()
                .enumerate()
                .filter(|(_, sample)| !state.install_only() || sample.install_probe)
                .map(|(index, _)| index)
                .collect();
            drop(rig);
            write!(
                text,
                "\n## {} / {}\n  baseline:\n{}\n",
                state.name(),
                pipeline.name(),
                baseline_text(state)
            )
            .unwrap();
            for index in selected {
                let (label, cell) = run_cell(state, pipeline, index);
                writeln!(text, "  {label} => {cell}").unwrap();
                cells += 1;
            }
        }
    }
    writeln!(text, "\ncells: {cells}").unwrap();
    (text, cells)
}

#[test]
fn guard_matrix_matches_committed_snapshot() {
    // Save cells depend on the native writer, which is supported on Unix only.
    if !cfg!(unix) {
        return;
    }
    let (text, cells) = matrix();
    assert!(cells > 500, "matrix unexpectedly small: {cells}");
    super::snapshots::assert_snapshot("guard_matrix.txt", &text);
}

#[test]
fn guard_matrix_is_deterministic() {
    assert_eq!(matrix().0, matrix().0);
}
