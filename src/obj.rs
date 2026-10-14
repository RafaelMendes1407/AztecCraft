use crate::scene::{Scene, SceneError};
use std::{
    fmt,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};

// Names use UTF-8 percent encoding except ASCII letters, digits, '-', '_' and '.'.
// Encoding '%' too preserves distinctions while preventing whitespace/comment/directive injection.
pub fn export_new(path: &Path, scene: &Scene) -> Result<(), ObjError> {
    scene.validate().map_err(ObjError::Scene)?;
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(ObjError::Io {
                path: path.into(),
                source: io::Error::new(io::ErrorKind::AlreadyExists, "destination already exists"),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(ObjError::Io {
                path: path.into(),
                source,
            });
        }
    }
    craft_atomic::write_new_with(path, |file| {
        let mut writer = BufWriter::new(file);
        writeln!(writer, "# AztecCraft world-space polygon geometry")?;
        let mut offset = 1;
        for node in scene.nodes() {
            write!(writer, "o ")?;
            for byte in node.name().bytes() {
                if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.') {
                    writer.write_all(&[byte])?;
                } else {
                    write!(writer, "%{byte:02X}")?;
                }
            }
            writeln!(writer)?;
            for vertex in node.world_vertices() {
                writeln!(writer, "v {} {} {}", vertex.x, vertex.y, vertex.z)?;
            }
            let scale = node.transform().scale;
            let reflected = scale.x != 0.0
                && scale.y != 0.0
                && scale.z != 0.0
                && (scale.x.is_sign_negative()
                    ^ scale.y.is_sign_negative()
                    ^ scale.z.is_sign_negative());
            for face in node.mesh().faces() {
                let mut face = *face;
                if reflected {
                    face.reverse();
                }
                writeln!(
                    writer,
                    "f {} {} {} {}",
                    face[0] + offset,
                    face[1] + offset,
                    face[2] + offset,
                    face[3] + offset
                )?;
            }
            offset += node.mesh().vertices().len();
        }
        writer.flush()
    })
    .map_err(|source| ObjError::Io {
        path: path.into(),
        source,
    })
}

#[derive(Debug)]
pub enum ObjError {
    Io { path: PathBuf, source: io::Error },
    Scene(SceneError),
}

impl fmt::Display for ObjError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => write!(f, "export OBJ {}: {source}", path.display()),
            Self::Scene(source) => write!(f, "invalid scene for OBJ export: {source}"),
        }
    }
}

impl std::error::Error for ObjError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Scene(source) => Some(source),
        }
    }
}
