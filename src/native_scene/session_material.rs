//! Session-only lambert materials assigned to whole authored shapes.
//!
//! The table is not saved and never touches source appearance: sourced scenes keep their
//! `MaterialDiffuse` path and refuse session edits. The initial shading group's default shader
//! is unverified for Maya 2026, so its colour is a placeholder equal to the viewport base fill.

use super::{NativeError, NativeId, NativeMeshShape, NativeScene};
use crate::shading::SURFACE_BASE_RGB;
use std::collections::{BTreeMap, HashSet};

/// Arbitrary bound on materials per scene, chosen to keep validation and snapshots cheap.
pub const MAX_SESSION_MATERIALS: usize = 4096;

/// An index into the current table. It is valid only at the current history position: undoing a
/// create frees its index for the next create, so callers must take ids from operation outcomes
/// and never cache them across edits.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SessionMaterialId(usize);

impl SessionMaterialId {
    pub fn from_index(index: usize) -> Self {
        Self(index)
    }
    pub fn index(self) -> usize {
        self.0
    }
}

/// Which material a whole-shape assignment uses.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MaterialChoice {
    Existing(SessionMaterialId),
    /// Creates a lambert with this colour inside the same edit.
    New {
        color: [f64; 3],
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Lambert {
    pub(super) name: String,
    pub(super) color: [f64; 3],
}

impl Lambert {
    pub fn name(&self) -> &str {
        &self.name
    }
    /// Linear RGB, each channel within 0 through 1.
    pub fn color(&self) -> [f64; 3] {
        self.color
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShadingGroup {
    pub(super) name: String,
    pub(super) material: SessionMaterialId,
}

impl ShadingGroup {
    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn material(&self) -> SessionMaterialId {
        self.material
    }
}

/// Group 0 is the initial shading group; shapes without an entry resolve to it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct SessionMaterials {
    pub(super) materials: Vec<Lambert>,
    pub(super) groups: Vec<ShadingGroup>,
    pub(super) assignments: BTreeMap<NativeId, usize>,
}

impl Default for SessionMaterials {
    fn default() -> Self {
        Self {
            materials: vec![Lambert {
                name: "sessionDefault".into(),
                color: SURFACE_BASE_RGB.map(|channel| f64::from(channel) / 255.0),
            }],
            groups: vec![ShadingGroup {
                name: "initialShadingGroup".into(),
                material: SessionMaterialId(0),
            }],
            assignments: BTreeMap::new(),
        }
    }
}

fn check_color(color: [f64; 3]) -> Result<(), NativeError> {
    if color.iter().any(|channel| !channel.is_finite()) {
        return Err(NativeError::NonFinite);
    }
    if color.iter().any(|channel| !(0.0..=1.0).contains(channel)) {
        return Err(NativeError::SessionColorOutOfRange);
    }
    Ok(())
}

impl SessionMaterials {
    pub(super) fn validate(
        &self,
        shapes: &[NativeMeshShape],
        sourced: bool,
    ) -> Result<(), NativeError> {
        if self.materials.is_empty()
            || self.materials.len() > MAX_SESSION_MATERIALS
            || self.groups.len() != self.materials.len()
            || self
                .groups
                .iter()
                .enumerate()
                .any(|(index, group)| group.material.0 != index)
        {
            return Err(NativeError::InvalidSessionMaterials);
        }
        for material in &self.materials {
            check_color(material.color)?;
        }
        if sourced && !self.assignments.is_empty() {
            return Err(NativeError::InvalidSessionMaterials);
        }
        let known: HashSet<NativeId> = shapes.iter().map(|shape| shape.id).collect();
        for (shape, group) in &self.assignments {
            if *group >= self.groups.len() || !known.contains(shape) {
                return Err(NativeError::InvalidSessionMaterials);
            }
        }
        Ok(())
    }

    pub(super) fn prune(&mut self, shapes: &[NativeMeshShape]) {
        self.assignments
            .retain(|shape, _| shapes.iter().any(|candidate| candidate.id == *shape));
    }

    pub(super) fn add(&mut self, color: [f64; 3]) -> Result<SessionMaterialId, NativeError> {
        check_color(color)?;
        if self.materials.len() >= MAX_SESSION_MATERIALS {
            return Err(NativeError::ResourceLimit);
        }
        let id = SessionMaterialId(self.materials.len());
        self.materials.push(Lambert {
            name: format!("sessionLambert{}", id.0),
            color,
        });
        self.groups.push(ShadingGroup {
            name: format!("sessionLambert{}SG", id.0),
            material: id,
        });
        Ok(id)
    }

    fn group_of(&self, material: SessionMaterialId) -> Result<usize, NativeError> {
        self.groups
            .iter()
            .position(|group| group.material == material)
            .ok_or(NativeError::UnknownSessionMaterial(material))
    }
}

impl NativeScene {
    pub fn session_materials(&self) -> &[Lambert] {
        &self.session_materials.materials
    }
    pub fn session_shading_groups(&self) -> &[ShadingGroup] {
        &self.session_materials.groups
    }

    /// The lambert every instance of this authored shape displays; `None` for sourced or
    /// unknown shapes, whose appearance the session table does not own.
    pub fn session_material(&self, shape: NativeId) -> Option<&Lambert> {
        if self.source.is_some() || !self.shapes.iter().any(|candidate| candidate.id == shape) {
            return None;
        }
        let group = self.session_materials.assignments.get(&shape).copied();
        let group = self.session_materials.groups.get(group.unwrap_or(0))?;
        self.session_materials.materials.get(group.material.0)
    }

    pub fn create_session_material(
        &mut self,
        color: [f64; 3],
    ) -> Result<SessionMaterialId, NativeError> {
        self.structural_edit()?;
        self.session_materials.add(color)
    }

    /// Assigns the whole shape, so every instance of it changes.
    pub fn assign_session_material(
        &mut self,
        shape: NativeId,
        choice: MaterialChoice,
    ) -> Result<SessionMaterialId, NativeError> {
        self.structural_edit()?;
        if !self.shapes.iter().any(|candidate| candidate.id == shape) {
            return Err(NativeError::MissingShape(shape));
        }
        let materials = &mut self.session_materials;
        let (material, group) = match choice {
            MaterialChoice::Existing(material) => (material, materials.group_of(material)?),
            MaterialChoice::New { color } => {
                check_color(color)?;
                let material = materials.add(color)?;
                (material, materials.groups.len() - 1)
            }
        };
        if group == 0 {
            materials.assignments.remove(&shape);
        } else {
            materials.assignments.insert(shape, group);
        }
        Ok(material)
    }

    pub fn set_session_material_color(
        &mut self,
        material: SessionMaterialId,
        color: [f64; 3],
    ) -> Result<(), NativeError> {
        self.structural_edit()?;
        check_color(color)?;
        if material.0 == 0 {
            return Err(NativeError::InitialSessionMaterialLocked);
        }
        self.session_materials
            .materials
            .get_mut(material.0)
            .ok_or(NativeError::UnknownSessionMaterial(material))?
            .color = color;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
