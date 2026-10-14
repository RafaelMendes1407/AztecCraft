mod mesh;
pub use mesh::{DirectedEdge, NativeEdge, NativeFace, NativeMesh, Points};

use crate::scene::{NodeId, Primitive, Scene, SceneError, Transform};
use glam::Vec3;
use std::{fmt, ops::Range};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NativeId(u64);

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinearUnit {
    Centimeter,
    Millimeter,
    Meter,
    Inch,
    Foot,
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AngularUnit {
    Degree,
    Radian,
    Unsupported(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Units {
    pub linear: LinearUnit,
    pub angular: AngularUnit,
    pub time: String,
}

impl Default for Units {
    fn default() -> Self {
        Self {
            linear: LinearUnit::Centimeter,
            angular: AngularUnit::Degree,
            time: "film".into(),
        }
    }
}

impl Units {
    fn factors(&self) -> Result<(f64, f64), NativeError> {
        let linear = match self.linear {
            LinearUnit::Centimeter => 1.0,
            LinearUnit::Millimeter => 0.1,
            LinearUnit::Meter => 100.0,
            LinearUnit::Inch => 2.54,
            LinearUnit::Foot => 30.48,
            LinearUnit::Unsupported(_) => return Err(NativeError::UnsupportedView("linear unit")),
        };
        let angular = match self.angular {
            AngularUnit::Degree => 1.0,
            AngularUnit::Radian => 180.0 / std::f64::consts::PI,
            AngularUnit::Unsupported(_) => {
                return Err(NativeError::UnsupportedView("angular unit"));
            }
        };
        Ok((linear, angular))
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NativeTransform {
    pub translation: [f64; 3],
    pub rotation: [f64; 3],
    pub scale: [f64; 3],
}

impl Default for NativeTransform {
    fn default() -> Self {
        Self {
            translation: [0.0; 3],
            rotation: [0.0; 3],
            scale: [1.0; 3],
        }
    }
}

impl PartialEq for NativeTransform {
    fn eq(&self, other: &Self) -> bool {
        self.values().map(f64::to_bits) == other.values().map(f64::to_bits)
    }
}

impl NativeTransform {
    fn values(self) -> [f64; 9] {
        let [x, y, z] = self.translation;
        let [rx, ry, rz] = self.rotation;
        let [sx, sy, sz] = self.scale;
        [x, y, z, rx, ry, rz, sx, sy, sz]
    }
    fn from_values(v: [f64; 9]) -> Self {
        Self {
            translation: [v[0], v[1], v[2]],
            rotation: [v[3], v[4], v[5]],
            scale: [v[6], v[7], v[8]],
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Channel {
    TranslateX,
    TranslateY,
    TranslateZ,
    RotateX,
    RotateY,
    RotateZ,
    ScaleX,
    ScaleY,
    ScaleZ,
}

#[derive(Clone, Debug)]
struct Scalar {
    value: f64,
    source_lexeme: Option<String>,
    replaced: bool,
}

impl PartialEq for Scalar {
    fn eq(&self, other: &Self) -> bool {
        self.value.to_bits() == other.value.to_bits()
            && self.source_lexeme == other.source_lexeme
            && self.replaced == other.replaced
    }
}

impl Scalar {
    fn replace(&mut self, value: f64) {
        if self.value.to_bits() != value.to_bits() || self.source_lexeme.is_some() {
            self.value = value;
            self.replaced = true;
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeNode {
    id: NodeId,
    transform_id: NativeId,
    shape_id: NativeId,
    transform_uuid: Option<Uuid>,
    shape_uuid: Option<Uuid>,
    name: String,
    shape_name: String,
    channels: [Scalar; 9],
    mesh: NativeMesh,
    restrictions: Vec<String>,
    unsupported_transform: Vec<String>,
}

impl NativeNode {
    pub fn id(&self) -> NodeId {
        self.id
    }
    pub fn transform_id(&self) -> NativeId {
        self.transform_id
    }
    pub fn shape_id(&self) -> NativeId {
        self.shape_id
    }
    pub fn transform_uuid(&self) -> Option<Uuid> {
        self.transform_uuid
    }
    pub fn shape_uuid(&self) -> Option<Uuid> {
        self.shape_uuid
    }
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn shape_name(&self) -> &str {
        &self.shape_name
    }
    pub fn mesh(&self) -> &NativeMesh {
        &self.mesh
    }
    pub fn transform(&self) -> NativeTransform {
        NativeTransform::from_values(self.channels.each_ref().map(|v| v.value))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct DagEdge {
    parent: Option<NativeId>,
    child: NativeId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SourceRetention {
    bytes: Vec<u8>,
    records: Vec<Range<usize>>,
}

#[derive(Clone, Debug)]
pub struct NativeScene {
    nodes: Vec<NativeNode>,
    dag: Vec<DagEdge>,
    units: Units,
    source: Option<SourceRetention>,
    next_id: u64,
    next_native_id: u64,
}

impl PartialEq for NativeScene {
    fn eq(&self, other: &Self) -> bool {
        self.nodes == other.nodes
            && self.dag == other.dag
            && self.units == other.units
            && self.source == other.source
    }
}

impl Default for NativeScene {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            dag: Vec::new(),
            units: Units::default(),
            source: None,
            next_id: 1,
            next_native_id: 1,
        }
    }
}

impl NativeScene {
    pub fn from_legacy(scene: Scene) -> Result<Self, NativeError> {
        scene.validate()?;
        let mut native = Self {
            next_id: scene.next_id(),
            ..Self::default()
        };
        for node in scene.nodes() {
            let t = node.transform();
            let transform = NativeTransform {
                translation: t.translation.as_dvec3().to_array(),
                rotation: t.rotation_degrees.as_dvec3().to_array(),
                scale: t.scale.as_dvec3().to_array(),
            };
            native.append(
                node.id(),
                node.name(),
                NativeMesh::from_legacy(node.mesh()),
                transform,
            )?;
        }
        native.validate()?;
        Ok(native)
    }

    pub fn nodes(&self) -> &[NativeNode] {
        &self.nodes
    }
    pub fn units(&self) -> &Units {
        &self.units
    }
    pub fn node(&self, id: NodeId) -> Result<&NativeNode, NativeError> {
        self.nodes
            .iter()
            .find(|n| n.id == id)
            .ok_or(NativeError::Legacy(SceneError::MissingNode(id)))
    }
    pub fn restore_authoring(&mut self, mut snapshot: Self) {
        snapshot.next_id = snapshot.next_id.max(self.next_id);
        snapshot.next_native_id = snapshot.next_native_id.max(self.next_native_id);
        *self = snapshot;
    }

    pub fn create(&mut self, primitive: Primitive, name: &str) -> Result<NodeId, NativeError> {
        self.validate_new_name(None, name)?;
        let next = self
            .next_id
            .checked_add(1)
            .ok_or(SceneError::IdsExhausted)?;
        let id = NodeId::new(self.next_id);
        self.append(
            id,
            name,
            NativeMesh::primitive(primitive),
            NativeTransform::default(),
        )?;
        self.next_id = next;
        Ok(id)
    }

    fn append(
        &mut self,
        id: NodeId,
        name: &str,
        mesh: NativeMesh,
        transform: NativeTransform,
    ) -> Result<(), NativeError> {
        let next = self
            .next_native_id
            .checked_add(2)
            .ok_or(SceneError::IdsExhausted)?;
        let transform_id = NativeId(self.next_native_id);
        let shape_id = NativeId(self.next_native_id + 1);
        self.nodes.push(NativeNode {
            id,
            transform_id,
            shape_id,
            transform_uuid: Some(Uuid::new_v4()),
            shape_uuid: Some(Uuid::new_v4()),
            name: name.into(),
            shape_name: format!("{name}Shape"),
            channels: transform.values().map(|value| Scalar {
                value,
                source_lexeme: None,
                replaced: false,
            }),
            mesh,
            restrictions: Vec::new(),
            unsupported_transform: Vec::new(),
        });
        self.dag.extend([
            DagEdge {
                parent: None,
                child: transform_id,
            },
            DagEdge {
                parent: Some(transform_id),
                child: shape_id,
            },
        ]);
        self.next_native_id = next;
        Ok(())
    }

    fn editable(&self, id: NodeId) -> Result<usize, NativeError> {
        let index = self
            .nodes
            .iter()
            .position(|n| n.id == id)
            .ok_or(SceneError::MissingNode(id))?;
        let node = &self.nodes[index];
        if !node.restrictions.is_empty()
            || !node.unsupported_transform.is_empty()
            || !self.supported_dag(node)
        {
            return Err(NativeError::UnsupportedEdit);
        }
        Ok(index)
    }
    fn supported_dag(&self, node: &NativeNode) -> bool {
        let children: Vec<_> = self
            .dag
            .iter()
            .filter(|e| e.parent == Some(node.transform_id) || e.parent == Some(node.shape_id))
            .collect();
        let links: Vec<_> = self
            .dag
            .iter()
            .filter(|e| e.child == node.transform_id || e.child == node.shape_id)
            .collect();
        children.len() == 1
            && children[0].parent == Some(node.transform_id)
            && children[0].child == node.shape_id
            && links.len() == 2
            && links.contains(&&DagEdge {
                parent: None,
                child: node.transform_id,
            })
            && links.contains(&&DagEdge {
                parent: Some(node.transform_id),
                child: node.shape_id,
            })
    }
    fn validate_new_name(&self, id: Option<NodeId>, name: &str) -> Result<(), NativeError> {
        let mut chars = name.bytes();
        if !chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == b'_')
            || chars.any(|c| !c.is_ascii_alphanumeric() && c != b'_')
        {
            return Err(NativeError::InvalidName);
        }
        if self
            .nodes
            .iter()
            .any(|n| Some(n.id) != id && n.name == name)
        {
            return Err(NativeError::NameCollision);
        }
        Ok(())
    }
    pub fn rename(&mut self, id: NodeId, name: &str) -> Result<(), NativeError> {
        let index = self.editable(id)?;
        self.validate_new_name(Some(id), name)?;
        self.nodes[index].name = name.into();
        Ok(())
    }
    pub fn remove(&mut self, id: NodeId) -> Result<(), NativeError> {
        let index = self.editable(id)?;
        let node = self.nodes.remove(index);
        self.dag
            .retain(|e| e.child != node.transform_id && e.child != node.shape_id);
        Ok(())
    }
    pub fn set_transform(
        &mut self,
        id: NodeId,
        transform: NativeTransform,
    ) -> Result<(), NativeError> {
        let values = transform.values();
        if values.iter().any(|v| !v.is_finite()) {
            return Err(NativeError::NonFinite);
        }
        let index = self.editable(id)?;
        for (channel, value) in self.nodes[index].channels.iter_mut().zip(values) {
            channel.replace(value);
        }
        Ok(())
    }
    pub fn set_channel(
        &mut self,
        id: NodeId,
        channel: Channel,
        value: f64,
    ) -> Result<(), NativeError> {
        if !value.is_finite() {
            return Err(NativeError::NonFinite);
        }
        let index = self.editable(id)?;
        let target = &mut self.nodes[index].channels[channel as usize];
        target.replace(value);
        Ok(())
    }
    pub fn extrude_face(
        &mut self,
        id: NodeId,
        face: usize,
        distance: f64,
    ) -> Result<(), NativeError> {
        let index = self.editable(id)?;
        let mesh = self.nodes[index].mesh.extruded_face(face, distance)?;
        self.nodes[index].mesh = mesh;
        Ok(())
    }
    pub fn supported_view(&self) -> Result<Scene, NativeError> {
        let (linear, angular) = self.units.factors()?;
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            if !node.unsupported_transform.is_empty()
                || !self.supported_dag(node)
                || !node.restrictions.is_empty()
            {
                return Err(NativeError::UnsupportedView(
                    "DAG, transform or dependencies",
                ));
            }
            let t = node.transform();
            let transform = Transform {
                translation: convert(t.translation, linear)?,
                rotation_degrees: convert(t.rotation, angular)?,
                scale: convert(t.scale, 1.0)?,
            };
            nodes.push((
                node.id,
                node.name.clone(),
                node.mesh.view(linear)?,
                transform,
            ));
        }
        Scene::from_view(nodes, self.next_id)
            .map_err(|_| NativeError::UnsupportedView("f32 world coordinates"))
    }
    pub fn validate(&self) -> Result<(), NativeError> {
        let mut ids = std::collections::HashSet::new();
        let mut native_ids = std::collections::HashSet::new();
        if self.next_id == 0 || self.next_native_id == 0 {
            return Err(NativeError::InvalidIdentity);
        }
        for node in &self.nodes {
            let id = node.id.value();
            if id == 0 || id >= self.next_id || !ids.insert(node.id) {
                return Err(NativeError::InvalidIdentity);
            }
            for id in [node.transform_id, node.shape_id] {
                if id.0 == 0 || id.0 >= self.next_native_id || !native_ids.insert(id) {
                    return Err(NativeError::InvalidIdentity);
                }
            }
            if node.channels.iter().any(|v| !v.value.is_finite()) {
                return Err(NativeError::NonFinite);
            }
            node.mesh.validate()?;
        }
        if self.dag.iter().any(|e| {
            !native_ids.contains(&e.child) || e.parent.is_some_and(|p| !native_ids.contains(&p))
        }) {
            return Err(NativeError::InvalidIdentity);
        }
        if let Some(source) = &self.source {
            let mut end = 0;
            for range in &source.records {
                if range.start < end || range.end < range.start || range.end > source.bytes.len() {
                    return Err(NativeError::InvalidRetention);
                }
                end = range.end;
            }
        }
        Ok(())
    }
}

fn convert(values: [f64; 3], factor: f64) -> Result<Vec3, NativeError> {
    let value = Vec3::from_array(values.map(|v| (v * factor) as f32));
    if !value.is_finite() {
        return Err(NativeError::UnsupportedView("f32 coordinates"));
    }
    Ok(value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NativeError {
    Legacy(SceneError),
    NonFinite,
    InvalidIdentity,
    InvalidRetention,
    InvalidTopology,
    InvalidName,
    NameCollision,
    UnsupportedEdit,
    UnsupportedView(&'static str),
}
impl From<SceneError> for NativeError {
    fn from(value: SceneError) -> Self {
        Self::Legacy(value)
    }
}
impl fmt::Display for NativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Legacy(error) => error.fmt(f),
            Self::NonFinite => f.write_str("native authoring values must be finite"),
            Self::InvalidIdentity => f.write_str("invalid native identities or DAG references"),
            Self::InvalidRetention => f.write_str("invalid retained source ranges"),
            Self::InvalidTopology => f.write_str("invalid native polygon connectivity"),
            Self::InvalidName => {
                f.write_str("new names require the supported ASCII identifier subset")
            }
            Self::NameCollision => f.write_str("root transform name already exists"),
            Self::UnsupportedEdit => {
                f.write_str("native ownership or edit effects are unsupported")
            }
            Self::UnsupportedView(reason) => {
                write!(f, "native viewport representation unsupported: {reason}")
            }
        }
    }
}
impl std::error::Error for NativeError {}

#[cfg(test)]
mod tests;
