use std::path::Path;

use crate::error::Result;

/// Port for writing generated artifacts to the target project.
pub trait ArtifactWriter {
    /// Write content to a file at the given path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    fn write_file(&self, path: &Path, content: &str) -> Result<()>;

    /// Ensure a directory exists, creating it if necessary.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be created.
    fn ensure_dir(&self, path: &Path) -> Result<()>;
}

/// Port for reading plan files.
pub trait PlanReader {
    /// Read a plan TOML file from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    fn read_plan(&self, path: &Path) -> Result<String>;
}

/// Port for reading/writing progress state.
pub trait ProgressStore {
    /// Read current progress content from disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    fn read_progress(&self, path: &Path) -> Result<String>;

    /// Write updated progress content to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be written.
    fn write_progress(&self, path: &Path, content: &str) -> Result<()>;
}
