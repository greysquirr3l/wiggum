//! Fetch a remote Git repo into a tempdir.
//!
//! Two paths are provided:
//!
//! - [`clone_shallow`] — `git clone --depth=1`. Cheap, works for any URL.
//! - [`clone_sparse_subdir`] — `git clone --depth=1 --filter=blob:none --sparse`
//!   followed by `git sparse-checkout set <subdir>`. Use when the caller only
//!   cares about a single subfolder of the repo (e.g. a monorepo subpackage).
//!
//! Both fall back to a clear error if `git` is not installed or the URL is
//! unreachable.

use std::path::PathBuf;
use std::process::Command;

use tempfile::TempDir;

use crate::error::{Result, WiggumError};

/// A cloned repo plus the RAII guard that owns the tempdir.
///
/// Drop the `guard` to delete the tempdir; keep it (or call
/// [`std::mem::forget`] on a clone) to inspect the working tree later.
pub struct ClonedRepo {
    /// Path to the cloned working tree (always the tempdir root).
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
    run_git(&[&format!("clone --depth=1 --single-branch {url}")], None)
}

/// Clone only the requested subfolder of a remote repo.
///
/// Uses `git clone --depth=1 --filter=blob:none --sparse` so we don't pull the
/// rest of the working tree, then `git sparse-checkout set <subdir>` to expand
/// just that path. The resulting working tree contains `<subdir>` plus a
/// sparse-checkout bookkeeping file at the root.
///
/// # Errors
///
/// Same as [`clone_shallow`]. Additionally fails if `subdir` is empty.
pub fn clone_sparse_subdir(url: &str, subdir: &str) -> Result<ClonedRepo> {
    if subdir.trim().is_empty() {
        return Err(WiggumError::Validation(
            "subdir must be a non-empty path".to_string(),
        ));
    }

    let cloned = run_git(
        &[&format!(
            "clone --depth=1 --filter=blob:none --single-branch --sparse {url}"
        )],
        None,
    )?;

    // sparse-checkout init + set the path. Done in two commands because
    // some git versions don't accept `set` before `init`.
    let status = Command::new("git")
        .args(["sparse-checkout", "set", subdir])
        .current_dir(&cloned.path)
        .status()
        .map_err(|e| {
            WiggumError::Validation(format!("failed to invoke `git sparse-checkout`: {e}"))
        })?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(WiggumError::Validation(format!(
            "`git sparse-checkout set {subdir}` exited with status {code}"
        )));
    }

    Ok(cloned)
}

/// Run a `git` command and wrap the tempdir ownership in a `ClonedRepo`.
///
/// `cmdline` is a single string that is split on whitespace (no shell
/// expansion, so URLs with spaces must quote them). `cwd` overrides the
/// current directory when needed.
fn run_git(cmdline: &[&str], cwd: Option<&std::path::Path>) -> Result<ClonedRepo> {
    let guard = TempDir::new()
        .map_err(|e| WiggumError::Validation(format!("create tempdir for clone: {e}")))?;

    for line in cmdline {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.is_empty() {
            continue;
        }

        // The first token is the `git` subcommand (e.g. `clone`, `sparse-checkout`),
        // not the binary name. We always invoke the `git` binary directly.
        let mut cmd = Command::new("git");
        cmd.args(&parts);
        cmd.arg(guard.path());
        if let Some(dir) = cwd {
            cmd.current_dir(dir);
        }
        let status = cmd.status().map_err(|e| {
            WiggumError::Validation(format!(
                "failed to invoke `git {line}`: {e}. Is git installed and on PATH?"
            ))
        })?;
        if !status.success() {
            let code = status.code().unwrap_or(-1);
            return Err(WiggumError::Validation(format!(
                "`git {line}` exited with status {code}. Check the URL and your credentials."
            )));
        }
    }

    let path = guard.path().to_path_buf();
    if !path.is_dir() {
        return Err(WiggumError::Validation(
            "git produced no directory — output was empty?".to_string(),
        ));
    }

    Ok(ClonedRepo { path, guard })
}

/// Parse an optional subfolder out of a GitHub-style URL.
///
/// Recognised forms (return the subfolder path):
/// - `https://github.com/owner/repo/tree/<branch>/<subdir>`
/// - `https://github.com/owner/repo/blob/<branch>/<subdir>`
///
/// Returns `None` if the URL doesn't carry a subfolder. Non-GitHub URLs also
/// return `None` (use the explicit `--subdir` flag for those).
#[must_use]
pub fn parse_github_subdir(url: &str) -> Option<String> {
    // Split on `/tree/` or `/blob/`.
    let marker = if url.contains("/tree/") {
        "/tree/"
    } else if url.contains("/blob/") {
        "/blob/"
    } else {
        return None;
    };

    let idx = url.find(marker)?;
    let after = &url[idx + marker.len()..];
    // First segment is the branch name; remainder is the subdir.
    let (_, subdir) = after.split_once('/')?;
    if subdir.is_empty() {
        return None;
    }
    Some(subdir.to_string())
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

    #[test]
    fn clone_sparse_subdir_rejects_empty_subdir() {
        let result = clone_sparse_subdir("https://example.com/repo.git", "");
        assert!(result.is_err());
    }

    #[test]
    fn parse_github_subdir_extracts_tree_path() {
        assert_eq!(
            parse_github_subdir("https://github.com/foo/bar/tree/main/src/api"),
            Some("src/api".to_string())
        );
        assert_eq!(
            parse_github_subdir("https://github.com/foo/bar/tree/main/src"),
            Some("src".to_string())
        );
    }

    #[test]
    fn parse_github_subdir_extracts_blob_path() {
        assert_eq!(
            parse_github_subdir("https://github.com/foo/bar/blob/main/docs/README.md"),
            Some("docs/README.md".to_string())
        );
    }

    #[test]
    fn parse_github_subdir_returns_none_for_plain_repo_url() {
        assert_eq!(parse_github_subdir("https://github.com/foo/bar"), None);
        assert_eq!(parse_github_subdir("https://github.com/foo/bar/"), None);
    }

    #[test]
    fn parse_github_subdir_returns_none_for_non_github() {
        assert_eq!(parse_github_subdir("https://gitlab.com/foo/bar"), None);
        assert_eq!(parse_github_subdir("file:///tmp/repo"), None);
    }
}
