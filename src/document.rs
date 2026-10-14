use crate::scene::{Scene, SceneError};
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

pub const DOCUMENT_VERSION: u32 = 1;
pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document<T> {
    version: u32,
    scene: T,
}

pub fn load(path: &Path) -> Result<Scene, DocumentError> {
    let result = || -> io::Result<Vec<u8>> {
        if !std::fs::metadata(path)?.is_file() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "document must be a regular file",
            ));
        }
        let mut bytes = Vec::new();
        File::open(path)?
            .take((MAX_DOCUMENT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)?;
        Ok(bytes)
    };
    let bytes = result().map_err(|source| DocumentError::Io {
        operation: "read document",
        path: path.into(),
        source,
    })?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err(DocumentError::TooLarge);
    }
    let document: Document<Scene> = serde_json::from_slice(&bytes).map_err(DocumentError::Json)?;
    if document.version != DOCUMENT_VERSION {
        return Err(DocumentError::UnsupportedVersion(document.version));
    }
    Ok(document.scene)
}

pub fn save_new(path: &Path, scene: &Scene) -> Result<(), DocumentError> {
    scene.validate().map_err(DocumentError::Scene)?;
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            return Err(DocumentError::Io {
                operation: "create document",
                path: path.into(),
                source: io::Error::new(io::ErrorKind::AlreadyExists, "destination already exists"),
            });
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(DocumentError::Io {
                operation: "inspect document destination",
                path: path.into(),
                source,
            });
        }
    }
    let mut bytes = LimitedBytes(Vec::new());
    let document = Document {
        version: DOCUMENT_VERSION,
        scene,
    };
    if let Err(error) = serde_json::to_writer_pretty(&mut bytes, &document) {
        return Err(if bytes.0.len() == MAX_DOCUMENT_BYTES {
            DocumentError::TooLarge
        } else {
            DocumentError::Json(error)
        });
    }
    craft_atomic::write_new(path, &bytes.0).map_err(|source| DocumentError::Io {
        operation: "create document",
        path: path.into(),
        source,
    })
}

struct LimitedBytes(Vec<u8>);

impl Write for LimitedBytes {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let count = bytes.len().min(MAX_DOCUMENT_BYTES - self.0.len());
        if count == 0 && !bytes.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::FileTooLarge,
                "document size limit exceeded",
            ));
        }
        self.0.extend_from_slice(&bytes[..count]);
        Ok(count)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub enum DocumentError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Json(serde_json::Error),
    Scene(SceneError),
    UnsupportedVersion(u32),
    TooLarge,
}

impl fmt::Display for DocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(f, "{operation} {}: {source}", path.display()),
            Self::Json(source) => write!(f, "invalid scene document: {source}"),
            Self::Scene(source) => write!(f, "invalid scene: {source}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported document version {version}")
            }
            Self::TooLarge => write!(f, "document exceeds {MAX_DOCUMENT_BYTES} byte limit"),
        }
    }
}

impl std::error::Error for DocumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::Json(source) => Some(source),
            Self::Scene(source) => Some(source),
            _ => None,
        }
    }
}
