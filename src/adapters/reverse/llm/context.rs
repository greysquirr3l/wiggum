//! Project context gathered from the cloned repo before the LLM prompt.
//!
//! Bounded — we never send the entire repo, just the pieces the LLM needs
//! to make a reasonable phase decomposition: language/manifest summary,
//! top-level tree, README excerpt, user hints.

use std::fmt::Write as _;
use std::path::Path;

use serde::Serialize;

use crate::adapters::bootstrap::ScanResult;
use crate::adapters::reverse::hints::Hints;
use crate::error::{Result, WiggumError};

/// Maximum bytes of README to include. Anthropic-Messages has a 1M context
/// window (with the right model) but bigger inputs cost more and slow the
/// call. 16 KiB is plenty for "what does this project do".
pub const MAX_README_BYTES: usize = 16 * 1024;

/// Maximum tree depth to expand. 3 is enough to see top-level packages
/// without blowing up on monorepos with hundreds of `node_modules` siblings.
pub const MAX_TREE_DEPTH: usize = 3;

/// Bounded snapshot of the repo we hand to the LLM.
#[derive(Debug, Clone, Serialize)]
pub struct LlmContext {
    /// Detected language (lowercased name).
    pub language: String,
    /// Detected project name.
    pub name: String,
    /// Detected description (may be empty).
    pub description: String,
    /// Detected architecture (may be empty).
    pub architecture: Option<String>,
    /// Already-inferred orchestrator rules.
    pub detected_rules: Vec<String>,
    /// Top-of-repo directory tree (depth-limited).
    pub tree: String,
    /// First 16 KiB of README.md (or empty if no README).
    pub readme_excerpt: String,
    /// User-supplied hints, serialised as a stable string.
    pub user_hints: Option<String>,
}

impl LlmContext {
    /// Render a compact single-string summary used for log lines + cost reporting.
    #[allow(dead_code)] // surfaced via Debug + summary log lines (future)
    #[must_use]
    pub fn summary(&self) -> String {
        let tree_lines = self.tree.lines().count();
        format!(
            "{} `{}` — {} top-level lines, README {}B, hints {}",
            self.language,
            self.name,
            tree_lines,
            self.readme_excerpt.len(),
            self.user_hints.as_deref().map_or("none", |_| "present"),
        )
    }
}

/// Gather the LLM context from a cloned repo on disk.
///
/// # Errors
///
/// Returns an error if the path is not a directory or the tree walk fails.
pub fn gather_context(
    repo_path: &Path,
    scan: &ScanResult,
    hints: Option<&Hints>,
) -> Result<LlmContext> {
    if !repo_path.is_dir() {
        return Err(WiggumError::Validation(format!(
            "context gather: not a directory: {}",
            repo_path.display()
        )));
    }

    let tree = render_tree(repo_path, MAX_TREE_DEPTH)?;
    let readme_excerpt = read_readme_excerpt(repo_path, MAX_README_BYTES)?;
    let user_hints = hints.map(serialise_hints);

    Ok(LlmContext {
        language: scan.language.to_string(),
        name: scan.name.clone(),
        description: scan.description.clone(),
        architecture: scan.architecture.clone(),
        detected_rules: scan.rules.clone(),
        tree,
        readme_excerpt,
        user_hints,
    })
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn render_tree(root: &Path, max_depth: usize) -> Result<String> {
    let mut out = String::new();
    walk_tree(root, 0, max_depth, &mut out)?;
    Ok(out)
}

fn walk_tree(dir: &Path, depth: usize, max_depth: usize, out: &mut String) -> Result<()> {
    if depth > max_depth {
        return Ok(());
    }
    let entries = std::fs::read_dir(dir)
        .map_err(|e| WiggumError::Validation(format!("read_dir {}: {e}", dir.display())))?;
    let mut sorted: Vec<_> = entries.flatten().collect();
    sorted.sort_by_key(std::fs::DirEntry::file_name);

    for entry in sorted {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        // Skip noise directories and dotfiles.
        if is_excluded(&name_str) {
            continue;
        }

        let indent = "  ".repeat(depth);
        let file_type = entry.file_type().map_err(|e| {
            WiggumError::Validation(format!("file_type {}: {e}", entry.path().display()))
        })?;

        if file_type.is_dir() {
            let _ = writeln!(out, "{indent}{name_str}/");
            walk_tree(&entry.path(), depth + 1, max_depth, out)?;
        } else if file_type.is_file() {
            let _ = writeln!(out, "{indent}{name_str}");
        }
    }
    Ok(())
}

fn is_excluded(name: &str) -> bool {
    // `.git`, `node_modules`, `target`, `vendor`, `dist`, `build`, `.cache` — and hidden.
    if name.starts_with('.') {
        return true;
    }
    matches!(
        name,
        "node_modules" | "target" | "vendor" | "dist" | "build" | ".cache" | "__pycache__"
    )
}

fn read_readme_excerpt(repo_path: &Path, max_bytes: usize) -> Result<String> {
    let candidates = ["README.md", "README.markdown", "README.rst", "README"];
    for name in candidates {
        let path = repo_path.join(name);
        if path.is_file() {
            let raw = std::fs::read(&path)
                .map_err(|e| WiggumError::Validation(format!("read {}: {e}", path.display())))?;
            // Truncate at a clean UTF-8 boundary — use str::is_char_boundary
            // on the lossy-decoded string.
            let s = String::from_utf8_lossy(&raw);
            let end = s.len().min(max_bytes);
            let end = s
                .char_indices()
                .map(|(i, _)| i)
                .take_while(|&i| i <= end)
                .last()
                .unwrap_or(0);
            return Ok(s[..end].to_string());
        }
    }
    Ok(String::new())
}

fn serialise_hints(h: &Hints) -> String {
    let mut s = String::new();
    if let Some(name) = &h.project.name {
        let _ = writeln!(s, "# project.name = {name}");
    }
    if let Some(desc) = &h.project.description {
        let _ = writeln!(s, "# project.description = {desc}");
    }
    if let Some(lang) = h.project.language {
        let _ = writeln!(s, "# project.language = {lang}");
    }
    if let Some(arch) = &h.project.architecture {
        let _ = writeln!(s, "# project.architecture = {arch}");
    }
    if !h.project.extra_rules.is_empty() {
        s.push_str("# project.extra_rules:\n");
        for r in &h.project.extra_rules {
            let _ = writeln!(s, "- {r}");
        }
    }
    if let Some(persona) = &h.orchestrator.persona {
        let _ = writeln!(s, "# orchestrator.persona = {persona}");
    }
    if let Some(strategy) = h.orchestrator.strategy {
        let _ = writeln!(s, "# orchestrator.strategy = {strategy}");
    }
    if let Some(max) = h.orchestrator.max_retries {
        let _ = writeln!(s, "# orchestrator.max_retries = {max}");
    }
    if !h.orchestrator.extra_rules.is_empty() {
        s.push_str("# orchestrator.extra_rules:\n");
        for r in &h.orchestrator.extra_rules {
            let _ = writeln!(s, "- {r}");
        }
    }
    for ph in &h.phases {
        let _ = writeln!(s, "# phase: {}", ph.name);
        for t in &ph.tasks {
            let _ = writeln!(s, "# task: {} — {}", t.slug, t.title);
        }
    }
    s
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn excluded_dirs_are_skipped() {
        assert!(is_excluded(".git"));
        assert!(is_excluded("node_modules"));
        assert!(is_excluded("target"));
        assert!(is_excluded("dist"));
        assert!(!is_excluded("src"));
        assert!(!is_excluded("Cargo.toml"));
    }

    #[test]
    fn tree_renders_depth_limited() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("a/b/c/d")).unwrap();
        std::fs::write(dir.path().join("a/b/c/d/leaf.txt"), "x").unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\n").unwrap();
        let tree = render_tree(dir.path(), 2).unwrap();
        assert!(tree.contains("Cargo.toml"));
        assert!(tree.contains("a/"));
        assert!(tree.contains("b/"));
        assert!(!tree.contains("d/"), "depth 3+ should be excluded");
        assert!(!tree.contains("leaf.txt"));
    }

    #[test]
    fn readme_excerpt_truncates_at_utf8_boundary() {
        let dir = tempfile::TempDir::new().unwrap();
        // 100 ASCII chars, then a 4-byte emoji, then 100 more ASCII chars.
        let mut content = vec![b'a'; 100];
        content.extend_from_slice("🦀".as_bytes());
        content.extend_from_slice(&[b'b'; 100]);
        std::fs::write(dir.path().join("README.md"), &content).unwrap();
        let excerpt = read_readme_excerpt(dir.path(), 150).unwrap();
        assert!(excerpt.len() <= 150);
        // Must be valid UTF-8.
        assert!(excerpt.is_char_boundary(excerpt.len()));
    }

    #[test]
    fn readme_excerpt_returns_empty_when_absent() {
        let dir = tempfile::TempDir::new().unwrap();
        let excerpt = read_readme_excerpt(dir.path(), 1024).unwrap();
        assert!(excerpt.is_empty());
    }
}
