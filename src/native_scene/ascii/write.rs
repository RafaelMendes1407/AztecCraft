use super::{BUILTINS, add, mul, resource};
use crate::maya_ascii::{AsciiError, MAX_BYTES, Work};
use crate::native_scene::{
    NativeError, NativeScene, Units,
    admission::{self, Budget},
};
use std::{error::Error, fmt};

const HEADER: &str = "//Maya ASCII 2026 scene\nrequires maya \"2026\";\ncurrentUnit -l centimeter -a degree -t film;\nfileInfo \"AztecCraft\" \"1\";\n";
const OPENER: &str = "createNode transform -n \"";
const CLOSER: &str = "\";\n";
const RESERVED: [&str; 11] = [
    "persp",
    "perspShape",
    "top",
    "topShape",
    "front",
    "frontShape",
    "side",
    "sideShape",
    "lambert1",
    "standardSurface1",
    "particleCloud1",
];

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum AsciiWriteRefusal {
    SourcedScene,
    Curves,
    InvalidScene(NativeError),
    UnsupportedUnits,
    NonAuthoredHierarchy,
    InvalidName,
    ReservedName,
    NameCollision,
    EmptyMesh,
    ResourceLimit,
    TooLarge,
}
impl fmt::Display for AsciiWriteRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::SourcedScene => "retained Maya sources cannot use the authored prefix",
            Self::Curves => "curve declarations are unsupported",
            Self::InvalidScene(error) => return error.fmt(f),
            Self::UnsupportedUnits => "only centimeter, degree and film units are supported",
            Self::NonAuthoredHierarchy => "only flat transform-over-mesh ownership is supported",
            Self::InvalidName => "node names require the supported ASCII identifier subset",
            Self::ReservedName => "node name collides with a default Maya node",
            Self::NameCollision => "two declared nodes share a name",
            Self::EmptyMesh => "a mesh requires points and faces",
            Self::ResourceLimit => "authored prefix resource policy exceeded",
            Self::TooLarge => "authored prefix exceeds the 16 MiB byte limit",
        })
    }
}
impl Error for AsciiWriteRefusal {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidScene(error) => Some(error),
            _ => None,
        }
    }
}

fn names(scene: &NativeScene) -> impl Iterator<Item = (Option<crate::scene::NodeId>, &str)> {
    scene
        .nodes
        .iter()
        .map(|n| (Some(n.id), n.name()))
        .chain(scene.shapes.iter().map(|s| (None, s.name())))
}
fn charge_face_counts(
    work: &mut Work,
    outer_references: usize,
    hole_containers: usize,
) -> Result<(), AsciiError> {
    work.charge(mul(32, add(outer_references, hole_containers)?)?)
}
fn resource_admission(scene: &NativeScene) -> Result<(), AsciiError> {
    let n = scene.nodes.len();
    let s = scene.shapes.len();
    let d = scene.dag.len();
    let t = add(n, s)?;
    Budget::new(admission::validation_bytes(n, t, d).map_err(|_| resource())?)
        .map_err(|_| resource())?;
    let mut work = Work::new();
    work.charge(mul(mul(2, t)?, d)?)?;
    work.charge(mul(n, add(d, s)?)?)?;
    for count in [
        n,
        s,
        d,
        scene.session_materials.assignments.len(),
        scene.session_materials.materials.len(),
        scene.session_materials.groups.len(),
        1,
    ] {
        work.charge(mul(32, count)?)?;
    }
    for (_, name) in names(scene) {
        work.charge(mul(t, name.len())?)?;
    }
    for shape in &scene.shapes {
        let mesh = shape.mesh();
        for count in [mesh.point_count(), mesh.edges().len(), mesh.faces().len()] {
            work.charge(mul(32, count)?)?;
        }
        for face in mesh.faces() {
            charge_face_counts(&mut work, face.outer().len(), face.holes().len())?;
            for hole in face.holes() {
                work.charge(mul(32, hole.len())?)?;
            }
        }
    }
    Ok(())
}

fn check(scene: &NativeScene) -> Result<(), AsciiWriteRefusal> {
    use AsciiWriteRefusal as R;
    if scene.source.is_some() {
        return Err(R::SourcedScene);
    }
    if !scene.curves.is_empty() {
        return Err(R::Curves);
    }
    resource_admission(scene).map_err(|_| R::ResourceLimit)?;
    scene.validate().map_err(R::InvalidScene)?;
    if scene.units != Units::default() {
        return Err(R::UnsupportedUnits);
    }
    if scene.shapes.len() != scene.nodes.len()
        || scene.nodes.len().checked_mul(2) != Some(scene.dag.len())
    {
        return Err(R::NonAuthoredHierarchy);
    }
    for node in &scene.nodes {
        let (mut roots, mut children, mut child) = (0, 0, None);
        for edge in &scene.dag {
            if edge.parent.is_none() && edge.child == node.transform_id {
                roots += 1;
            }
            if edge.parent == Some(node.transform_id) {
                children += 1;
                child = Some(edge.child);
            }
        }
        if roots != 1 || children != 1 || !scene.shapes.iter().any(|s| Some(s.id) == child) {
            return Err(R::NonAuthoredHierarchy);
        }
    }
    for (index, (id, name)) in names(scene).enumerate() {
        scene.validate_new_name(id, name).map_err(|e| match e {
            NativeError::InvalidName => R::InvalidName,
            NativeError::NameCollision => R::NameCollision,
            other => R::InvalidScene(other),
        })?;
        if RESERVED.contains(&name) || BUILTINS.contains(&name) {
            return Err(R::ReservedName);
        }
        if names(scene).take(index).any(|(_, earlier)| earlier == name) {
            return Err(R::NameCollision);
        }
    }
    if scene
        .shapes
        .iter()
        .any(|s| s.mesh().point_count() == 0 || s.mesh().faces().is_empty())
    {
        return Err(R::EmptyMesh);
    }
    Ok(())
}

fn declaration_bytes(nodes: usize, name_bytes: usize) -> Result<usize, AsciiWriteRefusal> {
    nodes
        .checked_mul(OPENER.len() + CLOSER.len())
        .and_then(|fixed| fixed.checked_add(name_bytes))
        .and_then(|body| body.checked_add(HEADER.len()))
        .filter(|length| *length <= MAX_BYTES)
        .ok_or(AsciiWriteRefusal::TooLarge)
}

// C2b extends this same loop with UUIDs and values before any complete writer is exposed.
#[cfg_attr(not(test), expect(dead_code, reason = "no Editor caller yet"))]
pub(crate) fn emit_transform_declarations_prefix(
    scene: &NativeScene,
) -> Result<Vec<u8>, AsciiWriteRefusal> {
    check(scene)?;
    let name_bytes = scene
        .nodes
        .iter()
        .try_fold(0, |sum, n| add(sum, n.name().len()))
        .map_err(|_| AsciiWriteRefusal::TooLarge)?;
    let length = declaration_bytes(scene.nodes.len(), name_bytes)?;
    let mut out = Budget::new(0)
        .and_then(|mut b| b.vector::<u8>(length))
        .map_err(|_| AsciiWriteRefusal::ResourceLimit)?;
    out.extend_from_slice(HEADER.as_bytes());
    for node in &scene.nodes {
        out.extend_from_slice(OPENER.as_bytes());
        out.extend_from_slice(node.name().as_bytes());
        out.extend_from_slice(CLOSER.as_bytes());
    }
    Ok(out)
}

#[cfg(test)]
mod tests;
