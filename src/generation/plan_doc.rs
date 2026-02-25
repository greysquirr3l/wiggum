use tera::{Context, Tera};

use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the implementation plan document using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    render_with(get_tera(), plan, tasks)
}

/// Render the implementation plan document using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);
    ctx.insert("project_description", &plan.project.description);
    ctx.insert("language", &plan.project.language.to_string());
    ctx.insert("architecture", &plan.project.architecture);
    ctx.insert("preflight_build", &plan.preflight.build);
    ctx.insert("preflight_test", &plan.preflight.test);
    ctx.insert("preflight_lint", &plan.preflight.lint);

    // Group tasks by phase
    let mut phases: Vec<serde_json::Value> = Vec::new();
    let mut current_phase: Option<(String, u32, Vec<serde_json::Value>)> = None;

    for task in tasks {
        let phase_key = (task.phase_name.clone(), task.phase_order);

        if matches!(&current_phase, Some((name, order, _)) if *name != phase_key.0 || *order != phase_key.1)
            && let Some((name, order, phase_tasks)) = current_phase.take()
        {
            phases.push(serde_json::json!({
                "name": name,
                "order": order,
                "tasks": phase_tasks,
            }));
        }

        let depends_on_list = if task.depends_on.is_empty() {
            String::new()
        } else {
            task.depends_on.join(", ")
        };

        let task_val = serde_json::json!({
            "number_padded": format!("{:02}", task.number),
            "title": task.title,
            "goal": task.goal,
            "depends_on_list": depends_on_list,
        });

        if let Some((_, _, ref mut phase_tasks)) = current_phase {
            phase_tasks.push(task_val);
        } else {
            current_phase = Some((task.phase_name.clone(), task.phase_order, vec![task_val]));
        }
    }

    if let Some((name, order, phase_tasks)) = current_phase {
        phases.push(serde_json::json!({
            "name": name,
            "order": order,
            "tasks": phase_tasks,
        }));
    }

    ctx.insert("phases", &phases);

    tera.render("plan_doc.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}
