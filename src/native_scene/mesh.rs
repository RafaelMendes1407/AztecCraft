use super::{NativeError, convert};
use crate::{
    mesh::Mesh,
    scene::{Primitive, SceneError},
};
use glam::DVec3;

#[derive(Clone, Debug)]
pub enum Points {
    Float3(Vec<[f32; 3]>),
    Double3(Vec<[f64; 3]>),
}

impl PartialEq for Points {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Float3(a), Self::Float3(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.map(f32::to_bits) == b.map(f32::to_bits))
            }
            (Self::Double3(a), Self::Double3(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b)
                        .all(|(a, b)| a.map(f64::to_bits) == b.map(f64::to_bits))
            }
            _ => false,
        }
    }
}
impl Points {
    fn values(&self) -> Vec<DVec3> {
        match self {
            Self::Float3(points) => points
                .iter()
                .map(|p| DVec3::from_array(p.map(f64::from)))
                .collect(),
            Self::Double3(points) => points.iter().copied().map(DVec3::from_array).collect(),
        }
    }
    fn append(&mut self, added: [DVec3; 4]) -> Result<(), NativeError> {
        match self {
            Self::Float3(points) => {
                let converted = added.map(|p| p.as_vec3());
                if converted.iter().any(|p| !p.is_finite()) {
                    return Err(NativeError::NonFinite);
                }
                points.extend(converted.map(|p| p.to_array()));
            }
            Self::Double3(points) => points.extend(added.map(|p| p.to_array())),
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DirectedEdge {
    pub edge: usize,
    pub reversed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeEdge {
    endpoints: [usize; 2],
    smooth: bool,
}
impl NativeEdge {
    pub fn endpoints(&self) -> [usize; 2] {
        self.endpoints
    }
    pub fn smooth(&self) -> bool {
        self.smooth
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NativeFace {
    outer: Vec<DirectedEdge>,
    holes: Vec<Vec<DirectedEdge>>,
}
impl NativeFace {
    pub fn outer(&self) -> &[DirectedEdge] {
        &self.outer
    }
    pub fn holes(&self) -> &[Vec<DirectedEdge>] {
        &self.holes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NativeMesh {
    points: Points,
    edges: Vec<NativeEdge>,
    faces: Vec<NativeFace>,
    unsupported_channels: Vec<String>,
}
impl NativeMesh {
    pub fn points(&self) -> &Points {
        &self.points
    }
    pub fn edges(&self) -> &[NativeEdge] {
        &self.edges
    }
    pub fn faces(&self) -> &[NativeFace] {
        &self.faces
    }
    pub(super) fn primitive(primitive: Primitive) -> Self {
        Self::from_legacy(&Mesh::primitive(primitive))
    }
    pub(super) fn from_legacy(mesh: &Mesh) -> Self {
        let mut native = Self {
            points: Points::Float3(mesh.vertices().iter().map(|v| v.to_array()).collect()),
            edges: Vec::new(),
            faces: Vec::new(),
            unsupported_channels: Vec::new(),
        };
        for face in mesh.faces() {
            let outer = (0..4)
                .map(|i| native.edge([face[i], face[(i + 1) % 4]]))
                .collect();
            native.faces.push(NativeFace {
                outer,
                holes: Vec::new(),
            });
        }
        native
    }
    fn edge(&mut self, endpoints: [usize; 2]) -> DirectedEdge {
        for (edge, existing) in self.edges.iter().enumerate() {
            if existing.endpoints == endpoints {
                return DirectedEdge {
                    edge,
                    reversed: false,
                };
            }
            if existing.endpoints == [endpoints[1], endpoints[0]] {
                return DirectedEdge {
                    edge,
                    reversed: true,
                };
            }
        }
        self.append_edge(endpoints)
    }
    fn append_edge(&mut self, endpoints: [usize; 2]) -> DirectedEdge {
        let edge = self.edges.len();
        self.edges.push(NativeEdge {
            endpoints,
            smooth: false,
        });
        DirectedEdge {
            edge,
            reversed: false,
        }
    }
    fn endpoints(&self, edge: DirectedEdge) -> Result<[usize; 2], NativeError> {
        let points = self
            .edges
            .get(edge.edge)
            .ok_or(NativeError::InvalidTopology)?
            .endpoints;
        Ok(if edge.reversed {
            [points[1], points[0]]
        } else {
            points
        })
    }
    fn loop_vertices(&self, edges: &[DirectedEdge]) -> Result<Vec<usize>, NativeError> {
        if edges.len() < 3 {
            return Err(NativeError::InvalidTopology);
        }
        let points = edges
            .iter()
            .map(|e| self.endpoints(*e))
            .collect::<Result<Vec<_>, _>>()?;
        for i in 0..points.len() {
            if points[i][1] != points[(i + 1) % points.len()][0] {
                return Err(NativeError::InvalidTopology);
            }
        }
        Ok(points.into_iter().map(|p| p[0]).collect())
    }
    pub(super) fn validate(&self) -> Result<(), NativeError> {
        let points = self.points.values();
        if points.iter().any(|p| !p.is_finite()) {
            return Err(NativeError::NonFinite);
        }
        if self
            .edges
            .iter()
            .any(|e| e.endpoints.iter().any(|i| *i >= points.len()))
        {
            return Err(NativeError::InvalidTopology);
        }
        for face in &self.faces {
            self.loop_vertices(&face.outer)?;
            for hole in &face.holes {
                self.loop_vertices(hole)?;
            }
        }
        Ok(())
    }
    pub(super) fn view(&self, factor: f64) -> Result<Mesh, NativeError> {
        if !self.unsupported_channels.is_empty() {
            return Err(NativeError::UnsupportedView("mesh channels"));
        }
        let vertices = self
            .points
            .values()
            .into_iter()
            .map(|p| convert(p.to_array(), factor))
            .collect::<Result<Vec<_>, _>>()?;
        let mut faces = Vec::new();
        for face in &self.faces {
            if !face.holes.is_empty() {
                return Err(NativeError::UnsupportedView("polygon holes"));
            }
            let vertices = self.loop_vertices(&face.outer)?;
            let quad: [usize; 4] = vertices
                .try_into()
                .map_err(|_| NativeError::UnsupportedView("non-quad polygon"))?;
            faces.push(quad);
        }
        Mesh::from_view(vertices, faces)
            .map_err(|_| NativeError::UnsupportedView("quad connectivity"))
    }
    pub(super) fn extruded_face(
        &self,
        face_index: usize,
        distance: f64,
    ) -> Result<Self, NativeError> {
        if !distance.is_finite() {
            return Err(SceneError::InvalidExtrusionDistance.into());
        }
        let face = self
            .faces
            .get(face_index)
            .ok_or(SceneError::MissingFace(face_index))?;
        if !self.unsupported_channels.is_empty() || !face.holes.is_empty() {
            return Err(NativeError::UnsupportedEdit);
        }
        if self.faces.len() == 1 {
            return Err(SceneError::UnsupportedAllFacesExtrusion.into());
        }
        let vertices: [usize; 4] = self
            .loop_vertices(&face.outer)?
            .try_into()
            .map_err(|_| NativeError::UnsupportedEdit)?;
        if vertices
            .iter()
            .enumerate()
            .any(|(i, v)| vertices[..i].contains(v))
        {
            return Err(NativeError::UnsupportedEdit);
        }
        let points = self.points.values();
        let delta = crate::mesh::quad_extrusion(
            vertices,
            vertices.map(|i| points[i]),
            points.len(),
            distance,
        )?;
        let mut result = self.clone();
        result.points.append(delta.added)?;
        let vertical: [_; 4] =
            std::array::from_fn(|i| result.append_edge([vertices[i], delta.cap[i]]));
        let top: [_; 4] =
            std::array::from_fn(|i| result.append_edge([delta.cap[i], delta.cap[(i + 1) % 4]]));
        result.faces[face_index].outer = top.to_vec();
        for i in 0..4 {
            let next = (i + 1) % 4;
            result.faces.push(NativeFace {
                outer: vec![
                    face.outer[i],
                    vertical[next],
                    DirectedEdge {
                        reversed: true,
                        ..top[i]
                    },
                    DirectedEdge {
                        reversed: true,
                        ..vertical[i]
                    },
                ],
                holes: Vec::new(),
            });
        }
        result.validate()?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests;
