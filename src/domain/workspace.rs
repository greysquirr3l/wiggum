//! Workspace model — orchestrate multiple `plan.toml` files as a unit.
//!
//! A `workspace.toml` sits at the root of a multi-plan project and lists each
//! component plan with optional inter-plan dependencies.
//!
//! ```toml
//! [workspace]
//! name = "my-platform"
//! description = "Multi-service platform workspace"
//!
//! [[plans]]
//! name = "shared-lib"
//! path = "libs/shared/plan.toml"
//!
//! [[plans]]
//! name = "api-service"
//! path = "services/api/plan.toml"
//! depends_on = ["shared-lib"]
//!
//! [[plans]]
//! name = "worker"
//! path = "services/worker/plan.toml"
//! depends_on = ["shared-lib"]
//! ```

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Result, WiggumError};

// ─── Workspace model ──────────────────────────────────────────────────────────

/// Top-level workspace descriptor, parsed from `workspace.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    pub workspace: WorkspaceMeta,
    pub plans: Vec<PlanRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceMeta {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// A reference to a component plan within the workspace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanRef {
    /// Short identifier used in `depends_on` references (e.g. `"api-service"`).
    pub name: String,
    /// Relative path to the plan TOML (relative to the workspace.toml directory).
    pub path: String,
    /// Names of other `PlanRef.name` values this plan depends on.
    #[serde(default)]
    pub depends_on: Vec<String>,
}

// ─── Resolved workspace ───────────────────────────────────────────────────────

/// A plan reference with its resolved filesystem path.
#[derive(Debug, Clone)]
pub struct ResolvedPlanRef {
    pub name: String,
    pub path: PathBuf,
    pub depends_on: Vec<String>,
}

/// Resolved workspace with topologically sorted execution order.
#[derive(Debug)]
pub struct ResolvedWorkspace {
    pub meta: WorkspaceMeta,
    /// Plans in topological order — safe to generate/execute in this order.
    pub plans: Vec<ResolvedPlanRef>,
}

// ─── Parsing ──────────────────────────────────────────────────────────────────

impl Workspace {
    /// Parse a workspace from TOML text.
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML is malformed or missing required fields.
    pub fn from_toml(content: &str) -> Result<Self> {
        toml::from_str(content).map_err(|e| WiggumError::PlanParse(e.to_string()))
    }

    /// Resolve all plan paths relative to the workspace file's directory and
    /// produce a topologically sorted `ResolvedWorkspace`.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - A plan name is duplicated.
    /// - A `depends_on` references an unknown plan name.
    /// - The inter-plan dependency graph contains a cycle.
    pub fn resolve(&self, workspace_dir: &Path) -> Result<ResolvedWorkspace> {
        // Build a map of name → PlanRef for validation.
        let mut name_set: HashSet<&str> = HashSet::new();
        for p in &self.plans {
            if !name_set.insert(p.name.as_str()) {
                return Err(WiggumError::DuplicateSlug(p.name.clone()));
            }
        }

        // Validate all depends_on references.
        for p in &self.plans {
            for dep in &p.depends_on {
                if !name_set.contains(dep.as_str()) {
                    return Err(WiggumError::UnknownDependency {
                        referenced: dep.clone(),
                        referencing: p.name.clone(),
                    });
                }
            }
        }

        // Topological sort of the inter-plan graph.
        let sorted_names = toposort_plans(&self.plans)?;

        // Build resolved list in sorted order.
        let plan_map: HashMap<&str, &PlanRef> =
            self.plans.iter().map(|p| (p.name.as_str(), p)).collect();
        let plans = sorted_names
            .into_iter()
            .filter_map(|name| plan_map.get(name.as_str()))
            .map(|p| ResolvedPlanRef {
                name: p.name.clone(),
                path: workspace_dir.join(&p.path),
                depends_on: p.depends_on.clone(),
            })
            .collect();

        Ok(ResolvedWorkspace {
            meta: self.workspace.clone(),
            plans,
        })
    }
}

/// Topologically sort plan names based on `depends_on` edges.
///
/// Returns plan names in safe execution order.
fn toposort_plans(plans: &[PlanRef]) -> Result<Vec<String>> {
    use petgraph::algo::toposort;
    use petgraph::graph::DiGraph;

    let mut graph = DiGraph::<&str, ()>::new();
    let mut node_map: HashMap<&str, petgraph::graph::NodeIndex> = HashMap::new();

    for p in plans {
        let idx = graph.add_node(p.name.as_str());
        node_map.insert(p.name.as_str(), idx);
    }

    for p in plans {
        let Some(&p_node) = node_map.get(p.name.as_str()) else {
            continue;
        };
        for dep in &p.depends_on {
            if let Some(&dep_node) = node_map.get(dep.as_str()) {
                // Edge: dep must run before p.
                graph.add_edge(dep_node, p_node, ());
            }
        }
    }

    toposort(&graph, None)
        .map(|sorted| {
            sorted
                .into_iter()
                .filter_map(|idx| graph.node_weight(idx).map(|s| (*s).to_string()))
                .collect()
        })
        .map_err(|e| {
            let node = graph.node_weight(e.node_id()).copied().unwrap_or("unknown");
            WiggumError::CycleDetected(format!("inter-plan cycle involves: {node}"))
        })
}

/// Generate a default workspace.toml skeleton string for a given directory.
#[must_use]
pub fn skeleton_toml(workspace_name: &str, plan_dirs: &[(&str, &str)]) -> String {
    let mut lines = vec![
        "[workspace]".to_string(),
        format!("name = \"{workspace_name}\""),
        format!("description = \"Multi-plan workspace for {workspace_name}\""),
        String::new(),
    ];

    for (name, path) in plan_dirs {
        lines.push("[[plans]]".to_string());
        lines.push(format!("name = \"{name}\""));
        lines.push(format!("path = \"{path}\""));
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Generate a skeleton `plan.toml` string for a new project.
///
/// The caller supplies the project name, a description, the primary language,
/// and an ordered list of task slugs. Goals and hints are left as placeholders.
#[must_use]
pub fn skeleton_plan_toml(
    project_name: &str,
    description: &str,
    language: &str,
    task_slugs: &[String],
) -> String {
    let mut lines = vec![
        "[project]".to_string(),
        format!("name = \"{}\"", toml_escape(project_name)),
        format!("description = \"{}\"", toml_escape(description)),
        format!("language = \"{}\"", toml_escape(language)),
        format!("path = \"./{}\"", toml_escape(project_name)),
        String::new(),
        "[orchestrator]".to_string(),
        "persona = \"You are an expert software engineer. Follow all instructions carefully.\""
            .to_string(),
        "strategy = \"sequential\"".to_string(),
        String::new(),
        "[[phases]]".to_string(),
        "name = \"Phase 1 — Foundation\"".to_string(),
        "order = 1".to_string(),
        String::new(),
    ];

    for (i, slug) in task_slugs.iter().enumerate() {
        let title = slug
            .split('-')
            .map(|w| {
                let mut c = w.chars();
                c.next().map_or_else(String::new, |f| {
                    f.to_uppercase().collect::<String>() + c.as_str()
                })
            })
            .collect::<Vec<_>>()
            .join(" ");
        let num = i + 1;
        lines.push("[[tasks]]".to_string());
        lines.push(format!("slug = \"{}\"", toml_escape(slug)));
        lines.push(format!("title = \"T{num:02} — {}\"", toml_escape(&title)));
        lines.push("phase = \"Phase 1 — Foundation\"".to_string());
        lines.push("goal = \"TODO: describe what this task should accomplish\"".to_string());
        lines.push("hints = []".to_string());
        lines.push(String::new());
    }

    lines.join("\n")
}

/// Escape a string for inclusion in a TOML basic string literal (double-quoted).
///
/// Escapes `\` and `"` per the TOML spec, and renders ASCII control characters
/// as `\uXXXX` sequences so the output always round-trips through a TOML parser.
fn toml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\x00'..='\x1F' | '\x7F' => {
                let _ = std::fmt::Write::write_fmt(
                    &mut out,
                    format_args!("\\u{:04X}", ch as u32),
                );
            }
            _ => out.push(ch),
        }
    }
    out
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    const EXAMPLE_TOML: &str = r#"
[workspace]
name = "my-platform"
description = "Test workspace"

[[plans]]
name = "shared-lib"
path = "libs/shared/plan.toml"

[[plans]]
name = "api"
path = "services/api/plan.toml"
depends_on = ["shared-lib"]

[[plans]]
name = "worker"
path = "services/worker/plan.toml"
depends_on = ["shared-lib"]
"#;

    #[test]
    fn parse_workspace_toml() {
        let ws = Workspace::from_toml(EXAMPLE_TOML).expect("parse failed");
        assert_eq!(ws.workspace.name, "my-platform");
        assert_eq!(ws.plans.len(), 3);
        assert_eq!(ws.plans[1].name, "api");
        assert_eq!(ws.plans[1].depends_on, vec!["shared-lib"]);
    }

    #[test]
    fn resolve_workspace_topo_order() {
        let ws = Workspace::from_toml(EXAMPLE_TOML).expect("parse failed");
        let resolved = ws.resolve(Path::new("/tmp")).expect("resolve failed");
        // shared-lib must come before api and worker
        let names: Vec<&str> = resolved.plans.iter().map(|p| p.name.as_str()).collect();
        let shared_pos = names.iter().position(|n| *n == "shared-lib").unwrap();
        let api_pos = names.iter().position(|n| *n == "api").unwrap();
        let worker_pos = names.iter().position(|n| *n == "worker").unwrap();
        assert!(shared_pos < api_pos);
        assert!(shared_pos < worker_pos);
    }

    #[test]
    fn reject_duplicate_plan_names() {
        let toml = r#"
[workspace]
name = "bad"

[[plans]]
name = "foo"
path = "a/plan.toml"

[[plans]]
name = "foo"
path = "b/plan.toml"
"#;
        let ws = Workspace::from_toml(toml).expect("parse failed");
        let err = ws.resolve(Path::new("/tmp")).unwrap_err();
        assert!(matches!(err, WiggumError::DuplicateSlug(_)));
    }

    #[test]
    fn reject_unknown_dependency() {
        let toml = r#"
[workspace]
name = "bad"

[[plans]]
name = "foo"
path = "a/plan.toml"
depends_on = ["nonexistent"]
"#;
        let ws = Workspace::from_toml(toml).expect("parse failed");
        let err = ws.resolve(Path::new("/tmp")).unwrap_err();
        assert!(matches!(err, WiggumError::UnknownDependency { .. }));
    }

    #[test]
    fn reject_cycle_in_plans() {
        let toml = r#"
[workspace]
name = "cyclic"

[[plans]]
name = "a"
path = "a/plan.toml"
depends_on = ["b"]

[[plans]]
name = "b"
path = "b/plan.toml"
depends_on = ["a"]
"#;
        let ws = Workspace::from_toml(toml).expect("parse failed");
        let err = ws.resolve(Path::new("/tmp")).unwrap_err();
        assert!(matches!(err, WiggumError::CycleDetected(_)));
    }

    #[test]
    fn skeleton_toml_generation() {
        let toml = skeleton_toml(
            "my-app",
            &[("lib", "lib/plan.toml"), ("app", "app/plan.toml")],
        );
        assert!(toml.contains("my-app"));
        assert!(toml.contains("[[plans]]"));
        assert!(toml.contains("lib/plan.toml"));
    }
}
