use std::path::Path;
use std::process::Command;

/// Result of checking VCS status for a project directory.
pub enum VcsStatus {
    /// Directory is a git repo with uncommitted changes.
    Dirty(String),
    /// Directory is a git repo with a clean working tree.
    Clean,
    /// Directory is not a git repository (or git is not installed).
    NotARepo,
}

/// Check whether the target directory has uncommitted changes.
///
/// Shells out to `git status --porcelain` in `project_path`.
/// Returns [`VcsStatus::NotARepo`] if git is not available or the
/// directory is not inside a git repository.
#[must_use]
pub fn check_vcs_status(project_path: &Path) -> VcsStatus {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.trim().is_empty() {
                VcsStatus::Clean
            } else {
                VcsStatus::Dirty(stdout.trim().to_string())
            }
        }
        _ => VcsStatus::NotARepo,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn non_existent_dir_is_not_a_repo() {
        let path = PathBuf::from("/tmp/wiggum-nonexistent-dir-test");
        assert!(matches!(check_vcs_status(&path), VcsStatus::NotARepo));
    }
}
