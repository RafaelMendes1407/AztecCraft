use crate::scene::{Primitive, SceneError};
use glam::Vec3;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mesh {
    vertices: Vec<Vec3>,
    faces: Vec<[usize; 4]>,
}

impl Mesh {
    pub(crate) fn primitive(primitive: Primitive) -> Self {
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

    pub(crate) fn validate(&self) -> Result<(), SceneError> {
        if self.vertices.iter().any(|v| !v.is_finite()) {
            return Err(SceneError::InvalidMesh);
        }
        for face in &self.faces {
            if face.iter().any(|i| *i >= self.vertices.len())
                || face
                    .iter()
                    .enumerate()
                    .any(|(i, index)| face[..i].contains(index))
            {
                return Err(SceneError::InvalidMesh);
            }
        }
        Ok(())
    }
}
