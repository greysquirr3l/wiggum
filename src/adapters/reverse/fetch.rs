//! Fetch a remote Git repo into a tempdir.
//!
//! Uses `git clone --depth=1 --single-branch` for speed. Falls back to a
//! clear error if `git` is not installed or the URL is unreachable.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

use crate::error::{Result, WiggumError};

/// A cloned repo plus the RAII guard that owns the tempdir.
///
/// Drop the `guard` to delete the tempdir; keep it (or call
/// [`std::mem::forget`] on a clone) to inspect the working tree later.
pub struct ClonedRepo {
    /// Path to the cloned repo inside the tempdir.
    pub path: PathBuf,
    /// Tempdir guard — drop to clean up.
    pub guard: TempDir,
}

/// Clone a remote repo to a fresh tempdir with `git clone --depth=1`.
///
/// Accepts any URL `git` itself accepts (GitHub, GitLab, self-hosted, SSH).
///
/// # Errors
///
/// - [`WiggumError::Validation`] if `git` is not installed, the URL fails to
///   clone, or the resulting directory is missing the expected layout.
pub fn clone_shallow(url: &str) -> Result<ClonedRepo> {
    let guard = TempDir::new()
        .map_err(|e| WiggumError::Validation(format!("create tempdir for clone: {e}")))?;

    let status = Command::new("git")
        .args(["clone", "--depth=1", "--single-branch", url])
        .arg(guard.path())
        .status()
        .map_err(|e| {
            WiggumError::Validation(format!(
                "failed to invoke `git clone`: {e}. Is git installed and on PATH?"
            ))
        })?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(WiggumError::Validation(format!(
            "`git clone {url}` exited with status {code}. Check the URL and your credentials."
        )));
    }

    // `git clone <url> <dir>` writes the working tree directly into <dir>,
    // not <dir>/<basename>. So `clone.path` IS the tempdir root.
    let path = guard.path().to_path_buf();

    if !path.is_dir() {
        return Err(WiggumError::Validation(format!(
            "clone of {url} produced no directory — git output was empty?"
        )));
    }

    Ok(ClonedRepo { path, guard })
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn clone_nonexistent_url_errors() {
        let result = clone_shallow("https://github.com/this-org-does-not-exist-xyz123/repo-zzz");
        assert!(result.is_err(), "expected error for non-existent repo");
    }

    #[test]
    fn clone_non_git_url_errors() {
        let result = clone_shallow("not-a-url-at-all");
        assert!(result.is_err());
    }
}
