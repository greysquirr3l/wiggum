use tera::{Context, Tera};

use crate::domain::plan::{Plan, ResolvedTask};
use crate::error::{Result, WiggumError};
use crate::generation::templates::get_tera;

/// Render the progress tracker using the default template.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render(plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    render_with(get_tera(), plan, tasks)
}

/// Render the progress tracker using a custom Tera instance.
///
/// # Errors
///
/// Returns an error if template rendering fails.
pub fn render_with(tera: &Tera, plan: &Plan, tasks: &[ResolvedTask]) -> Result<String> {
    let mut ctx = Context::new();

    ctx.insert("project_name", &plan.project.name);

    // Group tasks by phase
    let mut phases: Vec<serde_json::Value> = Vec::new();
    let mut current_phase: Option<(String, u32, Vec<serde_json::Value>)> = None;

    for task in tasks {
        let phase_key = (task.phase_name.clone(), task.phase_order);

        if matches!(&current_phase, Some((name, order, _)) if *name != phase_key.0 || *order != phase_key.1)
            && let Some((name, order, phase_tasks)) = current_phase.take()
        {
            let depends_on_desc = if order == 1 {
                String::new()
            } else {
                format!("Phase {} all complete", order - 1)
            };
            phases.push(serde_json::json!({
                "name": name,
                "order": order,
                "depends_on_desc": depends_on_desc,
                "tasks": phase_tasks,
            }));
        }

        let task_val = serde_json::json!({
            "number_padded": format!("{:02}", task.number),
            "title": task.title,
        });

        if let Some((_, _, ref mut phase_tasks)) = current_phase {
            phase_tasks.push(task_val);
        } else {
            current_phase = Some((task.phase_name.clone(), task.phase_order, vec![task_val]));
        }
    }

    // Flush last phase
    if let Some((name, order, phase_tasks)) = current_phase {
        let depends_on_desc = if order == 1 {
            String::new()
        } else {
            format!("Phase {} all complete", order - 1)
        };
        phases.push(serde_json::json!({
            "name": name,
            "order": order,
            "depends_on_desc": depends_on_desc,
            "tasks": phase_tasks,
        }));
    }

    ctx.insert("phases", &phases);

    tera.render("progress.md", &ctx)
        .map_err(|e| WiggumError::Template(e.to_string()))
}
