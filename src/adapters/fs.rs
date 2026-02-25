use std::fs;
use std::path::Path;

use crate::error::Result;
use crate::ports::{ArtifactWriter, PlanReader, ProgressStore};

/// Filesystem implementation of all I/O ports.
pub struct FsAdapter;

impl ArtifactWriter for FsAdapter {
    fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn ensure_dir(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path)?;
        Ok(())
    }
}

impl PlanReader for FsAdapter {
    fn read_plan(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }
}

impl ProgressStore for FsAdapter {
    fn read_progress(&self, path: &Path) -> Result<String> {
        Ok(fs::read_to_string(path)?)
    }

    fn write_progress(&self, path: &Path, content: &str) -> Result<()> {
        fs::write(path, content)?;
        Ok(())
    }
}
