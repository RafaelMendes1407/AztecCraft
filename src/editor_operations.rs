#[cfg(test)]
use azteccraft::scene::Mesh;
use azteccraft::{
    scene::{NodeId, Primitive, SceneError, Transform},
    viewport::InvalidCameraInput,
};
use serde::Serialize;
use std::fmt;
#[cfg(test)]
use uuid::Uuid;

#[cfg(test)]
pub const TOOL_NAMES: [&str; 9] = [
    "inspect_scene",
    "create_primitive",
    "select_node",
    "rename_node",
    "set_transform",
    "delete_node",
    "undo",
    "redo",
    "frame_selected",
];

pub enum Operation {
    #[cfg(test)]
    Inspect,
    Create {
        primitive: Primitive,
        name: Option<String>,
    },
    Select(Option<NodeId>),
    Rename {
        id: NodeId,
        name: String,
    },
    SetTransform {
        id: NodeId,
        transform: Transform,
    },
    ExtrudeFace {
        id: NodeId,
        face_index: usize,
        distance: f32,
    },
    Delete(NodeId),
    Undo,
    Redo,
    FrameSelected,
}

#[derive(Debug, Serialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum OperationReply {
    #[cfg(test)]
    Scene {
        inspection: SceneInspection,
    },
    Created {
        node_id: String,
    },
    Changed,
    Unchanged,
}

#[derive(Debug, Serialize)]
#[cfg(test)]
pub struct SceneInspection {
    pub scene_token: String,
    pub nodes: Vec<NodeInspection>,
    pub selected_node: Option<String>,
    pub can_undo: bool,
    pub can_redo: bool,
    pub dirty: bool,
    pub has_draft: bool,
    pub text_focused: bool,
    pub modal: bool,
    pub camera: CameraInspection,
}

#[derive(Debug, Serialize)]
#[cfg(test)]
pub struct NodeInspection {
    pub id: String,
    pub name: String,
    pub transform: Transform,
    pub mesh: Mesh,
}

#[derive(Debug, Serialize)]
#[cfg(test)]
pub struct CameraInspection {
    pub target: [f32; 3],
    pub eye: [f32; 3],
    pub distance: f32,
}

#[derive(Debug)]
pub enum OperationError {
    #[cfg(test)]
    BusyEditing,
    #[cfg(test)]
    StaleScene,
    NoSelection,
    InvalidInput(String),
    Scene(SceneError),
    Camera(InvalidCameraInput),
}

impl fmt::Display for OperationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            #[cfg(test)]
            Self::BusyEditing => {
                f.write_str("Finish the native text edit or dialog before remote mutation")
            }
            #[cfg(test)]
            Self::StaleScene => f.write_str("Scene token is stale; inspect the current document"),
            Self::NoSelection => f.write_str("Select an object before framing"),
            Self::InvalidInput(message) => f.write_str(message),
            Self::Scene(error) => error.fmt(f),
            Self::Camera(error) => error.fmt(f),
        }
    }
}

impl std::error::Error for OperationError {}

#[cfg(test)]
pub struct RemoteRequest {
    pub expected_scene_token: Option<Uuid>,
    pub operation: Operation,
}
