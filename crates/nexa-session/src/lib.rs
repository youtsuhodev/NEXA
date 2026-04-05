//! Compilation session: loaded sources and paths.

use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub path: PathBuf,
    pub contents: String,
}

impl SourceFile {
    pub fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let contents = std::fs::read_to_string(&path)?;
        Ok(Self { path, contents })
    }
}
