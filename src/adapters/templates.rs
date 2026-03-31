//! Templates command implementation — reusable task snippets.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::adapters::fs::FsAdapter;
use crate::domain::plan::{Plan, TaskDef};
use crate::error::{Result, WiggumError};
use crate::ports::PlanReader;

/// A task template stored in `~/.wiggum/templates/`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTemplate {
    pub slug: String,
    pub title: String,
    pub goal: String,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub test_hints: Vec<String>,
    #[serde(default)]
    pub must_haves: Vec<String>,
    #[serde(default)]
    pub evaluation_criteria: Vec<String>,
}

impl From<&TaskDef> for TaskTemplate {
    fn from(task: &TaskDef) -> Self {
        Self {
            slug: task.slug.clone(),
            title: task.title.clone(),
            goal: task.goal.clone(),
            hints: task.hints.clone(),
            test_hints: task.test_hints.clone(),
            must_haves: task.must_haves.clone(),
            evaluation_criteria: task.evaluation_criteria.clone(),
        }
    }
}

impl From<TaskTemplate> for TaskDef {
    fn from(tmpl: TaskTemplate) -> Self {
        Self {
            slug: tmpl.slug,
            title: tmpl.title,
            goal: tmpl.goal,
            depends_on: vec![],
            hints: tmpl.hints,
            test_hints: tmpl.test_hints,
            must_haves: tmpl.must_haves,
            gate: None,
            evaluation_criteria: tmpl.evaluation_criteria,
        }
    }
}

/// Get the templates directory path.
#[must_use]
pub fn templates_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".wiggum")
        .join("templates")
}

/// Ensure the templates directory exists.
///
/// # Errors
///
/// Returns an error if the directory cannot be created.
pub fn ensure_templates_dir() -> Result<PathBuf> {
    let dir = templates_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// List all available templates.
///
/// # Errors
///
/// Returns an error if the templates directory cannot be read.
pub fn list_templates() -> Result<Vec<TemplateInfo>> {
    let dir = templates_dir();

    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut templates = Vec::new();

    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().is_some_and(|e| e == "toml")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
            && let Ok(content) = std::fs::read_to_string(&path)
            && let Ok(tmpl) = toml::from_str::<TaskTemplate>(&content)
        {
            templates.push(TemplateInfo {
                name: name.to_string(),
                title: tmpl.title,
                path,
            });
        }
    }

    templates.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(templates)
}

/// Information about an available template.
#[derive(Debug)]
pub struct TemplateInfo {
    pub name: String,
    pub title: String,
    pub path: PathBuf,
}

/// Load a template by name.
///
/// # Errors
///
/// Returns an error if the template cannot be found or parsed.
pub fn load_template(name: &str) -> Result<TaskTemplate> {
    let path = templates_dir().join(format!("{name}.toml"));

    if !path.exists() {
        return Err(WiggumError::Validation(format!(
            "Template '{name}' not found. Run `wiggum templates list` to see available templates."
        )));
    }

    let content = std::fs::read_to_string(&path)?;
    toml::from_str(&content).map_err(|e| WiggumError::PlanParse(e.to_string()))
}

/// Save a task as a template.
///
/// # Errors
///
/// Returns an error if the task cannot be found or the template cannot be written.
pub fn save_template(
    plan_path: &Path,
    task_slug: &str,
    template_name: Option<&str>,
) -> Result<PathBuf> {
    let fs = FsAdapter;
    let toml_content = fs.read_plan(plan_path)?;
    let plan = Plan::from_toml(&toml_content)?;

    // Find the task
    let task_def = plan
        .phases
        .iter()
        .flat_map(|p| &p.tasks)
        .find(|t| t.slug == task_slug)
        .ok_or_else(|| WiggumError::Validation(format!("Task '{task_slug}' not found")))?;

    let template: TaskTemplate = task_def.into();
    let name = template_name.unwrap_or(task_slug);

    let dir = ensure_templates_dir()?;
    let path = dir.join(format!("{name}.toml"));

    let toml_str =
        toml::to_string_pretty(&template).map_err(|e| WiggumError::PlanParse(e.to_string()))?;

    std::fs::write(&path, toml_str)?;

    Ok(path)
}

/// Format template list for display.
#[must_use]
pub fn format_template_list(templates: &[TemplateInfo]) -> String {
    if templates.is_empty() {
        return format!(
            "No templates found.\n\nCreate templates with:\n  wiggum templates save --plan plan.toml --task <slug>\n\nTemplates directory: {}",
            templates_dir().display()
        );
    }

    let mut lines = vec![format!("Available templates ({}):\n", templates.len())];

    for tmpl in templates {
        lines.push(format!("  {:<24} {}", tmpl.name, tmpl.title));
    }

    lines.push(String::new());
    lines.push(format!(
        "Templates directory: {}",
        templates_dir().display()
    ));
    lines.push("Use `wiggum templates show <name>` for details.".to_string());

    lines.join("\n")
}

/// Format a single template for display.
#[must_use]
pub fn format_template_show(template: &TaskTemplate) -> String {
    let mut lines = Vec::new();

    lines.push(format!("Template: {}\n", template.slug));
    lines.push(format!("Title: {}", template.title));
    lines.push(format!("Goal: {}", template.goal));

    if !template.hints.is_empty() {
        lines.push(String::new());
        lines.push("Hints:".to_string());
        for hint in &template.hints {
            lines.push(format!("  - {hint}"));
        }
    }

    if !template.test_hints.is_empty() {
        lines.push(String::new());
        lines.push("Test hints:".to_string());
        for hint in &template.test_hints {
            lines.push(format!("  - {hint}"));
        }
    }

    if !template.must_haves.is_empty() {
        lines.push(String::new());
        lines.push("Must-haves:".to_string());
        for mh in &template.must_haves {
            lines.push(format!("  - {mh}"));
        }
    }

    lines.join("\n")
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;

    #[test]
    fn template_roundtrip() {
        let template = TaskTemplate {
            slug: "test-task".into(),
            title: "Test Task".into(),
            goal: "Test the roundtrip".into(),
            hints: vec!["Hint 1".into()],
            test_hints: vec![],
            must_haves: vec![],
            evaluation_criteria: vec![],
        };

        let toml_str = toml::to_string_pretty(&template).expect("should serialize");
        let parsed: TaskTemplate = toml::from_str(&toml_str).expect("should deserialize");

        assert_eq!(parsed.slug, template.slug);
        assert_eq!(parsed.title, template.title);
    }
}
