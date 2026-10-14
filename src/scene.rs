use glam::{EulerRot, Mat4, Quat, Vec3};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Primitive {
    Cube,
    Plane,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    vertices: Vec<Vec3>,
    faces: Vec<[usize; 4]>,
}

impl Mesh {
    fn primitive(primitive: Primitive) -> Self {
        match primitive {
            Primitive::Cube => Self {
                vertices: vec![
                    Vec3::new(-0.5, -0.5, -0.5),
                    Vec3::new(0.5, -0.5, -0.5),
                    Vec3::new(0.5, 0.5, -0.5),
                    Vec3::new(-0.5, 0.5, -0.5),
                    Vec3::new(-0.5, -0.5, 0.5),
                    Vec3::new(0.5, -0.5, 0.5),
                    Vec3::new(0.5, 0.5, 0.5),
                    Vec3::new(-0.5, 0.5, 0.5),
                ],
                faces: vec![
                    [0, 3, 2, 1],
                    [4, 5, 6, 7],
                    [0, 4, 7, 3],
                    [1, 2, 6, 5],
                    [0, 1, 5, 4],
                    [3, 7, 6, 2],
                ],
            },
            Primitive::Plane => Self {
                vertices: vec![
                    Vec3::new(-0.5, 0.0, -0.5),
                    Vec3::new(-0.5, 0.0, 0.5),
                    Vec3::new(0.5, 0.0, 0.5),
                    Vec3::new(0.5, 0.0, -0.5),
                ],
                faces: vec![[0, 1, 2, 3]],
            },
        }
    }

    pub fn vertices(&self) -> &[Vec3] {
        &self.vertices
    }
    pub fn faces(&self) -> &[[usize; 4]] {
        &self.faces
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transform {
    pub translation: Vec3,
    pub rotation_degrees: Vec3,
    pub scale: Vec3,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation_degrees: Vec3::ZERO,
            scale: Vec3::ONE,
        }
    }
}

impl Transform {
    pub fn matrix(self) -> Mat4 {
        let r = self.rotation_degrees * (std::f32::consts::PI / 180.0);
        Mat4::from_scale_rotation_translation(
            self.scale,
            // Maya's row-vector RX * RY * RZ becomes RZ * RY * RX for glam columns.
            Quat::from_euler(EulerRot::XYZEx, r.x, r.y, r.z),
            self.translation,
        )
    }

    fn validate(self) -> Result<(), SceneError> {
        if !self.translation.is_finite()
            || !self.rotation_degrees.is_finite()
            || !self.scale.is_finite()
            || !self.matrix().is_finite()
        {
            return Err(SceneError::InvalidTransform);
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    id: NodeId,
    name: String,
    mesh: Mesh,
    transform: Transform,
}

impl Node {
    pub fn id(&self) -> NodeId {
        self.id
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn mesh(&self) -> &Mesh {
        &self.mesh
    }
    pub fn transform(&self) -> Transform {
        self.transform
    }
    pub fn world_vertices(&self) -> Vec<Vec3> {
        let matrix = self.transform.matrix();
        self.mesh
            .vertices
            .iter()
            .map(|v| matrix.transform_point3(*v))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "SceneData")]
pub struct Scene {
    nodes: Vec<Node>,
    next_id: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneData {
    nodes: Vec<Node>,
    next_id: u64,
}

impl TryFrom<SceneData> for Scene {
    type Error = SceneError;

    fn try_from(data: SceneData) -> Result<Self, Self::Error> {
        let scene = Self {
            nodes: data.nodes,
            next_id: data.next_id,
        };
        scene.validate()?;
        Ok(scene)
    }
}

impl Default for Scene {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            next_id: 1,
        }
    }
}

impl Scene {
    pub fn validate(&self) -> Result<(), SceneError> {
        let mut ids = std::collections::HashSet::new();
        if self.next_id == 0 {
            return Err(SceneError::InvalidIds);
        }
        for node in &self.nodes {
            if node.id.0 == 0 || node.id.0 >= self.next_id || !ids.insert(node.id) {
                return Err(SceneError::InvalidIds);
            }
            validate_name(&node.name)?;
            node.transform.validate()?;
            if node.mesh.vertices.iter().any(|v| !v.is_finite())
                || node.world_vertices().iter().any(|v| !v.is_finite())
            {
                return Err(SceneError::InvalidTransform);
            }
            for face in &node.mesh.faces {
                if face.iter().any(|i| *i >= node.mesh.vertices.len())
                    || face.iter().collect::<std::collections::HashSet<_>>().len() != 4
                {
                    return Err(SceneError::InvalidMesh);
                }
            }
            if node.mesh != Mesh::primitive(Primitive::Cube)
                && node.mesh != Mesh::primitive(Primitive::Plane)
            {
                return Err(SceneError::InvalidMesh);
            }
        }
        Ok(())
    }

    pub(crate) fn restore_authoring(&mut self, mut snapshot: Self) {
        // Undone creates remain reserved so stale editor selections cannot target a new node.
        snapshot.next_id = snapshot.next_id.max(self.next_id);
        *self = snapshot;
    }

    pub fn nodes(&self) -> &[Node] {
        &self.nodes
    }
    pub fn node(&self, id: NodeId) -> Result<&Node, SceneError> {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or(SceneError::MissingNode(id))
    }

    pub fn create(&mut self, primitive: Primitive, name: &str) -> Result<NodeId, SceneError> {
        validate_name(name)?;
        let next_id = self
            .next_id
            .checked_add(1)
            .ok_or(SceneError::IdsExhausted)?;
        let id = NodeId(self.next_id);
        self.nodes.push(Node {
            id,
            name: name.into(),
            mesh: Mesh::primitive(primitive),
            transform: Transform::default(),
        });
        self.next_id = next_id;
        Ok(id)
    }

    pub fn remove(&mut self, id: NodeId) -> Result<(), SceneError> {
        let index = self
            .nodes
            .iter()
            .position(|n| n.id == id)
            .ok_or(SceneError::MissingNode(id))?;
        self.nodes.remove(index);
        Ok(())
    }

    pub fn rename(&mut self, id: NodeId, name: &str) -> Result<(), SceneError> {
        validate_name(name)?;
        self.node_mut(id)?.name = name.into();
        Ok(())
    }

    pub fn set_transform(&mut self, id: NodeId, transform: Transform) -> Result<(), SceneError> {
        transform.validate()?;
        let node = self.node(id)?;
        if node
            .mesh
            .vertices
            .iter()
            .any(|v| !transform.matrix().transform_point3(*v).is_finite())
        {
            return Err(SceneError::InvalidTransform);
        }
        self.node_mut(id)?.transform = transform;
        Ok(())
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, SceneError> {
        self.nodes
            .iter_mut()
            .find(|n| n.id == id)
            .ok_or(SceneError::MissingNode(id))
    }
}

fn validate_name(name: &str) -> Result<(), SceneError> {
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        return Err(SceneError::InvalidName);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SceneError {
    MissingNode(NodeId),
    InvalidName,
    InvalidTransform,
    IdsExhausted,
    InvalidIds,
    InvalidMesh,
}

impl fmt::Display for SceneError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingNode(id) => write!(f, "scene node {} does not exist", id.0),
            Self::InvalidName => {
                f.write_str("node name must contain text and no control characters")
            }
            Self::InvalidTransform => {
                f.write_str("transform and transformed vertices must be finite")
            }
            Self::IdsExhausted => f.write_str("scene node IDs exhausted"),
            Self::InvalidIds => {
                f.write_str("scene node IDs must be unique, nonzero and below the allocator")
            }
            Self::InvalidMesh => {
                f.write_str("scene mesh must be a supported cube or plane with valid quad indices")
            }
        }
    }
}

impl std::error::Error for SceneError {}
