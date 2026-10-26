//! Anchor resolution and drift detection.
//!
//! An [`Anchor`] pins a task to a verified `file:line` location in the target
//! project. This module resolves those locations against the filesystem so
//! `wiggum validate --lint` can report *drift* — the case where a plan was
//! written against code that has since moved, been renamed, or been deleted.
//!
//! # Optional by design
//!
//! Anchors are a progressive enhancement. A plan with no anchors is completely
//! valid and produces no diagnostics. The rules here run **only** for anchors
//! the user actually declared — they never demand that a task be anchored.
//!
//! # Severity model
//!
//! | Condition | Severity |
//! |---|---|
//! | Anchored file missing (role `read`/`modify`) | `Error` — the plan points at nothing |
//! | Anchored file exists but is a directory | `Error` |
//! | Line range exceeds file length | `Warning` — code moved or the file shrank |
//! | Line range is empty (0 lines matched) | `Warning` |
//! | File exists but role is `create` | `Info` — stale intent, the file already landed |
//! | Path escapes the project root | `Warning` |
//! | Malformed `lines` value | `Warning` — parsed leniently, not fatal |
//! | Anchor carries no `note` | `Info` — a citation alone lacks rationale |

use std::path::{Component, Path, PathBuf};

use crate::domain::lint::{Diagnostic, Severity};
use crate::domain::plan::{Anchor, AnchorRole, ResolvedTask};

/// Resolve an anchor path against the project root, rejecting traversal.
///
/// Returns `None` if the path escapes the project root (e.g. `../../etc/passwd`)
/// or is absolute, since anchors are documented as project-relative.
fn resolve_anchor_path(root: &Path, anchor_path: &str) -> Option<PathBuf> {
    let candidate = Path::new(anchor_path);

    // Reject anything that is not project-relative. `Path::is_absolute` is
    // not sufficient on its own: on Windows it requires a drive prefix, so
    // `/etc/passwd` (a root with no prefix) and `C:foo` (a prefix with no
    // root) both report `false` yet still escape the project root when
    // joined. Inspecting the components catches every shape — `RootDir` on
    // Unix, `RootDir` or `Prefix` on Windows — regardless of platform.
    if candidate
        .components()
        .any(|c| matches!(c, Component::RootDir | Component::Prefix(_)))
    {
        return None;
    }
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return None;
    }

    Some(root.join(candidate))
}

/// Count the lines in a file, returning `None` when it cannot be read as UTF-8.
fn line_count(path: &Path) -> Option<usize> {
    let content = std::fs::read_to_string(path).ok()?;
    if content.is_empty() {
        return Some(0);
    }
    Some(content.lines().count())
}

/// Check that an anchored line range still falls inside the file.
///
/// Emits `anchor-line-drift` (warning) when the file is shorter than the
/// anchored region, and `anchor-unreadable` (info) when the file cannot be
/// read as UTF-8.
fn check_line_range(
    full: &Path,
    end: u32,
    task_label: &str,
    citation: &str,
    out: &mut Vec<Diagnostic>,
) {
    match line_count(full) {
        Some(total) if (end as usize) > total => {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "anchor-line-drift",
                message: format!(
                    "{task_label}: `{citation}` points past end of file \
                     ({total} lines) — the code likely moved"
                ),
            });
        }
        Some(_) => {}
        None => {
            out.push(Diagnostic {
                severity: Severity::Info,
                rule: "anchor-unreadable",
                message: format!(
                    "{task_label}: `{citation}` could not be read as UTF-8 — \
                     skipped the line-range check"
                ),
            });
        }
    }
}

/// Lint a single anchor, emitting diagnostics for drift and malformed input.
///
/// `root` is the target project root (`[project].path`).
fn lint_anchor(anchor: &Anchor, task_label: &str, root: &Path, out: &mut Vec<Diagnostic>) {
    let citation = anchor.citation();

    if anchor.path.trim().is_empty() {
        out.push(Diagnostic {
            severity: Severity::Error,
            rule: "anchor-empty-path",
            message: format!("{task_label}: anchor has an empty `path`"),
        });
        return;
    }

    // Parse the line range first — a malformed value is reported but does not
    // prevent the file-existence check from running.
    let range = match anchor.line_range() {
        Ok(r) => r,
        Err(e) => {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "anchor-bad-lines",
                message: format!("{task_label}: {citation} — {e}"),
            });
            None
        }
    };

    let Some(full) = resolve_anchor_path(root, &anchor.path) else {
        out.push(Diagnostic {
            severity: Severity::Warning,
            rule: "anchor-escapes-root",
            message: format!(
                "{task_label}: `{citation}` is absolute or escapes the project root — \
                 anchors must be relative to [project].path"
            ),
        });
        return;
    };

    match anchor.role {
        AnchorRole::Create => {
            if full.exists() {
                out.push(Diagnostic {
                    severity: Severity::Info,
                    rule: "anchor-already-created",
                    message: format!(
                        "{task_label}: `{citation}` is marked `create` but the file already \
                         exists — consider switching the role to `modify`"
                    ),
                });
            }
        }
        AnchorRole::Read | AnchorRole::Modify => {
            if !full.exists() {
                out.push(Diagnostic {
                    severity: Severity::Error,
                    rule: "anchor-file-missing",
                    message: format!(
                        "{task_label}: `{citation}` does not exist — the plan is grounded in \
                         code that has moved or been renamed"
                    ),
                });
                return;
            }
            if full.is_dir() {
                out.push(Diagnostic {
                    severity: Severity::Error,
                    rule: "anchor-is-directory",
                    message: format!("{task_label}: `{citation}` is a directory, not a file"),
                });
                return;
            }

            if let Some((_start, end)) = range {
                check_line_range(&full, end, task_label, &citation, out);
            }
        }
    }

    let note_is_blank = anchor.note.as_deref().is_none_or(|n| n.trim().is_empty());

    if note_is_blank {
        out.push(Diagnostic {
            severity: Severity::Info,
            rule: "anchor-no-note",
            message: format!(
                "{task_label}: `{citation}` has no `note` — add why this location matters \
                 so the subagent knows what to do here"
            ),
        });
    }
}

/// Run anchor drift checks across every resolved task.
///
/// This is a no-op when the plan declares no anchors at all — existing plans
/// are entirely unaffected by this feature.
///
/// `root` is the target project root (`[project].path`). When the root does
/// not exist, every anchor is reported once as unresolved rather than
/// producing a diagnostic per anchor.
#[must_use]
pub fn lint_anchors(resolved: &[ResolvedTask], root: &Path) -> Vec<Diagnostic> {
    let mut out = Vec::new();

    if !root.exists() {
        let total: usize = resolved.iter().map(|t| t.anchors.len()).sum();
        if total > 0 {
            out.push(Diagnostic {
                severity: Severity::Warning,
                rule: "anchor-root-missing",
                message: format!(
                    "project root `{}` does not exist — skipped drift checks for \
                     {total} anchor(s)",
                    root.display()
                ),
            });
        }
        return out;
    }

    for task in resolved {
        let label = format!("T{:02}-{}", task.number, task.slug);
        for anchor in &task.anchors {
            lint_anchor(anchor, &label, root, &mut out);
        }
    }

    out
}

/// Total number of anchors declared across a task list.
///
/// Used by callers to decide whether to mention anchors at all.
#[must_use]
pub fn total_anchors(resolved: &[ResolvedTask]) -> usize {
    resolved.iter().map(|t| t.anchors.len()).sum()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::domain::plan::TaskKind;

    fn write(rel: &str, contents: &str) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(&p, contents).unwrap();
        dir
    }

    fn task_with(anchors: Vec<Anchor>) -> ResolvedTask {
        ResolvedTask {
            number: 1,
            slug: "sample".to_string(),
            anchors,
            kind: TaskKind::default(),
            ..ResolvedTask::default()
        }
    }

    fn anchor(path: &str, lines: Option<&str>, role: AnchorRole) -> Anchor {
        Anchor {
            path: path.to_string(),
            lines: lines.map(std::string::ToString::to_string),
            role,
            note: Some("because".to_string()),
        }
    }

    #[test]
    fn no_anchors_produces_no_diagnostics() {
        let dir = tempfile::tempdir().unwrap();
        let diags = lint_anchors(&[task_with(vec![])], dir.path());
        assert!(diags.is_empty(), "expected clean, got {diags:?}");
    }

    #[test]
    fn missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let task = task_with(vec![anchor("src/gone.rs", None, AnchorRole::Modify)]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-file-missing"));
        assert_eq!(diags.first().map(|d| d.severity), Some(Severity::Error));
    }

    #[test]
    fn in_range_lines_are_clean() {
        let dir = write("src/a.rs", "one\ntwo\nthree\nfour\n");
        let task = task_with(vec![anchor("src/a.rs", Some("2-3"), AnchorRole::Modify)]);
        assert!(lint_anchors(&[task], dir.path()).is_empty());
    }

    #[test]
    fn line_past_eof_is_a_drift_warning() {
        let dir = write("src/a.rs", "one\ntwo\n");
        let task = task_with(vec![anchor("src/a.rs", Some("40-80"), AnchorRole::Read)]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-line-drift"));
        assert_eq!(diags.first().map(|d| d.severity), Some(Severity::Warning));
    }

    #[test]
    fn create_role_with_existing_file_is_info() {
        let dir = write("src/a.rs", "x\n");
        let task = task_with(vec![anchor("src/a.rs", None, AnchorRole::Create)]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.len(), 1);
        assert_eq!(
            diags.first().map(|d| d.rule),
            Some("anchor-already-created")
        );
        assert_eq!(diags.first().map(|d| d.severity), Some(Severity::Info));
    }

    #[test]
    fn create_role_with_missing_file_is_clean() {
        let dir = tempfile::tempdir().unwrap();
        let task = task_with(vec![anchor("src/new.rs", None, AnchorRole::Create)]);
        assert!(lint_anchors(&[task], dir.path()).is_empty());
    }

    #[test]
    fn traversal_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let task = task_with(vec![anchor("../outside.rs", None, AnchorRole::Read)]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-escapes-root"));
    }

    #[test]
    fn absolute_path_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let task = task_with(vec![anchor("/etc/passwd", None, AnchorRole::Read)]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-escapes-root"));
    }

    /// A path with a root but no drive prefix is *not* absolute on Windows,
    /// where `Path::is_absolute` requires a prefix. It still escapes the
    /// project root, so it must be rejected on every platform.
    #[test]
    fn rooted_path_without_prefix_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let task = task_with(vec![anchor(
            "//server/share/file.rs",
            None,
            AnchorRole::Read,
        )]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-escapes-root"));
    }

    #[test]
    fn directory_anchor_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let task = task_with(vec![anchor("src", None, AnchorRole::Modify)]);
        let diags = lint_anchors(&[task], dir.path());
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-is-directory"));
    }

    #[test]
    fn missing_note_is_info() {
        let dir = write("src/a.rs", "x\n");
        let mut a = anchor("src/a.rs", None, AnchorRole::Read);
        a.note = None;
        let diags = lint_anchors(&[task_with(vec![a])], dir.path());
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-no-note"));
        assert_eq!(diags.first().map(|d| d.severity), Some(Severity::Info));
    }

    #[test]
    fn missing_root_reports_once() {
        let task = ResolvedTask {
            number: 1,
            slug: "t".to_string(),
            anchors: vec![
                anchor("a.rs", None, AnchorRole::Read),
                anchor("b.rs", None, AnchorRole::Read),
            ],
            ..ResolvedTask::default()
        };
        let diags = lint_anchors(&[task], Path::new("/nonexistent-wiggum-root"));
        assert_eq!(diags.len(), 1);
        assert_eq!(diags.first().map(|d| d.rule), Some("anchor-root-missing"));
    }

    #[test]
    fn total_anchors_counts_across_tasks() {
        let t1 = task_with(vec![anchor("a.rs", None, AnchorRole::Read)]);
        let mut t2 = task_with(vec![]);
        t2.number = 2;
        assert_eq!(total_anchors(&[t1, t2]), 1);
    }

    #[test]
    fn line_range_parses_forms() {
        let mk = |l: Option<&str>| Anchor {
            path: "x.rs".to_string(),
            lines: l.map(std::string::ToString::to_string),
            ..Anchor::default()
        };
        assert_eq!(mk(None).line_range().unwrap(), None);
        assert_eq!(mk(Some("42")).line_range().unwrap(), Some((42, 42)));
        assert_eq!(mk(Some("42-80")).line_range().unwrap(), Some((42, 80)));
        // Reversed input is normalised rather than rejected.
        assert_eq!(mk(Some("80-42")).line_range().unwrap(), Some((42, 80)));
        assert!(mk(Some("0")).line_range().is_err());
        assert!(mk(Some("abc")).line_range().is_err());
        assert!(mk(Some("")).line_range().is_err());
    }

    #[test]
    fn citation_renders_compactly() {
        let mk = |l: Option<&str>| Anchor {
            path: "src/a.rs".to_string(),
            lines: l.map(std::string::ToString::to_string),
            ..Anchor::default()
        };
        assert_eq!(mk(None).citation(), "src/a.rs");
        assert_eq!(mk(Some("42")).citation(), "src/a.rs:42");
        assert_eq!(mk(Some("42-80")).citation(), "src/a.rs:42-80");
        // Unparseable falls back to the bare path rather than panicking.
        assert_eq!(mk(Some("nope")).citation(), "src/a.rs");
    }
}
